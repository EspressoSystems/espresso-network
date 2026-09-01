//! State certificate validation and error handling

use std::collections::HashMap;

use alloy::primitives::{FixedBytes, U256};
use anyhow::bail;
use disco_types::status::StatusCode;
use espresso_types::SeqTypes;
use hotshot_contract_adapter::light_client::derive_signed_state_digest;
use hotshot_query_service::availability::Error;
use hotshot_types::{
    data::EpochNumber,
    light_client::StateVerKey,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
    traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
    utils::epoch_from_block_number,
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
) -> anyhow::Result<()> {
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

    // If auth_root is the default value (all zeros), we're on consensus version V3, so verify LCV2 signatures only
    // For consensus >= V4, verify both LCV3 and LCV2 signatures
    let use_lcv2_only = cert.auth_root == FixedBytes::<32>::default();

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

            let is_valid = if use_lcv2_only {
                lcv2_valid
            } else {
                let lcv3_valid = <StateVerKey as LCV3StateSignatureKey>::verify_state_sig(
                    &peer.state_ver_key,
                    lcv3_sig,
                    signed_state_digest,
                );

                lcv3_valid && lcv2_valid
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
        light_client::{CircuitField, LightClientState, StakeTableState, StateKeyPair, StateVerKey},
        simple_certificate::LightClientStateUpdateCertificateV2,
        stake_table::HSStakeTable,
        traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, SignatureKey},
        PeerConfig,
    };

    use super::*;

    /// Non-zero, so the LCV3 signatures are checked. The exact bytes don't matter.
    const AUTH_ROOT: [u8; 32] = [9u8; 32];

    const NUM_SIGNERS: u64 = 4;
    const STAKE_PER_SIGNER: u64 = 100;

    /// Makes the fixture's block 100 the epoch root of epoch 1: (100 + 5) % 105 == 0.
    const EPOCH_HEIGHT: u64 = 105;

    /// A certificate with valid LCV2 and LCV3 signatures from every signer, and the stake
    /// table those signers belong to.
    fn valid_cert_and_stake_table() -> (
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
        let auth_root = FixedBytes::<32>::from(AUTH_ROOT);
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
        validate_state_cert(&cert, &stake_table, EpochNumber::new(1), EPOCH_HEIGHT)
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
        validate_state_cert(&cert, &stake_table, EpochNumber::new(2), EPOCH_HEIGHT).expect_err(
            "a cert whose signed block height belongs to epoch 1 must not satisfy a request \
             for epoch 2, however it is labelled",
        );
    }
}
