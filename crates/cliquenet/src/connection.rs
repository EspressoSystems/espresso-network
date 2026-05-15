use std::{
    cmp::min,
    io,
    iter::{once, repeat},
    net::SocketAddr,
    sync::Arc,
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
    Config, Version,
    addr::NetAddr,
    error::NetworkError,
    msg::{Header, MAX_NOISE_MESSAGE_SIZE, hello::Hello},
    x25519::PublicKey,
};

const MAX_NOISE_HANDSHAKE_SIZE: usize = 1024;

type Result<T> = std::result::Result<T, NetworkError>;

pub struct Connection {
    pub key: PublicKey,
    pub addr: SocketAddr,
    pub stream: TcpStream,
    pub state: TransportState,
}

type Prologue = Vec<u8>;

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

        let (version, prologue) = select_version(&conf, &mut stream, false).await?;

        debug!(
            name = %conf.name,
            node = %conf.keypair.public_key(),
            %version,
            "negotiated version"
        );

        let params = conf
            .noise_configs
            .get(&version)
            .expect("selected version has noise config");
        let hs = Builder::new(params.clone())
            .local_private_key(&conf.keypair.secret_key().as_bytes())
            .expect("valid private key")
            .prologue(&prologue)
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
        let new_handshake_state = |prologue: &Prologue, params: &NoiseParams| {
            Builder::new(params.clone())
                .local_private_key(conf.keypair.secret_key().as_slice())
                .expect("valid private key")
                .remote_public_key(peer.as_slice())
                .expect("valid remote pub key")
                .prologue(&prologue)
                .expect("1st time we set the prologue")
                .build_initiator()
                .expect("valid noise params yield valid handshake state")
        };

        let mut delays = once({
            if conf.random_connect_delay {
                Duration::from_millis(rand::rng().random_range(0..1000))
            } else {
                Duration::ZERO
            }
        })
        .chain(
            conf.retry_delays
                .iter()
                .map(|&d| Duration::from_secs(d.into())),
        )
        .chain(repeat(conf.max_retry_delay));

        let addr = addr.to_string();
        let node = conf.keypair.public_key();

        let mut backoff = None;

        loop {
            if let Some(d) = backoff.take() {
                sleep(d).await;
            } else {
                sleep(delays.next().expect("delays iterator is infinite")).await;
            }
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
                    let (version, prologue) = match select_version(&conf, &mut stream, true).await {
                        Ok((v, p)) => {
                            debug!(name = %conf.name, %node, version = %v, "negotiated version");
                            (v, p)
                        },
                        Err(err) => {
                            warn!(name = %conf.name, %node, %err, "failed to negotiate version");
                            continue;
                        },
                    };
                    let params = conf
                        .noise_configs
                        .get(&version)
                        .expect("selected version has noise config");
                    let state = new_handshake_state(&prologue, params);
                    match timeout(conf.handshake_timeout, handshake(&mut stream, state)).await {
                        Ok(Ok(state)) => {
                            debug!(name = %conf.name, %node, %peer, %addr, "connected");
                            match remote_static_key(&state) {
                                Some(key) if key == peer => {
                                    let mut conn = Self {
                                        key,
                                        addr,
                                        stream,
                                        state,
                                    };
                                    match conn
                                        .exchange_hello(conf.handshake_timeout, Hello::Ok)
                                        .await
                                    {
                                        Ok(h) if h.is_ok() => break conn,
                                        Ok(h) => {
                                            warn!(
                                                name = %conf.name,
                                                %node,
                                                %peer,
                                                remote = %key,
                                                %addr,
                                                "hello response was not ok"
                                            );
                                            backoff = h.backoff_duration();
                                            continue;
                                        },
                                        Err(err) => {
                                            warn!(
                                                name = %conf.name,
                                                %node,
                                                %peer,
                                                remote = %key,
                                                %addr,
                                                %err,
                                                "failed to exchange hello"
                                            );
                                            continue;
                                        },
                                    }
                                },
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
        }
    }

    async fn exchange_hello(&mut self, d: Duration, h: Hello) -> Result<Hello> {
        let future = async {
            self.send_hello(h).await?;
            self.recv_hello().await
        };
        match timeout(d, future).await {
            Ok(re) => re,
            Err(_) => Err(NetworkError::Timeout),
        }
    }

    /// Send a `Hello` frame.
    pub async fn send_hello(&mut self, h: Hello) -> Result<()> {
        let mut b = [0u8; 64];
        let n = self
            .state
            .write_message(h.to_bytes().as_ref(), &mut b[Header::SIZE..])?;
        let h = Header::data(n as u16);
        send_frame(&mut self.stream, h, &mut b[..Header::SIZE + n]).await?;
        Ok(())
    }

    /// Read a `Hello` frame.
    pub async fn recv_hello(&mut self) -> Result<Hello> {
        let mut a = [0u8; 64];
        let h = recv_frame(&mut self.stream, &mut a).await?;
        let mut b = [0u8; 64];
        let n = self.state.read_message(&a[..h.len().into()], &mut b)?;
        let h = Hello::from_bytes(&b[..n]).ok_or(NetworkError::InvalidHello)?;
        Ok(h)
    }
}

fn remote_static_key(state: &TransportState) -> Option<PublicKey> {
    let k = state.get_remote_static()?;
    PublicKey::try_from(k).ok()
}

/// Select a version from the range that both sides support.
///
/// This will be the minimum of the max. supported ones from both sides.
async fn select_version(
    conf: &Config,
    stream: &mut TcpStream,
    is_initiator: bool,
) -> Result<(Version, Prologue)> {
    // NB that both ends send simultaneously before reading, hence the
    // initial frame must fit into the socket's send buffer, i.e. be
    // very small. Since we only send two u16 version numbers, that
    // should not be a problem, but the init frame should better not
    // grow.
    const INIT_PAYLOAD_LEN: usize = 4;

    let our_min = conf
        .noise_configs
        .keys()
        .min()
        .copied()
        .expect("noise_configs is not empty");
    let our_max = conf
        .noise_configs
        .keys()
        .max()
        .copied()
        .expect("noise_configs is not empty");

    let mut send_buf = [0u8; Header::SIZE + INIT_PAYLOAD_LEN];
    let mut recv_buf = [0u8; INIT_PAYLOAD_LEN];

    let payload = &mut send_buf[Header::SIZE..];
    payload[0..2].copy_from_slice(&u16::from(our_min).to_be_bytes());
    payload[2..4].copy_from_slice(&u16::from(our_max).to_be_bytes());

    let h = Header::init(INIT_PAYLOAD_LEN as u16);
    send_frame(stream, h, &mut send_buf[..]).await?;

    let h = recv_frame(stream, &mut recv_buf).await?;
    if !h.is_init() || h.is_partial() || h.len() != INIT_PAYLOAD_LEN as u16 {
        return Err(NetworkError::InvalidInit);
    }

    let their_min = Version::from(u16::from_be_bytes([recv_buf[0], recv_buf[1]]));
    let their_max = Version::from(u16::from_be_bytes([recv_buf[2], recv_buf[3]]));

    let selected = min(our_max, their_max);

    if selected < their_min || selected < our_min {
        return Err(NetworkError::IncompatibleVersions {
            ours: (our_min, our_max),
            theirs: (their_min, their_max),
        });
    }

    // Construct the prologue so that both sides end up with the same value.
    // We include the sent and received version ranges to ensure no one has
    // tampered with those values as they were sent in plain text.
    let mut prologue = Vec::new();
    prologue.extend_from_slice(conf.name.as_bytes());
    if is_initiator {
        prologue.extend_from_slice(&send_buf[Header::SIZE..]);
        prologue.extend_from_slice(&recv_buf);
    } else {
        prologue.extend_from_slice(&recv_buf);
        prologue.extend_from_slice(&send_buf[Header::SIZE..]);
    }

    Ok((selected, prologue))
}

/// Perform a noise handshake as initiator with the remote party.
async fn handshake(stream: &mut TcpStream, mut hs: HandshakeState) -> Result<TransportState> {
    let mut a = [0u8; MAX_NOISE_HANDSHAKE_SIZE];
    let n = hs.write_message(&[], &mut a[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(stream, h, &mut a[..Header::SIZE + n]).await?;
    let mut b = [0u8; MAX_NOISE_HANDSHAKE_SIZE];
    let h = recv_frame(stream, &mut b).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    hs.read_message(&b[..h.len().into()], &mut a)?;
    Ok(hs.into_transport_mode()?)
}

/// Perform a noise handshake as responder with a remote party.
async fn on_handshake(stream: &mut TcpStream, mut hs: HandshakeState) -> Result<TransportState> {
    let mut a = [0u8; MAX_NOISE_HANDSHAKE_SIZE];
    let h = recv_frame(stream, &mut a).await?;
    if !h.is_data() || h.is_partial() {
        return Err(NetworkError::InvalidHandshakeMessage);
    }
    let mut b = [0u8; MAX_NOISE_HANDSHAKE_SIZE];
    hs.read_message(&a[..h.len().into()], &mut b)?;
    let n = hs.write_message(&[], &mut b[Header::SIZE..])?;
    let h = Header::data(n as u16);
    send_frame(stream, h, &mut b[..Header::SIZE + n]).await?;
    Ok(hs.into_transport_mode()?)
}

/// Read a single frame (header + payload) from the remote.
async fn recv_frame<R, const N: usize>(stream: &mut R, buf: &mut [u8; N]) -> io::Result<Header>
where
    R: AsyncReadExt + Unpin,
{
    let h = {
        let n = stream.read_u32().await?;
        Header::unvalidated(n)
    };
    let n = h.len().into();
    if n > N {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    stream.read_exact(&mut buf[..n]).await?;
    Ok(h)
}

/// Write a single frame (header + payload) to the remote.
///
/// The header is serialised into the first 4 bytes of `msg`. It is the
/// caller's responsibility to ensure there is room at the beginning.
async fn send_frame<W>(stream: &mut W, hdr: Header, msg: &mut [u8]) -> io::Result<()>
where
    W: AsyncWriteExt + Unpin,
{
    debug_assert!(msg.len() <= MAX_NOISE_MESSAGE_SIZE);
    msg[..Header::SIZE].copy_from_slice(&hdr.to_bytes());
    stream.write_all(msg).await?;
    Ok(())
}
