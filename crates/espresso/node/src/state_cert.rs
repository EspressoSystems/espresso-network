//! State certificate validation and error handling

use std::collections::HashMap;

use alloy::primitives::U256;
use anyhow::{bail, ensure};
use disco_types::status::StatusCode;
use espresso_types::SeqTypes;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_query_service::availability::Error;
use hotshot_types::{
    data::{EpochNumber, ViewNumber},
    light_client::StateVerKey,
    message::UpgradeLock,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
    traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
    utils::{epoch_from_block_number, is_epoch_root},
};

/// Error type for state certificate fetching
#[derive(Debug, thiserror::Error)]
pub enum StateCertFetchError {
    #[error("Failed to fetch state certificate: {0}")]
    FetchError(#[source] anyhow::Error),

    #[error("State certificate validation failed: {0}")]
    ValidationError(#[source] anyhow::Error),

    #[error("State certificate error: {0}")]
    Other(#[source] anyhow::Error),
}

impl From<StateCertFetchError> for hotshot_query_service::availability::Error {
    fn from(err: StateCertFetchError) -> Self {
        match err {
            StateCertFetchError::FetchError(e) => Error::Custom {
                message: format!("Failed to fetch state cert from peers: {e}"),
                status: StatusCode::NOT_FOUND,
            },
            StateCertFetchError::ValidationError(e) => Error::Custom {
                message: format!("State certificate validation failed: {e}"),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
            StateCertFetchError::Other(e) => Error::Custom {
                message: format!("Failed to process state cert: {e}"),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            },
        }
    }
}

/// Validates a state certificate: that it belongs to `expected_epoch`, and that its
/// signatures reach the threshold under `stake_table`.
pub fn validate_state_cert(
    cert: &LightClientStateUpdateCertificateV2<SeqTypes>,
    stake_table: &HSStakeTable<SeqTypes>,
    expected_epoch: EpochNumber,
    epoch_height: u64,
    upgrade_lock: &UpgradeLock<SeqTypes>,
) -> anyhow::Result<()> {
    // Validators publish state signatures for ordinary blocks to a public relay, over the
    // same digest verified below. Only epoch roots ever carry a genuine certificate, so
    // without this a peer could assemble one from harvested relay signatures.
    ensure!(
        is_epoch_root(cert.light_client_state.block_height, epoch_height),
        "state certificate is for block {}, which is not an epoch root",
        cert.light_client_state.block_height
    );

    // `cert.epoch` is outside the signed digest, so a peer can set it to whatever was
    // requested. `block_height` is inside it, so derive the epoch from that instead.
    let derived_epoch = EpochNumber::new(epoch_from_block_number(
        cert.light_client_state.block_height,
        epoch_height,
    ));
    if derived_epoch != expected_epoch {
        bail!(
            "state certificate is for block {} in epoch {derived_epoch}, but epoch \
             {expected_epoch} was requested",
            cert.light_client_state.block_height
        );
    }

    if cert.epoch != derived_epoch {
        bail!(
            "state certificate is labelled epoch {}, but its block {} belongs to epoch \
             {derived_epoch}",
            cert.epoch,
            cert.light_client_state.block_height
        );
    }

    let signed_state_digest = derive_signed_state_digest(
        &cert.light_client_state,
        &cert.next_stake_table_state,
        &cert.auth_root,
    );

    // Take the version from our own upgrade lock, not the certificate: only LCV3 covers
    // `auth_root`, so trusting it here would let a peer skip that check by zeroing it.
    // `view_number` sits inside `light_client_state`, which LCV2 does cover.
    // V4 is where `Header::auth_root()` stops returning zero, so that is the gate.
    let require_lcv3 =
        upgrade_lock.upgraded_drb_and_header(ViewNumber::new(cert.light_client_state.view_number));

    let signature_map: HashMap<&StateVerKey, _> = cert
        .signatures
        .iter()
        .map(|(key, lcv3_sig, lcv2_sig)| (key, (lcv3_sig, lcv2_sig)))
        .collect();

    // Verify signatures and accumulate weight
    let mut accumulated_weight = U256::ZERO;

    for peer in stake_table.iter() {
        if let Some((lcv3_sig, lcv2_sig)) = signature_map.get(&peer.state_ver_key) {
            let lcv2_valid = <StateVerKey as LCV2StateSignatureKey>::verify_state_sig(
                &peer.state_ver_key,
                lcv2_sig,
                &cert.light_client_state,
                &cert.next_stake_table_state,
            );

            let is_valid = if require_lcv3 {
                let lcv3_valid = <StateVerKey as LCV3StateSignatureKey>::verify_state_sig(
                    &peer.state_ver_key,
                    lcv3_sig,
                    signed_state_digest,
                );

                lcv2_valid && lcv3_valid
            } else {
                lcv2_valid
            };

            if is_valid {
                accumulated_weight += peer.stake_table_entry.stake();
            } else {
                bail!(format!(
                    "Invalid signature from key: {}",
                    peer.state_ver_key
                ))
            }
        }
    }

    // Check if accumulated weight meets the threshold
    let total_stake = stake_table.total_stakes();
    let threshold = hotshot_types::stake_table::one_honest_threshold(total_stake);
    if accumulated_weight < threshold {
        bail!(
            "State certificate validation failed: accumulated weight {accumulated_weight} is \
             below threshold {threshold}",
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{FixedBytes, U256};
    use espresso_types::PubKey;
    use hotshot_contract_adapter::light_client::derive_signed_state_digest;
    use hotshot_types::{
        PeerConfig,
        light_client::{
            CircuitField, LightClientState, StakeTableState, StateKeyPair, StateVerKey,
        },
        simple_certificate::LightClientStateUpdateCertificateV2,
        stake_table::HSStakeTable,
        traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey},
    };
    use versions::{Upgrade, version};

    use super::*;

    /// Covered by the fixture's signatures.
    const SIGNED_AUTH_ROOT: [u8; 32] = [9u8; 32];
    /// Swapped in after signing, so no signature covers it.
    const UNSIGNED_AUTH_ROOT: [u8; 32] = [0xAAu8; 32];

    const NUM_SIGNERS: u64 = 4;
    const STAKE_PER_SIGNER: u64 = 100;

    /// Makes `ROOT_BLOCK` the epoch root of `FIXTURE_EPOCH`: (100 + 5) % 105 == 0.
    const EPOCH_HEIGHT: u64 = 105;
    /// The epoch root of `FIXTURE_EPOCH` under `EPOCH_HEIGHT`.
    const ROOT_BLOCK: u64 = 100;
    /// The epoch every fixture certificate is for.
    const FIXTURE_EPOCH: u64 = 1;

    /// With no decided upgrade certificate, the base version applies to every view.
    fn upgrade_lock_at(major: u16, minor: u16) -> UpgradeLock<SeqTypes> {
        UpgradeLock::new(Upgrade::trivial(version(major, minor)))
    }

    /// A V4-era certificate at the epoch root, and the stake table of its signers.
    fn valid_cert_and_stake_table() -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        cert_and_stake_table(ROOT_BLOCK, FixedBytes::<32>::from(SIGNED_AUTH_ROOT))
    }

    /// Signs over both the block height and `auth_root`, so the certificate is
    /// self-consistent for any pair: a caller can build a correctly signed certificate for
    /// a block that is not an epoch root, or for any `auth_root`.
    fn cert_and_stake_table(
        block_height: u64,
        auth_root: FixedBytes<32>,
    ) -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        let light_client_state = LightClientState {
            view_number: 42,
            block_height,
            block_comm_root: CircuitField::from(7u64),
        };
        let next_stake_table_state = StakeTableState {
            bls_key_comm: CircuitField::from(1u64),
            schnorr_key_comm: CircuitField::from(2u64),
            amount_comm: CircuitField::from(3u64),
            threshold: CircuitField::from(1u64),
        };
        let digest =
            derive_signed_state_digest(&light_client_state, &next_stake_table_state, &auth_root);

        let mut signatures = Vec::new();
        let mut peers = Vec::new();
        for i in 0..NUM_SIGNERS {
            let state_key_pair = StateKeyPair::generate_from_seed_indexed([0u8; 32], i);
            let sign_key = state_key_pair.sign_key_ref();

            let lcv2_sig = <StateVerKey as LCV2StateSignatureKey>::sign_state(
                sign_key,
                &light_client_state,
                &next_stake_table_state,
            )
            .expect("LCV2 sign");
            let lcv3_sig = <StateVerKey as LCV3StateSignatureKey>::sign_state(sign_key, digest)
                .expect("LCV3 sign");

            signatures.push((state_key_pair.ver_key(), lcv3_sig, lcv2_sig));

            let bls_key = PubKey::generated_from_seed_indexed([0u8; 32], i).0;
            peers.push(PeerConfig::<SeqTypes> {
                stake_table_entry: bls_key.stake_table_entry(U256::from(STAKE_PER_SIGNER)),
                state_ver_key: state_key_pair.ver_key(),
                connect_info: None,
            });
        }

        let cert = LightClientStateUpdateCertificateV2::<SeqTypes> {
            epoch: EpochNumber::new(FIXTURE_EPOCH),
            light_client_state,
            next_stake_table_state,
            signatures,
            auth_root,
        };
        (cert, HSStakeTable::from(peers))
    }

    /// Mirrors `From<LightClientStateUpdateCertificateV1>`, which clones the LCV2 signature
    /// into the LCV3 slot: legacy certificates carry no real LCV3 signature. Both persistence
    /// backends upcast V1 certificates on load, so this shape is live.
    fn legacy_cert_and_stake_table() -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        let (mut cert, stake_table) = cert_and_stake_table(ROOT_BLOCK, FixedBytes::<32>::default());
        for (_, lcv3_sig, lcv2_sig) in cert.signatures.iter_mut() {
            *lcv3_sig = lcv2_sig.clone();
        }
        (cert, stake_table)
    }

    #[test]
    fn test_valid_cert_is_accepted() {
        let (cert, stake_table) = valid_cert_and_stake_table();
        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect("well-formed certificate must validate");
    }

    /// `epoch` is a bare label: `derive_signed_state_digest` covers only the light client
    /// state, the next stake table state, and the auth root. A peer can relabel a certificate
    /// it holds for one epoch to match a request for another, and enough of the two epochs'
    /// signers normally overlap to clear the one-honest threshold against the wrong stake
    /// table. So the binding must be to `block_height`, which is signed.
    #[test]
    fn test_cert_from_another_epoch_is_rejected() {
        let (mut cert, stake_table) = valid_cert_and_stake_table();

        let digest_before = derive_signed_state_digest(
            &cert.light_client_state,
            &cert.next_stake_table_state,
            &cert.auth_root,
        );
        cert.epoch = EpochNumber::new(2);
        let digest_after = derive_signed_state_digest(
            &cert.light_client_state,
            &cert.next_stake_table_state,
            &cert.auth_root,
        );
        assert_eq!(
            digest_before, digest_after,
            "relabelling changed the signed digest; if this ever fails the epoch became \
             authenticated and the label check is redundant"
        );

        // Relabelling does not help: the signed block height still says epoch 1.
        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(2),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err(
            "a cert whose signed block height belongs to epoch 1 must not satisfy a request for \
             epoch 2, however it is labelled",
        );
    }

    /// The label is what downstream consumers key on after validation, so it has to agree
    /// with the block height even when the height itself satisfies the request.
    #[test]
    fn test_cert_with_mislabelled_epoch_is_rejected() {
        // `ROOT_BLOCK` is an epoch root and derives to the requested epoch, so only the
        // label check can reject this.
        assert!(is_epoch_root(ROOT_BLOCK, EPOCH_HEIGHT));
        assert_eq!(
            epoch_from_block_number(ROOT_BLOCK, EPOCH_HEIGHT),
            FIXTURE_EPOCH
        );

        let (mut cert, stake_table) = valid_cert_and_stake_table();
        cert.epoch = EpochNumber::new(2);

        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err("a cert whose label disagrees with its signed block height must be rejected");
    }

    /// Validators sign the light client state of ordinary blocks too, and publish those
    /// signatures to a public relay over the same digest verified here. Only epoch roots
    /// carry a genuine certificate, so a bundle assembled at any other height is a forgery.
    #[test]
    fn test_cert_for_a_non_epoch_root_block_is_rejected() {
        // Block 99 is inside epoch 1 but is not its root, so only the epoch-root check
        // can reject it: the signatures are valid and the derived epoch matches.
        assert!(!is_epoch_root(99, EPOCH_HEIGHT));
        assert_eq!(epoch_from_block_number(99, EPOCH_HEIGHT), FIXTURE_EPOCH);

        let (cert, stake_table) =
            cert_and_stake_table(99, FixedBytes::<32>::from(SIGNED_AUTH_ROOT));
        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err("a cert for a block that is not an epoch root must be rejected");
    }

    /// Already rejected before the fix; kept so a refactor can't narrow the check to zero.
    #[test]
    fn test_unsigned_auth_root_is_rejected() {
        let (mut cert, stake_table) = valid_cert_and_stake_table();
        cert.auth_root = FixedBytes::<32>::from(UNSIGNED_AUTH_ROOT);

        let err = validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err("certificate with a mutated auth_root must be rejected");
        assert!(
            err.to_string().contains("Invalid signature"),
            "expected signature failure, got: {err}"
        );
    }

    /// Zeroing `auth_root` must not disable the LCV3 check.
    /// The LCV2 signatures stay valid because they never covered `auth_root`, so before the
    /// fix this certificate was accepted with full stake weight.
    #[test]
    fn test_zero_auth_root_is_rejected_on_v4() {
        let (mut cert, stake_table) = valid_cert_and_stake_table();
        cert.auth_root = FixedBytes::<32>::default();

        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err("a zeroed auth_root must not disable the LCV3 check");
    }

    /// Regression guard for catchup: certificates from epochs predating V4 have a genuinely
    /// zero `auth_root` and carry no meaningful LCV3 signature. Under a pre-V4 upgrade lock
    /// they must still validate, so the fix cannot be "simplified" into rejecting all zeros.
    #[test]
    fn test_prev4_cert_with_zero_auth_root_is_accepted() {
        let (cert, stake_table) = legacy_cert_and_stake_table();

        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 3),
        )
        .expect("genuine pre-V4 certificates must still validate");
    }

    /// The other side of the gate: the same legacy shape must not satisfy a V4-era view.
    /// Together with the test above this pins `require_lcv3` from both directions, so
    /// hardcoding it either way fails.
    #[test]
    fn test_legacy_cert_is_rejected_on_v4() {
        let (cert, stake_table) = legacy_cert_and_stake_table();

        validate_state_cert(
            &cert,
            &stake_table,
            EpochNumber::new(FIXTURE_EPOCH),
            EPOCH_HEIGHT,
            &upgrade_lock_at(0, 5),
        )
        .expect_err("legacy LCV3 slot must not satisfy the V4 check");
    }
}
