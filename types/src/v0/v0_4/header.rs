use crate::{v0_3::RewardAmount, v0_4::RewardMerkleCommitmentV2, NsTable, TimestampMillis};

use super::{
    BlockMerkleCommitment, BuilderSignature, FeeInfo, FeeMerkleCommitment, L1BlockInfo,
    ResolvableChainConfig,
};
use ark_serialize::CanonicalSerialize;
use committable::{Commitment, Committable, RawCommitmentBuilder};
use hotshot_types::{data::VidCommitment, utils::BuilderCommitment};
use serde::{Deserialize, Serialize};

pub type StakeTableHash = [u8; 32];

/// A header is like a [`Block`] with the body replaced by a digest.
#[derive(Clone, Debug, Deserialize, Serialize, Hash, PartialEq, Eq)]
pub struct Header {
    /// A commitment to a ChainConfig or a full ChainConfig.
    pub(crate) chain_config: ResolvableChainConfig,
    pub(crate) height: u64,
    pub(crate) timestamp: u64,
    pub(crate) timestamp_millis: TimestampMillis,
    pub(crate) l1_head: u64,
    pub(crate) l1_finalized: Option<L1BlockInfo>,
    pub(crate) payload_commitment: VidCommitment,
    pub(crate) builder_commitment: BuilderCommitment,
    pub(crate) ns_table: NsTable,
    pub(crate) block_merkle_tree_root: BlockMerkleCommitment,
    pub(crate) fee_merkle_tree_root: FeeMerkleCommitment,
    pub(crate) fee_info: FeeInfo,
    pub(crate) builder_signature: Option<BuilderSignature>,
    pub(crate) reward_merkle_tree_root: RewardMerkleCommitmentV2,
    pub(crate) total_reward_distributed: RewardAmount,
    pub(crate) next_stake_table_hash: Option<StakeTableHash>,
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

        let mut rwd_bytes = vec![];
        self.reward_merkle_tree_root
            .serialize_with_mode(&mut rwd_bytes, ark_serialize::Compress::Yes)
            .unwrap();

        let mut cb = RawCommitmentBuilder::new(&Self::tag())
            .field("chain_config", self.chain_config.commit())
            .u64_field("height", self.height)
            .u64_field("timestamp", self.timestamp)
            .u64_field("timestamp_millis", self.timestamp_millis.u64())
            .u64_field("l1_head", self.l1_head)
            .optional("l1_finalized", &self.l1_finalized)
            .constant_str("payload_commitment")
            .fixed_size_bytes(self.payload_commitment.as_ref())
            .constant_str("builder_commitment")
            .fixed_size_bytes(self.builder_commitment.as_ref())
            .field("ns_table", self.ns_table.commit())
            .var_size_field("block_merkle_tree_root", &bmt_bytes)
            .var_size_field("fee_merkle_tree_root", &fmt_bytes)
            .field("fee_info", self.fee_info.commit())
            .var_size_field("reward_merkle_tree_root", &rwd_bytes)
            .var_size_field("total_reward_distributed", &self.total_reward_distributed.to_fixed_bytes());

        if let Some(next_stake_table_hash) = self.next_stake_table_hash {
            cb = cb.constant_str("next_stake_table_hash").fixed_size_bytes(&next_stake_table_hash);
        }

        cb.finalize()
    }

    fn tag() -> String {
        crate::v0_1::Header::tag()
    }
}
