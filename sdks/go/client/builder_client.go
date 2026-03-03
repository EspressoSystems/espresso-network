package client

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"strings"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	common "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	"github.com/coder/websocket"
)

var _ QueryService = (*BuilderClient)(nil)
var _ SubmitAPI = (*BuilderClient)(nil)
var _ EspressoClient = (*BuilderClient)(nil)
var _ EspressoBuilderClient = (*BuilderClient)(nil)

type BuilderClient struct {
	builderUrls          []string
	builderClients       []*http.Client
	queryUrl             string
	queryClient          *http.Client
	previousSubmitErrors []error
}

// Apply sanitation rules to url before creating the client.
// currently, this is just ensuring the url has the suffix `/`
// but more rules can be applied here later.
func sanitizeUrl(url string) string {
	if !strings.HasSuffix(url, "/") {
		url += "/"
	}
	return url
}

func NewBuilderClient(queryUrl string, builderUrls []string) *BuilderClient {
	sanitizedQueryUrl := sanitizeUrl(queryUrl)
	builderClients := make([]*http.Client, len(builderUrls))
	for i, url := range builderUrls {
		builderUrls[i] = sanitizeUrl(url)
	}

	return &BuilderClient{
		queryUrl:       sanitizedQueryUrl,
		queryClient:    http.DefaultClient,
		builderUrls:    builderUrls,
		builderClients: builderClients,
	}
}

var ErrAllBuildersFailed = errors.New("submission to all builders failed, check previousSubmitErrors")

func (c *BuilderClient) FetchVidCommonByHeight(ctx context.Context, blockHeight uint64) (common.VidCommon, error) {
	var res types.VidCommonQueryData
	if err := c.get(ctx, &res, "availability/vid/common/%d", blockHeight); err != nil {
		return types.VidCommon{}, err
	}
	return res.Common, nil
}

func (c *BuilderClient) FetchLatestBlockHeight(ctx context.Context) (uint64, error) {
	var res uint64
	if err := c.get(ctx, &res, "status/block-height"); err != nil {
		return 0, err
	}
	return res, nil
}

func (c *BuilderClient) FetchHeaderByHeight(ctx context.Context, blockHeight uint64) (types.HeaderImpl, error) {
	var res types.HeaderImpl
	if err := c.get(ctx, &res, "availability/header/%d", blockHeight); err != nil {
		return types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *BuilderClient) FetchRawHeaderByHeight(ctx context.Context, blockHeight uint64) (json.RawMessage, error) {
	res, err := c.getRawMessage(ctx, "availability/header/%d", blockHeight)
	if err != nil {
		return nil, err
	}
	return res, nil
}

func (c *BuilderClient) FetchHeadersByRange(ctx context.Context, from uint64, until uint64) ([]types.HeaderImpl, error) {
	var res []types.HeaderImpl
	if err := c.get(ctx, &res, "availability/header/%d/%d", from, until); err != nil {
		return []types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *BuilderClient) FetchExplorerTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.ExplorerTransactionQueryData, error) {
	if hash == nil {
		return types.ExplorerTransactionQueryData{}, fmt.Errorf("%w: hash is nil", ErrPermanent)
	}
	var res types.ExplorerTransactionQueryData
	if err := c.get(ctx, &res, "explorer/transaction/hash/%s", hash.String()); err != nil {
		return types.ExplorerTransactionQueryData{}, err
	}
	return res, nil
}

func (c *BuilderClient) FetchTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.TransactionQueryData, error) {
	if hash == nil {
		return types.TransactionQueryData{}, fmt.Errorf("hash is nil")
	}
	var res types.TransactionQueryData
	if err := c.get(ctx, &res, "availability/transaction/hash/%s", hash.String()); err != nil {
		return types.TransactionQueryData{}, err
	}
	return res, nil
}

// Fetches a block merkle proof at the snapshot rootHeight for the leaf at the provided HotShot height
func (c *BuilderClient) FetchBlockMerkleProof(ctx context.Context, rootHeight uint64, hotshotHeight uint64) (types.HotShotBlockMerkleProof, error) {
	var res types.HotShotBlockMerkleProof
	if err := c.get(ctx, &res, "block-state/%d/%d", rootHeight, hotshotHeight); err != nil {
		return types.HotShotBlockMerkleProof{}, err
	}
	return res, nil
}

func (c *BuilderClient) FetchTransactionsInBlock(ctx context.Context, blockHeight uint64, namespace uint64) (TransactionsInBlock, error) {
	var res NamespaceResponse
	if err := c.get(ctx, &res, "availability/block/%d/namespace/%d", blockHeight, namespace); err != nil {
		return TransactionsInBlock{}, err
	}

	if res.Transactions == nil {
		return TransactionsInBlock{}, fmt.Errorf("field transactions of type NamespaceResponse is required")
	}

	// Extract the transactions.
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

	return TransactionsInBlock{
		Transactions: txs,
		Proof:        *res.Proof,
		VidCommon:    vidCommon,
	}, nil

}

func (c *BuilderClient) FetchNamespaceTransactionsInRange(ctx context.Context, from uint64, until uint64, namespace uint64) ([]types.NamespaceTransactionsRangeData, error) {
	limits, err := c.FetchLimits(ctx)
	if err != nil {
		return nil, err
	}
	// check that from and until are within limits
	if until-from > limits.LargeObjectRangeLimit {
		return nil, fmt.Errorf("range too large: %d > %d", until-from, limits.LargeObjectRangeLimit)
	}
	var res []types.NamespaceTransactionsRangeData
	if err := c.get(ctx, &res, "availability/block/%d/%d/namespace/%d", from, until, namespace); err != nil {
		return nil, err
	}
	return res, nil
}

func (c *BuilderClient) FetchLimits(ctx context.Context) (types.LimitsData, error) {
	var res types.LimitsData
	if err := c.get(ctx, &res, "availability/limits"); err != nil {
		return types.LimitsData{}, err
	}
	return res, nil
}

func (c *BuilderClient) SubmitTransaction(ctx context.Context, tx types.Transaction) (*types.TaggedBase64, error) {
	c.previousSubmitErrors = make([]error, 0)
	for clientIdx, url := range c.builderUrls {
		response, err := c.tryPostRequest(ctx, url, clientIdx, tx)

		if err != nil {
			c.previousSubmitErrors = append(c.previousSubmitErrors, err)
			return nil, fmt.Errorf("%w: %v", ErrPermanent, err)
		}

		defer response.Body.Close()
		if response.StatusCode != 200 {
			return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
		}

		body, err := io.ReadAll(response.Body)
		if err != nil {
			return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
		}

		var hash types.TaggedBase64
		if err := json.Unmarshal(body, &hash); err != nil {
			return nil, fmt.Errorf("%w: %v", ErrPermanent, err)
		}
		// If we receive a successful submission from the builder, we can exit as we don't need to send to other builders.
		return &hash, nil
	}
	return nil, ErrAllBuildersFailed
}

// Open a `Stream` of Espresso transactions starting from a specific block height.
func (c *BuilderClient) StreamTransactions(ctx context.Context, height uint64) (Stream[types.TransactionQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.queryClient
	url := c.queryUrl + fmt.Sprintf("availability/stream/transactions/%d", height)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.TransactionQueryData]{conn: conn}, nil
}

// Open a `Stream` of Espresso transactions starting from a specific block height, filtered by namespace.
func (c *BuilderClient) StreamTransactionsInNamespace(ctx context.Context, height uint64, namespace uint64) (Stream[types.TransactionQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.queryClient
	url := c.queryUrl + fmt.Sprintf("availability/stream/transactions/%d/namespace/%d", height, namespace)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.TransactionQueryData]{conn: conn}, nil
}

func (c *BuilderClient) StreamPayloads(ctx context.Context, height uint64) (Stream[types.PayloadQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.queryClient
	url := c.queryUrl + fmt.Sprintf("availability/stream/payloads/%d", height)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.PayloadQueryData]{conn: conn}, nil
}

func (c *BuilderClient) getRawMessage(ctx context.Context, format string, args ...any) (json.RawMessage, error) {
	res, err := c.tryGetRequest(ctx, c.queryUrl, format, args...)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrPermanent, err)
	}

	defer res.Body.Close()

	if res.StatusCode != 200 {
		// Try to get the response body to include in the error message, as it may have useful
		// information about why the request failed. If this call fails, the response will be `nil`,
		// which is fine to include in the log, so we can ignore errors.
		body, _ := io.ReadAll(res.Body)
		err := fmt.Errorf("request failed with status %d and body %s", res.StatusCode, string(body))
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}

	// Read the response body into memory before we unmarshal it, rather than passing the io.Reader
	// to the json decoder, so that we still have the body and can inspect it if unmarshalling
	// failed.
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return body, nil
}

func (c *BuilderClient) get(ctx context.Context, out any, format string, args ...any) error {
	body, err := c.getRawMessage(ctx, format, args...)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(body, out); err != nil {
		return fmt.Errorf("%w: request failed with body %s and error %v", ErrPermanent, string(body), err)
	}
	return nil
}

func (c *BuilderClient) tryGetRequest(ctx context.Context, baseUrl, format string, args ...interface{}) (*http.Response, error) {

	url := baseUrl + fmt.Sprintf(format, args...)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	return c.queryClient.Do(req)

}

func (c *BuilderClient) tryPostRequest(ctx context.Context, baseUrl string, clientIndex int, tx types.Transaction) (*http.Response, error) {
	marshalled, err := json.Marshal(tx)
	if err != nil {
		return nil, err
	}

	request, err := http.NewRequestWithContext(ctx, "POST", baseUrl+"txn_submit/submit", bytes.NewBuffer(marshalled))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/json")
	return c.builderClients[clientIndex].Do(request)
}

func (c *BuilderClient) GetPreviousSubmissionErrors() []error {
	return c.previousSubmitErrors
}
