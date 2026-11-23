use std::path::Path;

use async_trait::async_trait;
use hotshot_query_service::data_source::FileSystemDataSource;

use super::data_source::{Provider, PruningDataSource, SequencerDataSource};
use crate::{SeqTypes, catchup::CatchupStorage, persistence::fs::Options};

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

impl PruningDataSource for DataSource {
    async fn get_oldest_block(
        &self,
    ) -> anyhow::Result<Option<hotshot_query_service::availability::BlockQueryData<SeqTypes>>> {
        Ok(None)
    }

    async fn get_oldest_leaf(
        &self,
    ) -> anyhow::Result<Option<hotshot_query_service::availability::LeafQueryData<SeqTypes>>> {
        Ok(None)
    }
}

#[cfg(test)]
mod test {
    use hotshot_query_service::{
        availability::LeafId,
        data_source::{
            Transaction, VersionedDataSource,
            storage::{AvailabilityStorage, FileSystemStorage, UpdateAvailabilityStorage},
        },
        types::HeightIndexed,
    };
    use tempfile::TempDir;
    use versions::EPOCH_VERSION;

    use super::*;
    use crate::legacy_chain::LegacyChain;

    /// The file system storage keeps leaves in a binary log. Leaves from before the 0.3 upgrade,
    /// the first 0.3 leaf, whose justify QC is still pre-epoch, and the leaf after it read back
    /// after a restart, by height and by hash.
    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_legacy_leaves_survive_restart() -> anyhow::Result<()> {
        let chain = LegacyChain::load("v2-v3")?;
        let upgraded = chain
            .blocks
            .iter()
            .position(|block| block.leaf.header().version() >= EPOCH_VERSION)
            .expect("the chain upgrades to 0.3");
        let leaves: Vec<_> = chain.blocks[..=upgraded + 1]
            .iter()
            .map(|block| block.leaf.clone())
            .collect();

        let dir = TempDir::new()?;
        {
            let storage = FileSystemStorage::<SeqTypes>::create(dir.path()).await?;
            let mut tx = storage.write().await?;
            for leaf in &leaves {
                tx.insert_leaf(leaf).await?;
            }
            tx.commit().await?;
        }

        let storage = FileSystemStorage::<SeqTypes>::open(dir.path()).await?;
        let mut tx = storage.read().await?;
        for leaf in &leaves {
            let height = leaf.height();
            assert_eq!(
                tx.get_leaf(LeafId::Number(height as usize)).await?,
                *leaf,
                "leaf {height} by height"
            );
            assert_eq!(
                tx.get_leaf(LeafId::Hash(leaf.hash())).await?,
                *leaf,
                "leaf {height} by hash"
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod impl_testable_data_source {
    use tempfile::TempDir;

    use super::*;
    use crate::api::{self, data_source::testing::TestableSequencerDataSource, options::Query};

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
            opt.query_fs(Query::default(), Options::new(storage.path().into()))
        }
    }
}
