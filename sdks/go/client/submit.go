package client

import (
	"context"

	common "github.com/EspressoSystems/espresso-network-go/types/common"
)

// Interface to the Espresso Sequencer submit API
type SubmitAPI interface {
	// Submit a transaction to the espresso sequencer.
	SubmitTransaction(ctx context.Context, tx common.Transaction) (*common.TaggedBase64, error)
}
