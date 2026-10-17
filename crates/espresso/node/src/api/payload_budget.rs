//! Admission control for handlers that load block payloads into memory.
//!
//! `large_object_range_limit` bounds how many blocks a request may cover, not how many bytes. A
//! [`PayloadBudget`] bounds the payload bytes loaded at once across all requests. A request
//! larger than the whole budget is served alone, so no block or range is ever unservable.
//!
//! Sizes are the ones the header's namespace table declares, so nothing is loaded before a
//! request is admitted. The permit is held until the response has been sent when the request
//! comes through the axum router (see [`espresso_api::hold_until_response_dropped`]), otherwise
//! until the handler returns. Either way it covers the loaded payloads and, on the axum path,
//! their serialized copy.

use std::{ops::Range, sync::Arc, time::Duration};

use espresso_api::{error::AvailabilityError, hold_until_response_dropped};
use espresso_types::SeqTypes;
use futures::StreamExt as _;
use hotshot_query_service::availability::{AvailabilityDataSource, BlockId};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

use super::state::FETCH_TIMEOUT;

/// Permits are KiB, so the budget fits the `u32` of `acquire_many`.
const UNIT: u64 = 1024;

/// Payload bytes a node loads at once, unless configured.
pub const DEFAULT_BUDGET_BYTES: u64 = 512 * 1024 * 1024;

/// How long a single-block request waits for budget before it is turned away.
const DEFAULT_WAIT: Duration = Duration::from_secs(5);

#[derive(Clone, Debug)]
pub struct PayloadBudget {
    semaphore: Arc<Semaphore>,
    capacity: u32,
    wait: Duration,
}

impl Default for PayloadBudget {
    fn default() -> Self {
        Self::new(DEFAULT_BUDGET_BYTES)
    }
}

impl PayloadBudget {
    pub fn new(bytes: u64) -> Self {
        let capacity = units(bytes);
        Self {
            semaphore: Arc::new(Semaphore::new(capacity as usize)),
            capacity,
            wait: DEFAULT_WAIT,
        }
    }

    pub fn with_wait(mut self, wait: Duration) -> Self {
        self.wait = wait;
        self
    }

    /// Wait up to the budget's wait for `bytes`. For single blocks, which are small and so are
    /// worth a short queue.
    pub async fn acquire(&self, bytes: u64) -> anyhow::Result<OwnedSemaphorePermit> {
        let acquire = self
            .semaphore
            .clone()
            .acquire_many_owned(self.permits(bytes));
        match tokio::time::timeout(self.wait, acquire).await {
            Ok(permit) => Ok(permit.expect("semaphore is never closed")),
            Err(_) => Err(self.overloaded(bytes)),
        }
    }

    /// Take `bytes` now or fail. For ranges: a queued range would hold up the single blocks
    /// queued behind it.
    pub fn try_acquire(&self, bytes: u64) -> anyhow::Result<OwnedSemaphorePermit> {
        self.semaphore
            .clone()
            .try_acquire_many_owned(self.permits(bytes))
            .map_err(|_| self.overloaded(bytes))
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
            .acquire(header.ns_table().payload_byte_len() as u64)
            .await?;
        Ok(hold_until_response_dropped(permit))
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

    /// Admit a range request, failing at once if its bytes are not available.
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
        let permit = self.try_acquire(bytes)?;
        Ok(hold_until_response_dropped(permit))
    }

    /// Permits for `bytes`, capped at the whole budget so a request larger than it takes all of
    /// it and is served when nothing else is in flight.
    fn permits(&self, bytes: u64) -> u32 {
        units(bytes).min(self.capacity)
    }

    fn overloaded(&self, bytes: u64) -> anyhow::Error {
        AvailabilityError::Overloaded {
            message: format!("payload memory budget exhausted, {bytes} bytes not available"),
            retry_after_secs: self.wait.as_secs().max(1),
        }
        .into()
    }
}

fn units(bytes: u64) -> u32 {
    u32::try_from(bytes.div_ceil(UNIT).max(1)).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KIB: u64 = 1024;

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
    async fn rejects_when_budget_is_held() {
        let budget = budget(100);
        let _held = budget.acquire(60 * KIB).await.unwrap();

        let err = budget.acquire(60 * KIB).await.unwrap_err();

        assert!(is_overloaded(&err), "{err}");
        let _admitted = budget.acquire(40 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn waiter_is_admitted_when_budget_is_released() {
        let budget = PayloadBudget::new(100 * KIB).with_wait(Duration::from_secs(30));
        let held = budget.acquire(100 * KIB).await.unwrap();
        let mut waiter = Box::pin(budget.acquire(100 * KIB));

        // Still blocked while the budget is held.
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
    async fn range_is_rejected_at_once_when_budget_is_insufficient() {
        let budget = PayloadBudget::new(100 * KIB).with_wait(Duration::from_secs(30));
        let _held = budget.acquire(60 * KIB).await.unwrap();

        let err = budget.try_acquire(60 * KIB).unwrap_err();

        assert!(is_overloaded(&err), "{err}");
    }

    #[tokio::test]
    async fn single_block_is_admitted_while_a_range_holds_part_of_the_budget() {
        let budget = budget(100);
        let _range = budget.try_acquire(60 * KIB).unwrap();

        let _admitted = budget.acquire(40 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn request_larger_than_budget_takes_all_of_it() {
        let budget = budget(10);

        let permit = budget.try_acquire(1000 * KIB).unwrap();

        assert!(is_overloaded(&budget.acquire(1).await.unwrap_err()));
        drop(permit);
        let _again = budget.acquire(1000 * KIB).await.unwrap();
    }

    #[tokio::test]
    async fn empty_payload_still_takes_a_permit() {
        let budget = budget(1);
        let _held = budget.acquire(0).await.unwrap();

        assert!(is_overloaded(&budget.acquire(0).await.unwrap_err()));
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
