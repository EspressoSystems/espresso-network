package v0_4

import (
	"encoding/json"
	"fmt"

	common_types "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	v03 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_3"
)

// The chain config kept its 0.3 shape.
type ChainConfig = v03.ChainConfig
type EitherChainConfig = v03.EitherChainConfig
type ResolvableChainConfig = v03.ResolvableChainConfig

// The 0.3 header plus `timestamp_millis`, `total_reward_distributed` and, at an epoch boundary,
// `next_stake_table_hash`. `reward_merkle_tree_root` is the root of the second reward tree.
type Header struct {
	ChainConfig            *ResolvableChainConfig     `json:"chain_config"`
	Height                 uint64                     `json:"height"`
	Timestamp              uint64                     `json:"timestamp"`
	TimestampMillis        uint64                     `json:"timestamp_millis"`
	L1Head                 uint64                     `json:"l1_head"`
	L1Finalized            *common_types.L1BlockInfo  `json:"l1_finalized"`
	PayloadCommitment      *common_types.TaggedBase64 `json:"payload_commitment"`
	BuilderCommitment      *common_types.TaggedBase64 `json:"builder_commitment"`
	NsTable                *common_types.NsTable      `json:"ns_table"`
	BlockMerkleTreeRoot    *common_types.TaggedBase64 `json:"block_merkle_tree_root"`
	FeeMerkleTreeRoot      *common_types.TaggedBase64 `json:"fee_merkle_tree_root"`
	FeeInfo                *common_types.FeeInfo      `json:"fee_info"`
	BuilderSignature       *common_types.Signature    `json:"builder_signature"`
	RewardMerkleTreeRoot   *common_types.TaggedBase64 `json:"reward_merkle_tree_root"`
	TotalRewardDistributed common_types.U256Decimal   `json:"total_reward_distributed"`
	NextStakeTableHash     *common_types.TaggedBase64 `json:"next_stake_table_hash"`
}

func (h *Header) Version() common_types.Version {
	return common_types.Version{Major: 0, Minor: 4}
}

func (h *Header) GetBlockHeight() uint64 {
	return h.Height
}
func (h *Header) GetPayloadCommitment() *common_types.TaggedBase64 {
	return h.PayloadCommitment
}
func (h *Header) GetL1Head() uint64 {
	return h.L1Head
}
func (h *Header) GetL1Finalized() *common_types.L1BlockInfo {
	return h.L1Finalized
}
func (h *Header) GetTimestamp() uint64 {
	return h.Timestamp
}
func (h *Header) GetTimestampMillis() uint64 {
	return h.TimestampMillis
}
func (h *Header) GetBuilderCommitment() *common_types.TaggedBase64 {
	return h.BuilderCommitment
}
func (h *Header) GetNsTable() *common_types.NsTable {
	return h.NsTable
}
func (h *Header) GetBlockMerkleTreeRoot() *common_types.TaggedBase64 {
	return h.BlockMerkleTreeRoot
}
func (h *Header) GetFeeMerkleTreeRoot() *common_types.TaggedBase64 {
	return h.FeeMerkleTreeRoot
}
func (h *Header) GetRewardMerkleTreeRoot() *common_types.TaggedBase64 {
	return h.RewardMerkleTreeRoot
}
func (h *Header) GetTotalRewardDistributed() *common_types.U256Decimal {
	return &h.TotalRewardDistributed
}
func (h *Header) GetNextStakeTableHash() *common_types.TaggedBase64 {
	return h.NextStakeTableHash
}

func (h *Header) UnmarshalJSON(b []byte) error {
	// Parse using pointers so we can distinguish between missing and default fields.
	type Dec struct {
		ChainConfig            **ResolvableChainConfig     `json:"chain_config"`
		Height                 *uint64                     `json:"height"`
		Timestamp              *uint64                     `json:"timestamp"`
		TimestampMillis        *uint64                     `json:"timestamp_millis"`
		L1Head                 *uint64                     `json:"l1_head"`
		L1Finalized            *common_types.L1BlockInfo   `json:"l1_finalized"`
		PayloadCommitment      **common_types.TaggedBase64 `json:"payload_commitment"`
		BuilderCommitment      **common_types.TaggedBase64 `json:"builder_commitment"`
		NsTable                **common_types.NsTable      `json:"ns_table"`
		BlockMerkleTreeRoot    **common_types.TaggedBase64 `json:"block_merkle_tree_root"`
		FeeMerkleTreeRoot      **common_types.TaggedBase64 `json:"fee_merkle_tree_root"`
		FeeInfo                **common_types.FeeInfo      `json:"fee_info"`
		BuilderSignature       *common_types.Signature     `json:"builder_signature"`
		RewardMerkleTreeRoot   **common_types.TaggedBase64 `json:"reward_merkle_tree_root"`
		TotalRewardDistributed *common_types.U256Decimal   `json:"total_reward_distributed"`
		NextStakeTableHash     *common_types.TaggedBase64  `json:"next_stake_table_hash"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.ChainConfig == nil {
		return fmt.Errorf("Field chain_config of type Header is required")
	}
	h.ChainConfig = *dec.ChainConfig

	if dec.Height == nil {
		return fmt.Errorf("Field height of type Header is required")
	}
	h.Height = *dec.Height

	if dec.Timestamp == nil {
		return fmt.Errorf("Field timestamp of type Header is required")
	}
	h.Timestamp = *dec.Timestamp

	if dec.TimestampMillis == nil {
		return fmt.Errorf("Field timestamp_millis of type Header is required")
	}
	h.TimestampMillis = *dec.TimestampMillis

	if dec.L1Head == nil {
		return fmt.Errorf("Field l1_head of type Header is required")
	}
	h.L1Head = *dec.L1Head

	if dec.PayloadCommitment == nil {
		return fmt.Errorf("Field payload_commitment of type Header is required")
	}
	h.PayloadCommitment = *dec.PayloadCommitment

	if dec.BuilderCommitment == nil {
		return fmt.Errorf("Field builder_commitment of type Header is required")
	}
	h.BuilderCommitment = *dec.BuilderCommitment

	if dec.NsTable == nil {
		return fmt.Errorf("Field ns_table of type Header is required")
	}
	h.NsTable = *dec.NsTable

	if dec.BlockMerkleTreeRoot == nil {
		return fmt.Errorf("Field block_merkle_tree_root of type Header is required")
	}
	h.BlockMerkleTreeRoot = *dec.BlockMerkleTreeRoot

	if dec.FeeMerkleTreeRoot == nil {
		return fmt.Errorf("Field fee_merkle_tree_root of type Header is required")
	}
	h.FeeMerkleTreeRoot = *dec.FeeMerkleTreeRoot

	if dec.FeeInfo == nil {
		return fmt.Errorf("Field fee_info of type Header is required")
	}
	h.FeeInfo = *dec.FeeInfo

	if dec.RewardMerkleTreeRoot == nil {
		return fmt.Errorf("Field reward_merkle_tree_root of type Header is required")
	}
	h.RewardMerkleTreeRoot = *dec.RewardMerkleTreeRoot

	if dec.TotalRewardDistributed == nil {
		return fmt.Errorf("Field total_reward_distributed of type Header is required")
	}
	h.TotalRewardDistributed = *dec.TotalRewardDistributed

	h.L1Finalized = dec.L1Finalized
	h.BuilderSignature = dec.BuilderSignature
	h.NextStakeTableHash = dec.NextStakeTableHash

	return nil
}

// The commitment to the header fields, which the versioned commitment wraps.
func (self *Header) CommitFields() common_types.Commitment {
	var l1FinalizedComm *common_types.Commitment
	if self.L1Finalized != nil {
		comm := self.L1Finalized.Commit()
		l1FinalizedComm = &comm
	}
	totalRewardDistributed := self.TotalRewardDistributed.ToU256().ToLittleEndianBytes()

	builder := common_types.NewRawCommitmentBuilder("BLOCK").
		Field("chain_config", self.ChainConfig.Commit()).
		Uint64Field("height", self.Height).
		Uint64Field("timestamp", self.Timestamp).
		Uint64Field("timestamp_millis", self.TimestampMillis).
		Uint64Field("l1_head", self.L1Head).
		OptionalField("l1_finalized", l1FinalizedComm).
		ConstantString("payload_commitment").
		FixedSizeBytes(self.PayloadCommitment.Value()).
		ConstantString("builder_commitment").
		FixedSizeBytes(self.BuilderCommitment.Value()).
		Field("ns_table", self.NsTable.Commit()).
		VarSizeField("block_merkle_tree_root", self.BlockMerkleTreeRoot.Value()).
		VarSizeField("fee_merkle_tree_root", self.FeeMerkleTreeRoot.Value()).
		Field("fee_info", self.FeeInfo.Commit()).
		VarSizeField("reward_merkle_tree_root", self.RewardMerkleTreeRoot.Value()).
		VarSizeField("total_reward_distributed", totalRewardDistributed[:])

	// Unlike the other optional fields, an absent hash contributes nothing.
	if self.NextStakeTableHash != nil {
		builder.FixedSizeField("next_stake_table_hash", self.NextStakeTableHash.Value())
	}

	return builder.Finalize()
}

func (self *Header) Commit() common_types.Commitment {
	return common_types.NewRawCommitmentBuilder("BLOCK").
		Uint64Field("version_major", 0).
		Uint64Field("version_minor", 4).
		Field("fields", self.CommitFields()).
		Finalize()
}
