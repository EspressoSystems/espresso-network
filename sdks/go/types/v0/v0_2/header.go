package v0_2

import (
	"encoding/json"

	"github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	v01 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_1"
)

type Header struct {
	v01.Header
}

func (h *Header) Version() common.Version {
	return common.Version{Major: 0, Minor: 2}
}

func (h *Header) GetBlockHeight() uint64 {
	return h.Height
}
func (h *Header) GetPayloadCommitment() *common.TaggedBase64 {
	return h.PayloadCommitment
}
func (h *Header) GetBuilderCommitment() *common.TaggedBase64 {
	return h.BuilderCommitment
}
func (h *Header) GetNsTable() *common.NsTable {
	return h.NsTable
}
func (h *Header) GetBlockMerkleTreeRoot() *common.TaggedBase64 {
	return h.BlockMerkleTreeRoot
}
func (h *Header) GetFeeMerkleTreeRoot() *common.TaggedBase64 {
	return h.FeeMerkleTreeRoot
}

func (h *Header) GetBuilderSignature() *common.Signature {
	return h.BuilderSignature
}

func (h *Header) UnmarshalJSON(b []byte) error {
	var header v01.Header
	err := json.Unmarshal(b, &header)
	if err != nil {
		return err
	}

	*h = Header{header}
	return nil
}

func (h *Header) Commit() common.Commitment {
	return common.NewRawCommitmentBuilder("BLOCK").
		Uint64Field("version_major", 0).
		Uint64Field("version_minor", 2).
		Field("fields", h.Header.Commit()).
		Finalize()
}
