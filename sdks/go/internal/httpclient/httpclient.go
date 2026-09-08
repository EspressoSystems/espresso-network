package httpclient

import (
	"net/http"
	"time"
)

// Bounds a request whose caller supplied no deadline: a node that accepts the
// connection and never answers would otherwise park it forever. coder/websocket
// applies this to the handshake only, so open streams survive it.
const Timeout = 30 * time.Second

func New() *http.Client {
	return &http.Client{Timeout: Timeout}
}
