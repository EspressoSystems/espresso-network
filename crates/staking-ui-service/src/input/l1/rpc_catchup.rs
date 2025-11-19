//! L1 catchup based on a JSON-RPC provider.

use std::{cmp::min, collections::BTreeMap};

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
    async fn fast_forward(
        &self,
        from: u64,
    ) -> Result<BTreeMap<L1BlockId, (Timestamp, Vec<L1Event>)>> {
        let finalized = self
            .provider
            .get_block(BlockId::finalized())
            .await
            .context(|| Error::internal().context("getting finalized block"))?;
        let Some(finalized) = finalized else {
            // If there is no finalized L1 block yet, then our starting block `from` cannot possibly
            // be behind. This occurs when the L1 has just been started, as in testing environments.
            tracing::info!("no finalized L1 block");
            return Ok(Default::default());
        };
        if finalized.number() <= from {
            tracing::info!(from, ?finalized, "current state is not behind");
            return Ok(Default::default());
        }

        // Fetch events from the starting block to the new finalized block.
        tracing::info!(
            from,
            to = finalized.number(),
            "fetching L1 events for catchup"
        );

        // To avoid making large RPC calls, divide the range into smaller chunks.
        let chunks = block_range_chunks(from + 1, finalized.number(), self.chunk_size);

        let mut events = BTreeMap::new();
        for (from, to) in chunks {
            tracing::debug!(from, to, "fetch L1 events in chunk");
            let chunk_events = get_events(
                &self.provider,
                Filter::new().from_block(from).to_block(to),
                self.stake_table_addr,
                self.reward_addr,
            )
            .await?;
            events.extend(chunk_events);
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
    use futures::StreamExt;
    use pretty_assertions::assert_eq;
    use tokio::time::sleep;

    use crate::input::l1::{
        ResettableStream, RpcStream,
        testing::{BackgroundStakeTableOps, ContractDeployment, assert_events_eq},
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
        let events_from_stream = stream.take(end_block as usize).collect::<Vec<_>>().await;
        tracing::info!(
            "finished streaming events from {} blocks",
            events_from_stream.len()
        );

        // Fast-forward to the finalized block.
        let catchup = RpcCatchup::new(&options).unwrap();
        let mut catchup_events = catchup.fast_forward(0).await.unwrap();
        // let catchup_finalized = catchup_events.last().unwrap();
        // let events = catchup_events.iter().flat_map(|(_, _, events)| events);
        tracing::info!(
            "fast forwarded events from {} non-empty blocks",
            catchup_events.len()
        );

        // We get the same events either way.
        for input in events_from_stream {
            // We don't skip any inputs during catchup, except empty ones.
            // Remove the input from `catchup_events` so that we can check that it does not contain
            // any extra inputs (at the end it should be empty).
            let Some((timestamp, events)) = catchup_events.remove(&input.block) else {
                assert!(
                    input.events.is_empty(),
                    "missing input with non-empty events {input:?}"
                );
                continue;
            };
            assert_eq!(timestamp, input.timestamp);
            assert_eq!(input.events.len(), events.len());
            for (i, (event, event_from_stream)) in input.events.iter().zip(&events).enumerate() {
                tracing::info!("checking events at input {:?} position {i}", input.block);
                assert_events_eq(event, event_from_stream);
            }
        }
    }
}
