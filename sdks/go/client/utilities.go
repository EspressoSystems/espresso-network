package client

import "strings"

// Apply formatting rules to url before creating the client.
// currently, this is just ensuring the url has the suffix `/`
// but more rules can be applied here later.
func formatUrl(url string) string {
	if !strings.HasSuffix(url, "/") {
		url += "/"
	}
	return url
}
