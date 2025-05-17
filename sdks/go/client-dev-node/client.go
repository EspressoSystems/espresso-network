package clientdevnode

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
)

type Client struct {
	baseUrl string
	client  *http.Client
}

func NewClient(url string) *Client {
	if !strings.HasSuffix(url, "/") {
		url += "/"
	}
	return &Client{
		baseUrl: url,
		client:  http.DefaultClient,
	}
}

func (c *Client) IsAvailable(ctx context.Context) (bool, error) {
	devInfo, err := c.FetchDevInfo(ctx)
	if err != nil {
		return false, err
	}
	return devInfo.BuilderUrl != "", nil
}

func (c *Client) FetchDevInfo(ctx context.Context) (DevInfo, error) {
	var res DevInfo
	if err := c.get(ctx, &res, "api/dev-info"); err != nil {
		return DevInfo{}, err
	}
	return res, nil
}

func (c *Client) getRawMessage(ctx context.Context, format string, args ...any) (json.RawMessage, error) {
	res, err := c.tryRequest(ctx, c.baseUrl, format, args...)
	if err != nil {
		return nil, err
	}

	defer res.Body.Close()

	if res.StatusCode != 200 {
		// Try to get the response body to include in the error message, as it may have useful
		// information about why the request failed. If this call fails, the response will be `nil`,
		// which is fine to include in the log, so we can ignore errors.
		body, _ := io.ReadAll(res.Body)
		return nil, fmt.Errorf("request failed with status %d and body %s", res.StatusCode, string(body))
	}

	// Read the response body into memory before we unmarshal it, rather than passing the io.Reader
	// to the json decoder, so that we still have the body and can inspect it if unmarshalling
	// failed.
	body, err := io.ReadAll(res.Body)
	if err != nil {
		return nil, err
	}
	return body, nil
}

func (c *Client) get(ctx context.Context, out any, format string, args ...any) error {
	body, err := c.getRawMessage(ctx, format, args...)
	if err != nil {
		return err
	}
	if err := json.Unmarshal(body, out); err != nil {
		return fmt.Errorf("request failed with body %s and error %v", string(body), err)
	}
	return nil
}

func (c *Client) tryRequest(ctx context.Context, baseUrl, format string, args ...interface{}) (*http.Response, error) {

	url := baseUrl + fmt.Sprintf(format, args...)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	return c.client.Do(req)
}
