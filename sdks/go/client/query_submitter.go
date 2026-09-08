package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"net/http"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
)

type QuerySubmitter struct {
	baseUrl string
	client  *http.Client
}

// QuerySubmitter's constructor.
func NewQuerySubmitter(baseUrl string) *QuerySubmitter {
	url := formatUrl(baseUrl)

	return &QuerySubmitter{
		baseUrl: url,
		client:  newHTTPClient(),
	}
}

// The QuerySubmitter's implementation of submit transaction, which targets the query services transaction submission endpoint.
func (q *QuerySubmitter) SubmitTransaction(ctx context.Context, tx types.Transaction) (*types.TaggedBase64, error) {
	response, err := q.tryPostRequest(ctx, q.baseUrl, tx)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return decodeSubmitResponse(response)
}

// This function handles the http post requests for the query submitter.
func (q *QuerySubmitter) tryPostRequest(ctx context.Context, baseUrl string, tx types.Transaction) (*http.Response, error) {

	marshalled, err := json.Marshal(tx)
	if err != nil {
		return nil, err
	}

	request, err := http.NewRequestWithContext(ctx, "POST", baseUrl+"submit/submit", bytes.NewBuffer(marshalled))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/json")
	return q.client.Do(request)
}
