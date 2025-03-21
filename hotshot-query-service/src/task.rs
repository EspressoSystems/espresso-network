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

//! Async task utilities.

use std::{fmt::Display, sync::Arc};

use derivative::Derivative;
use futures::future::Future;
use tokio::{
    spawn,
    task::{JoinError, JoinHandle},
};
use tracing::{info_span, Instrument};

/// A background task which is cancelled on [`Drop`]
///
/// This handle can be cloned; cloning it does not clone the underlying task. There may be many
/// handles to the same background task, and the task will be cancelled when all handles are
/// dropped.
#[derive(Clone, Debug)]
pub struct BackgroundTask {
    // A handle to the inner task. This exists solely so that we can hold it and have it be dropped
    // when the last clone of this object is dropped.
    _inner: Arc<Task<()>>,
}

impl BackgroundTask {
    /// Spawn a background task, which will be cancelled when every clone is dropped.
    ///
    /// The caller should ensure that `future` yields back to the executor fairly frequently, to
    /// ensure timely cancellation in case the task is dropped. If an operation in `future` may run
    /// for a long time without blocking or yielding, consider using
    /// [`yield_now`](async_std::task::yield_now) periodically, or using [`spawn`] or
    /// [`spawn_blocking`](async_std::task::spawn_blocking) to run long operations in a sub-task.
    pub fn spawn<F>(name: impl Display, future: F) -> Self
    where
        F: Future + Send + 'static,
    {
        // Ignore the output of the background task.
        let future = async move {
            future.await;
        };
        Self {
            _inner: Arc::new(Task::spawn(name, future)),
        }
    }
}

#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
struct TaskInner<T: Send> {
    name: String,
    #[derivative(Debug = "ignore")]
    handle: JoinHandle<T>,
}

/// A task handle which can be joined, but is cancelled on [`Drop`]
#[derive(Derivative)]
#[derivative(Debug(bound = ""))]
pub struct Task<T: Send + 'static> {
    // The task handle is an `Option` so we can `take()` out of it during `join` and `drop`. This
    // will always be `Some` except during joining or cancellation.
    inner: Option<TaskInner<T>>,
}

impl<T: Send + 'static> Task<T> {
    /// Spawn a task, which will be cancelled when dropped.
    ///
    /// The caller should ensure that `future` yields back to the executor fairly frequently, to
    /// ensure timely cancellation in case the task is dropped. If an operation in `future` may run
    /// for a long time without blocking or yielding, consider using
    /// [`yield_now`](async_std::task::yield_now) periodically, or using
    /// [`spawn`] or [`spawn_blocking`](async_std::task::spawn_blocking) to run long operations in a
    /// sub-task.
    pub fn spawn<F>(name: impl Display, future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
    {
        let name = name.to_string();
        let handle = {
            let span = info_span!("task", name);
            spawn(
                async move {
                    tracing::info!("spawning task");
                    let res = future.await;
                    tracing::info!("completed task");
                    res
                }
                .instrument(span),
            )
        };

        Self {
            inner: Some(TaskInner { name, handle }),
        }
    }

    /// Wait for the task to complete and get its output.
    pub async fn join(mut self) -> Result<T, JoinError> {
        // We take here so that we will not attempt to cancel the joined task when this handle is
        // dropped at the end of the function. We can unwrap here because `inner` is only `None`
        // during `join` or `drop`. Since `join` consumes `self`, it is not possible that `join`
        // already ran, and of course `self` has not been dropped yet.
        let inner = self.inner.take().unwrap();
        inner.handle.await
    }
}

impl<T: Send + 'static> Drop for Task<T> {
    fn drop(&mut self) {
        if let Some(inner) = self.inner.take() {
            tracing::info!(name = inner.name, "cancelling task");
            inner.handle.abort();
            tracing::info!(name = inner.name, "cancelled task");
        }
    }
}
