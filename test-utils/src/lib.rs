//! Test utilities for Espresso Network
//!
//! This crate provides self-contained test utilities with no workspace internal dependencies.
//! Use `test-utils` for utilities that only need standard library functionality.
//! Use crate-specific `utils` modules for utilities that need workspace dependencies.
//!
//! # Port Allocation
//!
//! The `reserve_tcp_port()` function uses the TIME_WAIT trick to provide race-free port
//! allocation, returning a port protected from ephemeral allocation for ~60s.

use std::net::{TcpListener, TcpStream};

/// Reserve a TCP port using the TIME_WAIT trick.
///
/// This function allocates an ephemeral port and forces it into TCP TIME_WAIT state
/// by completing a TCP handshake. The port is then:
/// - Protected from OS ephemeral allocation for ~60 seconds (TIME_WAIT duration)
/// - Still available for explicit binds (TIME_WAIT prevents collision with ephemeral
///   allocation, not intentional rebinding)
///
/// This pattern is based on Yelp's ephemeral-port-reserve approach and is useful
/// when you need to know a port number before starting a service, but the service
/// needs to bind to the port itself (not use a pre-bound listener).
///
/// # Errors
///
/// Returns an error if unable to bind, connect, or accept on 127.0.0.1.
///
/// # Example
///
/// ```
/// use test_utils::reserve_tcp_port;
///
/// let port = reserve_tcp_port().expect("Failed to reserve port");
/// // Port is now in TIME_WAIT state - protected from ephemeral allocation
/// // Services can still bind to it explicitly
/// ```
pub fn reserve_tcp_port() -> std::io::Result<u16> {
    let server = TcpListener::bind("127.0.0.1:0")?;
    let addr = server.local_addr()?;

    // Force TIME_WAIT by completing TCP handshake
    let _client = TcpStream::connect(addr)?;
    let (_accepted, _) = server.accept()?;
    // All sockets drop here - port enters TIME_WAIT

    Ok(addr.port())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reserve_tcp_port_returns_nonzero() {
        let port = reserve_tcp_port().unwrap();
        assert_ne!(port, 0);
    }

    #[test]
    fn test_reserve_tcp_port_unique() {
        let ports: Vec<u16> = (0..10).map(|_| reserve_tcp_port().unwrap()).collect();
        let unique: std::collections::HashSet<_> = ports.iter().collect();
        assert_eq!(unique.len(), 10, "All ports should be unique");
    }

    #[test]
    fn test_reserve_tcp_port_service_can_bind() {
        let port = reserve_tcp_port().unwrap();

        // Service should be able to bind (TIME_WAIT allows explicit binds)
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
            .expect("Service should bind to TIME_WAIT port");

        assert_eq!(listener.local_addr().unwrap().port(), port);
    }

    #[test]
    fn test_reserve_tcp_port_no_ephemeral_collision() {
        let reserved_port = reserve_tcp_port().unwrap();

        // Create many ephemeral connections - none should get our reserved port
        let mut ephemeral_ports = Vec::new();
        for _ in 0..100 {
            let socket = TcpListener::bind("127.0.0.1:0").unwrap();
            ephemeral_ports.push(socket.local_addr().unwrap().port());
        }

        assert!(
            !ephemeral_ports.contains(&reserved_port),
            "OS assigned reserved port {} to ephemeral allocation",
            reserved_port
        );
    }

    #[test]
    fn test_reserve_tcp_port_concurrent() {
        use std::{
            collections::HashSet,
            sync::{Arc, Mutex},
            thread,
        };

        let ports = Arc::new(Mutex::new(HashSet::new()));
        let mut handles = vec![];

        for _ in 0..10 {
            let ports = Arc::clone(&ports);
            handles.push(thread::spawn(move || {
                let port = reserve_tcp_port().unwrap();
                ports.lock().unwrap().insert(port);
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(
            ports.lock().unwrap().len(),
            10,
            "All ports should be unique"
        );
    }
}
