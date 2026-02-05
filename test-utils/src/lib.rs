//! Test utilities for Espresso Network
//!
//! This crate provides self-contained test utilities with no workspace internal dependencies.
//! Use `test-utils` for utilities that only need standard library functionality.
//! Use crate-specific `utils` modules for utilities that need workspace dependencies.
//!
//! # Port Binding
//!
//! The port binding utilities (`bind_tcp_port`, `bind_udp_port`) return struct wrappers
//! that keep ports reserved until dropped. This prevents race conditions in concurrent tests.
//!
//! These types have `#[must_use]` attributes that enforce correct usage at compile-time.

#![deny(unused_must_use)]

use std::net::{TcpListener, UdpSocket};

/// A TCP port binding that stays reserved until dropped.
///
/// This struct keeps the underlying `TcpListener` alive, preventing
/// port reuse race conditions common in tests. The port remains bound
/// until this struct is dropped.
///
/// # Compile-Time Safety
///
/// The `.port()` method returns `&u16` (not `u16`) to leverage the borrow checker.
/// This prevents the dangerous pattern `bind_tcp_port()?.port()` from compiling,
/// forcing you to keep `BoundPort` in scope.
///
/// # Examples
///
/// ```
/// use test_utils::bind_tcp_port;
///
/// # fn main() -> std::io::Result<()> {
/// // Correct usage: keep BoundPort in scope
/// let bound_port = bind_tcp_port()?;
/// let service_url = format!("http://localhost:{}", bound_port.port());
/// // Port stays reserved until bound_port drops
/// # Ok(())
/// # }
/// ```
///
/// This pattern will NOT compile (borrow checker error):
/// ```compile_fail
/// use test_utils::bind_tcp_port;
///
/// // ERROR: temporary value dropped while borrowed
/// let port = bind_tcp_port().unwrap().port();
/// println!("{}", port); // Must use it to trigger the error
/// ```
#[must_use = "Port binding must be kept alive to prevent race conditions"]
#[derive(Debug)]
pub struct BoundPort {
    listener: TcpListener,
    port: u16,
}

impl BoundPort {
    /// Get the bound port number.
    ///
    /// Returns a reference to prevent the common bug where `bind_tcp_port()?.port()`
    /// drops the listener immediately. The borrow checker enforces keeping `BoundPort`
    /// in scope.
    pub fn port(&self) -> &u16 {
        &self.port
    }

    /// Consume this binding and return the underlying listener.
    ///
    /// Use this if you need to call `accept()` or other listener methods.
    pub fn into_listener(self) -> TcpListener {
        self.listener
    }

    /// Get a reference to the underlying listener.
    pub fn listener(&self) -> &TcpListener {
        &self.listener
    }
}

/// A UDP socket binding that stays reserved until dropped.
///
/// This struct keeps the underlying `UdpSocket` alive, preventing
/// port reuse race conditions common in tests. The port remains bound
/// until this struct is dropped.
#[must_use = "Socket binding must be kept alive to prevent race conditions"]
#[derive(Debug)]
pub struct BoundSocket {
    socket: UdpSocket,
    port: u16,
}

impl BoundSocket {
    /// Get the bound port number.
    ///
    /// Returns a reference to prevent the common bug where `bind_udp_port()?.port()`
    /// drops the socket immediately. The borrow checker enforces keeping `BoundSocket`
    /// in scope.
    pub fn port(&self) -> &u16 {
        &self.port
    }

    /// Consume this binding and return the underlying socket.
    pub fn into_socket(self) -> UdpSocket {
        self.socket
    }

    /// Get a reference to the underlying socket.
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }
}

/// Atomically bind to an available TCP port and return the binding.
///
/// The returned `BoundPort` keeps the port reserved until dropped,
/// preventing race conditions where another process takes the port
/// before your service binds to it.
///
/// # Errors
///
/// Returns an error if unable to bind to any port on 127.0.0.1.
pub fn bind_tcp_port() -> std::io::Result<BoundPort> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    Ok(BoundPort { listener, port })
}

/// Atomically bind to an available UDP port and return the binding.
///
/// The returned `BoundSocket` keeps the port reserved until dropped,
/// preventing race conditions where another process takes the port
/// before your service binds to it.
///
/// # Errors
///
/// Returns an error if unable to bind to any port on 127.0.0.1.
pub fn bind_udp_port() -> std::io::Result<BoundSocket> {
    let socket = UdpSocket::bind("127.0.0.1:0")?;
    let port = socket.local_addr()?.port();
    Ok(BoundSocket { socket, port })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bind_tcp_port() {
        let bound = bind_tcp_port().expect("Failed to bind TCP port");
        assert_ne!(*bound.port(), 0);
    }

    #[test]
    fn test_bind_udp_port() {
        let bound = bind_udp_port().expect("Failed to bind UDP port");
        assert_ne!(*bound.port(), 0);
    }

    #[test]
    fn test_unique_ports() {
        let bound1 = bind_tcp_port().unwrap();
        let bound2 = bind_tcp_port().unwrap();
        let bound3 = bind_tcp_port().unwrap();

        assert_ne!(bound1.port(), bound2.port());
        assert_ne!(bound2.port(), bound3.port());
        assert_ne!(bound1.port(), bound3.port());
    }

    #[test]
    fn test_port_kept_bound() {
        let bound = bind_tcp_port().unwrap();
        let port = *bound.port(); // Copy the value

        // Port should still be bound
        let result = TcpListener::bind(format!("127.0.0.1:{}", port));
        assert!(result.is_err(), "Port should be occupied");

        drop(bound);

        // Port should now be available
        let result = TcpListener::bind(format!("127.0.0.1:{}", port));
        assert!(result.is_ok(), "Port should be available after drop");
    }

    // This test is intentionally commented out to demonstrate compile-time safety.
    // Uncommenting this test will cause a compile error:
    // "error[E0716]: temporary value dropped while borrowed"
    //
    // #[test]
    // fn test_dangerous_pattern_prevented() {
    //     let port = bind_tcp_port().unwrap().port();
    //     println!("Port: {}", port);
    // }

    #[test]
    fn test_correct_usage_keeps_port_bound() {
        // Correct pattern: keep BoundPort in scope
        let bound = bind_tcp_port().unwrap();
        let port = bound.port();

        // Port should still be bound
        let result = TcpListener::bind(format!("127.0.0.1:{}", port));
        assert!(
            result.is_err(),
            "Port should be occupied while BoundPort is in scope"
        );

        // Port used in URL formatting - common pattern in tests
        let url = format!("http://localhost:{}", bound.port());
        assert_eq!(url, format!("http://localhost:{}", port));

        // Port still bound
        let result = TcpListener::bind(format!("127.0.0.1:{}", port));
        assert!(result.is_err(), "Port should still be occupied");
    }
}
