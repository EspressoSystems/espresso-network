//! Ad hoc L1 client functions.

use crate::{
    Error, Result,
    types::common::{Address, L1BlockId, Timestamp},
};
use alloy::{eips::BlockId, providers::Provider};
use hotshot_contract_adapter::sol_types::StakeTableV2;

/// Get the Espresso stake table genesis block.
pub async fn load_genesis(
    provider: &impl Provider,
    stake_table: Address,
) -> Result<(L1BlockId, Timestamp)> {
    let stake_table_contract = StakeTableV2::new(stake_table, provider);

    // Fetch the finalized block first.
    // This avoids a race condition where the initialized block could change
    // due to a reorg between fetching it and fetching the finalized block.
    let finalized_block = provider
        .get_block(BlockId::finalized())
        .await
        .map_err(|err| {
            Error::internal().context(format!("Failed to fetch finalized block: {err}"))
        })?
        .ok_or_else(|| Error::internal().context("Finalized block not found"))?;
    // Get the block number when the contract was initialized
    let initialized_at_block = stake_table_contract
        .initializedAtBlock()
        .call()
        .await
        .map_err(|err| {
            Error::internal().context(format!("Failed to retrieve initialization block: {err}"))
        })?
        .to::<u64>();

    let finalized_block_number = finalized_block.header.number;

    if initialized_at_block > finalized_block_number {
        panic!(
            "Initialized block {initialized_at_block} must be less than finalized block {finalized_block_number}",
        );
    }

    let block = provider
        .get_block(BlockId::number(initialized_at_block))
        .await
        .map_err(|err| {
            Error::internal().context(format!(
                "Failed to fetch init block {initialized_at_block}: {err}"
            ))
        })?
        .ok_or_else(|| {
            Error::internal().context(format!("Init block {initialized_at_block} not found"))
        })?;

    // Fetch the exitEscrowPeriod at the initialized block
    let _exit_escrow_period = stake_table_contract
        .exitEscrowPeriod()
        .block(BlockId::number(initialized_at_block))
        .call()
        .await
        .map_err(|err| {
            Error::internal().context(format!("Failed to fetch exitEscrowPeriod: {err}"))
        })?
        .to::<u64>();

    let l1_block_id = L1BlockId {
        number: initialized_at_block,
        hash: block.header.hash,
        parent: block.header.parent_hash,
    };

    Ok((l1_block_id, block.header.timestamp))
}

#[cfg(test)]
mod test {
    use alloy::{
        node_bindings::Anvil,
        providers::{ProviderBuilder, ext::AnvilApi},
    };
    use tide_disco::Url;

    use crate::input::l1::testing::ContractDeployment;

    use super::*;

    #[tokio::test]
    #[test_log::test]
    async fn test_genesis_with_deployed_contracts() {
        let anvil = Anvil::new().args(["--slots-in-an-epoch", "0"]).spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();

        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();
        let stake_table = deployment.stake_table_addr;

        let provider = ProviderBuilder::new().connect_http(rpc_url.clone());

        provider.anvil_mine(Some(50), None).await.unwrap();

        let (block_id, _timestamp) = load_genesis(&provider, stake_table).await.unwrap();

        assert!(block_id.number > 0, "Block number should be greater than 0");
    }
}
