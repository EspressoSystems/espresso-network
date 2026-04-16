package client

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/EspressoSystems/espresso-network/sdks/go/internal/devnode"
	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	"github.com/EspressoSystems/espresso-network/sdks/go/verification"
	"github.com/stretchr/testify/require"
)

type devNodeInfo struct {
	nodeURL    string
	builderURL string
}

func setupDevNode(t *testing.T) (context.Context, devNodeInfo) {
	t.Helper()
	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)

	ports, err := devnode.AllocatePorts()
	require.NoError(t, err, "failed to allocate ports")

	dir := t.TempDir()
	devnode.Start(t, ctx, ports, dir)

	err = waitForEspressoNode(ctx, ports.NodeURL())
	require.NoError(t, err, "failed to start espresso dev node")

	return ctx, devNodeInfo{nodeURL: ports.NodeURL(), builderURL: ports.BuilderURL()}
}

func testNamespaceTransactionsInRange(t *testing.T, ctx context.Context, client EspressoClient, txPayload string) {
	t.Helper()

	namespace := uint64(12345)
	tx := types.Transaction{Namespace: namespace, Payload: []byte(txPayload)}
	_, err := client.SubmitTransaction(ctx, tx)
	require.NoError(t, err, "failed to submit transaction")

	var blocksWithNamespaceTransactions []types.NamespaceTransactionsRangeData
	err = waitForWith(ctx, 30*time.Second, 2*time.Second, func() bool {
		height, fetchErr := client.FetchLatestBlockHeight(ctx)
		if fetchErr != nil || height < 2 {
			return false
		}
		blocksWithNamespaceTransactions, fetchErr = client.FetchNamespaceTransactionsInRange(ctx, 1, height, namespace)
		return fetchErr == nil && len(blocksWithNamespaceTransactions) > 0
	})
	require.NoError(t, err, "failed to fetch namespace transactions in range")

	for _, block := range blocksWithNamespaceTransactions {
		for _, txn := range block.Transactions {
			require.Equal(t, namespace, txn.Namespace)
			require.NotEmpty(t, txn.Payload)
		}
	}
	_, err = client.FetchNamespaceTransactionsInRange(ctx, 0, 1000, namespace)
	require.Error(t, err, "expected error for large range")
}

func TestApiWithEspressoDevNode(t *testing.T) {
	ctx, info := setupDevNode(t)
	client := NewClient(info.nodeURL)

	ClientTestHelper(ctx, client, t)

	var clientOptions []EspressoClientConfigOption
	builderSubmitter, err := NewBuilderSubmitter([]string{info.builderURL})
	if err != nil {
		t.Fatal("failed to create builder submitter", err)
	}

	clientOptions = append(clientOptions, WithTransactionSubmitter(builderSubmitter))
	clientOptions = append(clientOptions, WithBaseUrl(info.nodeURL))

	client, err = NewClientFromOptions(clientOptions...)
	if err != nil {
		t.Fatal("failed to create espresso client with builder submitter")
	}

	ClientTestHelper(ctx, client, t)

	clientOptions = []EspressoClientConfigOption{}
	querySubmitter := NewQuerySubmitter(info.nodeURL)
	if err != nil {
		t.Fatal("failed to create builder submitter", err)
	}
	clientOptions = append(clientOptions, WithTransactionSubmitter(querySubmitter))
	clientOptions = append(clientOptions, WithBaseUrl(info.nodeURL))

	client, err = NewClientFromOptions(clientOptions...)
	if err != nil {
		t.Fatal("Failed to create query submitter based client")
	}
	ClientTestHelper(ctx, client, t)
}

func ClientTestHelper(ctx context.Context, client EspressoClient, t *testing.T) {

	_, err := client.FetchLatestBlockHeight(ctx)
	if err != nil {
		t.Fatal("failed to fetch block height", err)
	}

	blockHeight := uint64(1)
	_, err = client.FetchHeaderByHeight(ctx, blockHeight)
	if err != nil {
		t.Fatal("failed to fetch header by height", err)
	}

	_, err = client.FetchVidCommonByHeight(ctx, blockHeight)
	if err != nil {
		t.Fatal("failed to fetch vid common by height", err)
	}

	_, err = client.FetchHeadersByRange(ctx, 1, 1)
	if err != nil {
		t.Fatal("failed to fetch headers by range", err)
	}

	// Try submitting a transaction
	tx := types.Transaction{
		Namespace: 1,
		Payload:   []byte("hello world"),
	}
	hash, err := client.SubmitTransaction(ctx, tx)
	if err != nil {
		t.Fatal("failed to submit transaction", err)
	}
	fmt.Println("submitted transaction with hash", hash)

	stream, err := client.StreamTransactions(ctx, 1)
	require.NoError(t, err)

	txData, err := stream.Next(ctx)
	require.NoError(t, err)
	require.NotNil(t, txData)
	require.Equal(t, txData.Transaction.Payload, tx.Payload)
	require.Equal(t, txData.Transaction.Namespace, tx.Namespace)

	// Test streaming with namespace filter
	nsStream, err := client.StreamTransactionsInNamespace(ctx, 1, tx.Namespace)
	require.NoError(t, err)

	nsTxData, err := nsStream.Next(ctx)
	require.NoError(t, err)
	require.NotNil(t, nsTxData)
	require.Equal(t, nsTxData.Transaction.Payload, tx.Payload)
	require.Equal(t, nsTxData.Transaction.Namespace, tx.Namespace)

	payloadStream, err := client.StreamPayloads(ctx, 1)
	require.NoError(t, err)

	for {
		timeoutCtx, cancel := context.WithTimeout(ctx, time.Second)
		blockData, err := payloadStream.Next(timeoutCtx)
		cancel()
		require.NoError(t, err)
		require.NotNil(t, blockData)
		if blockData.Height == nsTxData.BlockHeight {
			txns, err := verification.DecodePayload(blockData.BlockPayload)
			require.NoError(t, err)
			require.NotNil(t, txns)
			require.Equal(t, txns[0].Payload, tx.Payload)
			require.Equal(t, txns[0].Namespace, tx.Namespace)
			break
		}
	}
}

func waitForWith(
	ctxinput context.Context,
	timeout time.Duration,
	interval time.Duration,
	condition func() bool,
) error {
	ctx, cancel := context.WithTimeout(ctxinput, timeout)
	defer cancel()

	for {
		if condition() {
			return nil
		}
		select {
		case <-time.After(interval):
		case <-ctx.Done():
			return ctx.Err()
		}
	}
}

func waitForEspressoNode(ctx context.Context, nodeURL string) error {
	client := NewClient(nodeURL)
	return waitForWith(ctx, 200*time.Second, 1*time.Second, func() bool {
		height, err := client.FetchLatestBlockHeight(ctx)
		return err == nil && height >= 2
	})
}

func TestExplorerFetchTransactionByHash(t *testing.T) {
	ctx, info := setupDevNode(t)
	client := NewClient(info.nodeURL)

	tx := types.Transaction{Namespace: 1, Payload: []byte("explorer test")}
	hash, err := client.SubmitTransaction(ctx, tx)
	require.NoError(t, err, "failed to submit transaction")

	// Explorer indexes asynchronously, so poll until the transaction is available.
	err = waitForWith(ctx, 30*time.Second, 2*time.Second, func() bool {
		_, fetchErr := client.FetchExplorerTransactionByHash(ctx, hash)
		return fetchErr == nil
	})
	require.NoError(t, err, "failed to fetch transaction by hash from explorer")
}

func TestFetchBlockSummaries(t *testing.T) {
	ctx, info := setupDevNode(t)
	client := NewClient(info.nodeURL)

	var height uint64
	err := waitForWith(ctx, 30*time.Second, 2*time.Second, func() bool {
		h, fetchErr := client.FetchLatestBlockHeight(ctx)
		if fetchErr != nil {
			return false
		}
		height = h
		return height >= 3
	})
	require.NoError(t, err, "failed to wait for blocks")

	resp, err := client.FetchBlockSummaries(ctx, nil, 3)
	require.NoError(t, err, "FetchBlockSummaries(nil, 3) failed")
	require.NotEmpty(t, resp.BlockSummaries)
	require.LessOrEqual(t, len(resp.BlockSummaries), 3)

	from := uint64(2)
	resp, err = client.FetchBlockSummaries(ctx, &from, 2)
	require.NoError(t, err, "FetchBlockSummaries(&2, 2) failed")
	require.NotEmpty(t, resp.BlockSummaries)
	require.LessOrEqual(t, len(resp.BlockSummaries), 2)

	for _, bs := range resp.BlockSummaries {
		require.LessOrEqual(t, bs.Height, from)
	}
}

func TestNamespaceTransactionsInRange(t *testing.T) {
	ctx, info := setupDevNode(t)
	client := NewClient(info.nodeURL)
	testNamespaceTransactionsInRange(t, ctx, client, "namespace range test")
}
