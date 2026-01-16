use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use anyhow::Result;
use tokio::sync::Semaphore;

/// Execute async tasks concurrently with bounded parallelism.
///
/// Logs progress every 100 items and at completion with the given prefix.
/// Returns results in the same order as inputs.
pub async fn map_concurrent<T, R, F, Fut>(
    prefix: &str,
    items: impl IntoIterator<Item = T>,
    concurrency: usize,
    f: F,
) -> Result<Vec<R>>
where
    T: Send + 'static,
    R: Send + 'static,
    F: Fn(T) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<R>> + Send,
{
    let items: Vec<T> = items.into_iter().collect();
    let total = items.len();
    let semaphore = Arc::new(Semaphore::new(concurrency));
    let completed = Arc::new(AtomicUsize::new(0));
    let f = Arc::new(f);
    let prefix = prefix.to_string();

    let handles: Vec<_> = items
        .into_iter()
        .map(|item| {
            let sem = semaphore.clone();
            let f = f.clone();
            let completed = completed.clone();
            let prefix = prefix.clone();
            tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                let result = f(item).await;
                let count = completed.fetch_add(1, Ordering::Relaxed) + 1;
                if count.is_multiple_of(100) || count == total {
                    tracing::info!("{}: {}/{}", prefix, count, total);
                }
                result
            })
        })
        .collect();

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        results.push(handle.await??);
    }
    Ok(results)
}
