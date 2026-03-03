package client

import (
	"context"
	"fmt"
	"os"
	"testing"
	"time"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"
	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	"github.com/EspressoSystems/espresso-network/sdks/go/verification"
	"github.com/stretchr/testify/require"
)

func TestBuilderApiWithEspressoDevNode(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	dir, err := os.MkdirTemp("", "espresso-dev-node")
	if err != nil {
		panic(err)
	}
	defer os.RemoveAll(dir)
	cleanup := runDevNode(ctx, dir)
	defer cleanup()

	err = waitForEspressoNode(ctx)
	if err != nil {
		t.Fatal("failed to start espresso dev node", err)
	}

	client := NewBuilderClient("http://localhost:21000", []string{"http://localhost:21000"})

	_, err = client.FetchLatestBlockHeight(ctx)
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

func TestExplorerFetchTransactionByHashBuilder(t *testing.T) {

	ctx := context.Background()
	client := NewBuilderClient("https://query-0.main.net.espresso.network", []string{"http://foo.not-needed:8080"})

	txHash, err := tagged_base64.Parse("TX~onVqqws4O51Phy0QLzXaQkVpV_8VyVbYtSvmRAlF6p-K")
	if err != nil {
		t.Fatal("failed to parse tx hash", err)
	}
	_, err = client.FetchExplorerTransactionByHash(ctx, txHash)
	if err != nil {
		t.Fatal("failed to fetch block height", err)
	}
}

func TestNamespaceTransactionsInRangeBuilder(t *testing.T) {
	ctx := context.Background()
	client := NewBuilderClient("https://query.decaf.testnet.espresso.network", []string{"http://foo.not-needed:9090"})

	namespace := uint64(22266222)
	startHeight := uint64(6386698)
	endHeight := uint64(6386700)

	blocksWithNamespaceTransactions, err := client.FetchNamespaceTransactionsInRange(ctx, startHeight, endHeight, namespace)
	if err != nil {
		t.Fatal("failed to fetch namespace transactions in range", err)
	}

	if len(blocksWithNamespaceTransactions) != 2 {
		t.Fatalf("expected 2 blocks with namespace transactions, got %d", len(blocksWithNamespaceTransactions))
	}

	for _, blocks := range blocksWithNamespaceTransactions {
		for _, tx := range blocks.Transactions {
			if tx.Namespace != namespace {
				t.Fatalf("expected namespace %d, got %d", namespace, tx.Namespace)
			}
			if len(tx.Payload) == 0 {
				t.Fatal("transaction payload is empty")
			}
		}
	}

	startHeight = uint64(6386698)
	endHeight = uint64(6389700)

	// test if startHeight and endHeight are greater than 100 (which is the limit) then it throws an error
	_, err = client.FetchNamespaceTransactionsInRange(ctx, startHeight, endHeight, namespace)
	if err == nil {
		t.Fatal("expected error for large range, but got none")
	}
}
