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
	"github.com/ethereum/go-ethereum/log"
)

var workingDir = "../../../"

func TestApiWithEspressoDevNode(t *testing.T) {
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

	client := NewClient("http://localhost:21000")

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

}

func runDevNode(ctx context.Context, tmpDir string) func() {
	tmpDir, err := filepath.Abs(tmpDir)
	if err != nil {
		panic(err)
	}

	invocation := []string{
		"run",
		"--bin",
		"espresso-dev-node",
		"--features=testing,embedded-db",
	}
	p := exec.CommandContext(ctx, "cargo", invocation...)
	p.Dir = workingDir

	env := os.Environ()
	env = append(env, "ESPRESSO_SEQUENCER_API_PORT=21000")
	env = append(env, "ESPRESSO_BUILDER_PORT=23000")
	env = append(env, "ESPRESSO_DEV_NODE_PORT=20000")
	env = append(env, "ESPRESSO_SEQUENCER_ETH_MNEMONIC=test test test test test test test test test test test junk")
	env = append(env, "ESPRESSO_DEPLOYER_ACCOUNT_INDEX=0")
	env = append(env, "ESPRESSO_SEQUENCER_STORAGE_PATH="+tmpDir)
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
	err := waitForWith(ctx, 200*time.Second, 1*time.Second, func() bool {
		out, err := exec.Command("curl", "-s", "-L", "-f", "http://localhost:21000/availability").Output()
		if err != nil {
			log.Warn("error executing curl command:", "err", err)
			return false
		}

		return len(out) > 0
	})
	if err != nil {
		return err
	}
	// Wait a bit for dev node to be ready totally
	time.Sleep(30 * time.Second)
	return nil
}
