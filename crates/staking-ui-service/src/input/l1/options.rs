//! Configuration options for Rpc Stream client.

use crate::types::common::Address;
use crate::{Result, input::l1::switching_transport::SwitchingTransport};
use alloy::{providers::RootProvider, rpc::client::RpcClient};
use clap::Parser;
use espresso_types::parse_duration;
use std::time::Duration;
use tide_disco::Url;

/// Configuration for an Rpc Stream client.
#[derive(Clone, Debug, Parser)]
pub struct L1ClientOptions {
    /// Delay when retrying failed L1 queries.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_RETRY_DELAY",
        default_value = "1s",
        value_parser = parse_duration,
    )]
    pub l1_retry_delay: Duration,

    /// Request rate when polling L1.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// Maximum time to wait for new heads before considering a stream invalid and reconnecting.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_SUBSCRIPTION_TIMEOUT",
        default_value = "2m",
        value_parser = parse_duration,
    )]
    pub subscription_timeout: Duration,

    /// Fail over to another provider if the current provider fails twice within this window.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_FREQUENT_FAILURE_TOLERANCE",
        default_value = "1m",
        value_parser = parse_duration,
    )]
    pub l1_frequent_failure_tolerance: Duration,

    /// Fail over to another provider if the current provider fails many times in a row.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_CONSECUTIVE_FAILURE_TOLERANCE",
        default_value = "10"
    )]
    pub l1_consecutive_failure_tolerance: usize,

    /// Revert back to the first provider this duration after failing over.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_FAILOVER_REVERT",
        default_value = "30m",
        value_parser = parse_duration,
    )]
    pub l1_failover_revert: Duration,

    /// Amount of time to wait after receiving a 429 response before making more L1 RPC requests.
    ///
    /// If not set, the general l1-retry-delay will be used.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_RATE_LIMIT_DELAY",
        value_parser = parse_duration,
    )]
    pub l1_rate_limit_delay: Option<Duration>,

    /// Maximum number of L1 blocks that can be scanned for events in a single query.
    #[clap(
        long,
        env = "ESPRESSO_STAKING_SERVICE_L1_EVENTS_MAX_BLOCK_RANGE",
        default_value = "10000"
    )]
    pub l1_events_max_block_range: u64,

    /// HTTP providers to use for L1 RPC requests.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_L1_HTTP", value_delimiter = ',', num_args = 1..)]
    pub http_providers: Vec<Url>,

    /// Separate provider to use for subscription feeds.
    ///
    /// Typically this would be a WebSockets endpoint while the main provider uses HTTP.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_L1_WS", value_delimiter = ',')]
    pub l1_ws_provider: Option<Vec<Url>>,

    /// Address of the stake table contract.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_STAKE_TABLE_ADDRESS")]
    pub stake_table_address: Address,

    /// Address of the reward contract.
    #[clap(long, env = "ESPRESSO_STAKING_SERVICE_REWARD_CONTRACT_ADDRESS")]
    pub reward_contract_address: Address,
}

impl Default for L1ClientOptions {
    fn default() -> Self {
        Self {
            l1_retry_delay: Duration::from_secs(1),
            l1_polling_interval: Duration::from_secs(7),
            subscription_timeout: Duration::from_secs(120),
            l1_frequent_failure_tolerance: Duration::from_secs(60),
            l1_consecutive_failure_tolerance: 10,
            l1_failover_revert: Duration::from_secs(1800),
            l1_rate_limit_delay: None,
            l1_events_max_block_range: 10000,
            http_providers: Vec::new(),
            l1_ws_provider: None,
            stake_table_address: Address::ZERO,
            reward_contract_address: Address::ZERO,
        }
    }
}

impl L1ClientOptions {
    pub fn provider(&self) -> Result<(RootProvider, SwitchingTransport)> {
        let transport = SwitchingTransport::new(self.clone())?;
        let rpc_client = RpcClient::new(transport.clone(), false);
        Ok((RootProvider::new(rpc_client), transport))
    }

    pub(super) fn rate_limit_delay(&self) -> Duration {
        self.l1_rate_limit_delay.unwrap_or(self.l1_retry_delay)
    }
}
