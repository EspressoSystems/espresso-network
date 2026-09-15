package client

import (
	"context"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/stretchr/testify/require"
)

// Cancelling a builder's share once its attempt has returned must not disturb
// the error that attempt produced.
func TestCancelDoesNotReplaceTheRecordedError(t *testing.T) {
	recorded := recordedErrors(t, 5*time.Second,
		failingBuilder(t, http.StatusServiceUnavailable),
		failingBuilder(t, http.StatusBadGateway))

	require.Len(t, recorded, 2)
	require.EqualError(t, recorded[0], "retryable: 503 Service Unavailable")
	require.EqualError(t, recorded[1], "retryable: 502 Bad Gateway")
}

// Walks urls under a caller deadline, so that every builder but the last is
// given a cancellable share, and returns what the failed walk recorded.
func recordedErrors(t *testing.T, budget time.Duration, urls ...string) []error {
	t.Helper()
	builders, err := NewBuilderSubmitter(urls)
	require.NoError(t, err)

	ctx, cancel := context.WithTimeout(context.Background(), budget)
	defer cancel()

	_, err = builders.SubmitTransaction(ctx, testTx)
	require.ErrorIs(t, err, ErrAllBuildersFailed)

	return builders.GetPreviousSubmissionErrors()
}

// Answers straight away with a failing status.
func failingBuilder(t *testing.T, status int) string {
	t.Helper()
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(status)
	}))
	t.Cleanup(server.Close)
	return server.URL
}
