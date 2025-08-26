package client

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"sort"
	"sync"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	common "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
)

var _ QueryService = (*MultipleNodesClient)(nil)
var _ SubmitAPI = (*MultipleNodesClient)(nil)
var _ EspressoClient = (*MultipleNodesClient)(nil)

var IncorrectUrlAmountErr = errors.New("the MultipleNodesClient must be constructed with more than one node url")

type MultipleNodesClient struct {
	nodes []*Client
}

func NewMultipleNodesClient(urls []string) (*MultipleNodesClient, error) {
	if len(urls) <= 1 {
		return nil, IncorrectUrlAmountErr
	}
	nodes := make([]*Client, len(urls))
	for i, url := range urls {
		nodes[i] = NewClient(url)
	}
	return &MultipleNodesClient{nodes: nodes}, nil
}

func (c *MultipleNodesClient) FetchLatestBlockHeight(ctx context.Context) (uint64, error) {
	var errs []error
	for _, node := range c.nodes {
		height, err := node.FetchLatestBlockHeight(ctx)
		if err == nil {
			return height, nil
		} else {
			errs = append(errs, err)
		}
	}
	return 0, fmt.Errorf("fetch latest block height failed with all nodes, Errors: %v\n", errs)
}

func (c *MultipleNodesClient) FetchHeaderByHeight(ctx context.Context, height uint64) (types.HeaderImpl, error) {
	var res types.HeaderImpl
	if err := c.getWithMajority(ctx, &res, "availability/header/%d", height).err; err != nil {
		return types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *MultipleNodesClient) FetchRawHeaderByHeight(ctx context.Context, height uint64) (json.RawMessage, TransactionError) {
	return FetchWithMajority(ctx, c.nodes, func(node *Client) (json.RawMessage, TransactionError) {
		return node.FetchRawHeaderByHeight(ctx, height)
	})
}

func (c *MultipleNodesClient) FetchTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.TransactionQueryData, TransactionError) {
	var res types.TransactionQueryData
	if txnErr := c.getWithMajority(ctx, &res, "availability/transaction/hash/%s", hash.String()); txnErr.err != nil {
		return types.TransactionQueryData{}, txnErr
	}
	return res, TransactionError{nil, Success}
}

func (c *MultipleNodesClient) FetchExplorerTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.ExplorerTransactionQueryData, error) {
	if hash == nil {
		return types.ExplorerTransactionQueryData{}, fmt.Errorf("hash is nil")
	}
	var res types.ExplorerTransactionQueryData
	if err := c.getWithMajority(ctx, &res, "explorer/transaction/hash/%s", hash.String()).err; err != nil {
		return types.ExplorerTransactionQueryData{}, err
	}
	return res, nil
}

func (c *MultipleNodesClient) FetchHeadersByRange(ctx context.Context, from uint64, until uint64) ([]types.HeaderImpl, error) {
	var res []types.HeaderImpl
	if err := c.getWithMajority(ctx, &res, "availability/header/%d/%d", from, until).err; err != nil {
		return []types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *MultipleNodesClient) getWithMajority(ctx context.Context, out any, format string, args ...any) TransactionError {
	body, txnErr := FetchWithMajority(ctx, c.nodes, func(node *Client) (json.RawMessage, TransactionError) {
		return node.getRawMessage(ctx, format, args...)
	})
	if txnErr.err != nil {
		return txnErr
	}
	return TransactionError{json.Unmarshal(body, out), InvalidInfo}
}

func (c *MultipleNodesClient) FetchTransactionsInBlock(ctx context.Context, blockHeight uint64, namespace uint64) (TransactionsInBlock, error) {
	var res NamespaceResponse
	if err := c.getWithMajority(ctx, &res, "availability/block/%d/namespace/%d", blockHeight, namespace).err; err != nil {
		return TransactionsInBlock{}, err
	}

	if res.Transactions == nil {
		return TransactionsInBlock{}, fmt.Errorf("field transactions of type NamespaceResponse is required")
	}

	var txs []types.Bytes
	for i, tx := range *res.Transactions {
		if tx.Namespace != namespace {
			return TransactionsInBlock{}, fmt.Errorf("transaction %d has wrong namespace (%d, expected %d)", i, tx.Namespace, namespace)
		}
		txs = append(txs, tx.Payload)
	}

	if len(txs) > 0 && res.Proof == nil {
		return TransactionsInBlock{}, fmt.Errorf("field proof of type NamespaceResponse is required")
	}

	if res.Proof == nil {
		return TransactionsInBlock{}, nil
	}

	vidCommon, err := c.FetchVidCommonByHeight(ctx, blockHeight)
	if err != nil {
		return TransactionsInBlock{}, err
	}

	return TransactionsInBlock{Transactions: txs, Proof: *res.Proof, VidCommon: vidCommon}, nil
}

func (c *MultipleNodesClient) FetchVidCommonByHeight(ctx context.Context, blockHeight uint64) (common.VidCommon, error) {
	var res types.VidCommonQueryData
	if err := c.getWithMajority(ctx, &res, "availability/vid/common/%d", blockHeight).err; err != nil {
		return types.VidCommon{}, err
	}
	return res.Common, nil
}

func (c *MultipleNodesClient) SubmitTransaction(ctx context.Context, tx common.Transaction) (*common.TaggedBase64, TransactionError) {
	// Consider the error type of the submission as
	// * `Success` if at least one node succeeded.
	// * `InvalidInfo` if all failed fetches were due to InvalidInfo.
	// * `Other` in all other cases.
	var combinedErrType TransactionErrorType = Success

	// Check if one node is successfully able to submit the transaction
	var errs []error
	for _, node := range c.nodes {
		hash, txnErr := node.SubmitTransaction(ctx, tx)
		if txnErr.err == nil {
			return hash, TransactionError{nil, Success}
		} else {
			errs = append(errs, txnErr.err)
			if txnErr.errType != InvalidInfo {
				combinedErrType = Other
			}
		}
	}
	return nil, TransactionError{fmt.Errorf("encountered an error with all nodes while attempting to SubmitTransaction.\n Errors: %v \n", errs), combinedErrType}
}

func FetchWithMajority[T any](ctx context.Context, nodes []*T, fetchFunc func(*T) (json.RawMessage, TransactionError)) (json.RawMessage, TransactionError) {
	type result struct {
		value  json.RawMessage
		txnErr TransactionError
	}

	results := make(chan result, len(nodes))
	ctx, cancel := context.WithCancel(ctx)
	defer cancel()

	for _, node := range nodes {
		go func(node *T) {
			value, txnErr := fetchFunc(node)
			select {
			case results <- result{value, txnErr}:
			case <-ctx.Done():
			}
		}(node)
	}

	var errs []error
	// Consider the error type of the fetch as
	// * `Success` if the majority of the nodes succeeded.
	// * `InvalidInfo` if all failed fetches were due to InvalidInfo.
	// * `Other` in all other cases.
	var combinedErrType TransactionErrorType = Success
	var valueCount sync.Map
	majorityCount := (len(nodes) / 2) + 1
	responseCount := 0

	for {
		select {
		case res := <-results:
			if res.txnErr.err == nil {
				hash, err := hashNormalizedJSON(res.value)
				// if err is not nil,
				// this means that we still increase the response count
				// but if err is nil, we check if the value is already in the map
				// and if it is, we increase the count and check for majority
				if err != nil {
					fmt.Printf("error: failed to normalize json value: %v, error: %v", res.value, err)
					errs = append(errs, err)
					if res.txnErr.errType != InvalidInfo {
						combinedErrType = Other
					}
				} else {
					count, _ := valueCount.LoadOrStore(hash, 0)
					if countInt, ok := count.(int); ok {
						if countInt+1 >= majorityCount {
							cancel()
							return res.value, TransactionError{nil, Success}
						}
						valueCount.Store(hash, countInt+1)
					}

				}
			} else {
				errs = append(errs, res.txnErr.err)
				if res.txnErr.errType != InvalidInfo {
					combinedErrType = Other
				}
			}

			responseCount++
			if responseCount == len(nodes) {
				return json.RawMessage{}, TransactionError{fmt.Errorf("no majority consensus reached with potential errors. Errors: %v\n", errs), combinedErrType}
			}
		case <-ctx.Done():
			return json.RawMessage{}, TransactionError{ctx.Err(), Other}
		}
	}
}

func hashNormalizedJSON(data json.RawMessage) (string, error) {
	var obj interface{}
	if err := json.Unmarshal(data, &obj); err != nil {
		return "", err
	}
	hash, err := normalizeAndHash(obj)
	if err != nil {
		return "", err
	}
	return hash, nil
}

func normalizeAndHash(obj interface{}) (string, error) {
	switch v := obj.(type) {
	case map[string]interface{}:
		return normalizeJSONMap(v)
	case []interface{}:
		return normalizeJSONArray(v)
	default:
		hash := sha256.Sum256([]byte(fmt.Sprintf("%v", v)))
		return hex.EncodeToString(hash[:]), nil
	}
}

func normalizeJSONMap(obj map[string]interface{}) (string, error) {
	normalized := make([][]string, len(obj))
	i := 0
	for k, v := range obj {
		s, err := normalizeAndHash(v)
		if err != nil {
			return "", err
		}
		normalized[i] = []string{k, s}
		i += 1
	}
	sort.SliceStable(normalized, func(i, j int) bool {
		return normalized[i][0] < normalized[j][0]
	})
	normalizedJSON, err := json.Marshal(normalized)
	if err != nil {
		return "", err
	}
	hash := sha256.Sum256(normalizedJSON)
	return hex.EncodeToString(hash[:]), nil
}

func normalizeJSONArray(arr []interface{}) (string, error) {
	normalized := make([]string, len(arr))
	for i, v := range arr {
		s, err := normalizeAndHash(v)
		if err != nil {
			return "", err
		}
		normalized[i] = s
	}
	normalizedJSON, err := json.Marshal(normalized)
	if err != nil {
		return "", err
	}
	hash := sha256.Sum256(normalizedJSON)
	return hex.EncodeToString(hash[:]), nil
}
