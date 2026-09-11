package httpclient

import (
	"net"
	"net/http"
	"time"
)

// Bounds how long a request waits for the response headers: a node that
// accepts the connection and never answers would otherwise park a caller who
// set no deadline forever. Reading the body is not bounded, so a large but
// healthy range fetch on a slow link still completes. A websocket handshake
// ends with its response headers, so this bounds the dial and not the stream.
const Timeout = 30 * time.Second

// One transport for every client, so they keep sharing a connection pool the
// way they did on http.DefaultTransport, whose settings the other fields copy.
var Transport = &http.Transport{
	Proxy: http.ProxyFromEnvironment,
	DialContext: (&net.Dialer{
		Timeout:   30 * time.Second,
		KeepAlive: 30 * time.Second,
	}).DialContext,
	ForceAttemptHTTP2:     true,
	MaxIdleConns:          100,
	IdleConnTimeout:       90 * time.Second,
	TLSHandshakeTimeout:   10 * time.Second,
	ExpectContinueTimeout: 1 * time.Second,
	ResponseHeaderTimeout: Timeout,
}

func New() *http.Client {
	return &http.Client{Transport: Transport}
}
