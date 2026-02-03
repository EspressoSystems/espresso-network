# Networking

The networking layer operates over TCP, using the [Noise protocol framework][noise]
to create secure, authenticated links between parties.

Creating a network requires

- a `Keypair` whose public key identifies a party,
- an address to accept inbound connections on, and
- a set of `(PublicKey, Address)` pairs, comprising all parties that want to communicate.

Once created, binary data can be sent to individual parties, addressed by their
`PublicKey`, or to all parties. Applications can also await receiving data from a party.
For details regarding the API, see [`Network`].

## Architecture

When a [`Network`] is created it spawns a server task, that binds a TCP listener to the
provided address and starts accepting connections. It also immediately creates tasks
to connect to each party (except to itself), using the provided address. It then
enters the main event loop which handles task creation and termination. Connections
undergo a series of stages.

### Accepting an inbound connection

If the TCP listener accepts a new inbound connection it creates a handshake task which
attempts to perform a Noise handshake which involves a Diffie-Hellman key exchange and
-- if successful -- results in an authenticated and secure link with an identified
peer.

### Connect task

A connect task will indefinitely try to establish a TCP connection to a single peer.
Between connection attempts it waits for an increasing amount of time, but no more
than 30s. If the connection has been established, the task will also perform a Noise
handshake with the remote party.

If either the handshake task or the connect task finish successfully, the connection
is ready to be used for the actual exchange of application data.

### IP address check

If a party's address is an IP address, we also check that the remote peer address is
actually the one given. For domain names, no such check takes place.

### Simultaneous connects

Given that all parties try to connect to each other, a network node may accept a
connection it has already established through its own connect task, or vice versa.
A node uses the order of public keys to decide which connection to keep, should
two connections exist at the same time, i.e. given two connections to the same
peer a node drops the one whose associated public key is smaller than its own.

### I/O tasks

After successful connection establishment, two tasks are created, one to continuously
read incoming data and one to send application data. The data is split and encrypted
into frames of 64 KiB (the maximum size of a Noise package) or less. Failure of either
task results in the termination of both and a new connect task is created to
re-establish the connection.

### Heartbeats and latency measurements

In addition to application data, a network node periodically sends a PING frame and
expects a PONG frame. When a PONG is received the embedded timestamp is used to
measure the network RTT. In addition, whenever a PING frame has been sent, a countdown
timer is started (if not already running) which will cause the connection to be dropped
if finished. Any data that is subsequently received will stop the countdown. This
mechanism is used like a heartbeat to ensure the remote peer is alive and responding.

### Channels

Communication between the various tasks proceeds over MPSC (multi producer, single
consumer) channels. When application code wishes to send data, it sends them over
the channel to the main event loop, which will forward the data over another MPSC
channel to the respective write task. The capacity of every channel is bounded.
If the one the application uses is full, backpressure is exercised, i.e. the
application has to wait. This can happen for example, if no connection is available
for some time. The channel to an I/O write task is also bounded, but if full, the
oldest item will be dropped.

[noise]: https://noiseprotocol.org/
