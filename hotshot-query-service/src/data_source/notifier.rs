// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! Multi-producer, multi-consumer oneshot notification channel
//!
//! [`Notifier`] is an asynchronous, multi-producer, multi-consumer, oneshot channel satisfying
//! three additional requirements:
//! 1. Dropped receivers do not consume resources.
//! 2. Messages are not copied for receivers who don't want them.
//! 3. Minimal resource contention for concurrent subscriptions.
//!
//! ## Dropped receivers do not consume resources
//!
//! This requirement is a direct prerequisite of the broader requirement that passive requests for
//! resources do not consume resources. This is important because in general, passive requests may
//! be for resources that are not guaranteed to exist, and thus may never terminate. Just like we
//! avoid spawning a task for passive requests, since it may never complete, we need receivers for
//! passive requests not to persist beyond the lifetime of the request, or they may never be closed.
//!
//! This requirement is implemented via garbage collection: each time a message is sent, resources
//! belonging to dropped receivers are cleaned up. Thus, strictly speaking, dropped receivers do
//! consume resources, but only briefly. There is no need to keep them around until the desired
//! message is delivered, for example.
//!
//! ## Messages are not copied for receivers who don't want them.
//!
//! The second requirement simplifies the higher level fetching logic by allowing us to maintain a
//! single channel for all notifications about a particular resource type, rather than separate
//! channels for each specific request. Since messages are not copied for all subscribers, but only
//! for the subscribers interested in a particular message, this simplification becomes nearly
//! cost-free.
//!
//! This requirement is implemented by attaching a predicate to each subscription, which takes a
//! message by reference. The predicate is checked on the sending side, and the message is only
//! copied to the subscription if the predicate is satisfied.
//!
//! ## Minimal resource contention for concurrent subscriptions.
//!
//! This is important because subscriptions are requested in response to read-only client requests,
//! which are supposed to run in parallel as much as possible. By contrast, notifications are
//! usually send from internal server tasks (e.g. the background task that updates the data source
//! when new blocks are committed). It is less of a problem if these internal tasks contend with
//! each other, because they are not directly blocking responses to clients, and we have more
//! control over how and when they acquire shared resources.
//!
//! This requirement also empowers us to create a simpler design for the high-level fetching logic.
//! Specifically, we can reuse the same code for fetching individual resources as we use for
//! long-lived subscription streams (e.g. `subscribe_blocks` is a thin wrapper around
//! `get_block_range`). We do not have to worry about adding complex logic to reuse notification
//! subscriptions for long-lived streams, because subscribing anew for each entry in the stream has
//! low overhead in terms of contention over shared resources -- the dominant caused in any
//! concurrent channel, after data copying (see above).
//!
//! This further lets us simplify the interface of this channel a bit: since all notifications are
//! oneshot, consumers deal with futures rather than streams.
//!
//! This requirement is satisfied by maintaining the list of subscribers to a [`Notifier`] in a way
//! that moves most resource contention to message senders, rather than receivers. We make the
//! assumption that there is less concurrency among senders. In the common case, there is just one
//! sender: the task monitoring HotShot for new blocks. Occasionally, there may be other tasks
//! spawned to fetch missing resources and send them through the [`Notifier`], but these should be
//! relatively few and rare.

use std::{
    future::IntoFuture,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

use async_lock::Mutex;
use derivative::Derivative;
use futures::future::{BoxFuture, FutureExt};
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};
use tracing::warn;

/// A predicate on a type `<T>`.
///
/// [`Predicate`] is an alias for any type implementing `Fn(&T) -> bool` (with a few extra bounds
/// to support concurrency). It is used by [`Notifier`] to represent the preferences of subscribers
/// when filtering messages of type `T`.
pub trait Predicate<T>: 'static + Send + Sync + Fn(&T) -> bool {}
impl<F, T> Predicate<T> for F where F: 'static + Send + Sync + Fn(&T) -> bool {}

#[derive(Derivative)]
#[derivative(Debug)]
struct Subscriber<T> {
    #[derivative(Debug = "ignore")]
    predicate: Box<dyn Predicate<T>>,
    #[derivative(Debug = "ignore")]
    sender: Option<oneshot::Sender<T>>,
    closed: Arc<AtomicBool>,
}

impl<T> Subscriber<T> {
    fn is_closed(&self) -> bool {
        // A subscriber can be closed because it has already been notified, which `take`s the
        // oneshot sender.
        self.sender.is_none() ||
        // Or because it was explicitly closed by its receiver (e.g. the receiver was dropped)
        self.closed.load(Ordering::Relaxed)
    }
}

impl<T: Clone> Subscriber<T> {
    fn notify(&mut self, msg: &T) {
        // First, check if the subscriber has been closed. If it has, we can skip it and save the
        // work of evaluating the predicate.
        if self.is_closed() {
            return;
        }
        // At this point, it is likely, but not guaranteed, that the subscriber is not closed.
        // It may have been closed the instant after the check above. However, it is harmless to
        // evaluate the predicate now; at worst we waste a bit of computation.
        if !(self.predicate)(msg) {
            return;
        }
        // Now we are committed to sending the message to this subscriber if possible. We can take
        // the sender. We need to check for closed again in case the subscriber was closed since the
        // previous check.
        if let Some(sender) = self.sender.take() {
            if sender.send(msg.clone()).is_err() {
                // This is equivalent to the previous behavior in `async-compatibility-layer`
                warn!("Failed to send notification: channel closed");
            }
        }
    }
}

/// Multi-producer, multi-consumer oneshot notification channel
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Notifier<T> {
    // Active subscribers.
    active: Mutex<Vec<Subscriber<T>>>,
    // Pending subscribers.
    //
    // When a new subscriber joins the subscriber set, they do not immediately add themselves to
    // `active`. Instead, they simply send themselves to this channel. Every time a message is sent,
    // it will drain pending subscribers from here and add them to `active`. In this way, almost all
    // of the cost is paid by senders, rather than receivers, and receivers do not contend with
    // senders. We adopt this design for
    // two reasons:
    // 1. Most basically, more messages are received than sent, since the intended use of this
    //    channel is broadcast, with each message being delivered to multiple receivers. Thus,
    //    moving cost from receivers to senders is always good.
    // 2. This channel is intended to be highly concurrent, and `active` is a shared resource that
    //    is ripe for contention. With this design, _only_ sends contend for a lock on the
    //    subscribers list, but new subscriptions do not. Again, since there are often more
    //    receivers than senders, this can reduce contention significantly.
    // Consider the example use case of `Fetcher::fetching_stream`. This design of the notification
    // channel lets us go with a very simple design of fetching stream, where we fetch each
    // subsequent entry individually, with a one-shot notification future. This pattern leads to
    // many concurrent subscription requests: each time a new entry is produced, every open stream
    // will subscribe anew to notifications for the next entry, at the same time. However, these
    // concurrent subscriptions go through the high-throughput multi-producer stream implementation,
    // and do not contend for the lock on `FilterSender::subscribers`.
    #[derivative(Debug = "ignore")]
    pending: Mutex<UnboundedReceiver<Subscriber<T>>>,
    #[derivative(Debug = "ignore")]
    subscribe: UnboundedSender<Subscriber<T>>,
}

impl<T> Notifier<T> {
    pub fn new() -> Self {
        let (subscribe, pending) = unbounded_channel();
        Self {
            active: Default::default(),
            pending: Mutex::new(pending),
            subscribe,
        }
    }
}

impl<T: Clone> Notifier<T> {
    /// Notify all subscribers whose predicate is satisfied by `msg`.
    pub async fn notify(&self, msg: &T) {
        let mut active = self.active.lock().await;

        // Try sending the message to each active subscriber.
        for subscriber in &mut *active {
            subscriber.notify(msg);
        }

        // Some subscribers may be closed, either because the receiver was dropped or because we
        // just sent it its message. Remove these from the `active` list.
        active.retain(|subscriber| !subscriber.is_closed());

        // Promote pending subscribers to active and send them the message.
        // There is no contention here since we only have one receiver. This is what
        // `async-compatibility-layer` did internally.
        let mut pending_guard = self.pending.lock().await;
        while let Ok(mut subscriber) = pending_guard.try_recv() {
            subscriber.notify(msg);
            if !subscriber.is_closed() {
                // If that message didn't satisfy the subscriber, or it was dropped, at it to the
                // active list so it will get future messages.
                active.push(subscriber);
            }
        }
        drop(pending_guard);
    }
}

impl<T> Notifier<T> {
    /// Wait for a message satisfying `predicate`.
    pub async fn wait_for(&self, predicate: impl Predicate<T>) -> WaitFor<T> {
        // Create a oneshot channel for receiving the notification.
        let (sender, receiver) = oneshot::channel();
        let sender = Some(sender);
        let closed = Arc::new(AtomicBool::new(false));

        // Create a handle which will close the subscription when dropped.
        let handle = ReceiveHandle {
            closed: closed.clone(),
        };

        // Create a subscriber with our predicate and the oneshot channel.
        let subscriber = Subscriber {
            predicate: Box::new(predicate),
            sender,
            closed,
        };

        // Add the subscriber to the channel and return it. We can ignore errors here: `send` only
        // fails when the receive end of the channel has been dropped, which means the notifier has
        // been dropped, and thus the send end of the oneshot handle has been dropped. The caller
        // will discover this when they try to await a notification and get [`None`].
        let _ = self.subscribe.send(subscriber);
        WaitFor { handle, receiver }
    }
}

/// A handle that closes a subscriber when dropped.
struct ReceiveHandle {
    closed: Arc<AtomicBool>,
}

impl Drop for ReceiveHandle {
    fn drop(&mut self) {
        self.closed.store(true, Ordering::Relaxed);
    }
}

/// A pending request for notification.
///
/// This object can be `await`ed to block until the requested notification arrives. The result is an
/// `Option<T>`, which is [`Some`] except in the case that the [`Notifier`] was dropped without ever
/// sending a satisfying message.
///
/// If [`WaitFor`] is dropped before a notification is delivered, it will automatically clean up its
/// resources in the [`Notifier`].
pub struct WaitFor<T> {
    handle: ReceiveHandle,
    receiver: oneshot::Receiver<T>,
}

impl<T> IntoFuture for WaitFor<T>
where
    T: Send + 'static,
{
    type Output = Option<T>;
    type IntoFuture = BoxFuture<'static, Self::Output>;

    fn into_future(self) -> Self::IntoFuture {
        async move {
            let res = self.receiver.await.ok();

            // Explicitly drop `handle` _after_ we're done with `receiver`. If the compiler decides
            // that it can drop `handle` earlier, we might never get a notification.
            drop(self.handle);

            res
        }
        .boxed()
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use tokio::time::timeout;

    use super::*;
    use crate::testing::setup_test;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_notify_drop() {
        setup_test();
        let n = Notifier::new();

        // Create two subscribers with different predicates.
        let wait_for_zero = n.wait_for(|i| *i == 0).await;
        let wait_for_one = n.wait_for(|i| *i == 1).await;

        // Send a message which satisfies only one of the subscribers.
        n.notify(&0).await;
        assert_eq!(wait_for_zero.await.unwrap(), 0);

        // Check that the other subscriber was not notified.
        timeout(Duration::from_secs(1), wait_for_one.into_future())
            .await
            .unwrap_err();

        // Check subscribers. The first subscriber should have been cleaned up when it was notified.
        // The second should have been closed when it was dropped without completing, but not yet
        // garbage collected.
        let active = n.active.lock().await;
        assert_eq!(active.len(), 1);
        assert!(active[0].is_closed());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_notify_active() {
        setup_test();
        let n = Notifier::new();

        // Create two subscribers.
        let s1 = n.wait_for(|i| *i == 1).await;
        let s2 = n.wait_for(|i| *i == 1).await;

        // Send a message that doesn't notify either subscriber, but just promotes them from pending
        // to active.
        n.notify(&0).await;
        // Check active subscribers.
        {
            let active = n.active.lock().await;
            assert_eq!(active.len(), 2);
            assert!(!active[0].is_closed());
            assert!(!active[1].is_closed());
        }

        // Drop one of the subscribers, then send another non-satisfying message. This should cause
        // the dropped subscriber to get garbage collected.
        drop(s2);
        n.notify(&0).await;
        {
            let active = n.active.lock().await;
            assert_eq!(active.len(), 1);
            assert!(!active[0].is_closed());
        }

        // Satisfy the final subscriber.
        n.notify(&1).await;
        assert_eq!(s1.await.unwrap(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pending_dropped() {
        setup_test();
        let n = Notifier::new();

        // Create and immediately drop a pending subscriber.
        drop(n.wait_for(|_| false).await);

        // Check that the subscriber gets garbage collected on the next notification.
        n.notify(&0).await;
        assert_eq!(n.active.lock().await.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_notifier_dropped() {
        setup_test();

        let n = Notifier::new();

        // Create an active subscriber.
        let fut1 = n.wait_for(|_| false).await;
        n.notify(&0).await;

        // Create a pending subscriber.
        let fut2 = n.wait_for(|_| false).await;

        // Drop the notifier while both kinds of subscribers are blocked.
        drop(n);
        assert_eq!(fut1.await, None);
        assert_eq!(fut2.await, None);
    }
}
