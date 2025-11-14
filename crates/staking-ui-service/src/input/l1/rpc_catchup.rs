//! L1 catchup based on a JSON-RPC provider.

use std::cmp::min;

use alloy::{
    eips::BlockId,
    providers::{Provider, RootProvider},
    rpc::types::Filter,
};

use crate::{
    Error, Result,
    error::ResultExt,
    input::l1::{L1Catchup, L1Event, options::L1ClientOptions, provider::get_events},
    types::common::{Address, L1BlockId, Timestamp},
};

/// L1 catchup based on a JSON-RPC provider.
pub struct RpcCatchup {
    provider: RootProvider,
    stake_table_addr: Address,
    reward_addr: Address,
    chunk_size: u64,
}

impl RpcCatchup {
    /// A catchup provider configured from the CLI.
    pub fn new(opt: &L1ClientOptions) -> Result<Self> {
        let provider = opt.provider()?.0;
        Ok(Self {
            provider,
            stake_table_addr: opt.stake_table_address,
            reward_addr: opt.reward_contract_address,
            chunk_size: opt.l1_events_max_block_range,
        })
    }
}

impl L1Catchup for RpcCatchup {
    async fn fast_forward(&self, from: u64) -> Result<Vec<(L1BlockId, Timestamp, Vec<L1Event>)>> {
        let finalized = self
            .provider
            .get_block(BlockId::finalized())
            .await
            .context(|| Error::internal().context("getting finalized block"))?;
        let Some(finalized) = finalized else {
            // If there is no finalized L1 block yet, then our starting block `from` cannot possibly
            // be behind. This occurs when the L1 has just been started, as in testing environments.
            tracing::info!("no finalized L1 block");
            return Ok(vec![]);
        };
        if finalized.number() <= from {
            tracing::info!(from, ?finalized, "current state is not behind");
            return Ok(vec![]);
        }

        // Fetch events from the starting block to the new finalized block.
        tracing::info!(
            from,
            to = finalized.number(),
            "fetching L1 events for catchup"
        );

        // To avoid making large RPC calls, divide the range into smaller chunks.
        let chunks = block_range_chunks(from + 1, finalized.number(), self.chunk_size);

        let mut events = vec![];
        for (from, to) in chunks {
            tracing::debug!(from, to, "fetch L1 events in chunk");
            let chunk_events = get_events(
                &self.provider,
                Filter::new().from_block(from).to_block(to),
                self.stake_table_addr,
                self.reward_addr,
            )
            .await?;
            for (id, timestamp, event) in chunk_events {
                events.push((id, timestamp, vec![event]));
            }
        }

        Ok(events)
    }
}

fn block_range_chunks(
    from_block: u64,
    to_block: u64,
    chunk_size: u64,
) -> impl Iterator<Item = (u64, u64)> {
    let mut start = from_block;
    let end = to_block;
    std::iter::from_fn(move || {
        let chunk_end = min(start + chunk_size - 1, end);
        if chunk_end < start {
            return None;
        }
        let chunk = (start, chunk_end);
        start = chunk_end + 1;
        Some(chunk)
    })
}

#[cfg(test)]
mod test {
    use std::{
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        time::Duration,
    };

    use alloy::node_bindings::Anvil;
    use futures::{StreamExt, stream};
    use hotshot_contract_adapter::sol_types::StakeTableV2::StakeTableV2Events;
    use pretty_assertions::assert_eq;
    use tokio::time::sleep;

    use crate::input::l1::{
        ResettableStream, RpcStream,
        testing::{BackgroundStakeTableOps, ContractDeployment},
    };

    use super::*;

    #[test_log::test]
    fn test_block_range_chunks_exact_multiple() {
        let chunks = block_range_chunks(0, 3, 2).collect::<Vec<_>>();
        assert_eq!(chunks, vec![(0, 1), (2, 3)]);
    }

    #[test_log::test]
    fn test_block_range_chunks_partial_chunk() {
        let chunks = block_range_chunks(0, 4, 2).collect::<Vec<_>>();
        assert_eq!(chunks, vec![(0, 1), (2, 3), (4, 4)]);
    }

    #[test_log::test]
    fn test_block_range_chunks_small() {
        let chunks = block_range_chunks(1, 1, 10).collect::<Vec<_>>();
        assert_eq!(chunks, vec![(1, 1)]);
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fast_forward_not_behind() {
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let deployment = ContractDeployment::deploy(anvil.endpoint_url())
            .await
            .unwrap();
        let options = L1ClientOptions {
            http_providers: vec![anvil.endpoint_url()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            ..Default::default()
        };
        let catchup = RpcCatchup::new(&options).unwrap();
        let res = catchup.fast_forward(1000000).await.unwrap();
        assert!(res.is_empty(), "expected empty list, got {res:?}");
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_fast_forward_consistency() {
        // Spawn an L1 and generate a bunch of events.
        let anvil = Anvil::new()
            .block_time(1)
            .args(["--slots-in-an-epoch", "0"])
            .spawn();
        let rpc_url = anvil.endpoint_url();

        let deployment = ContractDeployment::deploy(rpc_url.clone()).await.unwrap();
        let options = L1ClientOptions {
            http_providers: vec![rpc_url.clone()],
            stake_table_address: deployment.stake_table_addr,
            reward_contract_address: deployment.reward_claim_addr,
            l1_events_max_block_range: 3,
            ..Default::default()
        };
        let provider = options.provider().unwrap().0;

        // Generate events for about 100 blocks.
        let cancel: Arc<AtomicBool> = Arc::new(false.into());
        let task = BackgroundStakeTableOps::spawn(
            rpc_url,
            deployment.stake_table_addr,
            cancel.clone(),
            None,
        );
        sleep(Duration::from_secs(100)).await;
        cancel.store(true, Ordering::SeqCst);
        task.join().await;
        let end_block = provider.get_block_number().await.unwrap();
        tracing::info!(end_block, "finished generating events");

        // Stream events from genesis.
        let mut stream = RpcStream::new(options.clone()).await.unwrap();
        stream.reset(0).await;
        let events_from_stream = stream
            .take(end_block as usize)
            .flat_map(|input| stream::iter(input.events))
            .collect::<Vec<_>>()
            .await;
        tracing::info!("finished streaming {} events", events_from_stream.len());

        // Fast-forward to the finalized block.
        let catchup = RpcCatchup::new(&options).unwrap();
        let catchup_events = catchup.fast_forward(0).await.unwrap();
        let catchup_finalized = catchup_events.last().unwrap();
        let events = catchup_events.iter().flat_map(|(_, _, events)| events);

        // We get the same events either way.
        assert_eq!(catchup_events.len(), events_from_stream.len());
        for (i, (event, event_from_stream)) in
            events.into_iter().zip(events_from_stream).enumerate()
        {
            // Workaround for events not implementing [`PartialEq`].
            match (event, event_from_stream) {
                (L1Event::StakeTable(l), L1Event::StakeTable(r)) => {
                    match (l.as_ref(), r.as_ref()) {
                        (
                            StakeTableV2Events::ValidatorRegistered(l),
                            StakeTableV2Events::ValidatorRegistered(r),
                        ) => {
                            assert_eq!(l.account, r.account);
                            assert_eq!(l.blsVk, r.blsVk);
                            assert_eq!(l.schnorrVk, r.schnorrVk);
                            assert_eq!(l.commission, r.commission);
                        }
                        (
                            StakeTableV2Events::ValidatorRegisteredV2(l),
                            StakeTableV2Events::ValidatorRegisteredV2(r),
                        ) => {
                            assert_eq!(l.account, r.account);
                            assert_eq!(l.blsVK, r.blsVK);
                            assert_eq!(l.schnorrVK, r.schnorrVK);
                            assert_eq!(l.commission, r.commission);
                        }
                        (
                            StakeTableV2Events::ValidatorExit(l),
                            StakeTableV2Events::ValidatorExit(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (StakeTableV2Events::Delegated(l), StakeTableV2Events::Delegated(r)) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::Undelegated(l),
                            StakeTableV2Events::Undelegated(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::ConsensusKeysUpdated(l),
                            StakeTableV2Events::ConsensusKeysUpdated(r),
                        ) => {
                            assert_eq!(l.account, r.account);
                            assert_eq!(l.blsVK, r.blsVK);
                            assert_eq!(l.schnorrVK, r.schnorrVK);
                        }
                        (
                            StakeTableV2Events::ConsensusKeysUpdatedV2(l),
                            StakeTableV2Events::ConsensusKeysUpdatedV2(r),
                        ) => {
                            assert_eq!(l.account, r.account);
                            assert_eq!(l.blsVK, r.blsVK);
                            assert_eq!(l.schnorrVK, r.schnorrVK);
                        }
                        (
                            StakeTableV2Events::CommissionUpdated(l),
                            StakeTableV2Events::CommissionUpdated(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::ExitEscrowPeriodUpdated(l),
                            StakeTableV2Events::ExitEscrowPeriodUpdated(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::MaxCommissionIncreaseUpdated(l),
                            StakeTableV2Events::MaxCommissionIncreaseUpdated(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::MinCommissionUpdateIntervalUpdated(l),
                            StakeTableV2Events::MinCommissionUpdateIntervalUpdated(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::OwnershipTransferred(l),
                            StakeTableV2Events::OwnershipTransferred(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (StakeTableV2Events::Paused(l), StakeTableV2Events::Paused(r)) => {
                            assert_eq!(l, r);
                        }
                        (StakeTableV2Events::Unpaused(l), StakeTableV2Events::Unpaused(r)) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::Initialized(l),
                            StakeTableV2Events::Initialized(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::RoleAdminChanged(l),
                            StakeTableV2Events::RoleAdminChanged(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::RoleGranted(l),
                            StakeTableV2Events::RoleGranted(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (
                            StakeTableV2Events::RoleRevoked(l),
                            StakeTableV2Events::RoleRevoked(r),
                        ) => {
                            assert_eq!(l, r);
                        }
                        (StakeTableV2Events::Upgraded(l), StakeTableV2Events::Upgraded(r)) => {
                            assert_eq!(l, r);
                        }
                        (StakeTableV2Events::Withdrawal(l), StakeTableV2Events::Withdrawal(r)) => {
                            assert_eq!(l, r);
                        }
                        (l, r) => {
                            panic!(
                                "mismatched stake table events at position {i}:\n{l:#?}\n{r:#?}"
                            );
                        }
                    }
                }
                (L1Event::Reward(l), L1Event::Reward(r)) => {
                    assert_eq!(*l, r);
                }
                (l, r) => {
                    panic!(
                        "Reward event mismatched with stake table event at position {i}:\n{l:#?}\n{r:#?}"
                    );
                }
            }
        }

        // Check consistency of the block info returned by catchup.
        let (id, timestamp, _) = catchup_finalized;
        let block = provider.get_block(id.number.into()).await.unwrap().unwrap();
        assert_eq!(block.hash(), id.hash);
        assert_eq!(block.header.parent_hash, id.parent);
        assert_eq!(block.header.timestamp, *timestamp);

        // Ensure block is finalized.
        let finalized = provider
            .get_block(BlockId::finalized())
            .await
            .unwrap()
            .unwrap();
        assert!(finalized.number() >= id.number);
    }
}
