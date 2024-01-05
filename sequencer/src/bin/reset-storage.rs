use clap::Parser;
use hotshot_query_service::data_source::VersionedDataSource;
use sequencer::{
    api::data_source::{DataSourceOptions, SequencerDataSource},
    persistence::{self},
};

/// Reset the persistent storage of a sequencer.
///
/// This will remove all the persistent storage of a sequencer node, effectively resetting it to
/// its genesis state. Do not run this program while the sequencer is running.
#[derive(Clone, Debug, Parser)]
enum Options {
    /// Reset file system storage.
    Fs(persistence::fs::Options),
    /// Reset SQL storage.
    Sql(persistence::sql::Options),
}

#[async_std::main]
async fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    match opt {
        Options::Fs(opt) => reset_storage(opt).await,
        Options::Sql(opt) => reset_storage(opt).await,
    }
}

async fn reset_storage<O: DataSourceOptions>(opt: O) -> anyhow::Result<()> {
    // Reset query service storage.
    let mut ds = O::DataSource::create(opt.clone(), true).await?;
    ds.commit().await?;

    // Reset consensus storage.
    opt.reset().await?;

    Ok(())
}
