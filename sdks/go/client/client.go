package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"

	types "github.com/EspressoSystems/espresso-network-go/types"
	common "github.com/EspressoSystems/espresso-network-go/types/common"
)

var _ QueryService = (*Client)(nil)
var _ SubmitAPI = (*Client)(nil)
var _ EspressoClient = (*Client)(nil)

type Client struct {
	baseUrl string
	client  *http.Client
}

func NewClient(url string) *Client {
	if !strings.HasSuffix(url, "/") {
		url += "/"
	}

	return &Client{
		baseUrl: url,
		client:  http.DefaultClient,
	}
}

func (c *Client) FetchVidCommonByHeight(ctx context.Context, blockHeight uint64) (common.VidCommon, error) {
	var res types.VidCommonQueryData
	if err := c.get(ctx, &res, "availability/vid/common/%d", blockHeight); err != nil {
		return types.VidCommon{}, err
	}
	return res.Common, nil
}

func (c *Client) FetchLatestBlockHeight(ctx context.Context) (uint64, error) {
	var res uint64
	if err := c.get(ctx, &res, "status/block-height"); err != nil {
		return 0, err
	}
	return res, nil
}

func (c *Client) FetchHeaderByHeight(ctx context.Context, blockHeight uint64) (types.HeaderImpl, error) {
	var res types.HeaderImpl
	if err := c.get(ctx, &res, "availability/header/%d", blockHeight); err != nil {
		return types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *Client) FetchRawHeaderByHeight(ctx context.Context, blockHeight uint64) (json.RawMessage, error) {
	res, err := c.getRawMessage(ctx, "availability/header/%d", blockHeight)
	if err != nil {
		return nil, err
	}
	return res, nil
}

func (c *Client) FetchHeadersByRange(ctx context.Context, from uint64, until uint64) ([]types.HeaderImpl, error) {
	var res []types.HeaderImpl
	if err := c.get(ctx, &res, "availability/header/%d/%d", from, until); err != nil {
		return []types.HeaderImpl{}, err
	}
	return res, nil
}

func (c *Client) FetchTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.TransactionQueryData, error) {
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
func (c *Client) FetchBlockMerkleProof(ctx context.Context, rootHeight uint64, hotshotHeight uint64) (types.HotShotBlockMerkleProof, error) {
	var res types.HotShotBlockMerkleProof
	if err := c.get(ctx, &res, "block-state/%d/%d", rootHeight, hotshotHeight); err != nil {
		return types.HotShotBlockMerkleProof{}, err
	}
	return res, nil
}

func (c *Client) FetchTransactionsInBlock(ctx context.Context, blockHeight uint64, namespace uint64) (TransactionsInBlock, error) {
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

func (c *Client) SubmitTransaction(ctx context.Context, tx types.Transaction) (*types.TaggedBase64, error) {
	response, err := c.tryPostRequest(ctx, c.baseUrl, tx)
	if err != nil {
		return nil, err
	}

	defer response.Body.Close()
	if response.StatusCode != 200 {
		return nil, fmt.Errorf("received unexpected status code: %v", response.StatusCode)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, err
	}

	var hash types.TaggedBase64
	if err := json.Unmarshal(body, &hash); err != nil {
		return nil, err
	}
	return &hash, nil
}

type NamespaceResponse struct {
	Proof        *json.RawMessage     `json:"proof"`
	Transactions *[]types.Transaction `json:"transactions"`
}

func (c *Client) getRawMessage(ctx context.Context, format string, args ...any) (json.RawMessage, error) {
	res, err := c.tryGetRequest(ctx, c.baseUrl, format, args...)
	if err != nil {
		return nil, err
	}

	defer res.Body.Close()

	if res.StatusCode != 200 {
		// Try to get the response body to include in the error message, as it may have useful
		// information about why the request failed. If this call fails, the response will be `nil`,
		// which is fine to include in the log, so we can ignore errors.
		body, _ := io.ReadAll(res.Body)
		return nil, fmt.Errorf("request failed with status %d and body %s", res.StatusCode, string(body))
	}

	// Read the response body into memory before we unmarshal it, rather than passing the io.Reader
	// to the json decoder, so that we still have the body and can inspect it if unmarshalling
	// failed.
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	return body, nil
}

func (c *Client) get(ctx context.Context, out any, format string, args ...any) error {
	body, err := c.getRawMessage(ctx, format, args...)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(body, out); err != nil {
		return fmt.Errorf("request failed with body %s and error %v", string(body), err)
	}
	return nil
}

func (c *Client) tryGetRequest(ctx context.Context, baseUrl, format string, args ...interface{}) (*http.Response, error) {

	url := baseUrl + fmt.Sprintf(format, args...)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	return c.client.Do(req)

}

func (c *Client) tryPostRequest(ctx context.Context, baseUrl string, tx types.Transaction) (*http.Response, error) {

	marshalled, err := json.Marshal(tx)
	if err != nil {
		return nil, err
	}

	request, err := http.NewRequestWithContext(ctx, "POST", baseUrl+"submit/submit", bytes.NewBuffer(marshalled))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/json")
	return c.client.Do(request)
}
