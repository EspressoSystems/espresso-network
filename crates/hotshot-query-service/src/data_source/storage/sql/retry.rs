use std::{cmp::min, future::Future, time::Duration};

use rand::Rng;
use tokio::time::sleep;

/// Retry policy for transactions aborted by PostgreSQL serialization conflicts (SQLSTATE 40001).
#[derive(Clone, Copy, Debug)]
pub struct SerializableRetryConfig {
    /// Initial delay before the first retry.
    base: Duration,
    /// Maximum delay between retries.
    max: Duration,
    /// Multiplier applied to the delay between successive retries.
    factor: u32,
    /// Backoff jitter as a `(numerator, denominator)` ratio of the delay.
    jitter: (u64, u64),
    /// Maximum number of retries before giving up.
    retry_max: u32,
    /// When set, each retried conflict spawns a background `pg_stat_activity` / `pg_locks` snapshot.
    pub(crate) pg_stat_diag: bool,
}

impl Default for SerializableRetryConfig {
    fn default() -> Self {
        Self {
            base: Duration::from_millis(10),
            max: Duration::from_millis(500),
            factor: 2,
            jitter: (5, 10),
            retry_max: 100,
            pg_stat_diag: false,
        }
    }
}

impl SerializableRetryConfig {
    /// Construct a retry policy. `jitter` is a `(numerator, denominator)` ratio of the delay.
    pub const fn new(
        base: Duration,
        max: Duration,
        factor: u32,
        jitter: (u64, u64),
        retry_max: u32,
        pg_stat_diag: bool,
    ) -> Self {
        Self {
            base,
            max,
            factor,
            jitter,
            retry_max,
            pg_stat_diag,
        }
    }

    /// Run `f`, retrying (up to `retry_max` times with exponential backoff + jitter) whenever
    /// `should_retry` returns `true` for the error. `f` is re-run from scratch on each attempt.
    pub(crate) async fn retry_if<F, Fut, T, E>(
        &self,
        op: &'static str,
        mut should_retry: impl FnMut(&E) -> bool,
        f: F,
    ) -> Result<T, E>
    where
        F: Fn() -> Fut,
        Fut: Future<Output = Result<T, E>>,
    {
        let mut delay = self.base;
        for i in 0..=self.retry_max {
            match f().await {
                Ok(res) => return Ok(res),
                Err(err) if i < self.retry_max && should_retry(&err) => {
                    tracing::warn!(
                        op,
                        attempt = i + 1,
                        max_retries = self.retry_max,
                        delay_ms = delay.as_millis(),
                        "serialization conflict, retrying transaction after {delay:?}"
                    );
                    sleep(delay).await;
                    delay = self.backoff(delay);
                },
                Err(err) => return Err(err),
            }
        }
        unreachable!()
    }

    /// Compute the next backoff delay: `delay * factor` plus random jitter, capped at `max`.
    fn backoff(&self, delay: Duration) -> Duration {
        if delay >= self.max {
            return self.max;
        }
        let ms = delay
            .saturating_mul(self.factor)
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let (jitter_num, jitter_den) = self.jitter;
        let jitter = if jitter_num == 0 || jitter_den == 0 {
            0
        } else {
            let mut rng = rand::thread_rng();
            ms * rng.gen_range(0..jitter_num) / jitter_den
        };
        min(Duration::from_millis(ms + jitter), self.max)
    }
}

/// Returns `true` if `err`'s [`Display`](std::fmt::Display) output identifies it as a PostgreSQL
/// serialization conflict (SQLSTATE 40001, message "could not serialize access").
pub(crate) fn is_serialization_conflict_err<E: std::fmt::Display>(err: &E) -> bool {
    format!("{err:#}").contains("could not serialize access")
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicU32, Ordering},
    };

    use super::{Duration, SerializableRetryConfig, is_serialization_conflict_err};

    /// A retry policy with small delays and a low retry cap, so the exhaustion test runs quickly.
    const TEST_RETRY: SerializableRetryConfig = SerializableRetryConfig::new(
        Duration::from_millis(1),
        Duration::from_millis(5),
        2,
        (5, 10),
        5,
        false,
    );

    /// An error whose message matches a PostgreSQL serialization conflict.
    fn mock_serialization_error() -> anyhow::Error {
        anyhow::anyhow!(
            "could not serialize access due to read/write dependencies among transactions"
        )
    }

    #[test]
    fn test_is_serialization_conflict_err() {
        // The PostgreSQL serialization-conflict message must be recognised.
        assert!(is_serialization_conflict_err(&mock_serialization_error()));
        // Other database errors must NOT match.
        assert!(!is_serialization_conflict_err(&anyhow::anyhow!(
            "duplicate key value violates unique constraint"
        )));
        // Non-database errors must not match.
        assert!(!is_serialization_conflict_err(&anyhow::anyhow!(
            "plain error"
        )));
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_retry_if_succeeds_immediately() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        let result: anyhow::Result<()> = TEST_RETRY
            .retry_if("test", is_serialization_conflict_err, || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_retry_if_retries_on_serialization_error() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        // The closure fails twice with a serialization error, then succeeds on the third attempt.
        let result: anyhow::Result<()> = TEST_RETRY
            .retry_if("test", is_serialization_conflict_err, || {
                let calls = calls_clone.clone();
                async move {
                    let n = calls.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        Err(mock_serialization_error())
                    } else {
                        Ok(())
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(calls.load(Ordering::SeqCst), 3);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_retry_if_exhausts_retries() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        // The closure always fails; retry must give up after TEST_RETRY.retry_max (5) retries.
        let result: anyhow::Result<()> = TEST_RETRY
            .retry_if("test", is_serialization_conflict_err, || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(mock_serialization_error())
                }
            })
            .await;

        assert!(result.is_err());
        // 1 initial attempt + 5 retries = 6 total calls.
        assert_eq!(calls.load(Ordering::SeqCst), 6);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_retry_if_no_retry_on_other_errors() {
        let calls = Arc::new(AtomicU32::new(0));
        let calls_clone = calls.clone();

        // Non-serialization errors must not be retried.
        let result: anyhow::Result<()> = TEST_RETRY
            .retry_if("test", is_serialization_conflict_err, || {
                let calls = calls_clone.clone();
                async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Err(anyhow::anyhow!("unrelated error"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    /// Verify that [`function_name!`](crate::function_name) resolves to the unqualified name of the
    /// enclosing function, both from a nested helper and from the test fn itself.
    #[test]
    fn test_function_name_macro() {
        fn outer_test_fn() -> &'static str {
            crate::function_name!()
        }
        assert_eq!(outer_test_fn(), "outer_test_fn");
        assert_eq!(crate::function_name!(), "test_function_name_macro");

        // Non-ASCII identifiers must not panic: the macro slices the `type_name` output, and
        // a multi-byte codepoint near a slice boundary would panic if the macro used fixed
        // byte offsets instead of char-boundary-safe operations.
        fn télécharger() -> &'static str {
            crate::function_name!()
        }
        assert_eq!(télécharger(), "télécharger");
    }

    /// Verify that [`function_name!`](crate::function_name) resolves to the function name even from
    /// `async` contexts, where the body is lowered into a generator/closure. This is the case that
    /// every production call site hits, and a naive macro reports `{{closure}}` here.
    #[test_log::test(tokio::test)]
    async fn test_function_name_macro_async() {
        // Plain `async fn`: the body becomes a generator, adding a `{{closure}}` path segment.
        async fn plain_async_fn() -> &'static str {
            crate::function_name!()
        }
        assert_eq!(plain_async_fn().await, "plain_async_fn");

        // Closure returning an async block inside an `async fn`, mirroring the real
        // `serializable_retry!(self, || async { .. })` call sites: adds multiple `{{closure}}`
        // segments to the path.
        async fn nested_async_blocks() -> &'static str {
            let f = || async { crate::function_name!() };
            f().await
        }
        assert_eq!(nested_async_blocks().await, "nested_async_blocks");

        // `#[async_trait]` method: the body is rewritten to `Box::pin(async move { .. })`, exactly
        // as the real `serializable_retry!` call sites are.
        struct S;
        #[async_trait::async_trait]
        trait T {
            async fn async_trait_method(&self) -> &'static str;
        }
        #[async_trait::async_trait]
        impl T for S {
            async fn async_trait_method(&self) -> &'static str {
                crate::function_name!()
            }
        }
        assert_eq!(S.async_trait_method().await, "async_trait_method");
    }
}
