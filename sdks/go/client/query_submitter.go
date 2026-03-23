package client

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
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
		client:  http.DefaultClient,
	}
}

// The QuerySubmitter's implementation of submit transaction, which targets the query services transaction submission endpoint.
func (q *QuerySubmitter) SubmitTransaction(ctx context.Context, tx types.Transaction) (*types.TaggedBase64, error) {
	response, err := q.tryPostRequest(ctx, q.baseUrl, tx)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrPermanent, err)
	}

	defer response.Body.Close()
	if response.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}

	var hash types.TaggedBase64
	if err := json.Unmarshal(body, &hash); err != nil {
		return nil, fmt.Errorf("%w: %v", ErrPermanent, err)
	}

	return &hash, nil
}

// This function handles the http post requests for the query submitter.
// This could likely be abstracted in a future PR to avoid code duplication between the individual submitter types.
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
