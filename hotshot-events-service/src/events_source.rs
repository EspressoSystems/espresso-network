use std::{marker::PhantomData, sync::Arc};

use async_broadcast::{broadcast, InactiveReceiver, Sender as BroadcastSender};
use async_trait::async_trait;
use futures::{
    future::BoxFuture,
    stream::{BoxStream, Stream, StreamExt},
};
use hotshot_types::{
    event::{Event, EventType, LegacyEvent},
    traits::node_implementation::NodeType,
    PeerConfig,
};
use serde::{Deserialize, Serialize};
use tide_disco::method::ReadState;
const RETAINED_EVENTS_COUNT: usize = 4096;

#[async_trait]
pub trait EventsSource<Types>
where
    Types: NodeType,
{
    type EventStream: Stream<Item = Arc<Event<Types>>> + Unpin + Send + 'static;
    type LegacyEventStream: Stream<Item = Arc<LegacyEvent<Types>>> + Unpin + Send + 'static;
    async fn get_event_stream(&self, filter: Option<EventFilterSet<Types>>) -> Self::EventStream;
    async fn get_legacy_event_stream(
        &self,
        filter: Option<EventFilterSet<Types>>,
    ) -> Self::LegacyEventStream;
    async fn get_startup_info(&self) -> StartupInfo<Types>;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(bound = "Types::SignatureKey: for<'a> Deserialize<'a>")]
pub struct StartupInfo<Types: NodeType> {
    pub known_node_with_stake: Vec<PeerConfig<Types>>,
    pub non_staked_node_count: usize,
}

#[async_trait]
pub trait EventConsumer<Types>
where
    Types: NodeType,
{
    async fn handle_event(&mut self, event: Event<Types>);
}

#[derive(Debug)]
pub struct EventsStreamer<Types: NodeType> {
    // required for api subscription
    inactive_to_subscribe_clone_recv: InactiveReceiver<Arc<Event<Types>>>,
    subscriber_send_channel: BroadcastSender<Arc<Event<Types>>>,

    // required for sending startup info
    known_nodes_with_stake: Vec<PeerConfig<Types>>,
    non_staked_node_count: usize,
}

impl<Types: NodeType> EventsStreamer<Types> {
    pub fn known_node_with_stake(&self) -> Vec<PeerConfig<Types>> {
        self.known_nodes_with_stake.clone()
    }

    pub fn non_staked_node_count(&self) -> usize {
        self.non_staked_node_count
    }
}

#[async_trait]
impl<Types: NodeType> EventConsumer<Types> for EventsStreamer<Types> {
    async fn handle_event(&mut self, event: Event<Types>) {
        if let Err(e) = self.subscriber_send_channel.broadcast(event.into()).await {
            tracing::debug!("Error broadcasting the event: {:?}", e);
        }
    }
}

/// Wrapper struct representing a set of event filters.
#[derive(Clone, Debug)]
pub struct EventFilterSet<Types: NodeType>(pub(crate) Vec<EventFilter<Types>>);

/// `From` trait impl to create an `EventFilterSet` from a vector of `EventFilter`s.
impl<Types: NodeType> From<Vec<EventFilter<Types>>> for EventFilterSet<Types> {
    fn from(filters: Vec<EventFilter<Types>>) -> Self {
        EventFilterSet(filters)
    }
}

/// `From` trait impl to create an `EventFilterSet` from a single `EventFilter`.
impl<Types: NodeType> From<EventFilter<Types>> for EventFilterSet<Types> {
    fn from(filter: EventFilter<Types>) -> Self {
        EventFilterSet(vec![filter])
    }
}

impl<Types: NodeType> EventFilterSet<Types> {
    /// Determines whether the given hotshot event should be broadcast based on the filters in the set.
    ///
    ///  Returns `true` if the event should be broadcast, `false` otherwise.
    pub(crate) fn should_broadcast(&self, hotshot_event: &EventType<Types>) -> bool {
        let filter = &self.0;

        match hotshot_event {
            EventType::Error { .. } => filter.contains(&EventFilter::Error),
            EventType::Decide { .. } => filter.contains(&EventFilter::Decide),
            EventType::ReplicaViewTimeout { .. } => {
                filter.contains(&EventFilter::ReplicaViewTimeout)
            },
            EventType::ViewFinished { .. } => filter.contains(&EventFilter::ViewFinished),
            EventType::ViewTimeout { .. } => filter.contains(&EventFilter::ViewTimeout),
            EventType::Transactions { .. } => filter.contains(&EventFilter::Transactions),
            EventType::DaProposal { .. } => filter.contains(&EventFilter::DaProposal),
            EventType::QuorumProposal { .. } => filter.contains(&EventFilter::QuorumProposal),
            EventType::UpgradeProposal { .. } => filter.contains(&EventFilter::UpgradeProposal),
            _ => false,
        }
    }
}

/// Possible event filters
/// If the hotshot`EventType` enum is modified, this enum should also be updated
#[derive(Clone, Debug, PartialEq)]
pub enum EventFilter<Types: NodeType> {
    Error,
    Decide,
    ReplicaViewTimeout,
    ViewFinished,
    ViewTimeout,
    Transactions,
    DaProposal,
    QuorumProposal,
    UpgradeProposal,
    Pd(PhantomData<Types>),
}

#[async_trait]
impl<Types: NodeType> EventsSource<Types> for EventsStreamer<Types> {
    type EventStream = BoxStream<'static, Arc<Event<Types>>>;
    type LegacyEventStream = BoxStream<'static, Arc<LegacyEvent<Types>>>;

    async fn get_event_stream(&self, filter: Option<EventFilterSet<Types>>) -> Self::EventStream {
        let receiver = self.inactive_to_subscribe_clone_recv.activate_cloned();

        if let Some(filter) = filter {
            receiver
                .filter(move |event| {
                    futures::future::ready(filter.should_broadcast(&event.as_ref().event))
                })
                .boxed()
        } else {
            receiver.boxed()
        }
    }

    async fn get_legacy_event_stream(
        &self,
        filter: Option<EventFilterSet<Types>>,
    ) -> Self::LegacyEventStream {
        let receiver = self.inactive_to_subscribe_clone_recv.activate_cloned();

        if let Some(filter) = filter {
            receiver
                .filter(move |event| {
                    futures::future::ready(filter.should_broadcast(&event.as_ref().event))
                })
                .filter_map(|a| {
                    futures::future::ready(Event::to_legacy(a.as_ref().clone()).ok().map(Arc::new))
                })
                .boxed()
        } else {
            receiver
                .filter_map(|a| {
                    futures::future::ready(Event::to_legacy(a.as_ref().clone()).ok().map(Arc::new))
                })
                .boxed()
        }
    }

    async fn get_startup_info(&self) -> StartupInfo<Types> {
        StartupInfo {
            known_node_with_stake: self.known_node_with_stake(),
            non_staked_node_count: self.non_staked_node_count(),
        }
    }
}

impl<Types: NodeType> EventsStreamer<Types> {
    pub fn new(
        known_nodes_with_stake: Vec<PeerConfig<Types>>,
        non_staked_node_count: usize,
    ) -> Self {
        let (mut subscriber_send_channel, to_subscribe_clone_recv) =
            broadcast::<Arc<Event<Types>>>(RETAINED_EVENTS_COUNT);
        // set the overflow to true to drop older messages from the channel
        subscriber_send_channel.set_overflow(true);
        // set the await active to false to not block the sender
        subscriber_send_channel.set_await_active(false);
        let inactive_to_subscribe_clone_recv = to_subscribe_clone_recv.deactivate();

        EventsStreamer {
            subscriber_send_channel,
            inactive_to_subscribe_clone_recv,
            known_nodes_with_stake,
            non_staked_node_count,
        }
    }
}

#[async_trait]
impl<Types: NodeType> ReadState for EventsStreamer<Types> {
    type State = Self;

    async fn read<T>(
        &self,
        op: impl Send + for<'a> FnOnce(&'a Self::State) -> BoxFuture<'a, T> + 'async_trait,
    ) -> T {
        op(self).await
    }
}
