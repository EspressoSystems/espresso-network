use std::path::PathBuf;

use anyhow::Context;
use clap::{Parser, Subcommand};
use espresso_node::{
    api::data_source::{DataSourceOptions, SequencerDataSource},
    membership_store::MembershipStore,
    persistence,
};
use espresso_types::v0::traits::MembershipPersistence;
use espresso_utils::logging;
use hotshot_new_protocol::store::ConsensusStore;

/// Reset the persistent storage of a sequencer.
///
/// This will remove all the persistent storage of a sequencer node, effectively resetting it to
/// its genesis state. Do not run this program while the sequencer is running.
#[derive(Clone, Debug, Parser)]
struct Options {
    #[clap(flatten)]
    logging: logging::Config,

    /// Only clear stake table events
    /// This can be used to recover from a fatal error when applying stake table events without
    /// deleting any other data
    #[clap(long)]
    stake_table_only: bool,

    /// The node's consensus store, which holds its stake tables and DRB results, and with a full
    /// reset its votes and locks too. Without it, only the storage below is reset.
    #[clap(long, env = "ESPRESSO_NODE_CONSENSUS_STORE_PATH")]
    consensus_store_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Debug, Subcommand)]
enum Command {
    /// Reset file system storage.
    Fs(persistence::fs::Options),
    /// Reset SQL storage.
    Sql(Box<persistence::sql::Options>),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    opt.logging.init();

    match &opt.consensus_store_path {
        Some(path) if opt.stake_table_only => {
            tracing::warn!(?path, "clearing stake tables from the consensus store");
            let (store, _) = ConsensusStore::open(path.clone(), async || {
                anyhow::bail!("there is no consensus store at {}", path.display())
            })
            .await?;
            MembershipStore::new(store).delete_stake_tables().await?;
        },
        Some(path) => {
            tracing::warn!(?path, "deleting the consensus store");
            if path.exists() {
                std::fs::remove_dir_all(path)
                    .with_context(|| format!("deleting {}", path.display()))?;
            }
        },
        None => tracing::warn!(
            "no consensus store given; its stake tables, votes and locks stay as they are"
        ),
    }

    match opt.command {
        Command::Fs(persistence_opt) => {
            if opt.stake_table_only {
                tracing::warn!(
                    "clearing stake table events from file system storage {persistence_opt:?}"
                );
                clear_stake_table_events(persistence_opt).await
            } else {
                tracing::warn!("resetting file system storage {persistence_opt:?}");
                reset_storage(persistence_opt).await
            }
        },
        Command::Sql(persistence_opt) => match persistence_opt.into_backend() {
            persistence::sql::SqlBackendOptions::Sqlite(persistence_opt) => {
                if opt.stake_table_only {
                    tracing::warn!(
                        "clearing stake table events from SQL storage {persistence_opt:?}"
                    );
                    clear_stake_table_events(persistence_opt).await
                } else {
                    tracing::warn!("resetting SQL storage {persistence_opt:?}");
                    reset_storage(persistence_opt).await
                }
            },
            persistence::sql::SqlBackendOptions::Postgres(persistence_opt) => {
                if opt.stake_table_only {
                    tracing::warn!(
                        "clearing stake table events from SQL storage {persistence_opt:?}"
                    );
                    clear_stake_table_events(persistence_opt).await
                } else {
                    tracing::warn!("resetting SQL storage {persistence_opt:?}");
                    reset_storage(persistence_opt).await
                }
            },
        },
    }
}

async fn reset_storage<O: DataSourceOptions>(opt: O) -> anyhow::Result<()> {
    // Reset query service storage.
    O::DataSource::create(opt.clone(), Default::default(), true).await?;
    // Reset consensus storage.
    opt.reset().await?;

    Ok(())
}

async fn clear_stake_table_events<O: DataSourceOptions>(mut opt: O) -> anyhow::Result<()> {
    let persistence = opt.create().await?;
    persistence.delete_stake_tables().await?;
    Ok(())
}
