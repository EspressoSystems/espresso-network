use alloy::{
    primitives::{Address, utils::format_ether},
    providers::{Provider, ProviderBuilder},
};
use anyhow::{Context as _, Result};
use espresso_types::{
    L1Client, RegisteredValidatorMap,
    v0_3::{Fetcher, RegisteredValidator},
    validators_from_l1_events,
};
use hotshot_contract_adapter::sol_types::StakeTableV3;
pub use hotshot_contract_adapter::stake_table::StakeTableContractVersion;
use hotshot_types::signature_key::BLSPubKey;
use url::Url;

use crate::{output::output_success, parse::Commission};

pub async fn stake_table_info(
    l1_url: Url,
    stake_table_address: Address,
    l1_block_number: u64,
) -> Result<Vec<RegisteredValidator<BLSPubKey>>> {
    let l1 = L1Client::new(vec![l1_url])?;
    let validators = fetch_validators_adaptively(l1, stake_table_address, l1_block_number).await?;

    Ok(validators
        .into_iter()
        .map(|(_address, validator)| validator)
        .collect())
}

/// Fetch the stake table in one request, falling back to the chunked fetcher if that is refused.
///
/// Providers pull in opposite directions: the gateways this CLI defaults to serve the whole range
/// in one request but rate limit the hundreds of requests chunking needs, while others cap the
/// range and only work chunked. The single request is tried without retrying, so a provider that
/// refuses it costs one round trip rather than the chunked fetcher's retry budget.
async fn fetch_validators_adaptively(
    l1: L1Client,
    stake_table_address: Address,
    l1_block_number: u64,
) -> Result<RegisteredValidatorMap> {
    let stake_table = StakeTableV3::new(stake_table_address, &l1.provider);
    let from_block = stake_table.initializedAtBlock().call().await?.to::<u64>();

    // A pinned range means the provider caps it, so don't spend a request on a rejection.
    if crate::entry::configured_block_range().is_none() {
        match Fetcher::try_fetch_events_from_contract(
            l1.clone(),
            stake_table_address,
            from_block,
            l1_block_number,
        )
        .await
        {
            Ok(events) => {
                return Ok(validators_from_l1_events(events.into_iter().map(|(_, e)| e))?.0);
            },
            Err(err) => tracing::info!(
                %err,
                "could not fetch the stake table in one request, retrying in smaller ranges"
            ),
        }
    }

    Ok(
        Fetcher::fetch_all_validators_from_contract(l1, stake_table_address, l1_block_number)
            .await?
            .0,
    )
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
