use std::{
    io,
    iter::{once, repeat},
    net::SocketAddr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use rand::RngExt;
use snow::{Builder, HandshakeState, TransportState, params::NoiseParams};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    time::{sleep, timeout},
};
use tracing::{debug, warn};

use crate::{
    Config,
    addr::NetAddr,
    error::NetworkError,
    msg::{Header, MAX_NOISE_MESSAGE_SIZE},
    x25519::PublicKey,
};

const MAX_NOISE_HANDSHAKE_SIZE: usize = 1024;

static NOISE_PARAMS: LazyLock<NoiseParams> = LazyLock::new(|| {
    "Noise_IK_25519_AESGCM_BLAKE2s"
        .parse()
        .expect("valid noise params")
});

type Result<T> = std::result::Result<T, NetworkError>;

pub struct Connection {
    pub key: PublicKey,
    pub addr: SocketAddr,
    pub stream: TcpStream,
    pub state: TransportState,
}

impl Connection {
    pub async fn accept(conf: Arc<Config>, mut stream: TcpStream) -> Result<Self> {
        if let Err(err) = stream.set_nodelay(true) {
            warn!(
                name = %conf.name,
                node = %conf.keypair.public_key(),
                %err,
                "failed to enable NO_DELAY option"
            )
        }
        let hs = Builder::new(NOISE_PARAMS.clone())
            .local_private_key(&conf.keypair.secret_key().as_bytes())
            .expect("valid private key")
            .prologue(conf.name.as_bytes())
            .expect("1st time we set the prologue")
            .build_responder()
            .expect("valid noise params yield valid handshake state");
        let node = conf.keypair.public_key();
        let addr = stream.peer_addr()?;
        match timeout(conf.handshake_timeout, on_handshake(&mut stream, hs)).await {
            Ok(Ok(state)) => match remote_static_key(&state) {
                Some(key) => Ok(Self {
                    key,
                    addr,
                    stream,
                    state,
                }),
                None => {
                    warn! {
                        name = %conf.name,
                        %node,
                        %addr,
                        "missing or invalid remote static key"
                    }
                    Err(NetworkError::InvalidHandshakeMessage)
                },
            },
            Ok(Err(e)) => Err(e),
            Err(_) => Err(NetworkError::Timeout),
        }
    }

    pub async fn connect(conf: Arc<Config>, peer: PublicKey, addr: NetAddr) -> Self {
        let new_handshake_state = || {
            Builder::new(NOISE_PARAMS.clone())
                .local_private_key(conf.keypair.secret_key().as_slice())
                .expect("valid private key")
                .remote_public_key(peer.as_slice())
                .expect("valid remote pub key")
                .prologue(conf.name.as_bytes())
                .expect("1st time we set the prologue")
                .build_initiator()
                .expect("valid noise params yield valid handshake state")
        };

        let mut delays = once(Duration::from_millis(rand::rng().random_range(0..1000)))
            .chain(
                conf.retry_delays
                    .iter()
                    .map(|&d| Duration::from_secs(d.into())),
            )
            .chain(repeat(conf.max_retry_delay));

        let addr = addr.to_string();
        let node = conf.keypair.public_key();

        let (key, addr, stream, state) = loop {
            sleep(delays.next().expect("delays iterator is infinite")).await;
            debug!(name = %conf.name, %node, %peer, %addr, "connecting");
            match timeout(conf.connect_timeout, TcpStream::connect(&addr)).await {
                Ok(Ok(mut stream)) => {
                    let addr = match stream.peer_addr() {
                        Ok(addr) => addr,
                        Err(err) => {
                            warn!(name = %conf.name, %node, %err, "failed to get peer address");
                            continue;
                        },
                    };
                    if let Err(err) = stream.set_nodelay(true) {
                        warn!(name = %conf.name, %node, %err, "failed to enable NO_DELAY option")
                    }
                    let state = new_handshake_state();
                    match timeout(conf.handshake_timeout, handshake(&mut stream, state)).await {
                        Ok(Ok(state)) => {
                            debug!(name = %conf.name, %node, %peer, %addr, "connected");
                            match remote_static_key(&state) {
                                Some(key) if key == peer => break (key, addr, stream, state),
                                Some(key) => {
                                    warn!(
                                        name = %conf.name,
                                        %node,
                                        %peer,
                                        remote = %key,
                                        %addr,
                                        "remote static key mismatch"
                                    )
                                },
                                None => {
                                    warn!(
                                        name = %conf.name,
                                        %node,
                                        %peer,
                                        %addr,
                                        "missing or invalid remote static key"
                                    )
                                },
                            }
                        },
                        Ok(Err(err)) => {
                            warn!(
                                name = %conf.name,
                                %node,
                                %peer,
                                %addr,
                                %err, "handshake failure"
                            )
                        },
                        Err(_) => {
                            warn!(name = %conf.name, %node, %peer, %addr, "handshake timeout")
                        },
                    }
                },
                Ok(Err(err)) => {
                    warn!(name = %conf.name, %node, %peer, %addr, %err, "connect failure");
                },
                Err(_) => {
                    warn!(name = %conf.name, %node, %peer, %addr, "connect timeout");
                },
            }
        };

        Self {
            key,
            addr,
            stream,
            state,
        }
    }
}

fn remote_static_key(state: &TransportState) -> Option<PublicKey> {
    let k = state.get_remote_static()?;
    PublicKey::try_from(k).ok()
}

/// Perform a noise handshake as initiator with the remote party.
async fn handshake(stream: &mut TcpStream, mut hs: HandshakeState) -> Result<TransportState> {
    let mut b = vec![0; MAX_NOISE_HANDSHAKE_SIZE];
    let n = hs.write_message(&[], &mut b[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(stream, h, &mut b[..Header::SIZE + n]).await?;
    let mut m = Vec::new();
    let h = recv_frame(stream, &mut m).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    hs.read_message(&m, &mut b)?;
    Ok(hs.into_transport_mode()?)
}

/// Perform a noise handshake as responder with a remote party.
async fn on_handshake(stream: &mut TcpStream, mut hs: HandshakeState) -> Result<TransportState> {
    let mut m = Vec::new();
    let h = recv_frame(stream, &mut m).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    let mut b = vec![0; MAX_NOISE_HANDSHAKE_SIZE];
    hs.read_message(&m, &mut b)?;
    let n = hs.write_message(&[], &mut b[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(stream, h, &mut b[..Header::SIZE + n]).await?;
    Ok(hs.into_transport_mode()?)
}

/// Read a single frame (header + payload) from the remote.
pub async fn recv_frame<R>(stream: &mut R, buf: &mut Vec<u8>) -> io::Result<Header>
where
    R: AsyncReadExt + Unpin,
{
    let h = {
        let n = stream.read_u32().await?;
        Header::unvalidated(n)
    };
    buf.resize(h.len().into(), 0);
    stream.read_exact(buf).await?;
    Ok(h)
}

/// Write a single frame (header + payload) to the remote.
///
/// The header is serialised into the first 4 bytes of `msg`. It is the
/// caller's responsibility to ensure there is room at the beginning.
pub async fn send_frame<W>(stream: &mut W, hdr: Header, msg: &mut [u8]) -> io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    debug_assert!(msg.len() <= MAX_NOISE_MESSAGE_SIZE);
    msg[..Header::SIZE].copy_from_slice(&hdr.to_bytes());
    stream.write_all(msg).await?;
    Ok(())
}
