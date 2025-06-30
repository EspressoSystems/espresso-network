use std::{
    future::Future,
    io::{Error as IoError, ErrorKind as IoErrorKind},
    pin::Pin,
    sync::Arc,
    task::Poll,
};

use anyhow::{Context, Result as AnyhowResult, ensure};
use bimap::BiMap;
use futures::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, future::poll_fn};
use hotshot_types::traits::signature_key::SignatureKey;
use libp2p::{
    Transport,
    core::{
        StreamMuxer,
        muxing::StreamMuxerExt,
        transport::{DialOpts, TransportEvent},
    },
    identity::PeerId,
};
use parking_lot::Mutex;
use pin_project::pin_project;
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tracing::warn;

/// The maximum size of an authentication message. This is used to prevent
/// DoS attacks by sending large messages.
const MAX_AUTH_MESSAGE_SIZE: usize = 1024;

/// The timeout for the authentication handshake. This is used to prevent
/// attacks that keep connections open indefinitely by half-finishing the
/// handshake.
const AUTH_HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// A wrapper for a `Transport` that bidirectionally associates (and verifies)
/// the corresponding consensus keys.
#[pin_project]
pub struct ConsensusKeyAuthentication<
    T: Transport,
    S: SignatureKey + 'static,
    C: StreamMuxer + Unpin,
> {
    #[pin]
    /// The underlying transport we are wrapping
    pub inner: T,

    /// A pre-signed message that we (depending on if it's specified or not) send to the remote peer for authentication
    pub auth_message: Arc<Option<Vec<u8>>>,

    /// The (verified) map of consensus keys to peer IDs
    pub consensus_key_to_pid_map: Arc<Mutex<BiMap<S, PeerId>>>,

    /// Phantom data for the connection type
    pd: std::marker::PhantomData<(C, S)>,
}

/// A type alias for the future that upgrades a connection to perform the authentication handshake
type UpgradeFuture<T> =
    Pin<Box<dyn Future<Output = Result<<T as Transport>::Output, <T as Transport>::Error>> + Send>>;

impl<T: Transport, S: SignatureKey + 'static, C: StreamMuxer + Unpin>
    ConsensusKeyAuthentication<T, S, C>
{
    /// Create a new `ConsensusKeyAuthentication` transport that wraps the given transport
    /// and authenticates connections against the stake table. If the auth message is `None`,
    /// the authentication is disabled.
    pub fn new(
        inner: T,
        auth_message: Option<Vec<u8>>,
        consensus_key_to_pid_map: Arc<Mutex<BiMap<S, PeerId>>>,
    ) -> Self {
        Self {
            inner,
            auth_message: Arc::from(auth_message),
            consensus_key_to_pid_map,
            pd: std::marker::PhantomData,
        }
    }

    /// Prove to the remote peer that we are in the stake table by sending
    /// them our authentication message.
    ///
    /// # Errors
    /// - If we fail to write the message to the stream
    pub async fn authenticate_with_remote_peer<W: AsyncWrite + Unpin>(
        stream: &mut W,
        auth_message: &[u8],
    ) -> AnyhowResult<()> {
        // Write the length-delimited message
        write_length_delimited(stream, auth_message).await?;

        Ok(())
    }

    /// Verify that the remote peer is:
    /// - In the stake table
    /// - Sending us a valid authentication message
    /// - Sending us a valid signature
    /// - Matching the peer ID we expect
    ///
    /// # Errors
    /// If the peer fails verification. This can happen if:
    /// - We fail to read the message from the stream
    /// - The message is too large
    /// - The message is invalid
    /// - The peer is not in the stake table
    /// - The signature is invalid
    pub async fn verify_peer_authentication<R: AsyncReadExt + Unpin>(
        stream: &mut R,
        required_peer_id: &PeerId,
        consensus_key_to_pid_map: Arc<Mutex<BiMap<S, PeerId>>>,
    ) -> AnyhowResult<()> {
        // Read the length-delimited message from the remote peer
        let message = read_length_delimited(stream, MAX_AUTH_MESSAGE_SIZE).await?;

        // Deserialize the authentication message
        let auth_message: AuthMessage<S> =
            bincode::deserialize(&message).with_context(|| "Failed to deserialize auth message")?;

        // Verify the signature on the public keys
        let public_key = auth_message
            .validate()
            .with_context(|| "Failed to verify authentication message")?;

        // Deserialize the `PeerId`
        let peer_id = PeerId::from_bytes(&auth_message.peer_id_bytes)
            .with_context(|| "Failed to deserialize peer ID")?;

        // Verify that the peer ID is the same as the remote peer
        if peer_id != *required_peer_id {
            return Err(anyhow::anyhow!("Peer ID mismatch"));
        }

        // If we got here, the peer is authenticated. Add the consensus key to the map
        consensus_key_to_pid_map.lock().insert(public_key, peer_id);

        Ok(())
    }

    /// Wrap the supplied future in an upgrade that performs the authentication handshake.
    ///
    /// `outgoing` is a boolean that indicates if the connection is incoming or outgoing.
    /// This is needed because the flow of the handshake is different for each.
    fn gen_handshake<F: Future<Output = Result<T::Output, T::Error>> + Send + 'static>(
        original_future: F,
        outgoing: bool,
        auth_message: Arc<Option<Vec<u8>>>,
        consensus_key_to_pid_map: Arc<Mutex<BiMap<S, PeerId>>>,
    ) -> UpgradeFuture<T>
    where
        T::Error: From<<C as StreamMuxer>::Error> + From<IoError>,
        T::Output: AsOutput<C> + Send,
        C::Substream: Unpin + Send,
    {
        // Create a new upgrade that performs the authentication handshake on top
        Box::pin(async move {
            // Wait for the original future to resolve
            let mut stream = original_future.await?;

            // Time out the authentication block
            timeout(AUTH_HANDSHAKE_TIMEOUT, async {
                // Open a substream for the handshake.
                // The handshake order depends on whether the connection is incoming or outgoing.
                let mut substream = if outgoing {
                    poll_fn(|cx| stream.as_connection().poll_outbound_unpin(cx)).await?
                } else {
                    poll_fn(|cx| stream.as_connection().poll_inbound_unpin(cx)).await?
                };

                // Conditionally authenticate depending on whether we specified an auth message
                if let Some(auth_message) = auth_message.as_ref() {
                    if outgoing {
                        // If the connection is outgoing, authenticate with the remote peer first
                        Self::authenticate_with_remote_peer(&mut substream, auth_message)
                            .await
                            .map_err(|e| {
                                warn!("Failed to authenticate with remote peer: {e:?}");
                                IoError::other(e)
                            })?;

                        // Verify the remote peer's authentication
                        Self::verify_peer_authentication(
                            &mut substream,
                            stream.as_peer_id(),
                            consensus_key_to_pid_map,
                        )
                        .await
                        .map_err(|e| {
                            warn!("Failed to verify remote peer: {e:?}");
                            IoError::other(e)
                        })?;
                    } else {
                        // If it is incoming, verify the remote peer's authentication first
                        Self::verify_peer_authentication(
                            &mut substream,
                            stream.as_peer_id(),
                            consensus_key_to_pid_map,
                        )
                        .await
                        .map_err(|e| {
                            warn!("Failed to verify remote peer: {e:?}");
                            IoError::other(e)
                        })?;

                        // Authenticate with the remote peer
                        Self::authenticate_with_remote_peer(&mut substream, auth_message)
                            .await
                            .map_err(|e| {
                                warn!("Failed to authenticate with remote peer: {e:?}");
                                IoError::other(e)
                            })?;
                    }
                }

                Ok(stream)
            })
            .await
            .map_err(|e| {
                warn!("Timed out performing authentication handshake: {e:?}");
                IoError::new(IoErrorKind::TimedOut, e)
            })?
        })
    }
}

/// The deserialized form of an authentication message that is sent to the remote peer
#[derive(Clone, Serialize, Deserialize)]
struct AuthMessage<S: SignatureKey> {
    /// The encoded (stake table) public key of the sender. This, along with the peer ID, is
    /// signed. It is still encoded here to enable easy verification.
    public_key_bytes: Vec<u8>,

    /// The encoded peer ID of the sender. This is appended to the public key before signing.
    /// It is still encoded here to enable easy verification.
    peer_id_bytes: Vec<u8>,

    /// The signature on the public key
    signature: S::PureAssembledSignatureType,
}

impl<S: SignatureKey> AuthMessage<S> {
    /// Validate the signature on the public key and return it if valid
    pub fn validate(&self) -> AnyhowResult<S> {
        // Deserialize the stake table public key
        let public_key = S::from_bytes(&self.public_key_bytes)
            .with_context(|| "Failed to deserialize public key")?;

        // Reconstruct the signed message from the public key and peer ID
        let mut signed_message = public_key.to_bytes();
        signed_message.extend(self.peer_id_bytes.clone());

        // Check if the signature is valid across both
        if !public_key.validate(&self.signature, &signed_message) {
            return Err(anyhow::anyhow!("Invalid signature"));
        }

        Ok(public_key)
    }
}

/// Create an sign an authentication message to be sent to the remote peer
///
/// # Errors
/// - If we fail to sign the public key
/// - If we fail to serialize the authentication message
pub fn construct_auth_message<S: SignatureKey + 'static>(
    public_key: &S,
    peer_id: &PeerId,
    private_key: &S::PrivateKey,
) -> AnyhowResult<Vec<u8>> {
    // Serialize the stake table public key
    let mut public_key_bytes = public_key.to_bytes();

    // Serialize the peer ID and append it
    let peer_id_bytes = peer_id.to_bytes();
    public_key_bytes.extend_from_slice(&peer_id_bytes);

    // Sign our public key
    let signature =
        S::sign(private_key, &public_key_bytes).with_context(|| "Failed to sign public key")?;

    // Create the auth message
    let auth_message = AuthMessage::<S> {
        public_key_bytes,
        peer_id_bytes,
        signature,
    };

    // Serialize the auth message
    bincode::serialize(&auth_message).with_context(|| "Failed to serialize auth message")
}

impl<T: Transport, S: SignatureKey + 'static, C: StreamMuxer + Unpin> Transport
    for ConsensusKeyAuthentication<T, S, C>
where
    T::Dial: Future<Output = Result<T::Output, T::Error>> + Send + 'static,
    T::ListenerUpgrade: Send + 'static,
    T::Output: AsOutput<C> + Send,
    T::Error: From<<C as StreamMuxer>::Error> + From<IoError>,
    C::Substream: Unpin + Send,
{
    // `Dial` is for connecting out, `ListenerUpgrade` is for accepting incoming connections
    type Dial = Pin<Box<dyn Future<Output = Result<T::Output, T::Error>> + Send>>;
    type ListenerUpgrade = Pin<Box<dyn Future<Output = Result<T::Output, T::Error>> + Send>>;

    // These are just passed through
    type Output = T::Output;
    type Error = T::Error;

    /// Dial a remote peer. This function is changed to perform an authentication handshake
    /// on top.
    fn dial(
        &mut self,
        addr: libp2p::Multiaddr,
        opts: DialOpts,
    ) -> Result<Self::Dial, libp2p::TransportError<Self::Error>> {
        // Perform the inner dial
        let res = self.inner.dial(addr, opts);

        // Clone the necessary fields
        let auth_message = Arc::clone(&self.auth_message);

        // If the dial was successful, perform the authentication handshake on top
        match res {
            Ok(dial) => Ok(Self::gen_handshake(
                dial,
                true,
                auth_message,
                Arc::clone(&self.consensus_key_to_pid_map),
            )),
            Err(err) => Err(err),
        }
    }

    /// This function is where we perform the authentication handshake for _incoming_ connections.
    /// The flow in this case is the reverse of the `dial` function: we first verify the remote peer's
    /// authentication, and then authenticate with them.
    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<libp2p::core::transport::TransportEvent<Self::ListenerUpgrade, Self::Error>>
    {
        match Transport::poll(self.as_mut().project().inner, cx) {
            Poll::Ready(event) => Poll::Ready(match event {
                // If we have an incoming connection, we need to perform the authentication handshake
                TransportEvent::Incoming {
                    listener_id,
                    upgrade,
                    local_addr,
                    send_back_addr,
                } => {
                    // Clone the necessary fields
                    let auth_message = Arc::clone(&self.auth_message);

                    // Generate the handshake upgrade future (inbound)
                    let auth_upgrade = Self::gen_handshake(
                        upgrade,
                        false,
                        auth_message,
                        Arc::clone(&self.consensus_key_to_pid_map),
                    );

                    // Return the new event
                    TransportEvent::Incoming {
                        listener_id,
                        upgrade: auth_upgrade,
                        local_addr,
                        send_back_addr,
                    }
                },

                // We need to re-map the other events because we changed the type of the upgrade
                TransportEvent::AddressExpired {
                    listener_id,
                    listen_addr,
                } => TransportEvent::AddressExpired {
                    listener_id,
                    listen_addr,
                },
                TransportEvent::ListenerClosed {
                    listener_id,
                    reason,
                } => TransportEvent::ListenerClosed {
                    listener_id,
                    reason,
                },
                TransportEvent::ListenerError { listener_id, error } => {
                    TransportEvent::ListenerError { listener_id, error }
                },
                TransportEvent::NewAddress {
                    listener_id,
                    listen_addr,
                } => TransportEvent::NewAddress {
                    listener_id,
                    listen_addr,
                },
            }),

            Poll::Pending => Poll::Pending,
        }
    }

    /// The below functions just pass through to the inner transport, but we had
    /// to define them
    fn remove_listener(&mut self, id: libp2p::core::transport::ListenerId) -> bool {
        self.inner.remove_listener(id)
    }
    fn listen_on(
        &mut self,
        id: libp2p::core::transport::ListenerId,
        addr: libp2p::Multiaddr,
    ) -> Result<(), libp2p::TransportError<Self::Error>> {
        self.inner.listen_on(id, addr)
    }
}

/// A helper trait that allows us to access the underlying connection
/// and `PeerId` from a transport output
trait AsOutput<C: StreamMuxer + Unpin> {
    /// Get a mutable reference to the underlying connection
    fn as_connection(&mut self) -> &mut C;

    /// Get a mutable reference to the underlying `PeerId`
    fn as_peer_id(&mut self) -> &mut PeerId;
}

/// The implementation of the `AsConnection` trait for a tuple of a `PeerId`
/// and a connection.
impl<C: StreamMuxer + Unpin> AsOutput<C> for (PeerId, C) {
    /// Get a mutable reference to the underlying connection
    fn as_connection(&mut self) -> &mut C {
        &mut self.1
    }

    /// Get a mutable reference to the underlying `PeerId`
    fn as_peer_id(&mut self) -> &mut PeerId {
        &mut self.0
    }
}

/// A helper function to read a length-delimited message from a stream. Takes into
/// account the maximum message size.
///
/// # Errors
/// - If the message is too big
/// - If we fail to read from the stream
pub async fn read_length_delimited<S: AsyncRead + Unpin>(
    stream: &mut S,
    max_size: usize,
) -> AnyhowResult<Vec<u8>> {
    // Receive the first 8 bytes of the message, which is the length
    let mut len_bytes = [0u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .await
        .with_context(|| "Failed to read message length")?;

    // Parse the length of the message as a `u32`
    let len = usize::try_from(u32::from_be_bytes(len_bytes))?;

    // Quit if the message is too large
    ensure!(len <= max_size, "Message too large");

    // Read the actual message
    let mut message = vec![0u8; len];
    stream
        .read_exact(&mut message)
        .await
        .with_context(|| "Failed to read message")?;

    Ok(message)
}

/// A helper function to write a length-delimited message to a stream.
///
/// # Errors
/// - If we fail to write to the stream
pub async fn write_length_delimited<S: AsyncWrite + Unpin>(
    stream: &mut S,
    message: &[u8],
) -> AnyhowResult<()> {
    // Write the length of the message
    stream
        .write_all(&u32::try_from(message.len())?.to_be_bytes())
        .await
        .with_context(|| "Failed to write message length")?;

    // Write the actual message
    stream
        .write_all(message)
        .await
        .with_context(|| "Failed to write message")?;

    Ok(())
}

#[cfg(test)]
mod test {
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey};
    use libp2p::{core::transport::dummy::DummyTransport, quic::Connection};
    use rand::Rng;

    use super::*;

    /// A mock type to help with readability
    type MockStakeTableAuth = ConsensusKeyAuthentication<DummyTransport, BLSPubKey, Connection>;

    // Helper macro for generating a new identity and authentication message
    macro_rules! new_identity {
        () => {{
            // Gen a new seed
            let seed = rand::rngs::OsRng.r#gen::<[u8; 32]>();

            // Create a new keypair
            let keypair = BLSPubKey::generated_from_seed_indexed(seed, 1337);

            // Create a peer ID
            let peer_id = libp2p::identity::Keypair::generate_ed25519()
                .public()
                .to_peer_id();

            // Construct an authentication message
            let auth_message =
                super::construct_auth_message(&keypair.0, &peer_id, &keypair.1).unwrap();

            (keypair, peer_id, auth_message)
        }};
    }

    // Helper macro to generator a cursor from a length-delimited message
    macro_rules! cursor_from {
        ($auth_message:expr) => {{
            let mut stream = futures::io::Cursor::new(vec![]);
            write_length_delimited(&mut stream, &$auth_message)
                .await
                .expect("Failed to write message");
            stream.set_position(0);
            stream
        }};
    }

    /// Test valid construction and verification of an authentication message
    #[test]
    fn signature_verify() {
        // Create a new identity
        let (_, _, auth_message) = new_identity!();

        // Verify the authentication message
        let public_key = super::AuthMessage::<BLSPubKey>::validate(
            &bincode::deserialize(&auth_message).unwrap(),
        );
        assert!(public_key.is_ok());
    }

    /// Test invalid construction and verification of an authentication message with
    /// an invalid public key. This ensures we are signing over it correctly.
    #[test]
    fn signature_verify_invalid_public_key() {
        // Create a new identity
        let (_, _, auth_message) = new_identity!();

        // Deserialize the authentication message
        let mut auth_message: super::AuthMessage<BLSPubKey> =
            bincode::deserialize(&auth_message).unwrap();

        // Change the public key
        auth_message.public_key_bytes[0] ^= 0x01;

        // Serialize the message again
        let auth_message = bincode::serialize(&auth_message).unwrap();

        // Verify the authentication message
        let public_key = super::AuthMessage::<BLSPubKey>::validate(
            &bincode::deserialize(&auth_message).unwrap(),
        );
        assert!(public_key.is_err());
    }

    /// Test invalid construction and verification of an authentication message with
    /// an invalid peer ID. This ensures we are signing over it correctly.
    #[test]
    fn signature_verify_invalid_peer_id() {
        // Create a new identity
        let (_, _, auth_message) = new_identity!();

        // Deserialize the authentication message
        let mut auth_message: super::AuthMessage<BLSPubKey> =
            bincode::deserialize(&auth_message).unwrap();

        // Change the peer ID
        auth_message.peer_id_bytes[0] ^= 0x01;

        // Serialize the message again
        let auth_message = bincode::serialize(&auth_message).unwrap();

        // Verify the authentication message
        let public_key = super::AuthMessage::<BLSPubKey>::validate(
            &bincode::deserialize(&auth_message).unwrap(),
        );
        assert!(public_key.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn valid_authentication() {
        // Create a new identity
        let (keypair, peer_id, auth_message) = new_identity!();

        // Create a stream and write the message to it
        let mut stream = cursor_from!(auth_message);

        // Create a map from consensus keys to peer IDs
        let consensus_key_to_pid_map = Arc::new(parking_lot::Mutex::new(BiMap::new()));

        // Verify the authentication message
        let result = MockStakeTableAuth::verify_peer_authentication(
            &mut stream,
            &peer_id,
            Arc::clone(&consensus_key_to_pid_map),
        )
        .await;

        // Make sure the map has the correct entry
        assert!(
            consensus_key_to_pid_map
                .lock()
                .get_by_left(&keypair.0)
                .unwrap()
                == &peer_id,
            "Map does not have the correct entry"
        );

        assert!(
            result.is_ok(),
            "Should have passed authentication but did not"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn peer_id_mismatch() {
        // Create a new identity and authentication message
        let (_, _, auth_message) = new_identity!();

        // Create a second (malicious) identity
        let (_, malicious_peer_id, _) = new_identity!();

        // Create a stream and write the message to it
        let mut stream = cursor_from!(auth_message);

        // Create a map from consensus keys to peer IDs
        let consensus_key_to_pid_map = Arc::new(parking_lot::Mutex::new(BiMap::new()));

        // Check against the malicious peer ID
        let result = MockStakeTableAuth::verify_peer_authentication(
            &mut stream,
            &malicious_peer_id,
            Arc::clone(&consensus_key_to_pid_map),
        )
        .await;

        // Make sure it errored for the right reason
        assert!(
            result
                .expect_err("Should have failed authentication but did not")
                .to_string()
                .contains("Peer ID mismatch"),
            "Did not fail with the correct error"
        );

        // Make sure the map does not have the malicious peer ID
        assert!(
            consensus_key_to_pid_map.lock().is_empty(),
            "Malicious peer ID should not be in the map"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn read_and_write_length_delimited() {
        // Create a message
        let message = b"Hello, world!";

        // Write the message to a buffer
        let mut buffer = Vec::new();
        write_length_delimited(&mut buffer, message).await.unwrap();

        // Read the message from the buffer
        let read_message = read_length_delimited(&mut buffer.as_slice(), 1024)
            .await
            .unwrap();

        // Check if the messages are the same
        assert_eq!(message, read_message.as_slice());
    }
}
