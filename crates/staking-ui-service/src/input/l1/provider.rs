//! Ad hoc L1 client functions.

use std::{collections::BTreeMap, sync::Arc};

use crate::{
    Error, Result,
    error::{ResultExt, ensure},
    input::l1::{L1BlockSnapshot, L1Event},
    types::common::{Address, L1BlockId, Timestamp},
};
use alloy::{eips::BlockId, providers::Provider, rpc::types::Filter, sol_types::SolEventInterface};
use hotshot_contract_adapter::sol_types::{
    RewardClaim::RewardClaimEvents,
    StakeTableV2::{self, StakeTableV2Events},
};

/// Get the Espresso stake table genesis block.
pub async fn load_genesis(
    provider: &impl Provider,
    stake_table: Address,
) -> Result<L1BlockSnapshot> {
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

    ensure!(
        initialized_at_block <= finalized_block_number,
        Error::internal().context(format!(
            "Initialized block {initialized_at_block} must be less than finalized block \
                {finalized_block_number}"
        ))
    );

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
    let exit_escrow_period = stake_table_contract
        .exitEscrowPeriod()
        .block(BlockId::number(initialized_at_block))
        .call()
        .await
        .map_err(|err| {
            Error::internal().context(format!("Failed to fetch exitEscrowPeriod: {err}"))
        })?
        .to::<u64>();

    let id = L1BlockId {
        number: initialized_at_block,
        hash: block.header.hash,
        parent: block.header.parent_hash,
    };

    Ok(L1BlockSnapshot {
        id,
        timestamp: block.header.timestamp,
        exit_escrow_period,
    })
}

pub(super) async fn get_events(
    provider: &impl Provider,
    filter: Filter,
    stake_table_address: Address,
    reward_contract_address: Address,
) -> Result<BTreeMap<L1BlockId, (Timestamp, Vec<L1Event>)>> {
    let filter = filter.address(vec![stake_table_address, reward_contract_address]);
    let logs = provider
        .get_logs(&filter)
        .await
        .context(|| Error::internal().context("getting L1 logs"))?;

    // Decode events from logs
    let mut events = BTreeMap::new();

    for log in logs {
        let hash = log.block_hash.ok_or_else(|| {
            Error::internal().context(format!("event log missing block hash: {log:?}"))
        })?;
        let block = provider
            .get_block(hash.into())
            .await
            .context(|| Error::internal().context(format!("getting header for log {log:?}")))?
            .ok_or_else(|| {
                Error::internal().context(format!("header for log {log:?} not available"))
            })?;
        let id = L1BlockId {
            number: block.number(),
            hash,
            parent: block.header.parent_hash,
        };
        let timestamp = block.header.timestamp;
        let (_, events_for_block) = events.entry(id).or_insert((timestamp, vec![]));

        // Try to decode stake table event
        if log.address() == stake_table_address {
            let event = StakeTableV2Events::decode_raw_log(log.topics(), &log.data().data)
                .unwrap_or_else(|e| {
                    // This is a panic, not an error, as it should be impossible to successfully
                    // retrieve an event from the stake table address but not be able to decode it.
                    panic!(
                        "failed to decode event from stake table {stake_table_address}, tx {:?}: {e:#}",
                        log.transaction_hash
                    );
                });
            events_for_block.push(L1Event::StakeTable(Arc::new(event)));
            continue;
        }

        // Try to decode reward claim event
        if log.address() == reward_contract_address {
            let event = RewardClaimEvents::decode_raw_log(log.topics(), &log.data().data)
                .unwrap_or_else(|e| {
                    panic!(
                        "failed to decode event from reward contract {reward_contract_address}, tx {:?}: {e:#}",
                        log.transaction_hash
                    );
                });
            events_for_block.push(L1Event::Reward(Arc::new(event)));
            continue;
        }

        tracing::warn!(
            ?log,
            %stake_table_address,
            %reward_contract_address,
            "filter returned event which is not from either contract"
        );
    }

    Ok(events)
}

#[cfg(test)]
mod test {
    use alloy::{
        node_bindings::Anvil,
        providers::{ProviderBuilder, WalletProvider, ext::AnvilApi},
        signers::local::MnemonicBuilder,
    };
    use futures::future::join_all;
    use rand::{SeedableRng, rngs::StdRng};
    use staking_cli::DEV_MNEMONIC;
    use tide_disco::Url;

    use crate::input::l1::testing::{
        ContractDeployment, assert_events_eq, validator_registered_event_with_account,
    };

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

        let block = load_genesis(&provider, stake_table).await.unwrap();

        assert!(block.number() > 0, "Block number should be greater than 0");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_genesis_exit_escrow_period() {
        let anvil = Anvil::new()
            .args(["--slots-in-an-epoch", "0"])
            .block_time(1)
            .spawn();
        let deployment = ContractDeployment::deploy(anvil.endpoint_url())
            .await
            .unwrap();
        let provider = ProviderBuilder::new()
            .wallet(
                MnemonicBuilder::english()
                    .phrase(DEV_MNEMONIC)
                    .build()
                    .unwrap(),
            )
            .connect_http(anvil.endpoint_url());

        let stake_table_address = deployment.stake_table_addr;
        let contract = StakeTableV2::new(stake_table_address, &provider);

        // Change the exit escrow period, to verify that the genesis snapshot loads the exit escrow
        // period from the time when the contract was initialized, not what it is now.
        let genesis_exit_escrow_period: u64 = contract
            .exitEscrowPeriod()
            .call()
            .await
            .unwrap()
            .try_into()
            .unwrap();
        let receipt = contract
            // Add one day
            .updateExitEscrowPeriod(genesis_exit_escrow_period + 86_400)
            .send()
            .await
            .unwrap()
            .get_receipt()
            .await
            .unwrap();
        assert!(receipt.status());

        let genesis = load_genesis(&provider, *contract.address()).await.unwrap();
        assert_eq!(genesis.exit_escrow_period, genesis_exit_escrow_period);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_get_events_multiple_events_per_block() {
        // Start Anvil with on-demand mining, so the contract deployment is fast.
        let anvil = Anvil::new().spawn();
        let rpc_url: Url = anvil.endpoint().parse().unwrap();
        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();

        // Once contracts are deployed, set a pretty long block time, so multiple transactions can
        // end up in the same block.
        let provider = ProviderBuilder::new().connect_http(rpc_url);
        provider.anvil_set_interval_mining(12).await.unwrap();

        // Register two nodes at the same time and wait for the transactions to get mined, then do
        // it again in another block.
        let mut blocks = [0; 2];
        let mut events = [vec![], vec![]];
        for (i, block) in blocks.iter_mut().enumerate() {
            let mut results = join_all((0..2).map(|j| {
                let index = 2 * i + j;
                let provider = ProviderBuilder::new()
                    .wallet(
                        MnemonicBuilder::english()
                            .phrase(DEV_MNEMONIC)
                            .index(index as u32)
                            .unwrap()
                            .build()
                            .unwrap(),
                    )
                    .connect_http(anvil.endpoint_url());
                let address = provider.default_signer_address();
                let node = validator_registered_event_with_account(
                    StdRng::seed_from_u64(index as u64),
                    address,
                );
                let stake_table = StakeTableV2::new(deployment.stake_table_addr, provider.clone());
                tracing::info!(index, %address, "submitting registration");
                async move {
                    let tx = stake_table
                        .registerValidatorV2(
                            node.blsVK,
                            node.schnorrVK,
                            node.blsSig,
                            node.schnorrSig.clone(),
                            node.commission,
                            "https://example.com/validator-metadata.json".to_string(),
                        )
                        .send()
                        .await
                        .unwrap();
                    tracing::info!(index, "transaction submitted, waiting for receipt");
                    let receipt = tx.get_receipt().await.unwrap();
                    assert!(receipt.status());
                    tracing::info!(index, "transaction mined");

                    let expected_event = L1Event::StakeTable(Arc::new(
                        StakeTableV2Events::ValidatorRegisteredV2(node),
                    ));
                    (receipt, expected_event)
                }
            }))
            .await;

            // Put the transaction results in the order they appeared within the block (the order in
            // which we expect to see the events later when we query them from the provider).
            results.sort_by_key(|(receipt, _)| receipt.transaction_index);
            let (receipts, block_events): (Vec<_>, Vec<_>) = results.into_iter().unzip();

            // Sanity check the transactions did get included in the same block.
            assert_eq!(receipts[0].block_number, receipts[1].block_number);
            tracing::info!("two registrations mined in block {block}");

            // Remember the block, and the events we expect to have been emitted, so we can later
            // check against the provider.
            *block = receipts[0].block_number.unwrap();
            events[i] = block_events;
        }
        assert_ne!(blocks[0], blocks[1]);

        // Now we have two different blocks with two events each. Retrieve the events and see if it
        // matches.
        let events_from_provider = get_events(
            &provider,
            Filter::new().from_block(blocks[0]),
            deployment.stake_table_addr,
            deployment.reward_claim_addr,
        )
        .await
        .unwrap();
        tracing::info!("retrieved events from provider: {events_from_provider:#?}");
        assert_eq!(events_from_provider.len(), 2);

        let (id, (_, block_events)) = events_from_provider.first_key_value().unwrap();
        assert_eq!(id.number, blocks[0]);
        assert_eq!(block_events.len(), 2);
        assert_events_eq(&block_events[0], &events[0][0]);
        assert_events_eq(&block_events[1], &events[0][1]);

        let (id, (_, block_events)) = events_from_provider.last_key_value().unwrap();
        assert_eq!(id.number, blocks[1]);
        assert_eq!(block_events.len(), 2);
        assert_events_eq(&block_events[0], &events[1][0]);
        assert_events_eq(&block_events[1], &events[1][1]);
    }
}
