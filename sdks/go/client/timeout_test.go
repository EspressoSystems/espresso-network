package client

import (
	"context"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	"github.com/EspressoSystems/espresso-network/sdks/go/internal/httpclient"
	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	"github.com/coder/websocket"
	"github.com/stretchr/testify/require"
)

// Stands in for httpclient.Timeout so the tests run in milliseconds.
const testTimeout = 100 * time.Millisecond

var testTx = types.Transaction{Namespace: 1, Payload: []byte("tx")}

func TestConstructorsBoundTheirHTTPClients(t *testing.T) {
	client := NewClient("http://localhost:1")
	require.Equal(t, httpclient.Timeout, client.client.Timeout)
	require.Equal(t, httpclient.Timeout, client.transactionSubmitter.(*QuerySubmitter).client.Timeout)

	fromOptions, err := NewClientFromOptions(WithBaseUrl("http://localhost:1"), WithTransactionSubmitter(NewQuerySubmitter("http://localhost:1")))
	require.NoError(t, err)
	require.Equal(t, httpclient.Timeout, fromOptions.client.Timeout)

	builders, err := NewBuilderSubmitter([]string{"http://localhost:1", "http://localhost:2"})
	require.NoError(t, err)
	require.Equal(t, httpclient.Timeout, builders.client.Timeout)

	nodes, err := NewMultipleNodesClient([]string{"http://localhost:1", "http://localhost:2"})
	require.NoError(t, err)
	for _, node := range nodes.nodes {
		require.Equal(t, httpclient.Timeout, node.client.Timeout)
	}
}

// Accepts the connection and never answers, like a black-holed endpoint.
func blackHoleNode(t *testing.T) (url string, requests func() int64) {
	t.Helper()
	var received atomic.Int64
	release := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		received.Add(1)
		<-release
	}))
	// Close waits for the handlers, which only return once released.
	t.Cleanup(func() {
		close(release)
		server.Close()
	})
	return server.URL, received.Load
}

func TestBlackHoledNodeDoesNotParkTheCaller(t *testing.T) {
	url, _ := blackHoleNode(t)

	client := NewClient(url)
	client.client.Timeout = testTimeout
	submitter := NewQuerySubmitter(url)
	submitter.client.Timeout = testTimeout
	builders, err := NewBuilderSubmitter([]string{url})
	require.NoError(t, err)
	builders.client.Timeout = testTimeout

	calls := []struct {
		name string
		call func(context.Context) error
	}{
		{"fetch", func(ctx context.Context) error {
			_, err := client.FetchLatestBlockHeight(ctx)
			return err
		}},
		{"query submit", func(ctx context.Context) error {
			_, err := submitter.SubmitTransaction(ctx, testTx)
			return err
		}},
		{"builder submit", func(ctx context.Context) error {
			_, err := builders.SubmitTransaction(ctx, testTx)
			return err
		}},
	}
	for _, tc := range calls {
		t.Run(tc.name, func(t *testing.T) {
			done := make(chan error, 1)
			go func() { done <- tc.call(context.Background()) }()
			select {
			case err := <-done:
				require.Error(t, err)
			case <-time.After(2 * time.Second):
				t.Fatal("the call was not bounded")
			}
		})
	}
}

func TestSequentialWalkGivesEachEndpointItsOwnShare(t *testing.T) {
	// Far below httpclient.Timeout, so it is the deadline split and not the client
	// timeout that has to leave the second endpoint a share.
	const callerBudget = 300 * time.Millisecond

	calls := []struct {
		name string
		call func(t *testing.T, ctx context.Context, urls []string) error
	}{
		{"multiple nodes fetch", func(t *testing.T, ctx context.Context, urls []string) error {
			nodes, err := NewMultipleNodesClient(urls)
			require.NoError(t, err)
			_, err = nodes.FetchLatestBlockHeight(ctx)
			return err
		}},
		{"multiple nodes submit", func(t *testing.T, ctx context.Context, urls []string) error {
			nodes, err := NewMultipleNodesClient(urls)
			require.NoError(t, err)
			_, err = nodes.SubmitTransaction(ctx, testTx)
			return err
		}},
		{"builder submit", func(t *testing.T, ctx context.Context, urls []string) error {
			builders, err := NewBuilderSubmitter(urls)
			require.NoError(t, err)
			_, err = builders.SubmitTransaction(ctx, testTx)
			return err
		}},
	}
	for _, tc := range calls {
		t.Run(tc.name, func(t *testing.T) {
			firstUrl, firstRequests := blackHoleNode(t)
			secondUrl, secondRequests := blackHoleNode(t)
			ctx, cancel := context.WithTimeout(context.Background(), callerBudget)
			defer cancel()

			require.Error(t, tc.call(t, ctx, []string{firstUrl, secondUrl}))
			require.Equal(t, int64(1), firstRequests())
			require.Equal(t, int64(1), secondRequests(), "the first endpoint consumed the whole deadline")
		})
	}
}

func TestShareRemainingBudget(t *testing.T) {
	t.Run("gives an endpoint half of what is left", func(t *testing.T) {
		caller, cancelCaller := context.WithTimeout(context.Background(), 4*time.Second)
		defer cancelCaller()

		share, cancel := shareRemainingBudget(caller, 4)
		defer cancel()
		deadline, ok := share.Deadline()
		require.True(t, ok)
		require.WithinDuration(t, time.Now().Add(2*time.Second), deadline, 100*time.Millisecond)
	})

	t.Run("gives the last endpoint everything that is left", func(t *testing.T) {
		caller, cancelCaller := context.WithTimeout(context.Background(), 4*time.Second)
		defer cancelCaller()

		share, cancel := shareRemainingBudget(caller, 1)
		defer cancel()
		deadline, ok := share.Deadline()
		require.True(t, ok)
		require.WithinDuration(t, time.Now().Add(4*time.Second), deadline, 100*time.Millisecond)
	})

	t.Run("leaves a caller without a deadline alone", func(t *testing.T) {
		share, cancel := shareRemainingBudget(context.Background(), 4)
		defer cancel()
		_, ok := share.Deadline()
		require.False(t, ok)
	})

	t.Run("expires immediately once the caller's deadline has passed", func(t *testing.T) {
		caller, cancelCaller := context.WithDeadline(context.Background(), time.Now().Add(-time.Second))
		defer cancelCaller()

		share, cancel := shareRemainingBudget(caller, 4)
		defer cancel()
		require.ErrorIs(t, share.Err(), context.DeadlineExceeded)
	})
}

func TestStreamOutlivesTheRequestTimeout(t *testing.T) {
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := websocket.Accept(w, r, nil)
		if err != nil {
			return
		}
		defer conn.CloseNow()
		time.Sleep(3 * testTimeout)
		_ = conn.Write(r.Context(), websocket.MessageText, []byte(`{}`))
	}))
	t.Cleanup(server.Close)

	client := NewClient(server.URL)
	client.client.Timeout = testTimeout

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	stream, err := client.StreamTransactions(ctx, 0)
	require.NoError(t, err)
	defer stream.Close()

	msg, err := stream.NextRaw(ctx)
	require.NoError(t, err)
	require.JSONEq(t, `{}`, string(msg))
}
