package client

import (
	"context"

	common "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
)

// Interface to the Espresso Sequencer submit API
type SubmitAPI interface {
	// Submit a transaction to the espresso sequencer.
	SubmitTransaction(ctx context.Context, tx common.Transaction) (*common.TaggedBase64, error)
}

// An interface for error txn submission error retrieval form the builder client if some builders fail, but one succeeds.
type BuilderErrorRetrieval interface {
	//retrieve the clients state of errors from it's previous attempt to submit to builders.
	GetPreviousSubmissionErrors() []error
}
