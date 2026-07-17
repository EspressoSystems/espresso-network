package client

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	common "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	"github.com/coder/websocket"
)

var _ QueryService = (*Client)(nil)
var _ SubmitAPI = (*Client)(nil)
var _ EspressoClient = (*Client)(nil)

type EspressoClientConfigOption func(*EspressoClientConfig)

type EspressoClientConfig struct {
	BaseUrl              string
	TransactionSubmitter SubmitAPI
}

var DefaultEspressoClientConfig = EspressoClientConfig{
	BaseUrl: "query.main.net.espresso.network",
}

// Validate that the espressoClientConfig is valid.
func ValidateEspressoClientConfig(config EspressoClientConfig) error {
	if config.TransactionSubmitter == nil {
		return fmt.Errorf("transaction submitter cannot be nil when creating an espresso client")
	}
	return nil
}

// This option is used to set the transaction submitter (any implementer of the SubmitAPI), during the constructor
// of the EspressoClient.
func WithTransactionSubmitter(transactionSubmitter SubmitAPI) EspressoClientConfigOption {
	return func(config *EspressoClientConfig) {
		config.TransactionSubmitter = transactionSubmitter
	}
}

// This option is used to set the base URL of the client in the constructor.
func WithBaseUrl(baseUrl string) EspressoClientConfigOption {
	return func(config *EspressoClientConfig) {
		formattedBaseUrl := formatUrl(baseUrl)
		config.BaseUrl = formattedBaseUrl
	}
}

type Client struct {
	baseUrl              string
	client               *http.Client
	transactionSubmitter SubmitAPI
}

// NewClientFromOptions:
// This function allows SDK users to construct an EspressoClient with any transaction submitter that implements
// the SubmitAPI. This is the preferred method of constructing an EspressoClient.
func NewClientFromOptions(options ...EspressoClientConfigOption) (*Client, error) {
	config := DefaultEspressoClientConfig
	for _, option := range options {
		option(&config)
	}

	if err := ValidateEspressoClientConfig(config); err != nil {
		return nil, err
	}
	return &Client{
		baseUrl:              config.BaseUrl,
		client:               http.DefaultClient,
		transactionSubmitter: config.TransactionSubmitter,
	}, nil
}

// NewClient:
// This function is the default construction of the espresso client.
// It has been left for compatibility reasons (namely, in the multiple-nodes client)
// New instances of using this client should use NewClientFromOptions()
func NewClient(baseUrl string) *Client {
	url := formatUrl(baseUrl)
	return &Client{
		baseUrl:              url,
		client:               http.DefaultClient,
		transactionSubmitter: NewQuerySubmitter(url),
	}
}

// Transaction submission or fetch error due to a server issue, IO error, timeout, etc., that may
// be fixed a retry.
var ErrEphemeral = errors.New("retryable")

// Transaction submission or fetch error due to invalid information or any failure that cannot be
// resolved by a retry.
var ErrPermanent = errors.New("not retryable")

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

func (c *Client) FetchExplorerTransactionByHash(ctx context.Context, hash *types.TaggedBase64) (types.ExplorerTransactionQueryData, error) {
	if hash == nil {
		return types.ExplorerTransactionQueryData{}, fmt.Errorf("%w: hash is nil", ErrPermanent)
	}
	var res types.ExplorerTransactionQueryData
	if err := c.get(ctx, &res, "explorer/transaction/hash/%s", hash.String()); err != nil {
		return types.ExplorerTransactionQueryData{}, err
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

func (c *Client) FetchNamespaceTransactionsInRange(ctx context.Context, from uint64, until uint64, namespace uint64) ([]types.NamespaceTransactionsRangeData, error) {
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

func (c *Client) FetchLimits(ctx context.Context) (types.LimitsData, error) {
	var res types.LimitsData
	if err := c.get(ctx, &res, "availability/limits"); err != nil {
		return types.LimitsData{}, err
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
	return c.transactionSubmitter.SubmitTransaction(ctx, tx)
}

// Stream of JSON-encoded objects over a WebSocket connection
type WsStream[S any] struct {
	conn *websocket.Conn
}

func (s *WsStream[S]) NextRaw(ctx context.Context) (json.RawMessage, error) {
	typ, msg, err := s.conn.Read(ctx)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	if typ != websocket.MessageText {
		return nil, fmt.Errorf("%w: unexpected non-text WebSocket message type: %v", ErrEphemeral, typ)
	}
	return msg, nil
}

func (s *WsStream[S]) Next(ctx context.Context) (*S, error) {
	typ, msg, err := s.conn.Read(ctx)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	if typ != websocket.MessageText {
		return nil, fmt.Errorf("%w: unexpected non-text WebSocket message type: %v", ErrEphemeral, typ)
	}
	var data S
	if err := json.Unmarshal(msg, &data); err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &data, nil
}

func (s *WsStream[S]) Close() error {
	return s.conn.Close(websocket.StatusNormalClosure, "")
}

// Open a `Stream` of Espresso transactions starting from a specific block height.
func (c *Client) StreamTransactions(ctx context.Context, height uint64) (Stream[types.TransactionQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.client
	url := c.baseUrl + fmt.Sprintf("availability/stream/transactions/%d", height)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.TransactionQueryData]{conn: conn}, nil
}

// Open a `Stream` of Espresso transactions starting from a specific block height, filtered by namespace.
func (c *Client) StreamTransactionsInNamespace(ctx context.Context, height uint64, namespace uint64) (Stream[types.TransactionQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.client
	url := c.baseUrl + fmt.Sprintf("availability/stream/transactions/%d/namespace/%d", height, namespace)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.TransactionQueryData]{conn: conn}, nil
}

func (c *Client) StreamPayloads(ctx context.Context, height uint64) (Stream[types.PayloadQueryData], error) {
	opts := &websocket.DialOptions{}
	opts.HTTPClient = c.client
	url := c.baseUrl + fmt.Sprintf("availability/stream/payloads/%d", height)
	conn, _, err := websocket.Dial(ctx, url, opts)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &WsStream[types.PayloadQueryData]{conn: conn}, nil
}

type NamespaceResponse struct {
	Proof        *json.RawMessage     `json:"proof"`
	Transactions *[]types.Transaction `json:"transactions"`
}

func (c *Client) getRawMessage(ctx context.Context, format string, args ...any) (json.RawMessage, error) {
	res, err := c.tryGetRequest(ctx, c.baseUrl, format, args...)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
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

func (c *Client) get(ctx context.Context, out any, format string, args ...any) error {
	body, err := c.getRawMessage(ctx, format, args...)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(body, out); err != nil {
		return fmt.Errorf("%w: request failed with body %s and error %v", ErrEphemeral, string(body), err)
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
