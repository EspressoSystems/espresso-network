package client

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"

	types "github.com/EspressoSystems/espresso-network/sdks/go/types"
)

// Apply formatting rules to url before creating the client.
// currently, this is just ensuring the url has the suffix `/`
// but more rules can be applied here later.
func formatUrl(url string) string {
	if !strings.HasSuffix(url, "/") {
		url += "/"
	}
	return url
}

// Consumes and closes response.Body.
func decodeSubmitResponse(response *http.Response) (*types.TaggedBase64, error) {
	defer response.Body.Close()

	if response.StatusCode != http.StatusOK {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, response.Status)
	}

	body, err := io.ReadAll(response.Body)
	if err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}

	var hash types.TaggedBase64
	if err := json.Unmarshal(body, &hash); err != nil {
		return nil, fmt.Errorf("%w: %v", ErrEphemeral, err)
	}
	return &hash, nil
}

// Gives one attempt of a sequential walk half of what is left of the caller's
// deadline, so an endpoint that never answers cannot spend the budget of the
// endpoints after it, while a healthy first endpoint still gets most of it.
func shareRemainingBudget(ctx context.Context, remaining int) (context.Context, context.CancelFunc) {
	deadline, ok := ctx.Deadline()
	if !ok || remaining <= 1 {
		return ctx, func() {}
	}
	return context.WithTimeout(ctx, time.Until(deadline)/2)
}
