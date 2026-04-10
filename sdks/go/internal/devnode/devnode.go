package devnode

import (
	"context"
	"fmt"
	"net"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"testing"
	"time"
)

type Ports struct {
	SequencerAPI int
	Builder      int
	DevNode      int
}

func (p Ports) NodeURL() string {
	return fmt.Sprintf("http://localhost:%d", p.SequencerAPI)
}

func (p Ports) BuilderURL() string {
	return fmt.Sprintf("http://localhost:%d", p.Builder)
}

func (p Ports) DevNodeURL() string {
	return fmt.Sprintf("http://localhost:%d", p.DevNode)
}

func AllocatePorts() (Ports, error) {
	ports := make([]int, 3)
	listeners := make([]net.Listener, 3)
	for i := range listeners {
		l, err := net.Listen("tcp", "127.0.0.1:0")
		if err != nil {
			// Close any already opened listeners.
			for j := 0; j < i; j++ {
				listeners[j].Close()
			}
			return Ports{}, fmt.Errorf("failed to allocate port: %w", err)
		}
		listeners[i] = l
		ports[i] = l.Addr().(*net.TCPAddr).Port
	}
	// Close all listeners so the dev node can bind to these ports.
	for _, l := range listeners {
		l.Close()
	}
	return Ports{
		SequencerAPI: ports[0],
		Builder:      ports[1],
		DevNode:      ports[2],
	}, nil
}

func Start(t *testing.T, ctx context.Context, ports Ports, storageDir string) func() {
	t.Helper()

	storageDir, err := filepath.Abs(storageDir)
	if err != nil {
		t.Fatalf("failed to get absolute path for storage dir: %v", err)
	}

	var p *exec.Cmd
	if bin := os.Getenv("ESPRESSO_DEV_NODE_BIN"); bin != "" {
		if _, err := os.Stat(bin); err != nil {
			t.Fatalf("ESPRESSO_DEV_NODE_BIN=%s does not exist: %v", bin, err)
		}
		t.Logf("using pre-built espresso-dev-node binary: %s", bin)
		p = exec.CommandContext(ctx, bin)
	} else {
		t.Log("ESPRESSO_DEV_NODE_BIN not set, falling back to cargo run")
		p = exec.CommandContext(ctx, "cargo", "run", "-p", "espresso-dev-node")
		// Resolve repo root from this source file: sdks/go/internal/devnode/devnode.go
		_, thisFile, _, _ := runtime.Caller(0)
		p.Dir = filepath.Join(filepath.Dir(thisFile), "..", "..", "..", "..")
	}

	logFile, err := os.CreateTemp(storageDir, "dev-node-*.log")
	if err != nil {
		t.Fatalf("failed to create log file: %v", err)
	}
	logPath := logFile.Name()
	t.Logf("dev-node logs: %s", logPath)

	p.Stdout = logFile
	p.Stderr = logFile

	env := os.Environ()
	env = append(env, fmt.Sprintf("ESPRESSO_SEQUENCER_API_PORT=%d", ports.SequencerAPI))
	env = append(env, fmt.Sprintf("ESPRESSO_BUILDER_PORT=%d", ports.Builder))
	env = append(env, fmt.Sprintf("ESPRESSO_DEV_NODE_PORT=%d", ports.DevNode))
	env = append(env, "ESPRESSO_SEQUENCER_ETH_MNEMONIC=test test test test test test test test test test test junk")
	env = append(env, "ESPRESSO_DEPLOYER_ACCOUNT_INDEX=0")
	env = append(env, fmt.Sprintf("ESPRESSO_SEQUENCER_STORAGE_PATH=%s", storageDir))
	p.Env = env

	if err := p.Start(); err != nil {
		logFile.Close()
		t.Fatalf("failed to start dev node: %v", err)
	}

	// Wait for process to exit in background so we can collect the exit status.
	done := make(chan error, 1)
	go func() {
		done <- p.Wait()
	}()

	t.Cleanup(func() {
		logFile.Close()
		if t.Failed() {
			data, err := os.ReadFile(logPath)
			if err != nil {
				t.Logf("failed to read dev-node log: %v", err)
			} else {
				t.Logf("=== dev-node logs ===\n%s\n=== end dev-node logs ===", string(data))
			}
		}
	})

	return func() {
		if p.Process != nil {
			_ = p.Process.Kill()
			// Wait for process to actually exit to avoid zombies.
			select {
			case <-done:
			case <-time.After(5 * time.Second):
			}
		}
	}
}
