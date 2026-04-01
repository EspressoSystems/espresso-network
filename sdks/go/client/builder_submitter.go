package client

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
)

var _ SubmitAPI = (*BuilderSubmitter)(nil)
var _ EspressoBuilderSubmitter = (*BuilderSubmitter)(nil)

type BuilderSubmitter struct {
	builderUrls          []string
	builderClients       []*http.Client
	previousSubmitErrors []error
}

// The builder submitter's constructor.
func NewBuilderSubmitter(builderUrls []string) (*BuilderSubmitter, error) {
	if len(builderUrls) < 1 {
		return nil, fmt.Errorf("One or more builder url's is required for the builder submitter")
	}

	builderClients := make([]*http.Client, len(builderUrls))
	formattedUrls := make([]string, len(builderUrls))
	for i, url := range builderUrls {
		formattedUrls[i] = formatUrl(url)
		builderClients[i] = http.DefaultClient
	}

	return &BuilderSubmitter{
		builderUrls:    formattedUrls,
		builderClients: builderClients,
	}, nil
}

// This error is meant to signal to consumer code (via the client methods), that despite the txn submission being successful,
// some attempts to submit were unsuccessful. This indicates that the user should check the cached errors in previousSubmitErrors
var ErrAllBuildersFailed = errors.New("submission to all builders failed, check previousSubmitErrors")

// SubmitTransaction:
//
// Submits a transaction to the espresso network via one of many  builder node that exposes the builder submit API.
// on a successful submission to one of any of the BuilderSubmitter's builder URL's, the caller will receive a TaggedBase64 transaction hash
// representing the transaction on the espresso network. If any attempts to submit to individual builders failed, they will be recorded
// in the BuilderSubmitter's previousSubmitErrors buffer. If the caller cares about these errors, they can retrieve them via
// BuilderSubmiter.GetPreviousSubmissionErrors()
//
// Parameters:
// - ctx context.Context: context used for cancelling in flight requests when callers need to end the process,
// - tx types.Transaction: The espresso transaction to submit with the client.
// Returns:
// - Transaction hash types.TaggedBase64: The hash of the transaction that has been submitted to espresso
// Errors:
// If all builders fail, this function will return an error. Otherwise, err will be nil.
func (c *BuilderSubmitter) SubmitTransaction(ctx context.Context, tx types.Transaction) (*types.TaggedBase64, error) {
	c.previousSubmitErrors = make([]error, 0)
	for clientIdx, url := range c.builderUrls {
		response, err := c.tryPostRequest(ctx, url, clientIdx, tx)

		if err != nil {
			c.previousSubmitErrors = append(c.previousSubmitErrors, err)
			continue
		}

		defer response.Body.Close()
		if response.StatusCode != http.StatusOK {
			c.previousSubmitErrors = append(c.previousSubmitErrors, fmt.Errorf("%w: %v", ErrEphemeral, response.Status))
			response.Body.Close()
			continue
		}

		body, err := io.ReadAll(response.Body)
		if err != nil {
			c.previousSubmitErrors = append(c.previousSubmitErrors, fmt.Errorf("%w: %v", ErrEphemeral, err))
			response.Body.Close()
			continue
		}

		var hash types.TaggedBase64
		if err := json.Unmarshal(body, &hash); err != nil {
			c.previousSubmitErrors = append(c.previousSubmitErrors, fmt.Errorf("%w: %v", ErrPermanent, err))
			response.Body.Close()
			continue
		}
		// If we receive a successful submission from the builder, we can exit as we don't need to send to other builders.
		return &hash, nil
	}
	return nil, ErrAllBuildersFailed
}

// post request handler for the builder submitter.
func (c *BuilderSubmitter) tryPostRequest(ctx context.Context, baseUrl string, clientIndex int, tx types.Transaction) (*http.Response, error) {
	marshalled, err := json.Marshal(tx)
	if err != nil {
		return nil, err
	}

	request, err := http.NewRequestWithContext(ctx, "POST", baseUrl+"txn_submit/submit", bytes.NewBuffer(marshalled))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/json")
	return c.builderClients[clientIndex].Do(request)
}

func (c *BuilderSubmitter) GetPreviousSubmissionErrors() []error {
	return c.previousSubmitErrors
}
