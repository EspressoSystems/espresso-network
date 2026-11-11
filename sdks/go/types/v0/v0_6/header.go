package v0_6

import (
	common_types "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	v05 "github.com/EspressoSystems/espresso-network/sdks/go/types/v0/v0_5"
)

// The 0.5 shape under the 0.6 version. Only `Version` and the version the commitment wraps
// differ; decoding and the getters are the embedded header's.
type Header struct {
	v05.Header
}

func (h *Header) Version() common_types.Version {
	return common_types.Version{Major: 0, Minor: 6}
}

func (h *Header) Commit() common_types.Commitment {
	return common_types.NewRawCommitmentBuilder("BLOCK").
		Uint64Field("version_major", 0).
		Uint64Field("version_minor", 6).
		Field("fields", h.CommitFields()).
		Finalize()
}
