package client

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"

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
