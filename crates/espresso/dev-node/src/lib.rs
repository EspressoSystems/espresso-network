use std::fmt;

use alloy::primitives::Address;
use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DevInfo {
    pub builder_url: Url,
    pub sequencer_api_port: u16,
    pub l1_prover_port: u16,
    pub l1_url: Url,
    pub l1_light_client_address: Address,
    pub alt_chains: Vec<AltChainInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AltChainInfo {
    pub chain_id: u64,
    pub provider_url: Url,
    pub light_client_address: Address,
    pub prover_port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetHotshotDownReqBody {
    pub chain_id: Option<u64>,
    pub height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetHotshotUpReqBody {
    pub chain_id: u64,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum DevNodeVersion {
    #[value(name = "0.3")]
    V0_3,
    #[value(name = "0.4")]
    V0_4,
}

impl fmt::Display for DevNodeVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DevNodeVersion::V0_3 => write!(f, "0.3"),
            DevNodeVersion::V0_4 => write!(f, "0.4"),
        }
    }
}
