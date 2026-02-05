use std::{
    fmt::{Display, Formatter},
    future::Future,
    str::FromStr,
    sync::Arc,
};

use anyhow::{Context, Result};
use hotshot::{
    traits::implementations::{CliquenetAddress, CliquenetPublicKey},
    types::SignatureKey,
};
use tagged_base64::TaggedBase64;
use tokio::{
    sync::Semaphore,
    task::{AbortHandle, JoinError, JoinSet},
};

/// A join set that limits the number of concurrent tasks
pub struct BoundedJoinSet<T> {
    // The inner join set
    inner: JoinSet<T>,
    // The semaphore we use to limit the number of concurrent tasks
    semaphore: Arc<Semaphore>,
}

impl<T> BoundedJoinSet<T> {
    /// Create a new [`BoundedJoinSet`] with a maximum number of concurrent tasks
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            inner: JoinSet::new(),
            semaphore: Arc::new(Semaphore::const_new(max_concurrency)),
        }
    }
}

impl<T: 'static> BoundedJoinSet<T> {
    /// Spawn the provided task on the JoinSet, returning an [AbortHandle] that can be used
    /// to remotely cancel the task.
    pub fn spawn<F>(&mut self, task: F) -> AbortHandle
    where
        F: Future<Output = T> + Send + 'static,
        T: Send,
    {
        // Clone the semaphore for the inner task
        let semaphore = self.semaphore.clone();

        // Wrap the task, making it wait for a semaphore permit first
        let task = async move {
            // Acquire the permit
            let permit = semaphore.acquire().await;

            // Perform the actual task
            let result = task.await;

            // Drop the permit
            drop(permit);

            // Return the result
            result
        };

        // Spawn the task in the inner join set
        self.inner.spawn(task)
    }

    /// Waits until one of the tasks in the set completes and returns its output.
    ///
    /// Returns None if the set is empty.
    pub async fn join_next(&mut self) -> Option<Result<T, JoinError>> {
        self.inner.join_next().await
    }

    /// Waits until one of the tasks in the set completes and returns its output, along with the task ID of the completed task.
    pub async fn join_next_with_id(&mut self) -> Option<Result<(tokio::task::Id, T), JoinError>> {
        self.inner.join_next_with_id().await
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NodeAddress<K: SignatureKey> {
    /// The HotShot-compatible consensus key
    pub consensus_key: K,

    /// The Cliquenet public key
    pub cliquenet_public_key: CliquenetPublicKey,

    /// The Cliquenet address
    pub address: CliquenetAddress,
}

impl<K: SignatureKey> Display for NodeAddress<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}/{}/{}",
            self.consensus_key, self.cliquenet_public_key, self.address
        )
    }
}

impl<K: SignatureKey> FromStr for NodeAddress<K> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split the string into the consensus key, cliquenet public key, and address
        let parts = s.split('/').collect::<Vec<&str>>();
        if parts.len() != 3 {
            anyhow::bail!("invalid p2p address: {}", s);
        }

        // Parse the consensus key
        let consensus_key =
            K::try_from(&TaggedBase64::parse(parts[0]).with_context(|| "invalid consensus key")?)
                .map_err(|_| anyhow::anyhow!("invalid consensus key: {}", parts[0]))?;

        // Parse the cliquenet public key
        let cliquenet_public_key = parts[1]
            .try_into()
            .with_context(|| "invalid cliquenet public key")?;

        // Parse the address
        let address = parts[2].try_into().with_context(|| "invalid address")?;

        Ok(Self {
            consensus_key,
            cliquenet_public_key,
            address,
        })
    }
}

impl<K: SignatureKey> NodeAddress<K> {
    pub fn new(
        consensus_key: K,
        cliquenet_public_key: CliquenetPublicKey,
        address: CliquenetAddress,
    ) -> Self {
        Self {
            consensus_key,
            cliquenet_public_key,
            address,
        }
    }
}
