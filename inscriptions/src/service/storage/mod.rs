use std::num::NonZero;
pub mod in_memory;
pub mod postgres;

use super::espresso_inscription::{EspressoInscription, InscriptionAndChainDetails};

/// [RecordPendingPutInscriptionError] is an error that occurs when attempting
/// to record a pending put inscription.
#[derive(Debug)]
pub enum RecordPendingPutInscriptionError {
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for RecordPendingPutInscriptionError {
    fn from(error: sqlx::Error) -> Self {
        RecordPendingPutInscriptionError::SqlxError(error)
    }
}

/// [ResolvePendingPutInscriptionError] is an error that occurs when attempting
/// to resolve a pending put inscription.
#[derive(Debug)]
pub enum ResolvePendingPutInscriptionError {
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for ResolvePendingPutInscriptionError {
    fn from(error: sqlx::Error) -> Self {
        ResolvePendingPutInscriptionError::SqlxError(error)
    }
}

/// [RetrievePendingPutInscriptionsError] is an error that occurs when
/// attempting to retrieve all pending put inscriptions.
#[derive(Debug)]
pub enum RetrievePendingPutInscriptionsError {
    SqlxError(sqlx::Error),
    TryFromSliceError(core::array::TryFromSliceError),
    FromHexError(const_hex::FromHexError),
}

impl From<sqlx::Error> for RetrievePendingPutInscriptionsError {
    fn from(error: sqlx::Error) -> Self {
        RetrievePendingPutInscriptionsError::SqlxError(error)
    }
}

impl From<std::array::TryFromSliceError> for RetrievePendingPutInscriptionsError {
    fn from(error: std::array::TryFromSliceError) -> Self {
        RetrievePendingPutInscriptionsError::TryFromSliceError(error)
    }
}

impl From<const_hex::FromHexError> for RetrievePendingPutInscriptionsError {
    fn from(error: const_hex::FromHexError) -> Self {
        RetrievePendingPutInscriptionsError::FromHexError(error)
    }
}

/// [RecordConfirmedInscriptionAndChainDetailsError] is an error that occurs
/// when attempting to record a confirmed inscription and chain details.
#[derive(Debug)]
pub enum RecordConfirmedInscriptionAndChainDetailsError {
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for RecordConfirmedInscriptionAndChainDetailsError {
    fn from(error: sqlx::Error) -> Self {
        RecordConfirmedInscriptionAndChainDetailsError::SqlxError(error)
    }
}

/// [RetrieveLatestInscriptionAndChainDetailsError] is an error that occurs
/// when attempting to retrieve the latest inscriptions and chain details.
#[derive(Debug)]
pub enum RetrieveLatestInscriptionAndChainDetailsError {
    SqlxError(sqlx::Error),
    TryFromSliceError(core::array::TryFromSliceError),
    FromHexError(const_hex::FromHexError),
}

impl From<sqlx::Error> for RetrieveLatestInscriptionAndChainDetailsError {
    fn from(error: sqlx::Error) -> Self {
        RetrieveLatestInscriptionAndChainDetailsError::SqlxError(error)
    }
}

impl From<std::array::TryFromSliceError> for RetrieveLatestInscriptionAndChainDetailsError {
    fn from(error: std::array::TryFromSliceError) -> Self {
        RetrieveLatestInscriptionAndChainDetailsError::TryFromSliceError(error)
    }
}

impl From<const_hex::FromHexError> for RetrieveLatestInscriptionAndChainDetailsError {
    fn from(error: const_hex::FromHexError) -> Self {
        RetrieveLatestInscriptionAndChainDetailsError::FromHexError(error)
    }
}

/// [RecordLastReceivedBlockError] is an error that occurs when attempting to
/// record the last received block.
#[derive(Debug)]
pub enum RecordLastReceivedBlockError {
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for RecordLastReceivedBlockError {
    fn from(error: sqlx::Error) -> Self {
        RecordLastReceivedBlockError::SqlxError(error)
    }
}

/// [RetrieveLastReceivedBlockError] is an error that occurs when attempting to
/// retrieve the last received block.
#[derive(Debug)]
pub enum RetrieveLastReceivedBlockError {
    SqlxError(sqlx::Error),
}

impl From<sqlx::Error> for RetrieveLastReceivedBlockError {
    fn from(error: sqlx::Error) -> Self {
        RetrieveLastReceivedBlockError::SqlxError(error)
    }
}

/// [InscriptionPersistence] is a trait that governs the parts of data that we
/// need to store in order to minimize any loss of inscription data due to a
/// service restart.
///
/// This trait is predominately concerned with the persistence of the
/// inscriptions service's state. As such the information being stored and
/// retrieved is only concerned with restoring and populating the service's
/// state.
///
/// There may be some elements for the archival of inscriptions that are not
/// explicitly covered by this trait, but can be implied by it.
#[async_trait::async_trait]
pub trait InscriptionPersistence {
    /// [record_pending_put_inscription] records a pending put inscription.
    /// This is used to store inscriptions that are waiting to be submitted to
    /// the Espresso Block Chain in order to ensure we do not miss any
    /// submissions.
    async fn record_pending_put_inscription(
        &self,
        inscription: &EspressoInscription,
    ) -> Result<(), RecordPendingPutInscriptionError>;

    /// [record_submit_put_inscription] resolves a pending put inscription.
    /// This is used to clean the pending inscription from being considered
    /// again in the future.
    async fn record_submit_put_inscription(
        &self,
        inscription: &EspressoInscription,
    ) -> Result<(), ResolvePendingPutInscriptionError>;

    /// [retrieve_pending_put_inscriptions] retrieves the pending put
    /// inscriptions from the stor that have not yet been resolved.
    async fn retrieve_pending_put_inscriptions(
        &self,
    ) -> Result<Vec<EspressoInscription>, RetrievePendingPutInscriptionsError>;

    /// [record_confirmed_inscription_and_chain_details] records a confirmed
    /// inscription and block details.  This is used to store the confirmed
    /// inscriptions that have been submitted to the Espresso Block Chain and
    /// have been received back.
    async fn record_confirmed_inscription_and_chain_details(
        &self,
        inscription_and_block_details: &InscriptionAndChainDetails,
    ) -> Result<(), RecordConfirmedInscriptionAndChainDetailsError>;

    /// [retrieve_latest_inscription_and_chain_details] retrieves the latest
    /// inscriptions and chain details from the store.  This is used to retrieve
    /// the latest inscriptions that have been confirmed by the Espresso Block
    /// Chain.
    ///
    /// This is used during bootstrap in order to quickly repopulate the
    /// inscription list that is stored in memory.
    async fn retrieve_latest_inscription_and_chain_details(
        &self,
        number_of_inscriptions: NonZero<usize>,
    ) -> Result<Vec<InscriptionAndChainDetails>, RetrieveLatestInscriptionAndChainDetailsError>;

    /// [record_last_received_block] records the last received block from the
    /// Espresso Block Chain.  This is used to store the last block that has been
    /// received from the Espresso Block Chain.
    ///
    /// This is useful for ensuring that we do not reprocess previously missed
    /// blocks.
    async fn record_last_received_block(
        &self,
        block: u64,
    ) -> Result<(), RecordLastReceivedBlockError>;

    /// [retrieve_last_received_block] retrieves the last received block height
    /// from the Espresso Block Chain.  This is used to help bootstrap the
    /// block stream to ensure that we do not miss processing any blocks.
    async fn retrieve_last_received_block(&self) -> Result<u64, RetrieveLastReceivedBlockError>;
}
