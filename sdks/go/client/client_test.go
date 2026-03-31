package client

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	"github.com/EspressoSystems/espresso-network/sdks/go/verification"
	"github.com/ethereum/go-ethereum/log"
	"github.com/stretchr/testify/require"
)

var workingDir = "../../../"

const devNodeURL = "http://localhost:21000"

func setupDevNode(t *testing.T) context.Context {
	t.Helper()
	ctx, cancel := context.WithCancel(context.Background())
	t.Cleanup(cancel)

	dir := t.TempDir()
	cleanup := runDevNode(ctx, dir)
	t.Cleanup(cleanup)

	err := waitForEspressoNode(ctx)
	require.NoError(t, err, "failed to start espresso dev node")

	return ctx
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
	ctx := setupDevNode(t)
	client := NewClient(devNodeURL)

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

func runDevNode(ctx context.Context, tmpDir string) func() {
	tmpDir, err := filepath.Abs(tmpDir)
	if err != nil {
		panic(err)
	}

	var p *exec.Cmd
	if bin := os.Getenv("ESPRESSO_DEV_NODE_BIN"); bin != "" {
		if _, err := os.Stat(bin); err != nil {
			panic(fmt.Sprintf("ESPRESSO_DEV_NODE_BIN=%s does not exist: %v", bin, err))
		}
		fmt.Println("using pre-built espresso-dev-node binary:", bin)
		p = exec.CommandContext(ctx, bin)
	} else {
		fmt.Println("ESPRESSO_DEV_NODE_BIN not set, falling back to cargo run (this will compile espresso-dev-node)")
		p = exec.CommandContext(ctx, "cargo", "run", "-p", "espresso-dev-node")
		p.Dir = workingDir
	}

	env := os.Environ()
	env = append(env, "ESPRESSO_NODE_API_PORT=21000")
	env = append(env, "ESPRESSO_BUILDER_PORT=23000")
	env = append(env, "ESPRESSO_DEV_NODE_PORT=20000")
	env = append(env, "ESPRESSO_ETH_MNEMONIC=test test test test test test test test test test test junk")
	env = append(env, "ESPRESSO_DEPLOYER_ACCOUNT_INDEX=0")
	env = append(env, "ESPRESSO_NODE_STORAGE_PATH="+tmpDir)
	p.Env = env

	go func() {
		if err := p.Run(); err != nil {
			if err.Error() != "signal: killed" {
				log.Error(err.Error())
				panic(err)
			}
		}
	}()

	return func() {
		if p.Process != nil {
			err := p.Process.Kill()
			if err != nil {
				log.Error(err.Error())
				panic(err)
			}
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

func waitForEspressoNode(ctx context.Context) error {
	client := NewClient(devNodeURL)
	return waitForWith(ctx, 200*time.Second, 1*time.Second, func() bool {
		height, err := client.FetchLatestBlockHeight(ctx)
		return err == nil && height >= 2
	})
}

func TestExplorerFetchTransactionByHash(t *testing.T) {
	ctx := setupDevNode(t)
	client := NewClient(devNodeURL)

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

func TestNamespaceTransactionsInRange(t *testing.T) {
	ctx := setupDevNode(t)
	client := NewClient(devNodeURL)
	testNamespaceTransactionsInRange(t, ctx, client, "namespace range test")
}
