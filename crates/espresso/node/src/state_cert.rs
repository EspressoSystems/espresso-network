//! State certificate fetch errors, and when a certificate can exist

use hotshot_types::{
    data::EpochNumber,
    utils::{epoch_from_block_number, root_block_in_epoch},
};

/// Error type for state certificate fetching
#[derive(Debug, thiserror::Error)]
pub enum StateCertFetchError {
    #[error("failed to fetch state certificate: {0:#}")]
    FetchError(anyhow::Error),

    #[error("state certificate validation failed: {0:#}")]
    ValidationError(anyhow::Error),

    #[error("state certificate error: {0:#}")]
    Other(anyhow::Error),

    #[error("no state certificate for epoch {epoch}: epochs have not started")]
    EpochsNotStarted { epoch: u64 },

    #[error("no state certificate for epoch {epoch}: the first epoch is {first_epoch}")]
    BeforeFirstEpoch { epoch: u64, first_epoch: u64 },

    #[error(
        "no state certificate for epoch {epoch}: its root block is not decided, this node is at \
         block {decided_height}"
    )]
    RootNotDecided { epoch: u64, decided_height: u64 },
}

/// Fails until this node decides the root block that a state certificate for `epoch` signs.
///
/// `decided_height` is `None` while this node's decided leaf predates epochs. `current_epoch` is
/// the epoch this node's consensus is in, `None` if unknown: once epochs have started, a node whose
/// decided leaf trails its consensus still fetches any epoch that consensus has already left.
pub(crate) fn ensure_cert_can_exist(
    epoch: EpochNumber,
    first_epoch: EpochNumber,
    epoch_height: u64,
    decided_height: Option<u64>,
    current_epoch: Option<EpochNumber>,
) -> Result<(), StateCertFetchError> {
    let Some(decided_height) = decided_height else {
        return Err(StateCertFetchError::EpochsNotStarted { epoch: epoch.u64() });
    };
    if epoch < first_epoch {
        return Err(StateCertFetchError::BeforeFirstEpoch {
            epoch: epoch.u64(),
            first_epoch: first_epoch.u64(),
        });
    }
    if current_epoch.is_some_and(|current| current > epoch) {
        return Ok(());
    }
    // The cert signs the root block inside `epoch` itself, not the `epoch - 2` root that sets its
    // stake table. `epoch` comes from the URL, so epochs past the decided one are rejected before
    // `root_block_in_epoch` multiplies it.
    if epoch.u64() > epoch_from_block_number(decided_height, epoch_height)
        || decided_height < root_block_in_epoch(epoch.u64(), epoch_height)
    {
        return Err(StateCertFetchError::RootNotDecided {
            epoch: epoch.u64(),
            decided_height,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const DECAF_EPOCH_HEIGHT: u64 = 3000;
    const DECAF_FIRST_EPOCH: u64 = 1054;
    /// Read off decaf's live certificates for these epochs.
    const FIRST_EPOCH_ROOT: u64 = 3_161_995;
    const EPOCH_7361_ROOT: u64 = 22_082_995;

    fn decaf_gate(
        epoch: u64,
        decided_height: u64,
        current_epoch: Option<u64>,
    ) -> Result<(), StateCertFetchError> {
        ensure_cert_can_exist(
            EpochNumber::new(epoch),
            EpochNumber::new(DECAF_FIRST_EPOCH),
            DECAF_EPOCH_HEIGHT,
            Some(decided_height),
            current_epoch.map(EpochNumber::new),
        )
    }

    #[rstest::rstest]
    #[case::root_just_decided(7361, EPOCH_7361_ROOT, Some(7361))]
    #[case::epoch_last_block_decided(7361, 7361 * DECAF_EPOCH_HEIGHT, None)]
    #[case::past_epoch(DECAF_FIRST_EPOCH, EPOCH_7361_ROOT, None)]
    #[case::consensus_left_the_epoch(7361, EPOCH_7361_ROOT - 1000, Some(7362))]
    #[case::first_epoch(DECAF_FIRST_EPOCH, FIRST_EPOCH_ROOT, None)]
    fn cert_that_can_exist_is_fetched(
        #[case] epoch: u64,
        #[case] decided_height: u64,
        #[case] current_epoch: Option<u64>,
    ) {
        decaf_gate(epoch, decided_height, current_epoch).expect("peers may hold this cert");
    }

    #[rstest::rstest]
    #[case::root_one_block_ahead(7361, EPOCH_7361_ROOT - 1, Some(7361))]
    #[case::current_epoch_unknown(7361, EPOCH_7361_ROOT - 1, None)]
    #[case::previous_epoch_last_block_decided(7361, 7360 * DECAF_EPOCH_HEIGHT, None)]
    #[case::far_future_epoch(u64::MAX, EPOCH_7361_ROOT, Some(7361))]
    fn cert_whose_root_is_not_decided_is_not_fetched(
        #[case] epoch: u64,
        #[case] decided_height: u64,
        #[case] current_epoch: Option<u64>,
    ) {
        let err = decaf_gate(epoch, decided_height, current_epoch).unwrap_err();
        assert!(
            matches!(err, StateCertFetchError::RootNotDecided { .. }),
            "{err}"
        );
    }

    #[rstest::rstest]
    #[case::epoch_zero(0)]
    #[case::before_first_epoch(DECAF_FIRST_EPOCH - 1)]
    fn cert_before_the_first_epoch_is_not_fetched(#[case] epoch: u64) {
        let err = decaf_gate(epoch, EPOCH_7361_ROOT, Some(7361)).unwrap_err();
        assert!(
            matches!(err, StateCertFetchError::BeforeFirstEpoch { .. }),
            "{err}"
        );
    }

    #[test]
    fn cert_before_epochs_start_is_not_fetched() {
        let err = ensure_cert_can_exist(
            EpochNumber::new(DECAF_FIRST_EPOCH),
            EpochNumber::new(DECAF_FIRST_EPOCH),
            DECAF_EPOCH_HEIGHT,
            None,
            None,
        )
        .unwrap_err();
        assert!(
            matches!(err, StateCertFetchError::EpochsNotStarted { .. }),
            "{err}"
        );
    }
}
