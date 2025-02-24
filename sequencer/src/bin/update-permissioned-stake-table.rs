use anyhow::{Context, Result};
use clap::Parser;
use client::SequencerClient;
use espresso_types::parse_duration;
use ethers::types::Address;
use hotshot_types::{network::PeerConfigKeys, traits::signature_key::StakeTableEntryType};
use sequencer::api::data_source::PublicHotShotConfig;
use sequencer_utils::{
    logging,
    stake_table::{update_stake_table, PermissionedStakeTableUpdate},
};
use std::{path::PathBuf, time::Duration};

use url::Url;

#[derive(Debug, Clone, Parser)]
struct Options {
    /// RPC URL for the L1 provider.
    #[clap(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545"
    )]
    rpc_url: Url,

    /// Request rate when polling L1.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_POLLING_INTERVAL",
        default_value = "7s",
        value_parser = parse_duration,
    )]
    pub l1_polling_interval: Duration,

    /// Mnemonic for an L1 wallet.
    ///
    /// This wallet is used to deploy the contracts, so the account indicated by ACCOUNT_INDEX must
    /// be funded with with ETH.
    #[clap(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_SEQUENCER_ETH_MNEMONIC",
        default_value = "test test test test test test test test test test test junk"
    )]
    mnemonic: String,

    /// Account index in the L1 wallet generated by MNEMONIC to use when deploying the contracts.
    #[clap(
        long,
        name = "ACCOUNT_INDEX",
        env = "ESPRESSO_DEPLOYER_ACCOUNT_INDEX",
        default_value = "0"
    )]
    account_index: u32,

    /// Permissioned stake table contract address.
    #[clap(long, env = "ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_ADDRESS")]
    contract_address: Address,

    /// Path to the toml file containing the update information.
    ///
    /// Schema of toml file:
    /// ```toml
    /// stakers_to_remove = [
    ///   {
    ///     stake_table_key = "BLS_VER_KEY~...",
    ///   },
    /// ]
    ///
    /// new_stakers = [
    ///   {
    ///     stake_table_key = "BLS_VER_KEY~...",
    ///     state_ver_key = "SCHNORR_VER_KEY~...",
    ///     da = true,
    ///     stake = 1, # this value is ignored, but needs to be set
    ///   },
    /// ]
    /// ```
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_UPDATE_TOML_PATH",
        verbatim_doc_comment
    )]
    update_toml_path: Option<PathBuf>,
    /// Flag to update the contract with the initial stake table.
    ///  
    /// This stake table is fetched directly from hotshot config, and is pre-epoch stake table

    #[clap(
        long,
        short,
        env = "ESPRESSO_SEQUENCER_PERMISSIONED_STAKE_TABLE_INITIAL",
        default_value_t = false
    )]
    initial: bool,

    /// Peer nodes use to fetch missing state
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_STATE_PEERS",
        value_delimiter = ',',
        conflicts_with = "update_toml_path"
    )]
    pub state_peers: Option<Vec<Url>>,

    #[clap(flatten)]
    logging: logging::Config,
}

#[tokio::main]
async fn main() -> Result<()> {
    let opts = Options::parse();
    opts.logging.init();

    let mut update: Option<PermissionedStakeTableUpdate> = None;

    if opts.initial {
        let peers = opts.state_peers.context("No state peers found")?;
        let clients: Vec<SequencerClient> = peers.into_iter().map(SequencerClient::new).collect();

        for client in &clients {
            tracing::warn!("calling config endpoint of {client:?}");

            match client.config::<PublicHotShotConfig>().await {
                Ok(config) => {
                    let hotshot = config.into_hotshot_config();
                    let st = hotshot.known_nodes_with_stake;
                    let da_nodes = hotshot.known_da_nodes;

                    let new_stakers = st
                        .into_iter()
                        .map(|s| PeerConfigKeys {
                            stake_table_key: s.stake_table_entry.stake_key.clone(),
                            state_ver_key: s.state_ver_key.clone(),
                            stake: s.stake_table_entry.stake().as_u64(),
                            da: da_nodes.contains(&s),
                        })
                        .collect();

                    update = Some(PermissionedStakeTableUpdate::new(new_stakers, Vec::new()));
                    break;
                }
                Err(e) => {
                    tracing::warn!("Failed to fetch config from sequencer: {e}");
                }
            };
        }
    } else {
        let path = opts.update_toml_path.context("No update path found")?;
        tracing::error!("updating stake table from path: {path:?}");
        update = Some(PermissionedStakeTableUpdate::from_toml_file(&path)?);
    };

    update_stake_table(
        opts.rpc_url,
        opts.l1_polling_interval,
        opts.mnemonic,
        opts.account_index,
        opts.contract_address,
        update.unwrap(),
    )
    .await?;

    Ok(())
}
