package clientdevnode

import (
	"context"
	"fmt"
	"testing"
	"time"

	"github.com/EspressoSystems/espresso-network/sdks/go/internal/devnode"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func TestFetchDevInfo(t *testing.T) {
	ctx, cancel := context.WithTimeout(context.Background(), 200*time.Second)
	t.Cleanup(cancel)

	ports, err := devnode.AllocatePorts()
	require.NoError(t, err, "failed to allocate ports")

	dir := t.TempDir()
	devnode.Start(t, ctx, ports, dir)

	client := NewClient(fmt.Sprintf("%s/v0", ports.DevNodeURL()))

	for {
		available, err := client.IsAvailable(ctx)
		if available {
			break
		}
		if ctx.Err() != nil {
			t.Fatal("timed out waiting for node to be available")
		}
		t.Log("waiting for node to be available", err)
		time.Sleep(1 * time.Second)
	}

	devInfo, err := client.FetchDevInfo(ctx)
	if err != nil {
		t.Fatal("failed to fetch dev info", err)
	}
	assert.Equal(t, fmt.Sprintf("http://localhost:%d/", ports.Builder), devInfo.BuilderUrl)
	assert.Equal(t, ports.SequencerAPI, int(devInfo.SequencerApiPort))
	// This serves as a reminder that the L1 light client address has changed when it breaks.
	assert.Equal(t, "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0", devInfo.L1LightClientAddress)
}
