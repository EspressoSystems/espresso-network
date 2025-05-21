package v0_3

import (
	"encoding/json"
	"fmt"

	"github.com/EspressoSystems/espresso-network-go/types/common"
	common_types "github.com/EspressoSystems/espresso-network-go/types/common"
)

type Header struct {
	ChainConfig          *ResolvableChainConfig     `json:"chain_config"`
	Height               uint64                     `json:"height"`
	Timestamp            uint64                     `json:"timestamp"`
	L1Head               uint64                     `json:"l1_head"`
	L1Finalized          *common_types.L1BlockInfo  `json:"l1_finalized"           rlp:"nil"`
	PayloadCommitment    *common_types.TaggedBase64 `json:"payload_commitment"`
	BuilderCommitment    *common_types.TaggedBase64 `json:"builder_commitment"`
	NsTable              *common_types.NsTable      `json:"ns_table"`
	BlockMerkleTreeRoot  *common_types.TaggedBase64 `json:"block_merkle_tree_root"`
	FeeMerkleTreeRoot    *common_types.TaggedBase64 `json:"fee_merkle_tree_root"`
	FeeInfo              *common_types.FeeInfo      `json:"fee_info"`
	BuilderSignature     *common_types.Signature    `json:"builder_signature"  rlp:"nil"`
	RewardMerkleTreeRoot *common_types.TaggedBase64 `json:"reward_merkle_tree_root"`
}

func (h *Header) Version() common_types.Version {
	return common_types.Version{Major: 0, Minor: 3}
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
func (h *Header) GetTimestamp() uint64 {
	return h.Timestamp
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

func (h *Header) UnmarshalJSON(b []byte) error {
	type Dec struct {
		ChainConfig          **ResolvableChainConfig     `json:"chain_config"`
		Height               *uint64                     `json:"height"`
		Timestamp            *uint64                     `json:"timestamp"`
		L1Head               *uint64                     `json:"l1_head"`
		L1Finalized          *common_types.L1BlockInfo   `json:"l1_finalized"           rlp:"nil"`
		PayloadCommitment    **common_types.TaggedBase64 `json:"payload_commitment"`
		BuilderCommitment    **common_types.TaggedBase64 `json:"builder_commitment"`
		NsTable              **common_types.NsTable      `json:"ns_table"`
		BlockMerkleTreeRoot  **common_types.TaggedBase64 `json:"block_merkle_tree_root"`
		FeeMerkleTreeRoot    **common_types.TaggedBase64 `json:"fee_merkle_tree_root"`
		FeeInfo              **common_types.FeeInfo      `json:"fee_info"`
		BuilderSignature     *common_types.Signature     `json:"builder_signature"`
		RewardMerkleTreeRoot **common_types.TaggedBase64 `json:"reward_merkle_tree_root"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}
	if dec.Height == nil {
		return fmt.Errorf("Field height of type Header is required")
	}
	h.Height = *dec.Height

	if dec.Timestamp == nil {
		return fmt.Errorf("Field timestamp of type Header is required")
	}
	h.Timestamp = *dec.Timestamp

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
		return fmt.Errorf("Field transactions_root of type Header is required")
	}
	h.NsTable = *dec.NsTable

	if dec.BlockMerkleTreeRoot == nil {
		return fmt.Errorf("Field block_merkle_tree_root of type Header is required")
	}
	h.BlockMerkleTreeRoot = *dec.BlockMerkleTreeRoot

	if dec.RewardMerkleTreeRoot == nil {
		return fmt.Errorf("Field reward_merkle_tree_root of type Header is required")
	}
	h.RewardMerkleTreeRoot = *dec.RewardMerkleTreeRoot

	if dec.FeeMerkleTreeRoot == nil {
		return fmt.Errorf("Field fee_merkle_tree_root of type Header is required")
	}
	h.FeeMerkleTreeRoot = *dec.FeeMerkleTreeRoot

	if dec.FeeInfo == nil {
		return fmt.Errorf("Field fee_info of type Header is required")
	}
	h.FeeInfo = *dec.FeeInfo

	if dec.ChainConfig == nil {
		return fmt.Errorf("Field chain_info of type Header is required")
	}
	h.ChainConfig = *dec.ChainConfig

	h.L1Finalized = dec.L1Finalized
	h.BuilderSignature = dec.BuilderSignature

	return nil
}

func (self *Header) Commit() common_types.Commitment {
	var l1FinalizedComm *common_types.Commitment
	if self.L1Finalized != nil {
		comm := self.L1Finalized.Commit()
		l1FinalizedComm = &comm
	}

	comm := common_types.NewRawCommitmentBuilder("BLOCK").
		Field("chain_config", self.ChainConfig.Commit()).
		Uint64Field("height", self.Height).
		Uint64Field("timestamp", self.Timestamp).
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
		Finalize()

	return common.NewRawCommitmentBuilder("BLOCK").
		Uint64Field("version_major", 0).
		Uint64Field("version_minor", 3).
		Field("fields", comm).
		Finalize()
}
