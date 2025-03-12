// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

use std::future::Future;

use async_broadcast::{Receiver, RecvError};
use futures::{
    future::BoxFuture,
    stream::{FuturesUnordered, StreamExt},
    FutureExt,
};

/// Type which describes the idea of waiting for a dependency to complete
pub trait Dependency<T> {
    /// Complete will wait until it gets some value `T` then return the value
    fn completed(self) -> impl Future<Output = Option<T>> + Send;
    /// Create an or dependency from this dependency and another
    fn or<D: Dependency<T> + Send + 'static>(self, dep: D) -> OrDependency<T>
    where
        T: Send + Sync + Clone + 'static,
        Self: Sized + Send + 'static,
    {
        let mut or = OrDependency::from_deps(vec![self]);
        or.add_dep(dep);
        or
    }
    /// Create an and dependency from this dependency and another
    fn and<D: Dependency<T> + Send + 'static>(self, dep: D) -> AndDependency<T>
    where
        T: Send + Sync + Clone + 'static,
        Self: Sized + Send + 'static,
    {
        let mut and = AndDependency::from_deps(vec![self]);
        and.add_dep(dep);
        and
    }
}

/// Defines a dependency that completes when all of its deps complete
pub struct AndDependency<T> {
    /// Dependencies being combined
    deps: Vec<BoxFuture<'static, Option<T>>>,
}
impl<T: Clone + Send + Sync> Dependency<Vec<T>> for AndDependency<T> {
    /// Returns a vector of all of the results from it's dependencies.
    /// The results will be in a random order
    async fn completed(self) -> Option<Vec<T>> {
        let futures = FuturesUnordered::from_iter(self.deps);
        futures
            .collect::<Vec<Option<T>>>()
            .await
            .into_iter()
            .collect()
    }
}

impl<T: Clone + Send + Sync + 'static> AndDependency<T> {
    /// Create from a vec of deps
    #[must_use]
    pub fn from_deps(deps: Vec<impl Dependency<T> + Send + 'static>) -> Self {
        let mut pinned = vec![];
        for dep in deps {
            pinned.push(dep.completed().boxed());
        }
        Self { deps: pinned }
    }
    /// Add another dependency
    pub fn add_dep(&mut self, dep: impl Dependency<T> + Send + 'static) {
        self.deps.push(dep.completed().boxed());
    }
    /// Add multiple dependencies
    pub fn add_deps(&mut self, deps: AndDependency<T>) {
        for dep in deps.deps {
            self.deps.push(dep);
        }
    }
}

/// Defines a dependency that completes when one of it's dependencies completes
pub struct OrDependency<T> {
    /// Dependencies being combined
    deps: Vec<BoxFuture<'static, Option<T>>>,
}
impl<T: Clone + Send + Sync> Dependency<T> for OrDependency<T> {
    /// Returns the value of the first completed dependency
    async fn completed(self) -> Option<T> {
        let mut futures = FuturesUnordered::from_iter(self.deps);
        loop {
            if let Some(maybe) = futures.next().await {
                if maybe.is_some() {
                    return maybe;
                }
            } else {
                return None;
            }
        }
    }
}

impl<T: Clone + Send + Sync + 'static> OrDependency<T> {
    /// Creat an `OrDependency` from a vec of dependencies
    #[must_use]
    pub fn from_deps(deps: Vec<impl Dependency<T> + Send + 'static>) -> Self {
        let mut pinned = vec![];
        for dep in deps {
            pinned.push(dep.completed().boxed());
        }
        Self { deps: pinned }
    }
    /// Add another dependency
    pub fn add_dep(&mut self, dep: impl Dependency<T> + Send + 'static) {
        self.deps.push(dep.completed().boxed());
    }
}

/// A dependency that listens on a channel for an event
/// that matches what some value it wants.
pub struct EventDependency<T: Clone + Send + Sync> {
    /// Channel of incoming events
    pub(crate) event_rx: Receiver<T>,

    /// Closure which returns true if the incoming `T` is the
    /// thing that completes this dependency
    pub(crate) match_fn: Box<dyn Fn(&T) -> bool + Send>,

    /// The potentially externally completed dependency. If the dependency was seeded from an event
    /// message, we can mark it as already done in lieu of other events still pending.
    completed_dependency: Option<T>,
}

impl<T: Clone + Send + Sync + 'static> EventDependency<T> {
    /// Create a new `EventDependency`
    #[must_use]
    pub fn new(receiver: Receiver<T>, match_fn: Box<dyn Fn(&T) -> bool + Send>) -> Self {
        Self {
            event_rx: receiver,
            match_fn: Box::new(match_fn),
            completed_dependency: None,
        }
    }

    /// Mark a dependency as completed.
    pub fn mark_as_completed(&mut self, dependency: T) {
        self.completed_dependency = Some(dependency);
    }
}

impl<T: Clone + Send + Sync + 'static> Dependency<T> for EventDependency<T> {
    async fn completed(mut self) -> Option<T> {
        if let Some(dependency) = self.completed_dependency {
            return Some(dependency);
        }
        loop {
            if let Some(dependency) = self.completed_dependency {
                return Some(dependency);
            }

            match self.event_rx.recv_direct().await {
                Ok(event) => {
                    if (self.match_fn)(&event) {
                        return Some(event);
                    }
                },
                Err(RecvError::Overflowed(n)) => {
                    tracing::error!("Dependency Task overloaded, skipping {} events", n);
                },
                Err(RecvError::Closed) => {
                    return None;
                },
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use async_broadcast::{broadcast, Receiver};

    use super::{AndDependency, Dependency, EventDependency, OrDependency};

    fn eq_dep(rx: Receiver<usize>, val: usize) -> EventDependency<usize> {
        EventDependency {
            event_rx: rx,
            match_fn: Box::new(move |v| *v == val),
            completed_dependency: None,
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn it_works() {
        let (tx, rx) = broadcast(10);

        let mut deps = vec![];
        for i in 0..5 {
            tx.broadcast(i).await.unwrap();
            deps.push(eq_dep(rx.clone(), 5));
        }

        let and = AndDependency::from_deps(deps);
        tx.broadcast(5).await.unwrap();
        let result = and.completed().await;
        assert_eq!(result, Some(vec![5; 5]));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn or_dep() {
        let (tx, rx) = broadcast(10);

        tx.broadcast(5).await.unwrap();
        let mut deps = vec![];
        for _ in 0..5 {
            deps.push(eq_dep(rx.clone(), 5));
        }
        let or = OrDependency::from_deps(deps);
        let result = or.completed().await;
        assert_eq!(result, Some(5));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn and_or_dep() {
        let (tx, rx) = broadcast(10);

        tx.broadcast(1).await.unwrap();
        tx.broadcast(2).await.unwrap();
        tx.broadcast(3).await.unwrap();
        tx.broadcast(5).await.unwrap();
        tx.broadcast(6).await.unwrap();

        let or1 = OrDependency::from_deps([eq_dep(rx.clone(), 4), eq_dep(rx.clone(), 6)].into());
        let or2 = OrDependency::from_deps([eq_dep(rx.clone(), 4), eq_dep(rx.clone(), 5)].into());
        let and = AndDependency::from_deps([or1, or2].into());
        let result = and.completed().await;
        assert_eq!(result, Some(vec![6, 5]));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn or_and_dep() {
        let (tx, rx) = broadcast(10);

        tx.broadcast(1).await.unwrap();
        tx.broadcast(2).await.unwrap();
        tx.broadcast(3).await.unwrap();
        tx.broadcast(4).await.unwrap();
        tx.broadcast(5).await.unwrap();

        let and1 = eq_dep(rx.clone(), 4).and(eq_dep(rx.clone(), 6));
        let and2 = eq_dep(rx.clone(), 4).and(eq_dep(rx.clone(), 5));
        let or = and1.or(and2);
        let result = or.completed().await;
        assert_eq!(result, Some(vec![4, 5]));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn many_and_dep() {
        let (tx, rx) = broadcast(10);

        tx.broadcast(1).await.unwrap();
        tx.broadcast(2).await.unwrap();
        tx.broadcast(3).await.unwrap();
        tx.broadcast(4).await.unwrap();
        tx.broadcast(5).await.unwrap();
        tx.broadcast(6).await.unwrap();

        let mut and1 = eq_dep(rx.clone(), 4).and(eq_dep(rx.clone(), 6));
        let and2 = eq_dep(rx.clone(), 4).and(eq_dep(rx.clone(), 5));
        and1.add_deps(and2);
        let result = and1.completed().await;
        assert_eq!(result, Some(vec![4, 6, 4, 5]));
    }
}
