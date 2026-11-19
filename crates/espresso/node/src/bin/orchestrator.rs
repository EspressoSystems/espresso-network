use std::{num::NonZeroUsize, time::Duration};

use alloy::hex::{self, FromHexError};
use clap::Parser;
use derive_more::From;
use espresso_types::{Ratio, SeqTypes, parse_duration};
use espresso_utils::logging;
use hotshot_orchestrator::run_orchestrator;
use hotshot_types::{PeerConfig, network::NetworkConfig};
use snafu::Snafu;
use url::Url;

#[derive(Parser)]
struct Args {
    /// Port to run the server on.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_PORT")]
    port: u16,

    /// Port to run the server on.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_MANUAL_START_PASSWORD")]
    manual_start_password: Option<String>,

    /// Number of nodes in the network.
    #[clap(short, long, env = "ESPRESSO_ORCHESTRATOR_NUM_NODES")]
    num_nodes: NonZeroUsize,

    /// Deprecated: accepted for compatibility and ignored.
    #[clap(long, env = "ESPRESSO_ORCHESTRATOR_START_DELAY", value_parser = parse_duration)]
    start_delay: Option<Duration>,

    /// Deprecated: accepted for compatibility and ignored.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_NEXT_VIEW_TIMEOUT",
        value_parser = parse_duration
    )]
    next_view_timeout: Option<Duration>,

    /// Deprecated: accepted for compatibility and ignored.
    #[arg(long, env = "ESPRESSO_ORCHESTRATOR_TIMEOUT_RATIO")]
    timeout_ratio: Option<Ratio>,

    /// The threshold
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_START_THRESHOLD",
        default_value = "8:10"
    )]
    start_threshold: Ratio,

    /// Deprecated: accepted for compatibility and ignored.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_ROUND_START_DELAY",
        value_parser = parse_duration
    )]
    round_start_delay: Option<Duration>,

    /// Seed to use for generating node keys.
    ///
    /// The seed is a 32 byte integer, encoded in hex.
    #[arg(long, env = "ESPRESSO_ORCHESTRATOR_KEYGEN_SEED", default_value = "0x0000000000000000000000000000000000000000000000000000000000000000", value_parser = parse_seed)]
    keygen_seed: [u8; 32],

    /// Deprecated: accepted for compatibility and ignored. Leaders build blocks in-process.
    #[arg(long, env = "ESPRESSO_ORCHESTRATOR_BUILDER_URLS", num_args = 1.., value_delimiter = ',')]
    builder_urls: Vec<Url>,

    /// Deprecated: accepted for compatibility and ignored. Leaders build blocks in-process.
    #[arg(
        long,
        env = "ESPRESSO_ORCHESTRATOR_BUILDER_TIMEOUT",
        value_parser = parse_duration
    )]
    builder_timeout: Option<Duration>,

    #[clap(flatten)]
    logging: logging::Config,
}

#[derive(Debug, Snafu, From)]
enum ParseSeedError {
    #[snafu(display("seed must be valid hex: {source}"))]
    Hex { source: FromHexError },

    #[snafu(display("wrong length for seed {length} (expected 32)"))]
    WrongLength { length: usize },
}

fn parse_seed(s: &str) -> Result<[u8; 32], ParseSeedError> {
    <[u8; 32]>::try_from(hex::decode(s)?)
        .map_err(|vec| ParseSeedError::WrongLength { length: vec.len() })
}

fn main() {
    let migrated_envs = espresso_utils::env_compat::migrate_legacy_env_vars();
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async_main(migrated_envs))
}

async fn async_main(migrated_envs: Vec<(&str, &str)>) {
    let args = Args::parse();
    args.logging.init();
    espresso_utils::env_compat::log_migrated_env_vars(&migrated_envs);

    let mut config = NetworkConfig::<SeqTypes> {
        manual_start_password: args.manual_start_password,
        indexed_da: false,
        ..Default::default()
    };

    config.config.num_nodes_with_stake = args.num_nodes;
    config.config.known_nodes_with_stake = vec![PeerConfig::test_default(); args.num_nodes.get()];
    config.config.known_da_nodes = Vec::new();
    config.config.start_threshold = args.start_threshold.into();
    config.config.da_staked_committee_size = args.num_nodes.get();
    if !args.builder_urls.is_empty() || args.builder_timeout.is_some() {
        tracing::warn!(
            "builder URL and timeout overrides are deprecated and ignored; leaders build blocks \
             in-process"
        );
    }
    if args.start_delay.is_some()
        || args.next_view_timeout.is_some()
        || args.timeout_ratio.is_some()
        || args.round_start_delay.is_some()
    {
        tracing::warn!(
            "--start-delay, --next-view-timeout, --timeout-ratio and --round-start-delay are \
             deprecated and ignored"
        );
    }
    tokio::select! {
        res = run_orchestrator(
            config,
            format!("http://0.0.0.0:{}", args.port).parse().unwrap(),
        ) => res.unwrap(),
        _ = espresso_utils::shutdown::wait_for_shutdown_signal() => {},
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_deprecated_consensus_options() {
        let args = Args::try_parse_from([
            "orchestrator",
            "--port",
            "8080",
            "--num-nodes",
            "4",
            "--builder-urls",
            "http://localhost:8081",
            "--builder-timeout",
            "250ms",
            "--next-view-timeout",
            "2s",
            "--start-delay",
            "30s",
            "--timeout-ratio",
            "11:10",
            "--round-start-delay",
            "100ms",
        ])
        .unwrap();
        assert_eq!(
            args.builder_urls,
            ["http://localhost:8081".parse::<Url>().unwrap()]
        );
        assert_eq!(args.builder_timeout, Some(Duration::from_millis(250)));
        assert_eq!(args.next_view_timeout, Some(Duration::from_secs(2)));
        assert_eq!(args.start_delay, Some(Duration::from_secs(30)));
        assert_eq!(args.timeout_ratio, Some("11:10".parse::<Ratio>().unwrap()));
        assert_eq!(args.round_start_delay, Some(Duration::from_millis(100)));
    }
}
