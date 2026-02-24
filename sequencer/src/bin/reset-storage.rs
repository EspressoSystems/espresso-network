use clap::{Parser, Subcommand};
use espresso_types::v0::traits::MembershipPersistence;
use sequencer::{
    api::data_source::{DataSourceOptions, SequencerDataSource},
    persistence,
};
use sequencer_utils::logging;

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
        Command::Sql(persistence_opt) => {
            if opt.stake_table_only {
                tracing::warn!("clearing stake table events from SQL storage {persistence_opt:?}");
                clear_stake_table_events(*persistence_opt).await
            } else {
                tracing::warn!("resetting SQL storage {persistence_opt:?}");
                reset_storage(*persistence_opt).await
            }
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
