//! Admission control for handlers that load block payloads into memory.
//!
//! `large_object_range_limit` bounds how many blocks a request may cover, not how many bytes. A
//! [`PayloadBudget`] bounds the estimated memory of payload responses built at once across all
//! requests: payload bytes times a weight for the response encoding, [`JSON_WEIGHT`] or
//! [`BINARY_WEIGHT`]. Single blocks and ranges draw on separate pools, half the budget each, so a
//! stream of ranges cannot starve single-block requests or the reverse. A request larger than its
//! pool takes the whole pool and is served alone, so no block or range is ever unservable, and a
//! pool's memory is bounded by the larger of the pool and that one response.
//!
//! Payload sizes are the ones the header's namespace table declares, so nothing is loaded before
//! a request is admitted. Through the axum router the permit is held until the handler has built
//! its response (see [`espresso_api::hold_until_response_built`]), so it covers the loaded
//! payloads and their serialized copy but not the write to the client. Otherwise it is held until
//! the handler returns.

use std::{ops::Range, sync::Arc, time::Duration};

use espresso_api::{error::AvailabilityError, hold_until_response_built};
use espresso_types::SeqTypes;
use futures::StreamExt as _;
use hotshot_query_service::availability::{AvailabilityDataSource, BlockId};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::state::FETCH_TIMEOUT;

/// Permits are KiB, so a pool fits the `u32` of `acquire_many`.
const UNIT: u64 = 1024;

/// Estimated response memory, payload bytes times the encoding weight, a node holds at once,
/// unless configured.
pub const DEFAULT_BUDGET_BYTES: u64 = 512 * 1024 * 1024;

/// How long a request waits for budget before it is turned away.
const DEFAULT_WAIT: Duration = Duration::from_secs(5);

/// Peak memory of a JSON response per payload byte: the payload, its base64 `String` (1.33x),
/// and the output buffer, which grows by doubling.
const JSON_WEIGHT: u64 = 4;

/// Peak memory of a VBS response per payload byte: the payload and the output `Vec`, which grows
/// by doubling.
const BINARY_WEIGHT: u64 = 2;

#[derive(Clone, Debug)]
pub struct PayloadBudget {
    blocks: Pool,
    ranges: Pool,
    wait: Duration,
}

#[derive(Clone, Debug)]
struct Pool {
    semaphore: Arc<Semaphore>,
    capacity: u32,
}

impl Pool {
    fn new(capacity: u32) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(capacity as usize)),
            capacity,
        }
    }

    /// Wait up to `wait` for `bytes`, capped at the whole pool.
    async fn acquire(&self, bytes: u64, wait: Duration) -> Option<OwnedSemaphorePermit> {
        let permits = units(bytes).min(self.capacity);
        let acquire = self.semaphore.clone().acquire_many_owned(permits);
        let permit = tokio::time::timeout(wait, acquire).await.ok()?;
        Some(permit.expect("semaphore is never closed"))
    }
}

impl Default for PayloadBudget {
    fn default() -> Self {
        Self::new(DEFAULT_BUDGET_BYTES)
    }
}

impl PayloadBudget {
    /// A budget of `bytes` in total, split evenly between the single-block and range pools.
    pub fn new(bytes: u64) -> Self {
        let pool = || Pool::new((units(bytes) / 2).max(1));
        Self {
            blocks: pool(),
            ranges: pool(),
            wait: DEFAULT_WAIT,
        }
    }

    pub fn with_wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    pub async fn acquire_block(&self, bytes: u64) -> anyhow::Result<OwnedSemaphorePermit> {
        self.blocks
            .acquire(bytes, self.wait)
            .await
            .ok_or_else(|| self.overloaded(bytes))
    }

    pub async fn acquire_ranges(&self, bytes: u64) -> anyhow::Result<OwnedSemaphorePermit> {
        self.ranges
            .acquire(bytes, self.wait)
            .await
            .ok_or_else(|| self.overloaded(bytes))
    }

    /// Admit a single block, waiting for budget if needed.
    pub async fn admit_block<S>(
        &self,
        ds: &S,
        id: BlockId<SeqTypes>,
    ) -> anyhow::Result<Option<OwnedSemaphorePermit>>
    where
        S: AvailabilityDataSource<SeqTypes> + ?Sized,
    {
        let header = ds
            .get_header(id)
            .await
            .with_timeout(FETCH_TIMEOUT)
            .await
            .ok_or_else(|| AvailabilityError::NotFound(format!("header not found for {id}")))?;
        let permit = self
            .acquire_block(header.ns_table().payload_byte_len() as u64 * response_weight())
            .await?;
        Ok(hold_until_response_built(permit))
    }

    pub async fn admit_range<S>(
        &self,
        ds: &S,
        range: Range<usize>,
    ) -> anyhow::Result<Option<OwnedSemaphorePermit>>
    where
        S: AvailabilityDataSource<SeqTypes> + ?Sized,
    {
        self.admit_ranges(ds, [range]).await
    }

    /// Admit a range request, waiting for budget in the range pool if needed.
    ///
    /// Returns the permit unless the response will hold it.
    pub async fn admit_ranges<S>(
        &self,
        ds: &S,
        ranges: impl IntoIterator<Item = Range<usize>>,
    ) -> anyhow::Result<Option<OwnedSemaphorePermit>>
    where
        S: AvailabilityDataSource<SeqTypes> + ?Sized,
    {
        let mut bytes = 0;
        for range in ranges {
            let mut headers = ds.get_header_range(range.clone()).await.enumerate();
            while let Some((i, fetch)) = headers.next().await {
                let header = fetch.with_timeout(FETCH_TIMEOUT).await.ok_or_else(|| {
                    AvailabilityError::NotFound(format!("header {} not found", range.start + i))
                })?;
                bytes += header.ns_table().payload_byte_len() as u64;
            }
        }
        let permit = self.acquire_ranges(bytes * response_weight()).await?;
        Ok(hold_until_response_built(permit))
    }

    fn overloaded(&self, bytes: u64) -> anyhow::Error {
        AvailabilityError::Overloaded {
            message: format!(
                "payload memory budget exhausted, {bytes} bytes of response memory not available"
            ),
            retry_after_secs: self.wait.as_secs().max(1),
        }
        .into()
    }
}

/// Bytes of memory a response costs per payload byte, by the format it is encoded in.
fn response_weight() -> u64 {
    if espresso_api::response_is_binary() {
        BINARY_WEIGHT
    } else {
        JSON_WEIGHT
    }
}

fn units(bytes: u64) -> u32 {
    u32::try_from(bytes.div_ceil(UNIT).max(1)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIB: u64 = 1024;

    /// Pools of `kib / 2` KiB each.
    fn budget(kib: u64) -> PayloadBudget {
        PayloadBudget::new(kib * KIB).with_wait(Duration::from_millis(50))
    }

    fn is_overloaded(err: &anyhow::Error) -> bool {
        matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::Overloaded { .. })
        )
    }

    #[tokio::test]
    async fn rejects_when_pool_is_held() {
        let budget = budget(200);
        let _held = budget.acquire_block(60 * KIB).await.unwrap();

        let err = budget.acquire_block(60 * KIB).await.unwrap_err();

        assert!(is_overloaded(&err), "{err}");
        let _admitted = budget.acquire_block(40 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn waiter_is_admitted_when_budget_is_released() {
        let budget = PayloadBudget::new(200 * KIB).with_wait(Duration::from_secs(30));
        let held = budget.acquire_ranges(100 * KIB).await.unwrap();
        let mut waiter = Box::pin(budget.acquire_ranges(100 * KIB));

        // Still blocked while the pool is held.
        tokio::time::timeout(Duration::from_millis(100), &mut waiter)
            .await
            .unwrap_err();
        drop(held);

        let _permit = tokio::time::timeout(Duration::from_secs(5), waiter)
            .await
            .expect("waiter admitted once budget is released")
            .unwrap();
    }

    #[tokio::test]
    async fn ranges_are_admitted_while_single_blocks_saturate_their_pool() {
        let budget = budget(200);
        let _blocks = budget.acquire_block(100 * KIB).await.unwrap();
        assert!(is_overloaded(&budget.acquire_block(1).await.unwrap_err()));

        let _ranges = budget.acquire_ranges(100 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn single_blocks_are_admitted_while_ranges_saturate_their_pool() {
        let budget = budget(200);
        let _ranges = budget.acquire_ranges(100 * KIB).await.unwrap();
        assert!(is_overloaded(&budget.acquire_ranges(1).await.unwrap_err()));

        let _blocks = budget.acquire_block(100 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn request_larger_than_its_pool_takes_the_whole_pool() {
        let budget = budget(20);

        let permit = budget.acquire_ranges(1000 * KIB).await.unwrap();

        assert!(is_overloaded(&budget.acquire_ranges(1).await.unwrap_err()));
        drop(permit);
        let _again = budget.acquire_ranges(1000 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn empty_payload_still_takes_a_permit() {
        let budget = budget(2);
        let _held = budget.acquire_block(0).await.unwrap();

        assert!(is_overloaded(&budget.acquire_block(0).await.unwrap_err()));
    }

    #[test]
    fn responses_outside_a_negotiated_route_are_weighted_as_json() {
        assert_eq!(response_weight(), JSON_WEIGHT);
    }

    #[test]
    fn retry_after_is_the_wait() {
        let err = PayloadBudget::new(KIB)
            .with_wait(Duration::from_secs(5))
            .overloaded(1);

        assert!(matches!(
            err.downcast_ref::<AvailabilityError>(),
            Some(AvailabilityError::Overloaded {
                retry_after_secs: 5,
                ..
            })
        ));
    }
}
