//! State certificate validation and error handling

use std::collections::HashMap;

use alloy::primitives::U256;
use anyhow::bail;
use disco_types::status::StatusCode;
use espresso_types::SeqTypes;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_query_service::availability::Error;
use hotshot_types::{
    data::ViewNumber,
    light_client::StateVerKey,
    message::UpgradeLock,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
    traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
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

/// Validates a state certificate by verifying signatures and checking threshold
pub fn validate_state_cert(
    cert: &LightClientStateUpdateCertificateV2<SeqTypes>,
    stake_table: &HSStakeTable<SeqTypes>,
    upgrade_lock: &UpgradeLock<SeqTypes>,
) -> anyhow::Result<()> {
    let signed_state_digest = derive_signed_state_digest(
        &cert.light_client_state,
        &cert.next_stake_table_state,
        &cert.auth_root,
    );

    // Take the version from our own upgrade lock, not the certificate: only LCV3 covers
    // `auth_root`, so trusting it here would let a peer skip that check by zeroing it.
    // `view_number` sits inside `light_client_state`, which LCV2 does cover.
    // V4 is where `Header::auth_root()` stops returning zero, so that is the gate.
    let require_lcv3 = upgrade_lock
        .upgraded_drb_and_header(ViewNumber::new(cert.light_client_state.view_number));

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

    /// With no decided upgrade certificate, the base version applies to every view.
    fn upgrade_lock_at(major: u16, minor: u16) -> UpgradeLock<SeqTypes> {
        UpgradeLock::new(Upgrade::new(version(major, minor), version(major, minor)))
    }

    /// A V4-era certificate and the stake table of its signers.
    fn valid_cert_and_stake_table() -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        cert_and_stake_table(FixedBytes::<32>::from(SIGNED_AUTH_ROOT))
    }

    /// Signs over `auth_root`, so the certificate is self-consistent for any value.
    fn cert_and_stake_table(
        auth_root: FixedBytes<32>,
    ) -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        let light_client_state = LightClientState {
            view_number: 42,
            block_height: 100,
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
            epoch: hotshot_types::data::EpochNumber::new(1),
            light_client_state,
            next_stake_table_state,
            signatures,
            auth_root,
        };
        (cert, HSStakeTable::from(peers))
    }

    #[test]
    fn test_valid_cert_is_accepted() {
        let (cert, stake_table) = valid_cert_and_stake_table();
        validate_state_cert(&cert, &stake_table, &upgrade_lock_at(0, 5))
            .expect("well-formed certificate must validate");
    }

    /// Already rejected before the fix; kept so a refactor can't narrow the check to zero.
    #[test]
    fn test_unsigned_auth_root_is_rejected() {
        let (mut cert, stake_table) = valid_cert_and_stake_table();
        cert.auth_root = FixedBytes::<32>::from(UNSIGNED_AUTH_ROOT);

        let err = validate_state_cert(&cert, &stake_table, &upgrade_lock_at(0, 5))
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

        validate_state_cert(&cert, &stake_table, &upgrade_lock_at(0, 5))
            .expect_err("a zeroed auth_root must not disable the LCV3 check");
    }

    /// Mirrors `From<LightClientStateUpdateCertificateV1>`, which clones the LCV2 signature
    /// into the LCV3 slot: legacy certificates carry no real LCV3 signature. Both persistence
    /// backends upcast V1 certificates on load, so this shape is live.
    fn legacy_cert_and_stake_table() -> (
        LightClientStateUpdateCertificateV2<SeqTypes>,
        HSStakeTable<SeqTypes>,
    ) {
        let (mut cert, stake_table) = cert_and_stake_table(FixedBytes::<32>::default());
        for (_, lcv3_sig, lcv2_sig) in cert.signatures.iter_mut() {
            *lcv3_sig = lcv2_sig.clone();
        }
        (cert, stake_table)
    }

    /// Regression guard for catchup: certificates from epochs predating V4 have a genuinely
    /// zero `auth_root` and carry no meaningful LCV3 signature. Under a pre-V4 upgrade lock
    /// they must still validate, so the fix cannot be "simplified" into rejecting all zeros.
    #[test]
    fn test_prev4_cert_with_zero_auth_root_is_accepted() {
        let (cert, stake_table) = legacy_cert_and_stake_table();

        validate_state_cert(&cert, &stake_table, &upgrade_lock_at(0, 3))
            .expect("genuine pre-V4 certificates must still validate");
    }

    /// The other side of the gate: the same legacy shape must not satisfy a V4-era view.
    /// Together with the test above this pins `require_lcv3` from both directions, so
    /// hardcoding it either way fails.
    #[test]
    fn test_legacy_cert_is_rejected_on_v4() {
        let (cert, stake_table) = legacy_cert_and_stake_table();

        validate_state_cert(&cert, &stake_table, &upgrade_lock_at(0, 5))
            .expect_err("legacy LCV3 slot must not satisfy the V4 check");
    }
}
