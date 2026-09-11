package client

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"net/http"

	"github.com/EspressoSystems/espresso-network/sdks/go/internal/httpclient"
	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
)

var _ SubmitAPI = (*BuilderSubmitter)(nil)
var _ EspressoBuilderSubmitter = (*BuilderSubmitter)(nil)

type BuilderSubmitter struct {
	builderUrls          []string
	client               *http.Client
	previousSubmitErrors []error
}

// The builder submitter's constructor.
func NewBuilderSubmitter(builderUrls []string) (*BuilderSubmitter, error) {
	if len(builderUrls) < 1 {
		return nil, fmt.Errorf("One or more builder url's is required for the builder submitter")
	}

	formattedUrls := make([]string, len(builderUrls))
	for i, url := range builderUrls {
		formattedUrls[i] = formatUrl(url)
	}

	return &BuilderSubmitter{
		builderUrls: formattedUrls,
		client:      httpclient.New(),
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
	for i, url := range c.builderUrls {
		builderCtx, cancel := shareRemainingBudget(ctx, len(c.builderUrls)-i)
		hash, err := c.submitToBuilder(builderCtx, url, tx)
		cancel()
		if err != nil {
			c.previousSubmitErrors = append(c.previousSubmitErrors, err)
			continue
		}
		return hash, nil
	}
	return nil, ErrAllBuildersFailed
}

// Reads the response to completion before returning, so that the caller can
// cancel the request's context as soon as this returns.
func (c *BuilderSubmitter) submitToBuilder(ctx context.Context, baseUrl string, tx types.Transaction) (*types.TaggedBase64, error) {
	response, err := c.tryPostRequest(ctx, baseUrl, tx)
	if err != nil {
		return nil, err
	}
	return decodeSubmitResponse(response)
}

// post request handler for the builder submitter.
func (c *BuilderSubmitter) tryPostRequest(ctx context.Context, baseUrl string, tx types.Transaction) (*http.Response, error) {
	marshalled, err := json.Marshal(tx)
	if err != nil {
		return nil, err
	}

	request, err := http.NewRequestWithContext(ctx, "POST", baseUrl+"txn_submit/submit", bytes.NewBuffer(marshalled))
	if err != nil {
		return nil, err
	}
	request.Header.Set("Content-Type", "application/json")
	return c.client.Do(request)
}

func (c *BuilderSubmitter) GetPreviousSubmissionErrors() []error {
	return c.previousSubmitErrors
}
