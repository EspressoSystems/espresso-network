//! V1 status API.
//!
//! Mirrors the endpoints of the legacy `hotshot-query-service` status API (its tide-disco
//! `status.toml`, since removed).

use alloy::primitives::Address;
use async_trait::async_trait;
use hotshot_types::{addr::NetAddr, light_client::StateVerKey, signature_key::BLSPubKey, x25519};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodePublicKeys {
    pub eth_account: Option<Address>,
    pub consensus_key: BLSPubKey,
    pub state_ver_key: StateVerKey,
    #[serde(with = "x25519_tagged")]
    pub x25519_key: Option<x25519::PublicKey>,
    /// Cliquenet address peers dial: the configured advertise address, `None` when there is none.
    pub p2p_addr: Option<NetAddr>,
}

mod x25519_tagged {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

    use super::x25519;

    pub fn serialize<S: Serializer>(
        key: &Option<x25519::PublicKey>,
        s: S,
    ) -> Result<S::Ok, S::Error> {
        key.as_ref().map(ToString::to_string).serialize(s)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        d: D,
    ) -> Result<Option<x25519::PublicKey>, D::Error> {
        Option::<String>::deserialize(d)?
            .map(|s| s.parse().map_err(D::Error::custom))
            .transpose()
    }
}

#[async_trait]
pub trait StatusApi {
    async fn block_height(&self) -> anyhow::Result<u64>;
    async fn success_rate(&self) -> anyhow::Result<f64>;
    async fn time_since_last_decide(&self) -> anyhow::Result<u64>;

    async fn metrics(&self) -> anyhow::Result<String>;

    async fn keys(&self) -> anyhow::Result<NodePublicKeys>;
}
