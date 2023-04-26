use async_std::{sync::Arc, task::sleep};
use clap::Args;
use contract_bindings::HotShot;
use ethers::types::Address;
use ethers::{prelude::*, providers::Middleware as _, signers::coins_bip39::English};
use futures::{future::FutureExt, stream::StreamExt};
use hotshot_query_service::{availability::LeafQueryData, Block, Deltas, Resolvable};
use hotshot_types::traits::node_implementation::NodeImplementation;
use sequencer_utils::{commitment_to_u256, contract_send, Middleware};
use std::time::Duration;
use surf_disco::Url;

use crate::{network, Node, SeqTypes};

const RETRY_DELAY: Duration = Duration::from_secs(1);

type HotShotClient = surf_disco::Client<hotshot_query_service::Error>;

#[derive(Args, Clone, Debug)]
pub struct HotShotContractOptions {
    /// URL of layer 1 Ethereum JSON-RPC provider.
    #[clap(long, env = "ESPRESSO_ZKEVM_L1_PROVIDER")]
    pub l1_provider: Url,

    /// Chain ID for layer 1 Ethereum.
    ///
    /// This can be specified explicitly as a sanity check. No transactions will be executed if the
    /// RPC specified by `l1_provider` has a different chain ID. If not specified, the chain ID from
    /// the RPC will be used.
    #[clap(long, env = "ESPRESSO_ZKEVM_L1_CHAIN_ID")]
    pub l1_chain_id: Option<u64>,

    /// Address of HotShot contract on layer 1.
    #[clap(long, env = "ESPRESSO_ZKEVM_HOTSHOT_ADDRESS", default_value = None)]
    pub hotshot_address: Address,

    /// Mnemonic phrase for the sequencer wallet.
    ///
    /// This is the wallet that will be used to send blocks sequenced by HotShot to the rollup
    /// contract. It must be funded with ETH and MATIC on layer 1.
    #[clap(long, env = "ESPRESSO_ZKEVM_SEQUENCER_MNEMONIC", default_value = None)]
    pub sequencer_mnemonic: String,

    /// URL of HotShot Query Service
    ///
    /// If unspecified, defaults to the query service internal to the sequencer process.
    pub query_service_url: Url,
}

pub async fn run_hotshot_commitment_task(opt: &HotShotContractOptions) {
    let query_service_url = opt.query_service_url.join("availability").unwrap();
    let hotshot = HotShotClient::new(query_service_url);
    hotshot.connect(None).await;

    // Connect to the layer one HotShot contract.
    let Some(l1) = connect_l1(opt).await else {
        tracing::error!("unable to connect to L1, hotshot commitment task exiting");
        return;
    };
    let contract = HotShot::new(opt.hotshot_address, l1.clone());

    // Get the last block number sequenced.
    let from = match contract.block_height().call().await {
        Ok(from) => from.as_u64(),
        Err(err) => {
            tracing::error!("unable to read block_height from contract: {}", err);
            tracing::error!("hotshot commitment task will exit");
            return;
        }
    };
    tracing::info!("last block sequenced: {}", from);

    // Get the maximum number of blocks the contract will allow at a time.
    let max = match contract.max_blocks().call().await {
        Ok(max) => max.as_u64(),
        Err(err) => {
            tracing::error!("unable to read max_blocks from contract: {}", err);
            tracing::error!("hotshot commitment task will exit");
            return;
        }
    };
    sequence::<Node<network::Centralized>>(from, max, hotshot, contract).await;
}

async fn sequence<I: NodeImplementation<SeqTypes>>(
    from: u64,
    max_blocks: u64,
    hotshot: HotShotClient,
    contract: HotShot<Middleware>,
) where
    Deltas<SeqTypes, I>: Resolvable<Block<SeqTypes>>,
{
    let mut leaves = match hotshot
        .socket(&format!("stream/leaves/{from}"))
        .subscribe()
        .await
    {
        Ok(leaves) => Box::pin(leaves.peekable()),
        Err(err) => {
            tracing::error!("unable to subscribe to HotShot query service: {}", err);
            tracing::error!("hotshot commitment task will exit");
            return;
        }
    };

    loop {
        // Wait for HotShot to sequence a block.
        let leaf: LeafQueryData<SeqTypes, I> = match leaves.next().await {
            Some(Ok(leaf)) => leaf,
            Some(Err(err)) => {
                tracing::error!("error from HotShot, retrying: {}", err);
                continue;
            }
            None => {
                tracing::error!("HotShot leaf stream ended, hotshot commitment task will exit");
                return;
            }
        };
        tracing::info!("received leaf from HotShot: {:?}", leaf);

        // It is possible that multiple blocks are already available, if HotShot is running faster
        // than we are. Collect as many blocks as are ready (up to the allowed maximum) so we can
        // send them all to the contract at once to save a little gas.
        let mut to_sequence = vec![leaf];
        while to_sequence.len() + 1 < max_blocks as usize {
            if let Some(Some(Ok(leaf))) = leaves.as_mut().peek().now_or_never() {
                tracing::info!("an additional block is also ready: {:?}", leaf);
                // Since the block has been peeked, we can remove it from the stream with `next()`,
                // this should never block or return `None`.
                to_sequence.push(
                    leaves
                        .next()
                        .await
                        .expect("next() returned None after peek() returned Some")
                        .expect("next() returned Some(Err) after peek() returned Some(Ok)"),
                );
            } else {
                break;
            }
        }
        tracing::info!("sequencing {}/{} blocks", to_sequence.len(), max_blocks,);

        // Sequence the blocks.
        sequence_batches(&contract, to_sequence).await;
    }
}

async fn sequence_batches<I: NodeImplementation<SeqTypes>>(
    contract: &HotShot<Middleware>,
    leaves: impl IntoIterator<Item = LeafQueryData<SeqTypes, I>>,
) where
    Deltas<SeqTypes, I>: Resolvable<Block<SeqTypes>>,
{
    let (block_comms, qcs): (Vec<_>, Vec<_>) = leaves
        .into_iter()
        .map(|leaf| {
            (
                commitment_to_u256(leaf.block_hash()),
                Bytes::from(bincode::serialize(&leaf.qc()).unwrap()),
            )
        })
        .unzip();

    // Send teh block commitments and QCs to L1. This operation must succeed before we go any
    // further, because sequencing the next batch will depend on having successfully sequenced this
    // one. Thus we will retry until it succeeds.
    while contract_send(contract.new_blocks(block_comms.clone(), qcs.clone()))
        .await
        .is_none()
    {
        tracing::warn!("failed to sequence batches, retrying");
        sleep(RETRY_DELAY).await;
    }
}

pub async fn connect_l1(opt: &HotShotContractOptions) -> Option<Arc<Middleware>> {
    connect_rpc(&opt.l1_provider, &opt.sequencer_mnemonic, opt.l1_chain_id).await
}

async fn connect_rpc(
    provider: &Url,
    mnemonic: &str,
    chain_id: Option<u64>,
) -> Option<Arc<Middleware>> {
    let provider = match Provider::try_from(provider.to_string()) {
        Ok(provider) => provider,
        Err(err) => {
            tracing::error!("error connecting to RPC {}: {}", provider, err);
            return None;
        }
    };
    let chain_id = match chain_id {
        Some(id) => id,
        None => match provider.get_chainid().await {
            Ok(id) => id.as_u64(),
            Err(err) => {
                tracing::error!("error getting chain ID: {}", err);
                return None;
            }
        },
    };
    let wallet = match MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .build()
    {
        Ok(wallet) => wallet,
        Err(err) => {
            tracing::error!("error opening wallet: {}", err);
            return None;
        }
    };
    let wallet = wallet.with_chain_id(chain_id);
    let address = wallet.address();
    Some(Arc::new(NonceManagerMiddleware::new(
        SignerMiddleware::new(provider, wallet),
        address,
    )))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::{transaction::SequencerTransaction, Block, Leaf, Transaction};
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use commit::Committable;
    use contract_bindings::{hot_shot::NewBlocksCall, TestHermezContracts};
    use ethers::{abi::AbiDecode, providers::Middleware};
    use hotshot_types::{
        certificate::QuorumCertificate,
        data::{LeafType, ViewNumber},
        traits::{block_contents::Block as _, election::SignedCertificate, state::ConsensusTime},
    };
    use sequencer_utils::Anvil;

    const TEST_MNEMONIC: &str = "test test test test test test test test test test test junk";

    #[async_std::test]
    async fn test_sequencer_task() {
        setup_logging();
        setup_backtrace();

        let anvil = Anvil::spawn(None).await;
        let l1 = TestHermezContracts::deploy(&anvil.url(), "http://dummy".to_string()).await;

        let l1_initial_block = l1.provider.get_block_number().await.unwrap();
        let initial_batch_num = l1.hotshot.block_height().call().await.unwrap();

        let adaptor_l1_signer = connect_rpc(l1.provider.url(), TEST_MNEMONIC, None)
            .await
            .unwrap();

        // Create a few test batches.
        let num_batches = 2u64;
        let mut leaves: Vec<LeafQueryData<SeqTypes, Node<network::Memory>>> = vec![];
        for _ in 0..num_batches {
            let txn = SequencerTransaction::Wrapped(Transaction::new(1.into(), vec![]));
            let block = Block::new().add_transaction_raw(&txn).unwrap();

            // Fake a leaf that sequences this block.
            let mut qc = QuorumCertificate::genesis();
            let leaf = Leaf::new(ViewNumber::genesis(), qc.clone(), block, Default::default());
            qc.leaf_commitment = leaf.commit();
            leaves.push(LeafQueryData::new(leaf, qc).unwrap());
        }
        tracing::info!("sequencing batches: {:?}", leaves);

        // Connect to the HotShot contract with the expected L1 client.
        let hotshot = HotShot::new(l1.hotshot.address(), adaptor_l1_signer);

        // Sequence them in the HotShot contract.
        sequence_batches(&hotshot, leaves.clone()).await;

        // Check the NewBatches event.
        let (event, meta) = l1
            .hotshot
            .new_blocks_filter()
            .from_block(l1_initial_block)
            .query_with_meta()
            .await
            .unwrap()
            .remove(0);
        assert_eq!(event.first_block_number, initial_batch_num);

        let calldata = l1
            .provider
            .get_transaction(meta.transaction_hash)
            .await
            .unwrap()
            .unwrap()
            .input;
        let call = NewBlocksCall::decode(calldata).unwrap();
        assert_eq!(
            call.new_commitments,
            leaves
                .iter()
                .map(|leaf| U256::from_little_endian(&<[u8; 32]>::from(leaf.block_hash())))
                .collect::<Vec<_>>()
        );
    }
}
