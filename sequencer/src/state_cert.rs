//! State certificate validation and error handling

use std::collections::HashMap;

use alloy::primitives::{FixedBytes, U256};
use anyhow::bail;
use espresso_types::SeqTypes;
use hotshot_query_service::availability::Error;
use hotshot_task_impls::helpers::derive_signed_state_digest;
use hotshot_types::{
    light_client::StateVerKey,
    simple_certificate::LightClientStateUpdateCertificateV2,
    stake_table::HSStakeTable,
    traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey, StakeTableEntryType},
};
use tide_disco::StatusCode;

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
) -> anyhow::Result<()> {
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
