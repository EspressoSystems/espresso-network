use crate::{
    block::{entry::TxTableEntryWord, tables::NameSpaceTable, NsTable},
    chain_config::ResolvableChainConfig,
    eth_signature_key::EthKeyPair,
    l1_client::L1Snapshot,
    state::{BlockMerkleCommitment, FeeAccount, FeeAmount, FeeInfo, FeeMerkleCommitment},
    ChainConfig, L1BlockInfo, Leaf, NodeState, SeqTypes, ValidatedState,
};
use ark_serialize::CanonicalSerialize;

use committable::{Commitment, Committable, RawCommitmentBuilder};
use ethers::types;
use hotshot_query_service::availability::QueryableHeader;
use hotshot_types::{
    traits::{
        block_contents::{BlockHeader, BlockPayload, BuilderFee},
        node_implementation::NodeType,
        signature_key::BuilderSignatureKey,
        ValidatedState as HotShotState,
    },
    utils::BuilderCommitment,
    vid::VidCommitment,
};
use jf_primitives::merkle_tree::prelude::*;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// A header is like a [`Block`] with the body replaced by a digest.
#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Header {
    /// A commitment to a ChainConfig or a full ChainConfig.
    pub chain_config: ResolvableChainConfig,

    pub height: u64,
    pub timestamp: u64,

    /// The Espresso block header includes a reference to the current head of the L1 chain.
    ///
    /// Rollups can use this to facilitate bridging between the L1 and L2 in a deterministic way.
    /// This field deterministically associates an L2 block with a recent L1 block the instant the
    /// L2 block is sequenced. Rollups can then define the L2 state after this block as the state
    /// obtained by executing all the transactions in this block _plus_ all the L1 deposits up to
    /// the given L1 block number. Since there is no need to wait for the L2 block to be reflected
    /// on the L1, this bridge design retains the low confirmation latency of HotShot.
    ///
    /// This block number indicates the unsafe head of the L1 chain, so it is subject to reorgs. For
    /// this reason, the Espresso header does not include any information that might change in a
    /// reorg, such as the L1 block timestamp or hash. It includes only the L1 block number, which
    /// will always refer to _some_ block after a reorg: if the L1 head at the time this block was
    /// sequenced gets reorged out, the L1 chain will eventually (and probably quickly) grow to the
    /// same height once again, and a different block will exist with the same height. In this way,
    /// Espresso does not have to handle L1 reorgs, and the Espresso blockchain will always be
    /// reflective of the current state of the L1 blockchain. Rollups that use this block number
    /// _do_ have to handle L1 reorgs, but each rollup and each rollup client can decide how many
    /// confirmations they want to wait for on top of this `l1_head` before they consider an L2
    /// block finalized. This offers a tradeoff between low-latency L1-L2 bridges and finality.
    ///
    /// Rollups that want a stronger guarantee of finality, or that want Espresso to attest to data
    /// from the L1 block that might change in reorgs, can instead use the latest L1 _finalized_
    /// block at the time this L2 block was sequenced: `l1_finalized`.
    pub l1_head: u64,

    /// The Espresso block header includes information a bout the latest finalized L1 block.
    ///
    /// Similar to `l1_head`, rollups can use this information to implement a bridge between the L1
    /// and L2 while retaining the finality of low-latency block confirmations from HotShot. Since
    /// this information describes the finalized L1 block, a bridge using this L1 block will have
    /// much higher latency than a bridge using `l1_head`. In exchange, rollups that use the
    /// finalized block do not have to worry about L1 reorgs, and can inject verifiable attestations
    /// to the L1 block metadata (such as its timestamp or hash) into their execution layers, since
    /// Espresso replicas will sign this information for the finalized L1 block.
    ///
    /// This block may be `None` in the rare case where Espresso has started shortly after the
    /// genesis of the L1, and the L1 has yet to finalize a block. In all other cases it will be
    /// `Some`.
    pub l1_finalized: Option<L1BlockInfo>,

    pub payload_commitment: VidCommitment,
    pub builder_commitment: BuilderCommitment,
    pub ns_table: NameSpaceTable<TxTableEntryWord>,
    /// Root Commitment of Block Merkle Tree
    pub block_merkle_tree_root: BlockMerkleCommitment,
    /// Root Commitment of `FeeMerkleTree`
    pub fee_merkle_tree_root: FeeMerkleCommitment,
    /// Account (etheruem address) of builder
    pub builder_signature: Option<types::Signature>,
    pub fee_info: FeeInfo,
}

impl Committable for Header {
    fn commit(&self) -> Commitment<Self> {
        let mut bmt_bytes = vec![];
        self.block_merkle_tree_root
            .serialize_with_mode(&mut bmt_bytes, ark_serialize::Compress::Yes)
            .unwrap();
        let mut fmt_bytes = vec![];
        self.fee_merkle_tree_root
            .serialize_with_mode(&mut fmt_bytes, ark_serialize::Compress::Yes)
            .unwrap();

        RawCommitmentBuilder::new(&Self::tag())
            .field("chain_config", self.chain_config.commit())
            .u64_field("height", self.height)
            .u64_field("timestamp", self.timestamp)
            .u64_field("l1_head", self.l1_head)
            .optional("l1_finalized", &self.l1_finalized)
            .constant_str("payload_commitment")
            .fixed_size_bytes(self.payload_commitment.as_ref().as_ref())
            .constant_str("builder_commitment")
            .fixed_size_bytes(self.builder_commitment.as_ref())
            .field("ns_table", self.ns_table.commit())
            .var_size_field("block_merkle_tree_root", &bmt_bytes)
            .var_size_field("fee_merkle_tree_root", &fmt_bytes)
            .field("fee_info", self.fee_info.commit())
            .finalize()
    }

    fn tag() -> String {
        // We use the tag "BLOCK" since blocks are identified by the hash of their header. This will
        // thus be more intuitive to users than "HEADER".
        "BLOCK".into()
    }
}

impl Committable for NameSpaceTable<TxTableEntryWord> {
    fn commit(&self) -> Commitment<Self> {
        RawCommitmentBuilder::new(&Self::tag())
            .var_size_bytes(self.get_bytes())
            .finalize()
    }

    fn tag() -> String {
        "NSTABLE".into()
    }
}

impl Header {
    #[allow(clippy::too_many_arguments)]
    // TODO pub or merely pub(super)?
    pub fn from_info(
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
        ns_table: NsTable,
        parent_leaf: &Leaf,
        mut l1: L1Snapshot,
        l1_deposits: &[FeeInfo],
        mut timestamp: u64,
        parent_state: &ValidatedState,
        builder_address: &EthKeyPair,
        chain_config: ChainConfig,
    ) -> Self {
        // Increment height.
        let parent_header = parent_leaf.get_block_header();
        let height = parent_header.height + 1;

        // Ensure the timestamp does not decrease. We can trust `parent.timestamp` because `parent`
        // has already been voted on by consensus. If our timestamp is behind, either f + 1 nodes
        // are lying about the current time, or our clock is just lagging.
        if timestamp < parent_header.timestamp {
            tracing::warn!(
                "Espresso timestamp {timestamp} behind parent {}, local clock may be out of sync",
                parent_header.timestamp
            );
            timestamp = parent_header.timestamp;
        }

        // Ensure the L1 block references don't decrease. Again, we can trust `parent.l1_*` are
        // accurate.
        if l1.head < parent_header.l1_head {
            tracing::warn!(
                "L1 head {} behind parent {}, L1 client may be lagging",
                l1.head,
                parent_header.l1_head
            );
            l1.head = parent_header.l1_head;
        }
        if l1.finalized < parent_header.l1_finalized {
            tracing::warn!(
                "L1 finalized {:?} behind parent {:?}, L1 client may be lagging",
                l1.finalized,
                parent_header.l1_finalized
            );
            l1.finalized = parent_header.l1_finalized;
        }

        // Enforce that the sequencer block timestamp is not behind the L1 block timestamp. This can
        // only happen if our clock is badly out of sync with L1.
        if let Some(l1_block) = &l1.finalized {
            let l1_timestamp = l1_block.timestamp.as_u64();
            if timestamp < l1_timestamp {
                tracing::warn!("Espresso timestamp {timestamp} behind L1 timestamp {l1_timestamp}, local clock may be out of sync");
                timestamp = l1_timestamp;
            }
        }

        let mut state = parent_state.clone();
        state
            .block_merkle_tree
            .push(parent_header.commit())
            .unwrap();
        let block_merkle_tree_root = state.block_merkle_tree.commitment();

        // Insert the new L1 deposits
        for fee_info in l1_deposits {
            state
                .insert_fee_deposit(*fee_info)
                .expect("fee deposit previously verified");
            // TODO: Check LookupResult
        }

        // TODO Check that we have the fee to pay for the block.
        // We currently can't return an error from Header::new.

        let fee_merkle_tree_root = state.fee_merkle_tree.commitment();

        let fee_info = FeeInfo::base_fee(builder_address.address().into());
        let builder_signature = FeeAccount::sign_builder_message(
            builder_address,
            &fee_message(fee_info.amount(), payload_commitment, &ns_table),
        )
        .unwrap();
        Self {
            chain_config: chain_config.commit().into(),
            height,
            timestamp,
            l1_head: l1.head,
            l1_finalized: l1.finalized,
            payload_commitment,
            builder_commitment,
            ns_table,
            fee_merkle_tree_root,
            block_merkle_tree_root,
            fee_info,
            builder_signature: Some(builder_signature),
        }
    }

    /// Message authorizing a fee payment for inclusion of a certain payload.
    ///
    /// This message relates the fee info in this header to the payload corresponding to the header.
    /// The message is signed by the builder (or whoever is paying for inclusion of the block) and
    /// validated by consensus, as authentication for charging the fee to the builder account.
    pub fn fee_message(&self) -> Vec<u8> {
        fee_message(
            self.fee_info.amount(),
            self.payload_commitment,
            self.metadata(),
        )
    }
}

impl BlockHeader<SeqTypes> for Header {
    #[tracing::instrument(
        skip_all,
        fields(view = ?parent_leaf.get_view_number(), height = parent_leaf.get_block_header().height),
    )]
    async fn new(
        parent_state: &ValidatedState,
        instance_state: &NodeState,
        parent_leaf: &Leaf,
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
        metadata: <<SeqTypes as NodeType>::BlockPayload as BlockPayload>::Metadata,
        _builder_fee: BuilderFee<SeqTypes>,
    ) -> Self {
        let mut validated_state = parent_state.clone();

        let accounts = std::iter::once(FeeAccount::from(instance_state.builder_key.address()));

        // Fetch the latest L1 snapshot.
        let l1_snapshot = instance_state.l1_client().snapshot().await;
        // Fetch the new L1 deposits between parent and current finalized L1 block.
        let l1_deposits = if let Some(block_info) = l1_snapshot.finalized {
            instance_state
                .l1_client
                .get_finalized_deposits(
                    parent_leaf
                        .get_block_header()
                        .l1_finalized
                        .map(|block_info| block_info.number),
                    block_info.number,
                )
                .await
        } else {
            vec![]
        };
        // Find missing fee state entries
        let missing_accounts = parent_state
            .forgotten_accounts(accounts.chain(l1_deposits.iter().map(|info| info.account())));
        if !missing_accounts.is_empty() {
            tracing::warn!("fetching missing accounts {missing_accounts:?} from peers");

            // Fetch missing fee state entries
            // Unwrapping here is okay as we retry until we get the accounts or until the task is canceled.
            let missing_account_proofs = instance_state
                .peers
                .as_ref()
                .fetch_accounts(
                    parent_leaf.get_view_number(),
                    parent_state.fee_merkle_tree.commitment(),
                    missing_accounts,
                )
                .await
                .unwrap();

            // Insert missing fee state entries
            for account in missing_account_proofs.iter() {
                account
                    .proof
                    .remember(&mut validated_state.fee_merkle_tree)
                    .expect("proof previously verified");
            }
        }

        // Ensure merkle tree has frontier
        if validated_state.need_to_fetch_blocks_mt_frontier() {
            let view = parent_leaf.get_view_number();
            tracing::warn!("fetching block frontier for view {view:?} from peers");
            instance_state
                .peers
                .as_ref()
                .remember_blocks_merkle_tree(view, &mut validated_state.block_merkle_tree)
                .await
                .expect("failed to remember proof");
        }

        Self::from_info(
            payload_commitment,
            builder_commitment,
            metadata,
            parent_leaf,
            l1_snapshot,
            &l1_deposits,
            OffsetDateTime::now_utc().unix_timestamp() as u64,
            &validated_state,
            &instance_state.builder_key,
            instance_state.chain_config,
        )
    }

    fn genesis(
        instance_state: &NodeState,
        payload_commitment: VidCommitment,
        builder_commitment: BuilderCommitment,
        ns_table: <<SeqTypes as NodeType>::BlockPayload as BlockPayload>::Metadata,
    ) -> Self {
        let ValidatedState {
            fee_merkle_tree,
            block_merkle_tree,
        } = ValidatedState::genesis(instance_state).0;
        let block_merkle_tree_root = block_merkle_tree.commitment();
        let fee_merkle_tree_root = fee_merkle_tree.commitment();

        Self {
            // The genesis header needs to be completely deterministic, so we can't sample real
            // timestamps or L1 values.
            chain_config: instance_state.chain_config.into(),
            height: 0,
            timestamp: 0,
            l1_head: 0,
            l1_finalized: None,
            payload_commitment,
            builder_commitment,
            ns_table,
            block_merkle_tree_root,
            fee_merkle_tree_root,
            fee_info: FeeInfo::genesis(),
            builder_signature: None,
        }
    }

    fn block_number(&self) -> u64 {
        self.height
    }

    fn payload_commitment(&self) -> VidCommitment {
        self.payload_commitment
    }

    fn metadata(&self) -> &<<SeqTypes as NodeType>::BlockPayload as BlockPayload>::Metadata {
        &self.ns_table
    }

    /// Commit over fee_amount, payload_commitment and metadata
    fn builder_commitment(&self) -> BuilderCommitment {
        self.builder_commitment.clone()
    }
}

fn fee_message(
    amount: FeeAmount,
    payload_commitment: VidCommitment,
    metadata: &<<SeqTypes as NodeType>::BlockPayload as BlockPayload>::Metadata,
) -> Vec<u8> {
    let mut data = vec![];
    data.extend(amount.to_fixed_bytes());
    data.extend::<&[u8]>(payload_commitment.as_ref().as_ref());
    data.extend::<&[u8]>(metadata.commit().as_ref());
    data
}

impl QueryableHeader<SeqTypes> for Header {
    fn timestamp(&self) -> u64 {
        self.timestamp
    }
}

#[cfg(test)]
mod test_headers {
    use std::sync::Arc;

    use super::*;
    use crate::{
        catchup::mock::MockStateCatchup,
        eth_signature_key::EthKeyPair,
        l1_client::L1Client,
        state::{
            apply_proposal, get_l1_deposits, validate_proposal, BlockMerkleTree, Delta,
            FeeMerkleTree,
        },
        NodeState,
    };
    use async_compatibility_layer::logging::{setup_backtrace, setup_logging};
    use ethers::{
        types::{Address, U256},
        utils::Anvil,
    };
    use hotshot_types::traits::signature_key::BuilderSignatureKey;

    #[derive(Debug, Default)]
    #[must_use]
    struct TestCase {
        // Parent header info.
        parent_timestamp: u64,
        parent_l1_head: u64,
        parent_l1_finalized: Option<L1BlockInfo>,

        // Environment at the time the new header is created.
        l1_head: u64,
        l1_finalized: Option<L1BlockInfo>,
        timestamp: u64,
        l1_deposits: Vec<FeeInfo>,

        // Expected new header info.
        expected_timestamp: u64,
        expected_l1_head: u64,
        expected_l1_finalized: Option<L1BlockInfo>,
    }

    impl TestCase {
        fn run(self) {
            setup_logging();
            setup_backtrace();

            // Check test case validity.
            assert!(self.expected_timestamp >= self.parent_timestamp);
            assert!(self.expected_l1_head >= self.parent_l1_head);
            assert!(self.expected_l1_finalized >= self.parent_l1_finalized);

            let genesis = GenesisForTest::default();
            let mut parent = genesis.header.clone();
            parent.timestamp = self.parent_timestamp;
            parent.l1_head = self.parent_l1_head;
            parent.l1_finalized = self.parent_l1_finalized;

            let mut parent_leaf = genesis.leaf.clone();
            *parent_leaf.get_block_header_mut() = parent.clone();

            let block_merkle_tree =
                BlockMerkleTree::from_elems(Some(32), Vec::<Commitment<Header>>::new()).unwrap();

            let fee_info = FeeInfo::genesis();
            let fee_merkle_tree = FeeMerkleTree::from_kv_set(
                20,
                Vec::from([(fee_info.account(), fee_info.amount())]),
            )
            .unwrap();
            let mut validated_state = ValidatedState {
                block_merkle_tree: block_merkle_tree.clone(),
                fee_merkle_tree,
            };

            let header = Header::from_info(
                genesis.header.payload_commitment,
                genesis.header.builder_commitment,
                genesis.ns_table,
                &parent_leaf,
                L1Snapshot {
                    head: self.l1_head,
                    finalized: self.l1_finalized,
                },
                &self.l1_deposits,
                self.timestamp,
                &validated_state,
                &genesis.instance_state.builder_key,
                genesis.instance_state.chain_config,
            );
            assert_eq!(header.height, parent.height + 1);
            assert_eq!(header.timestamp, self.expected_timestamp);
            assert_eq!(header.l1_head, self.expected_l1_head);
            assert_eq!(header.l1_finalized, self.expected_l1_finalized);

            // Check deposits were inserted before computing the fee merkle tree root.
            for fee_info in self.l1_deposits {
                validated_state.insert_fee_deposit(fee_info).unwrap();
            }
            assert_eq!(
                validated_state.fee_merkle_tree.commitment(),
                header.fee_merkle_tree_root,
            );

            assert_eq!(
                block_merkle_tree,
                BlockMerkleTree::from_elems(Some(32), Vec::<Commitment<Header>>::new()).unwrap()
            );
        }
    }

    fn l1_block(number: u64) -> L1BlockInfo {
        L1BlockInfo {
            number,
            ..Default::default()
        }
    }

    #[test]
    fn test_new_header() {
        // Simplest case: building on genesis, L1 info and timestamp unchanged.
        TestCase::default().run()
    }

    #[test]
    fn test_new_header_advance_timestamp() {
        TestCase {
            timestamp: 1,
            expected_timestamp: 1,
            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_advance_l1_block() {
        TestCase {
            parent_l1_head: 0,
            parent_l1_finalized: Some(l1_block(0)),

            l1_head: 1,
            l1_finalized: Some(l1_block(1)),

            expected_l1_head: 1,
            expected_l1_finalized: Some(l1_block(1)),

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_advance_l1_finalized_from_none() {
        TestCase {
            l1_finalized: Some(l1_block(1)),
            expected_l1_finalized: Some(l1_block(1)),
            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_timestamp_behind_finalized_l1_block() {
        let l1_finalized = Some(L1BlockInfo {
            number: 1,
            timestamp: 1.into(),
            ..Default::default()
        });
        TestCase {
            l1_head: 1,
            l1_finalized,
            timestamp: 0,

            expected_l1_head: 1,
            expected_l1_finalized: l1_finalized,
            expected_timestamp: 1,

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_timestamp_behind() {
        TestCase {
            parent_timestamp: 1,
            timestamp: 0,
            expected_timestamp: 1,

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_l1_head_behind() {
        TestCase {
            parent_l1_head: 1,
            l1_head: 0,
            expected_l1_head: 1,

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_l1_finalized_behind_some() {
        TestCase {
            parent_l1_finalized: Some(l1_block(1)),
            l1_finalized: Some(l1_block(0)),
            expected_l1_finalized: Some(l1_block(1)),

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_l1_finalized_behind_none() {
        TestCase {
            parent_l1_finalized: Some(l1_block(0)),
            l1_finalized: None,
            expected_l1_finalized: Some(l1_block(0)),

            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_deposits_one() {
        TestCase {
            l1_deposits: vec![FeeInfo::new(Address::default(), 1)],
            ..Default::default()
        }
        .run()
    }

    #[test]
    fn test_new_header_deposits_many() {
        TestCase {
            l1_deposits: [
                (Address::default(), 1),
                (Address::default(), 2),
                (Address::random(), 3),
            ]
            .iter()
            .map(|(address, amount)| FeeInfo::new(*address, *amount))
            .collect(),
            ..Default::default()
        }
        .run()
    }

    struct GenesisForTest {
        pub instance_state: NodeState,
        pub validated_state: ValidatedState,
        pub leaf: Leaf,
        pub header: Header,
        pub ns_table: NameSpaceTable<TxTableEntryWord>,
    }

    impl Default for GenesisForTest {
        fn default() -> Self {
            let instance_state = NodeState::mock();
            let validated_state = ValidatedState::genesis(&instance_state).0;
            let leaf = Leaf::genesis(&instance_state);
            let header = leaf.get_block_header().clone();
            let ns_table = leaf.get_block_payload().unwrap().get_ns_table().clone();
            Self {
                instance_state,
                validated_state,
                leaf,
                header,
                ns_table,
            }
        }
    }

    #[test]
    fn test_validate_proposal_error_cases() {
        let genesis = GenesisForTest::default();

        let mut validated_state = ValidatedState::default();
        let mut block_merkle_tree = validated_state.block_merkle_tree.clone();

        let mut parent_header = genesis.header.clone();
        let mut parent_leaf = genesis.leaf.clone();
        *parent_leaf.get_block_header_mut() = parent_header.clone();

        // Populate the tree with an initial `push`.
        block_merkle_tree.push(genesis.header.commit()).unwrap();
        let block_merkle_tree_root = block_merkle_tree.commitment();
        validated_state.block_merkle_tree = block_merkle_tree.clone();
        parent_header.block_merkle_tree_root = block_merkle_tree_root;
        let mut proposal = parent_header.clone();

        let mut delta = Delta::default();
        // Pass a different chain config to trigger a chain config validation error.
        let state = apply_proposal(&validated_state, &mut delta, &parent_leaf, vec![]);

        let result = validate_proposal(
            &state,
            ChainConfig::new(U256::zero(), 0u64, U256::zero()),
            &parent_leaf,
            &proposal,
        )
        .unwrap_err();

        assert!(format!("{}", result.root_cause()).starts_with("Invalid Chain Config:"));

        // Advance `proposal.height` to trigger validation error.

        let validated_state = apply_proposal(&validated_state, &mut delta, &parent_leaf, vec![]);
        let result = validate_proposal(
            &validated_state,
            genesis.instance_state.chain_config,
            &parent_leaf,
            &proposal,
        )
        .unwrap_err();
        assert_eq!(
            format!("{}", result.root_cause()),
            "Invalid Height Error: 0, 0"
        );

        // proposed `Header` root should include parent + parent.commit
        proposal.height += 1;

        let validated_state = apply_proposal(&validated_state, &mut delta, &parent_leaf, vec![]);

        let result = validate_proposal(
            &validated_state,
            genesis.instance_state.chain_config,
            &parent_leaf,
            &proposal,
        )
        .unwrap_err();
        // Fails b/c `proposal` has not advanced from `parent`
        assert!(format!("{}", result.root_cause()).contains("Invalid Block Root Error"));
    }

    #[async_std::test]
    async fn test_validate_proposal_success() {
        setup_logging();
        setup_backtrace();

        let anvil = Anvil::new().block_time(1u32).spawn();
        let mut genesis_state = NodeState::mock().with_l1(L1Client::new(
            anvil.endpoint().parse().unwrap(),
            Address::default(),
        ));

        let genesis = GenesisForTest::default();

        let mut parent_state = genesis.validated_state.clone();

        let mut block_merkle_tree = parent_state.block_merkle_tree.clone();
        let fee_merkle_tree = parent_state.fee_merkle_tree.clone();

        // Populate the tree with an initial `push`.
        block_merkle_tree.push(genesis.header.commit()).unwrap();
        let block_merkle_tree_root = block_merkle_tree.commitment();
        let fee_merkle_tree_root = fee_merkle_tree.commitment();
        parent_state.block_merkle_tree = block_merkle_tree.clone();
        parent_state.fee_merkle_tree = fee_merkle_tree.clone();

        let mut parent_header = genesis.header.clone();
        parent_header.block_merkle_tree_root = block_merkle_tree_root;
        parent_header.fee_merkle_tree_root = fee_merkle_tree_root;

        let mut parent_leaf = genesis.leaf.clone();
        *parent_leaf.get_block_header_mut() = parent_header.clone();

        // Forget the state to trigger lookups in Header::new
        let forgotten_state = parent_state.forget();
        genesis_state.peers = Arc::new(MockStateCatchup::from_iter([(
            parent_leaf.get_view_number(),
            Arc::new(parent_state.clone()),
        )]));
        // Get a proposal from a parent

        // TODO this currently fails because after fetching the blocks frontier
        // the element (header commitment) does not match the one in the proof.
        let key_pair = EthKeyPair::for_test();
        let fee_amount = 0u64;
        let payload_commitment = parent_header.payload_commitment;
        let builder_commitment = parent_header.builder_commitment();
        let ns_table = genesis.ns_table;
        let fee_signature = FeeAccount::sign_builder_message(
            &key_pair,
            &fee_message(fee_amount.into(), payload_commitment, &ns_table),
        )
        .unwrap();
        let builder_fee = BuilderFee {
            fee_amount,
            fee_signature,
        };
        let proposal = Header::new(
            &forgotten_state,
            &genesis_state,
            &parent_leaf,
            payload_commitment,
            builder_commitment,
            ns_table,
            builder_fee,
        )
        .await;

        let mut proposal_state = parent_state.clone();
        for fee_info in genesis_state
            .l1_client
            .get_finalized_deposits(None, 0)
            .await
        {
            proposal_state.insert_fee_deposit(fee_info).unwrap();
        }

        let mut block_merkle_tree = proposal_state.block_merkle_tree.clone();
        block_merkle_tree.push(proposal.commit()).unwrap();

        let l1_deposits = get_l1_deposits(&genesis_state, &proposal, &parent_leaf).await;

        let mut delta = Delta::default();

        let proposal_state = apply_proposal(&proposal_state, &mut delta, &parent_leaf, l1_deposits);
        validate_proposal(
            &proposal_state,
            genesis.instance_state.chain_config,
            &parent_leaf,
            &proposal.clone(),
        )
        .unwrap();

        assert_eq!(
            proposal_state.block_merkle_tree.commitment(),
            proposal.block_merkle_tree_root
        );
    }

    #[test]
    fn verify_header_signature() {
        // easy way to get a wallet:
        let state = NodeState::mock();

        // simulate a fixed size hash by padding our message
        let message = ";)";
        let mut commitment = [0u8; 32];
        commitment[..message.len()].copy_from_slice(message.as_bytes());

        let key = state.builder_key;

        let signature = FeeAccount::sign_builder_message(&key, &commitment).unwrap();
        assert!(key
            .fee_account()
            .validate_builder_signature(&signature, &commitment));
    }
}
