package client

import (
	"context"
	"encoding/json"
	"fmt"

	types "github.com/EspressoSystems/espresso-network-go/types"
)

// Interface to the Espresso Sequencer query service.
type QueryService interface {
	// Get the latest block number
	FetchLatestBlockHeight(ctx context.Context) (uint64, error)
	// Get the header for block number `height`.
	FetchHeaderByHeight(ctx context.Context, height uint64) (types.HeaderImpl, error)
	// Get the raw header for block number `height`.
	FetchRawHeaderByHeight(ctx context.Context, height uint64) (json.RawMessage, error)
	// Get the headers starting from the given :from up until the given :until
	FetchHeadersByRange(ctx context.Context, from uint64, until uint64) ([]types.HeaderImpl, error)
	// Get the transactions belonging to the given namespace at the block height,
	// along with a proof that these are all such transactions.
	FetchTransactionsInBlock(ctx context.Context, blockHeight uint64, namespace uint64) (TransactionsInBlock, error)
	// Get the transaction by its hash.
	FetchTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.TransactionQueryData, error)
	// Get the VidCommon for the given block height.
	FetchVidCommonByHeight(ctx context.Context, blockHeight uint64) (types.VidCommon, error)
}

// Response to `FetchTransactionsInBlock`
type TransactionsInBlock struct {
	// The transactions.
	Transactions []types.Bytes `json:"transactions"`
	// A proof that these are all the transactions in the block with the requested namespace.
	Proof     types.NamespaceProof `json:"proof"`
	VidCommon types.VidCommon      `json:"vidCommon"`
}

func (t *TransactionsInBlock) UnmarshalJSON(b []byte) error {
	// Parse using pointers so we can distinguish between missing and default fields.
	type Dec struct {
		Transactions *[]types.Bytes        `json:"transactions"`
		Proof        *types.NamespaceProof `json:"proof"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.Transactions == nil {
		return fmt.Errorf("Field transactions of type TransactionsInBlock is required")
	}
	t.Transactions = *dec.Transactions

	if dec.Proof == nil {
		return fmt.Errorf("Field proof of type TransactionsInBlock is required")
	}
	t.Proof = *dec.Proof

	return nil
}
