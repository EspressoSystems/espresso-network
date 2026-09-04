package client

import (
	"context"
	"net/http"
	"net/http/httptest"
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
// black-holed endpoint looks like to the SDK.
func blackHoleNode(t *testing.T) string {
	t.Helper()
	release := make(chan struct{})
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		<-release
	}))
	// Close waits for the handlers, which only return once released.
	t.Cleanup(func() {
		close(release)
		server.Close()
	})
	return server.URL
}

// The wedge this exists for: a black-holed node must not park a caller whose
// context has no deadline. The timeout is shortened from the default so the
// test runs in milliseconds; the mechanism under test is the same.
func TestBlackHoledNodeDoesNotParkTheCaller(t *testing.T) {
	url := blackHoleNode(t)
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
