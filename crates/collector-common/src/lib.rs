use std::{
    net::{SocketAddr, UdpSocket},
    str::FromStr,
    sync::{Arc, OnceLock},
    time::SystemTime,
};

use anyhow::{Context, Result};
use rkyv::{Archive, Deserialize, Serialize};

/// A trace with a timestamp
#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub struct TraceWithTimestamp {
    pub trace: Trace,
    pub timestamp: f64,
}

/// The types of traces that can be collected
#[derive(Serialize, Deserialize, Archive, Clone, Debug)]
pub enum Trace {
    ProposalSendEventGenerated(u64),
    ProposalSent(u64),
    ProposalReceived(u64),
    ProposalReceivedEventGenerated(u64),
}

/// The UDP socket for sending traces
const UDP_SOCKET: OnceLock<Arc<(UdpSocket, SocketAddr)>> = OnceLock::new();

/// Send a trace with a specific timestamp (as seconds since the UNIX epoch)
pub fn send_trace_with_timestamp(trace: &Trace, timestamp: f64) -> Result<()> {
    // Wrap it in our type that contains the timestamp
    let trace_with_timestamp = TraceWithTimestamp {
        trace: trace.clone(),
        timestamp,
    };

    // Serialize it
    let trace_bytes = rkyv::to_bytes(&trace_with_timestamp)
        .map_err(|e: rkyv::rancor::Error| anyhow::anyhow!("failed to serialize trace: {}", e))?;

    // Create or get the UDP socket
    let udp_socket = UDP_SOCKET.get().cloned();
    let udp_socket = match udp_socket {
        Some(udp_socket) => udp_socket,
        None => {
            // Get the collector endpoint
            let collector_endpoint = std::env::var("COLLECTOR_ENDPOINT")
                .with_context(|| "failed to get collector endpoint")?;

            // Parse the collector endpoint
            let collector_endpoint = SocketAddr::from_str(&collector_endpoint)
                .with_context(|| "failed to parse collector endpoint")?;

            // Bind the UDP socket
            let udp_socket =
                UdpSocket::bind("0.0.0.0:0").with_context(|| "failed to bind UDP socket")?;

            // Set it to nonblocking
            udp_socket
                .set_nonblocking(true)
                .with_context(|| "failed to set nonblocking")?;

            let udp_socket = Arc::new((udp_socket, collector_endpoint));

            // Set it in the once lock
            let _ = UDP_SOCKET.set(udp_socket.clone());
            udp_socket
        },
    };

    // Send the trace
    udp_socket
        .0
        .send_to(&trace_bytes, udp_socket.1)
        .with_context(|| "failed to send trace")?;

    Ok(())
}

/// Write a trace to a stream along with the
pub fn send_trace(trace: &Trace) -> Result<()> {
    // Get the current timestamp for the trace
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .with_context(|| "failed to get current timestamp")?
        .as_secs_f64();

    send_trace_with_timestamp(trace, timestamp)
}
