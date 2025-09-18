use std::path::Path;

use async_trait::async_trait;
use hotshot_query_service::data_source::FileSystemDataSource;

use super::data_source::{Provider, SequencerDataSource};
use crate::{catchup::CatchupStorage, persistence::fs::Options, SeqTypes};

pub type DataSource = FileSystemDataSource<SeqTypes, Provider>;

#[async_trait]
impl SequencerDataSource for DataSource {
    type Options = Options;

    async fn create(opt: Self::Options, provider: Provider, reset: bool) -> anyhow::Result<Self> {
        let path = Path::new(opt.path());
        let data_source = {
            if reset {
                FileSystemDataSource::create(path, provider).await?
            } else {
                FileSystemDataSource::open(path, provider).await?
            }
        };

        Ok(data_source)
    }
}

impl CatchupStorage for DataSource {}

#[cfg(test)]
mod impl_testable_data_source {
    use tempfile::TempDir;

    use super::*;
    use crate::api::{self, data_source::testing::TestableSequencerDataSource};

    #[async_trait]
    impl TestableSequencerDataSource for DataSource {
        type Storage = TempDir;

        async fn create_storage() -> Self::Storage {
            TempDir::new().unwrap()
        }

        fn persistence_options(storage: &Self::Storage) -> Self::Options {
            Options::new(storage.path().into())
        }

        fn options(storage: &Self::Storage, opt: api::Options) -> api::Options {
            opt.query_fs(Default::default(), Options::new(storage.path().into()))
        }
    }
}
