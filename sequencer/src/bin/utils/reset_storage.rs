use clap::{Args, Subcommand};
use espresso_types::v0::traits::MembershipPersistence;
use sequencer::{
    api::data_source::{DataSourceOptions, SequencerDataSource},
    persistence,
};

/// Options for resetting persistent storage.
///
/// This will remove all the persistent storage of a sequencer node effectively resetting it to
/// its genesis state. Do not run this program while the sequencer is running.
#[derive(Clone, Debug, Subcommand)]
pub enum Commands {
    /// Contains subcommands for resetting sequencer storage.
    Sequencer(SequencerStorageOptions),
}

#[derive(Clone, Debug, Args)]
pub struct SequencerStorageOptions {
    /// Only clear stake table events
    ///
    /// This can be used to recover from a fatal error when applying stake table events without
    /// deleting other data
    #[clap(long)]
    pub stake_table_only: bool,

    #[command(subcommand)]
    pub storage: SequencerStorage,
}

#[derive(Clone, Debug, Subcommand)]
pub enum SequencerStorage {
    /// Reset file system storage.
    Fs(persistence::fs::Options),
    /// Reset SQL storage.
    Sql(Box<persistence::sql::Options>),
}

pub async fn run(opt: Commands) -> anyhow::Result<()> {
    match opt {
        Commands::Sequencer(SequencerStorageOptions {
            stake_table_only,
            storage,
        }) => match storage {
            SequencerStorage::Fs(opt) => {
                if stake_table_only {
                    tracing::warn!(
                        "clearing stake table events from sequencer file system storage {opt:?}"
                    );
                    clear_stake_table_events_storage(opt).await
                } else {
                    tracing::warn!("resetting sequencer file system storage {opt:?}");
                    reset_storage(opt).await
                }
            },
            SequencerStorage::Sql(opt) => {
                if stake_table_only {
                    tracing::warn!(
                        "clearing stake table events from sequencer SQL storage {opt:?}"
                    );
                    clear_stake_table_events_storage(*opt).await
                } else {
                    tracing::warn!("resetting sequencer SQL storage {opt:?}");
                    reset_storage(*opt).await
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

async fn clear_stake_table_events_storage<O: DataSourceOptions>(mut opt: O) -> anyhow::Result<()> {
    let persistence = opt.create().await?;
    persistence.delete_stake_tables().await?;
    Ok(())
}
