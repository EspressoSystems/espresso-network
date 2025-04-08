use alloy::primitives::{utils::format_ether, Address};
use anyhow::Result;
use espresso_types::{v0_3::Validator, L1Client};
use hotshot_types::signature_key::BLSPubKey;
use url::Url;

use crate::parse::Commission;

pub async fn stake_table_info(
    l1_url: Url,
    stake_table_address: Address,
    l1_block_number: u64,
) -> Result<Vec<Validator<BLSPubKey>>> {
    let l1 = L1Client::new(vec![l1_url])?;
    let st = l1
        .get_stake_table(stake_table_address, l1_block_number)
        .await?;
    Ok(st
        .into_iter()
        .map(|(_address, validator)| validator)
        .collect())
}

pub fn display_stake_table(stake_table: Vec<Validator<BLSPubKey>>) -> Result<()> {
    let mut stake_table = stake_table.clone();
    stake_table.sort_by(|a, b| a.stake.cmp(&b.stake));

    for validator in stake_table.iter() {
        let comm: Commission = validator.commission.try_into()?;
        let bls_key = validator.stake_table_key.to_string();
        let end = bls_key.chars().map(|c| c.len_utf8()).take(40).sum();
        tracing::info!(
            "Validator {}: {}... comm={} stake={} ESP",
            validator.account,
            &bls_key[..end],
            comm,
            format_ether(validator.stake),
        );
    }
    Ok(())
}
