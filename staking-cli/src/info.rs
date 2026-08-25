use std::time::Duration;

use alloy::{
    primitives::{Address, utils::format_ether},
    providers::{Provider, ProviderBuilder},
};
use anyhow::{Context as _, Result};
use espresso_types::{
    L1ClientOptions,
    v0_3::{Fetcher, RegisteredValidator},
};
use hotshot_contract_adapter::sol_types::StakeTableV3;
pub use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::signature_key::BLSPubKey;
use url::Url;

use crate::{output::output_success, parse::Commission};

/// The stake table event set is small enough for one query, provided the RPC serves a large block
/// range. The default RPCs do, and chunking against them trips their request rate limit instead.
const EVENTS_MAX_BLOCK_RANGE: u64 = 10_u64.pow(9);

/// Providers that cap the block range reject every chunk, and the fetcher panics only once the
/// retry budget runs out. Keep that short so the panic's `ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE`
/// hint arrives in seconds rather than after the 20 minute default.
const EVENTS_MAX_RETRY_DURATION: Duration = Duration::from_secs(30);

pub async fn stake_table_info(
    l1_url: Url,
    stake_table_address: Address,
    l1_block_number: u64,
) -> Result<Vec<RegisteredValidator<BLSPubKey>>> {
    let mut options = L1ClientOptions::default();
    // `default()` parses the environment, so only widen the range the user did not choose.
    if std::env::var_os("ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE").is_none() {
        options.l1_events_max_block_range = EVENTS_MAX_BLOCK_RANGE;
        options.l1_events_max_retry_duration = EVENTS_MAX_RETRY_DURATION;
    }
    let l1 = options.connect(vec![l1_url])?;
    let (validators, _) =
        Fetcher::fetch_all_validators_from_contract(l1, stake_table_address, l1_block_number)
            .await?;

    Ok(validators
        .into_iter()
        .map(|(_address, validator)| validator)
        .collect())
}

pub fn display_stake_table(
    stake_table: Vec<RegisteredValidator<BLSPubKey>>,
    compact: bool,
) -> Result<()> {
    let mut stake_table = stake_table.clone();
    stake_table.sort_by_key(|a| a.stake);

    let shorten = |s: String| {
        if compact {
            let end = s.chars().map(|c| c.len_utf8()).take(40).sum();
            format!("{}..", &s[..end])
        } else {
            s
        }
    };
    for validator in stake_table.iter() {
        let comm: Commission = validator.commission.try_into()?;
        let key_str = shorten(match &validator.stake_table_key {
            Some(key) => key.to_string(),
            None => "<no_bls_key>".to_string(),
        });
        output_success(format!(
            "Validator {}: {key_str} comm={comm} stake={} ESP",
            validator.account,
            format_ether(validator.stake),
        ));
        if let Some(x25519) = &validator.x25519_key {
            output_success(format!(" - {}", shorten(x25519.to_string())));
        }
        if let Some(p2p_addr) = &validator.p2p_addr {
            output_success(format!(" - p2p_addr={p2p_addr}"));
        }

        if validator.delegators.is_empty() {
            output_success(" - No delegators");
            continue;
        }

        // sort delegators by address for easier reading
        let mut delegators = validator.delegators.iter().collect::<Vec<_>>();
        delegators.sort_by(|a, b| a.0.cmp(b.0));
        for (delegator, stake) in delegators {
            output_success(format!(
                " - Delegator {delegator}: stake={} ESP",
                format_ether(*stake)
            ));
        }
    }
    Ok(())
}

pub async fn fetch_token_address(rpc_url: Url, stake_table_address: Address) -> Result<Address> {
    let provider = ProviderBuilder::new().connect_http(rpc_url);
    StakeTableV3::new(stake_table_address, provider)
        .token()
        .call()
        .await
        .with_context(|| {
            format!(
                "Failed to fetch token address from stake table contract at {stake_table_address}"
            )
        })
}

pub async fn fetch_stake_table_version(
    provider: impl Provider,
    stake_table_address: Address,
) -> Result<StakeTableContractVersion> {
    let stake_table = StakeTableV3::new(stake_table_address, provider);
    stake_table
        .getVersion()
        .call()
        .await?
        .try_into()
        .with_context(|| "Failed to parse stake table contract version")
}
