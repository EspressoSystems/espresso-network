use std::fs;

use anyhow::Result;
use libp2p::Multiaddr;
use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub listen: Multiaddr,
    pub private_key: tagged_base64::TaggedBase64,
    pub peers: Vec<(PeerId, Multiaddr)>,
    pub send_mode: bool,
    pub message: Option<String>,
    pub libp2p_test: Option<Libp2pTest>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TransportProtocol {
    Tcp {
        auth: AuthType,
        mplex: MultiplexerType,
    },
    Quic,
}

impl Default for TransportProtocol {
    fn default() -> Self {
        TransportProtocol::Tcp {
            auth: AuthType::Noise,
            mplex: MultiplexerType::Yamux,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum AuthType {
    Noise,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum MultiplexerType {
    Yamux,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Libp2pTest {
    Ping {
        transport_protocol: TransportProtocol,
    },
    RequestResponse {
        transport_protocol: TransportProtocol,
    },
    Gossipsub {
        transport_protocol: TransportProtocol,
    },
}

impl Libp2pTest {
    #[allow(dead_code)]
    pub fn transport_protocol(&self) -> &TransportProtocol {
        match self {
            Libp2pTest::Ping { transport_protocol } => transport_protocol,
            Libp2pTest::RequestResponse { transport_protocol } => transport_protocol,
            Libp2pTest::Gossipsub { transport_protocol } => transport_protocol,
        }
    }
}

impl AppConfig {
    pub fn from_file() -> Result<Self> {
        let s = fs::read_to_string("/app_config/libp2p_test.toml")?;
        Ok(toml::from_str(&s)?)
    }
}
