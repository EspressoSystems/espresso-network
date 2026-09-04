package client

import (
	"context"
	"net/http"
	"net/http/httptest"
	"sync/atomic"
	"testing"
	"time"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
	"github.com/coder/websocket"
	"github.com/stretchr/testify/require"
)

// Every constructor must hand out a bounded http.Client: a consumer that never
// sets a deadline on its context is exactly the caller that used to wedge.
func TestConstructorsBoundTheirHTTPClients(t *testing.T) {
	client := NewClient("http://localhost:1")
	require.Equal(t, requestTimeout, client.client.Timeout)
	require.Equal(t, requestTimeout, client.transactionSubmitter.(*QuerySubmitter).client.Timeout)

	fromOptions, err := NewClientFromOptions(WithBaseUrl("http://localhost:1"), WithTransactionSubmitter(NewQuerySubmitter("http://localhost:1")))
	require.NoError(t, err)
	require.Equal(t, requestTimeout, fromOptions.client.Timeout)

	builders, err := NewBuilderSubmitter([]string{"http://localhost:1", "http://localhost:2"})
	require.NoError(t, err)
	for _, builder := range builders.builderClients {
		require.Equal(t, requestTimeout, builder.Timeout)
	}

	nodes, err := NewMultipleNodesClient([]string{"http://localhost:1", "http://localhost:2"})
	require.NoError(t, err)
	for _, node := range nodes.nodes {
		require.Equal(t, requestTimeout, node.client.Timeout)
	}
}

// A node that accepts the connection and never answers, which is what a
// black-holed endpoint looks like to the SDK. requests reports how many
// requests reached it.
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

// The wedge this exists for: a black-holed node must not park a caller whose
// context has no deadline. The timeout is shortened from the default so the
// test runs in milliseconds; the mechanism under test is the same.
func TestBlackHoledNodeDoesNotParkTheCaller(t *testing.T) {
	url, _ := blackHoleNode(t)
	const timeout = 100 * time.Millisecond
	tx := types.Transaction{Namespace: 1, Payload: []byte("tx")}

	client := NewClient(url)
	client.client.Timeout = timeout
	submitter := NewQuerySubmitter(url)
	submitter.client.Timeout = timeout
	builders, err := NewBuilderSubmitter([]string{url})
	require.NoError(t, err)
	builders.builderClients[0].Timeout = timeout

	calls := map[string]func(context.Context) error{
		"fetch": func(ctx context.Context) error {
			_, err := client.FetchLatestBlockHeight(ctx)
			return err
		},
		"query submit": func(ctx context.Context) error {
			_, err := submitter.SubmitTransaction(ctx, tx)
			return err
		},
		"builder submit": func(ctx context.Context) error {
			_, err := builders.SubmitTransaction(ctx, tx)
			return err
		},
	}
	for name, call := range calls {
		t.Run(name, func(t *testing.T) {
			done := make(chan error, 1)
			go func() { done <- call(context.Background()) }()
			select {
			case err := <-done:
				require.Error(t, err)
			case <-time.After(10 * time.Second):
				t.Fatal("the call was not bounded")
			}
		})
	}
}

// Two black-holed nodes under one caller deadline that is far shorter than
// requestTimeout. The first node must not consume the whole deadline and hand
// the second an already-expired context: both have to be reached.
func TestSequentialWalkGivesEachEndpointItsOwnShare(t *testing.T) {
	firstUrl, firstRequests := blackHoleNode(t)
	secondUrl, secondRequests := blackHoleNode(t)
	const callerBudget = 600 * time.Millisecond
	tx := types.Transaction{Namespace: 1, Payload: []byte("tx")}

	nodes, err := NewMultipleNodesClient([]string{firstUrl, secondUrl})
	require.NoError(t, err)
	builders, err := NewBuilderSubmitter([]string{firstUrl, secondUrl})
	require.NoError(t, err)

	calls := map[string]func(context.Context) error{
		"multiple nodes fetch": func(ctx context.Context) error {
			_, err := nodes.FetchLatestBlockHeight(ctx)
			return err
		},
		"multiple nodes submit": func(ctx context.Context) error {
			_, err := nodes.SubmitTransaction(ctx, tx)
			return err
		},
		"builder submit": func(ctx context.Context) error {
			_, err := builders.SubmitTransaction(ctx, tx)
			return err
		},
	}
	for name, call := range calls {
		t.Run(name, func(t *testing.T) {
			before := firstRequests() + secondRequests()
			ctx, cancel := context.WithTimeout(context.Background(), callerBudget)
			defer cancel()

			require.Error(t, call(ctx))
			require.Equal(t, int64(2), firstRequests()+secondRequests()-before, "both endpoints should have been reached")
		})
	}
}

func TestShareRemainingBudget(t *testing.T) {
	t.Run("splits what is left evenly across the endpoints still to try", func(t *testing.T) {
		caller, cancelCaller := context.WithTimeout(context.Background(), 4*time.Second)
		defer cancelCaller()

		share, cancel := shareRemainingBudget(caller, 4)
		defer cancel()
		deadline, ok := share.Deadline()
		require.True(t, ok)
		require.WithinDuration(t, time.Now().Add(time.Second), deadline, 100*time.Millisecond)
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
}

// The request timeout bounds the WebSocket handshake only. An open stream has
// to outlive it, which is the coder/websocket behaviour the SDK relies on.
func TestStreamOutlivesTheRequestTimeout(t *testing.T) {
	const timeout = 100 * time.Millisecond
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		conn, err := websocket.Accept(w, r, nil)
		if err != nil {
			return
		}
		defer conn.CloseNow()
		time.Sleep(3 * timeout)
		_ = conn.Write(r.Context(), websocket.MessageText, []byte(`{}`))
	}))
	t.Cleanup(server.Close)

	client := NewClient(server.URL)
	client.client.Timeout = timeout

	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()
	stream, err := client.StreamTransactions(ctx, 0)
	require.NoError(t, err)
	defer stream.Close()

	msg, err := stream.NextRaw(ctx)
	require.NoError(t, err)
	require.JSONEq(t, `{}`, string(msg))
}
