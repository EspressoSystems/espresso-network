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

// reservePort allocates a free TCP port and puts it into TIME_WAIT state.
// This prevents the OS from handing it out via ephemeral allocation, while
// still allowing explicit binds (like the dev node will do).
// Mirrors the Rust reserve_tcp_port() in test-utils/src/lib.rs.
func reservePort() (int, error) {
	server, err := net.Listen("tcp", "127.0.0.1:0")
	if err != nil {
		return 0, fmt.Errorf("failed to listen: %w", err)
	}
	addr := server.Addr().(*net.TCPAddr)
	port := addr.Port

	// Complete a TCP handshake to force TIME_WAIT on close.
	client, err := net.Dial("tcp", addr.String())
	if err != nil {
		server.Close()
		return 0, fmt.Errorf("failed to dial: %w", err)
	}
	accepted, err := server.Accept()
	if err != nil {
		client.Close()
		server.Close()
		return 0, fmt.Errorf("failed to accept: %w", err)
	}
	// Close all sockets -- port enters TIME_WAIT.
	accepted.Close()
	client.Close()
	server.Close()

	return port, nil
}

func AllocatePorts() (Ports, error) {
	apiPort, err := reservePort()
	if err != nil {
		return Ports{}, err
	}
	builderPort, err := reservePort()
	if err != nil {
		return Ports{}, err
	}
	devNodePort, err := reservePort()
	if err != nil {
		return Ports{}, err
	}
	return Ports{
		SequencerAPI: apiPort,
		Builder:      builderPort,
		DevNode:      devNodePort,
	}, nil
}

func Start(t *testing.T, ctx context.Context, ports Ports, storageDir string) {
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
		// Resolve repo root from this source file at sdks/go/internal/devnode/
		_, thisFile, _, ok := runtime.Caller(0)
		if !ok {
			t.Fatal("runtime.Caller failed")
		}
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
		t.Fatalf("failed to start dev node: %v", err)
	}

	// Wait for process to exit in background so we can collect the exit status.
	done := make(chan error, 1)
	go func() {
		done <- p.Wait()
	}()

	// Cleanups run in LIFO order: kill process first, then read/close log.
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
	t.Cleanup(func() {
		if p.Process != nil {
			_ = p.Process.Kill()
			select {
			case <-done:
			case <-time.After(5 * time.Second):
			}
		}
	})
}
