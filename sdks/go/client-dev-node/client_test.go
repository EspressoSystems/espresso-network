package clientdevnode

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"testing"
	"time"

	"github.com/ethereum/go-ethereum/log"
	"github.com/stretchr/testify/assert"
)

var workingDir = "../../../"

func TestFetchDevInfo(t *testing.T) {
	ctx := context.Background()
	dir, err := os.MkdirTemp("", "espresso-dev-node")
	if err != nil {
		panic(err)
	}
	defer os.RemoveAll(dir)
	cleanup := runDevNode(ctx, dir)
	defer cleanup()

	client := NewClient("http://localhost:20000/v0")

	for {
		available, err := client.IsAvailable(ctx)
		if available {
			break
		}
		fmt.Println("waiting for node to be available", err)
		time.Sleep(1 * time.Second)
	}

	devInfo, err := client.FetchDevInfo(ctx)
	if err != nil {
		t.Fatal("failed to fetch dev info", err)
	}
	assert.Equal(t, "http://localhost:23000/", devInfo.BuilderUrl)
	assert.Equal(t, 21000, int(devInfo.SequencerApiPort))
	// This serves as a reminder that the L1 light client address has changed when it breaks.
	assert.Equal(t, "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0", devInfo.L1LightClientAddress)
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
