use std::{fs, path::PathBuf, process::ExitCode};

use anyhow::{Context, Result};
use clap::Parser;
use espresso_node::SequencerApiVersion;
use espresso_types::{EpochVersion, Header, config::PublicNetworkConfig};
use hotshot_types::{data::EpochNumber, utils::epoch_from_block_number};
use light_client::state::Genesis;
use light_client_query_service::{LogFormat, init_logging};
use surf_disco::{Client, Url};
use tracing::instrument;
use vbs::version::StaticVersionType;

/// Generate a light client genesis file for an existing chain.
///
/// WARNING: the genesis file is constructed from information provided by an untrusted query
/// service. It cannot be verified automatically because the genesis file itself is required to
/// verify chain data. If this program is used to generate a light client genesis file, it is
/// extremely important that a human reviews and verifies the accuracy of the generated file.
#[derive(Debug, Parser)]
struct Options {
    /// Destination file path for genesis file (default stdout).
    #[clap(short, long, env = "LIGHT_CLIENT_GENESIS")]
    output: Option<PathBuf>,

    /// URL for a trusted Espresso query service.
    #[clap(short, long, env = "LIGHT_CLIENT_ESPRESSO_URL")]
    espresso_url: Url,

    /// Formatting options for tracing.
    #[clap(long, env = "RUST_LOG_FORMAT")]
    log_format: Option<LogFormat>,
}

impl Options {
    #[instrument(skip(self))]
    async fn find_genesis(&self) -> Result<Genesis> {
        let client = Client::<hotshot_query_service::Error, SequencerApiVersion>::new(
            self.espresso_url.clone(),
        );

        // Find the epoch height.
        let config: PublicNetworkConfig = client
            .get("config/hotshot")
            .send()
            .await
            .context("fetching HotShot config")?;
        let epoch_height = config.hotshot_config().blocks_per_epoch();

        // We know the upgrade to proof of stake must have occurred before the first epoch.
        let upper_bound_pos = config.hotshot_config().epoch_start_block();

        // Through binary search, find the first block where the upgrade to PoS occurred.
        let target_version = EpochVersion::VERSION;
        let mut start = 0;
        let mut end = upper_bound_pos;
        tracing::info!(start, end, "searching for upgrade to PoS in range");

        // Search invariants:
        // * `start` is a block strictly before the upgrade (i.e. with version < 0.3)
        // * `end` is a block after the upgrade (i.e. with version >= 0.3)
        while start + 1 < end {
            let midpoint = (start + end) / 2;
            let header: Header = client
                .get(&format!("availability/header/{midpoint}"))
                .send()
                .await
                .context(format!("fetching header {midpoint}"))?;
            tracing::debug!(
                start,
                midpoint,
                end,
                version = %header.version(),
                "test midpoint"
            );
            if header.version() < target_version {
                start = midpoint;
            } else {
                end = midpoint;
            }
        }
        let upgrade_block = start + 1;
        let start_epoch = epoch_from_block_number(upgrade_block, epoch_height);
        tracing::info!("found upgrade to PoS at block {upgrade_block}, epoch {start_epoch}");

        // Start from the third epoch, since we need the prior epoch's root to have the upgraded
        // header with the stake table hash.
        let start_epoch = start_epoch + 2;

        Ok(Genesis {
            epoch_height,
            first_epoch_with_dynamic_stake_table: EpochNumber::new(start_epoch),
            stake_table: config
                .hotshot_config()
                .known_nodes_with_stake()
                .iter()
                .map(|node| node.stake_table_entry.clone())
                .collect(),
        })
    }
}

async fn run() -> Result<()> {
    let opt = Options::parse();
    init_logging(opt.log_format);

    let genesis = opt.find_genesis().await?;
    let toml = toml::to_string_pretty(&genesis).context("serializing genesis")?;
    if let Some(output) = opt.output {
        fs::write(output, toml).context("writing genesis file")?;
    } else {
        println!("{toml}");
    }

    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    if let Err(err) = run().await {
        tracing::error!("{err:#}");
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
