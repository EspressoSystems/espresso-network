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
}

impl AppConfig {
    pub fn from_file() -> Result<Self> {
        let s = fs::read_to_string("libp2p_test.toml")?;
        Ok(toml::from_str(&s)?)
    }
}
