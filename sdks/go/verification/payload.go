package verification

import "github.com/EspressoSystems/espresso-network/sdks/go/types/common"

func DecodePayload(payload *common.BlockPayload) ([]common.Transaction, error) {
	return decodePayload(payload.RawPayload, payload.NsTable.Bytes)
}
