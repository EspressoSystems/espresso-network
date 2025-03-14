use clap::{Parser, Subcommand};
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
    #[command(flatten)]
    logging: logging::Config,

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
        Command::Fs(opt) => {
            tracing::warn!("resetting file system storage {opt:?}");
            reset_storage(opt).await
        },
        Command::Sql(opt) => {
            tracing::warn!("resetting SQL storage {opt:?}");
            reset_storage(*opt).await
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
