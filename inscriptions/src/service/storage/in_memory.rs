use std::{
    num::NonZero,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::service::espresso_inscription::{EspressoInscription, InscriptionAndChainDetails};

use super::{
    InscriptionPersistence, RecordConfirmedInscriptionAndChainDetailsError,
    RecordLastReceivedBlockError, RecordPendingPutInscriptionError,
    ResolvePendingPutInscriptionError, RetrieveLastReceivedBlockError,
    RetrieveLatestInscriptionAndChainDetailsError, RetrievePendingPutInscriptionsError,
};

/// [HeightCachingInMemory] is a wrapper around an [InscriptionPersistence] that
/// caches the last received block height in memory for quick retrieval without
/// having to continually query the underlying storage.
pub struct HeightCachingInMemory<Persistence> {
    persistence: Persistence,
    last_received_block: AtomicU64,
}

impl<Persistence> HeightCachingInMemory<Persistence> {
    /// [new] creates a new [HeightCachingInMemory] with the given storage.
    pub fn new(storage: Persistence) -> Self {
        Self {
            persistence: storage,
            last_received_block: AtomicU64::new(0),
        }
    }
}

#[async_trait::async_trait]
impl<Storage> InscriptionPersistence for HeightCachingInMemory<Storage>
where
    Storage: InscriptionPersistence + Send + Sync,
{
    async fn record_pending_put_inscription(
        &self,
        inscription: &EspressoInscription,
    ) -> Result<(), RecordPendingPutInscriptionError> {
        self.persistence
            .record_pending_put_inscription(inscription)
            .await
    }

    async fn record_submit_put_inscription(
        &self,
        inscription: &EspressoInscription,
    ) -> Result<(), ResolvePendingPutInscriptionError> {
        self.persistence
            .record_submit_put_inscription(inscription)
            .await
    }

    async fn retrieve_pending_put_inscriptions(
        &self,
    ) -> Result<Vec<EspressoInscription>, RetrievePendingPutInscriptionsError> {
        self.persistence.retrieve_pending_put_inscriptions().await
    }

    async fn record_confirmed_inscription_and_chain_details(
        &self,
        inscription_and_block_details: &InscriptionAndChainDetails,
    ) -> Result<(), RecordConfirmedInscriptionAndChainDetailsError> {
        self.persistence
            .record_confirmed_inscription_and_chain_details(inscription_and_block_details)
            .await
    }

    async fn retrieve_latest_inscription_and_chain_details(
        &self,
        number_of_inscriptions: NonZero<usize>,
    ) -> Result<Vec<InscriptionAndChainDetails>, RetrieveLatestInscriptionAndChainDetailsError>
    {
        self.persistence
            .retrieve_latest_inscription_and_chain_details(number_of_inscriptions)
            .await
    }

    async fn record_last_received_block(
        &self,
        block: u64,
    ) -> Result<(), RecordLastReceivedBlockError> {
        let result = self.persistence.record_last_received_block(block).await;

        if result.is_ok() {
            self.last_received_block.store(block, Ordering::SeqCst);
        }

        result
    }

    async fn retrieve_last_received_block(&self) -> Result<u64, RetrieveLastReceivedBlockError> {
        if self.last_received_block.load(Ordering::SeqCst) != 0 {
            return Ok(self.last_received_block.load(Ordering::SeqCst));
        }

        // fallback to the underlying storage for boot-strapping
        let block_height = self.persistence.retrieve_last_received_block().await?;
        self.last_received_block
            .store(block_height, Ordering::SeqCst);

        Ok(block_height)
    }
}
