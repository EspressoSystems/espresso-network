package types

import (
	"encoding/json"
	"fmt"

	common_types "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	v01 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_1"
	v02 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_2"
	v03 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_3"
)

// Republic
type EitherChainConfig0_1 = v01.EitherChainConfig
type ResolvableChainConfig0_1 = v01.ResolvableChainConfig
type ChainConfig0_1 = v01.ChainConfig

type EitherChainConfig0_3 = v03.EitherChainConfig
type ResolvableChainConfig0_3 = v03.ResolvableChainConfig
type ChainConfig0_3 = v03.ChainConfig

type Header0_1 = v01.Header
type Header0_2 = v02.Header
type Header0_3 = v03.Header

type Bytes = common_types.Bytes

type BlockMerkleSnapshot = common_types.BlockMerkleSnapshot
type Commitment = common_types.Commitment
type FeeInfo = common_types.FeeInfo
type HotShotBlockMerkleProof = common_types.HotShotBlockMerkleProof
type L1BlockInfo = common_types.L1BlockInfo
type NamespaceProof = common_types.NamespaceProof
type NsTable = common_types.NsTable
type Signature = common_types.Signature
type TaggedBase64 = common_types.TaggedBase64
type Transaction = common_types.Transaction
type Version = common_types.Version
type VidCommon = common_types.VidCommon

type TransactionQueryData = common_types.TransactionQueryData
type ExplorerTransactionQueryData = common_types.ExplorerTransactionQueryData
type VidCommonQueryData = common_types.VidCommonQueryData

type U256 = common_types.U256
type U256Decimal = common_types.U256Decimal

// Consensus types
type ConsensusMessage = common_types.ConsensusMessage
type Event = common_types.Event
type ViewFinished = common_types.ViewFinished
type Decide = common_types.Decide
type LeafChain = common_types.LeafChain
type Leaf = common_types.Leaf
type QuorumProposalWrapper = common_types.QuorumProposalWrapper
type QuorumProposalDataWrapper = common_types.QuorumProposalDataWrapper
type QuorumProposalData = common_types.QuorumProposalData
type QuorumProposal = common_types.QuorumProposal
type BlockHeader = common_types.BlockHeader
type Fields = common_types.Fields
type ChainConfigWrapper = common_types.ChainConfigWrapper
type ChainConfig = common_types.ChainConfig
type L1Finalized = common_types.L1Finalized
type DaProposalWrapper = common_types.DaProposalWrapper
type DaProposalDataWrapper = common_types.DaProposalDataWrapper
type DAProposalData = common_types.DAProposalData
type Metadata = common_types.Metadata
type BlockPayload = common_types.BlockPayload
type BuilderCommitment = common_types.BuilderCommitment

var NewU256 = common_types.NewU256

type HeaderInterface interface {
	Commit() common_types.Commitment
	Version() common_types.Version
	GetBlockHeight() uint64
	GetL1Head() uint64
	GetL1Finalized() *common_types.L1BlockInfo
	GetTimestamp() uint64
	GetPayloadCommitment() *common_types.TaggedBase64
	GetBuilderCommitment() *common_types.TaggedBase64
	GetNsTable() *common_types.NsTable
	GetBlockMerkleTreeRoot() *common_types.TaggedBase64
	GetFeeMerkleTreeRoot() *common_types.TaggedBase64
}

type HeaderImpl struct {
	Header HeaderInterface
}

func (h *HeaderImpl) UnmarshalJSON(b []byte) error {
	header, err := parseHeader(b)
	if err != nil {
		return err
	}

	h.Header = header
	return nil
}

func (h HeaderImpl) MarshalJSON() ([]byte, error) {
	version := h.Header.Version()
	if version.Major == 0 && version.Minor == 1 {
		return json.Marshal(h.Header)
	}

	var rawHeader RawHeader
	rawHeader.Version = version

	bytes, err := json.Marshal(h.Header)
	if err != nil {
		return nil, err
	}

	rawHeader.Fields = bytes

	return json.Marshal(rawHeader)
}

type RawHeader struct {
	Version common_types.Version `json:"version"`
	Fields  json.RawMessage      `json:"fields"`
}

func (r *RawHeader) UnmarshalJSON(b []byte) error {
	type Dec struct {
		Version *common_types.Version `json:"version"`
		Fields  *json.RawMessage      `json:"fields"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.Version == nil {
		return fmt.Errorf("version is required")
	}

	if dec.Fields == nil {
		return fmt.Errorf("fields is required")
	}

	r.Version = *dec.Version
	r.Fields = *dec.Fields
	return nil
}

func parseHeader(data []byte) (HeaderInterface, error) {
	var rawHeader RawHeader
	if err := json.Unmarshal(data, &rawHeader); err != nil {
		var header v01.Header
		// Failed to parse the version header. Legacy header
		if err = json.Unmarshal(data, &header); err != nil {
			return nil, err
		}

		return &header, nil
	}

	version := rawHeader.Version
	if version.Major == 0 && version.Minor == 2 {
		var header v02.Header
		if err := json.Unmarshal(rawHeader.Fields, &header); err != nil {
			return nil, err
		}
		return &header, nil
	}

	// If minor version is not 2, we can assume it will be v0.3 header
	if version.Major == 0 {
		var header v03.Header
		if err := json.Unmarshal(rawHeader.Fields, &header); err != nil {
			return nil, err
		}
		return &header, nil
	}

	return nil, fmt.Errorf("version error: %v", version)
}
