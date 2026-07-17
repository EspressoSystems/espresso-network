use std::{future::Future, sync::Arc};

use anyhow::Result;
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
