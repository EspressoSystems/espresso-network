//! Update loop for query API state.

use std::{fmt::Debug, sync::Arc};

use anyhow::bail;
use async_trait::async_trait;
use derivative::Derivative;
use derive_more::From;
use espresso_types::{
    Payload,
    v0::traits::{NullEventConsumer, SequencerPersistence},
};
use hotshot_query_service::{
    availability::{
        AvailabilityDataSource, BlockInfo, BlockQueryData, LeafQueryData, UpdateAvailabilityData,
        VidCommonQueryData,
    },
    data_source::UpdateDataSource,
    node::NodeDataSource,
    types::HeightIndexed,
};
use hotshot_types::{
    new_protocol::CoordinatorEvent,
    simple_certificate::LightClientStateUpdateCertificateV2,
    traits::block_contents::{BlockHeader, BlockPayload},
};

use super::{
    StorageState,
    data_source::{SequencerDataSource, StateCertDataSource},
};
use crate::{EventConsumer, SeqTypes, chain_view::ChainView};

#[async_trait]
pub trait DecideSink: Send + Sync + Debug {
    /// Stores a decided block, with the state certificate of the epoch it is the root of.
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()>;

    /// The first height a resuming node has to append.
    async fn block_height(&self) -> anyhow::Result<u64>;

    /// The newest block stored, which a resuming node continues from.
    async fn last_leaf(&self) -> anyhow::Result<Option<LeafQueryData<SeqTypes>>>;
}

#[async_trait]
impl<T> DecideSink for Box<T>
where
    T: DecideSink + ?Sized,
{
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        (**self).append(info, state_cert).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        (**self).block_height().await
    }

    async fn last_leaf(&self) -> anyhow::Result<Option<LeafQueryData<SeqTypes>>> {
        (**self).last_leaf().await
    }
}

pub trait ApiSink: EventConsumer + DecideSink {}

impl<T: EventConsumer + DecideSink> ApiSink for T {}

/// Without a query module there is nowhere to store decided blocks, so a node that hands them
/// here fails on the first one instead of reporting progress it never persisted.
#[async_trait]
impl DecideSink for NullEventConsumer {
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        _: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        bail!(
            "cannot store block {}: the API has no query storage",
            info.height()
        )
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(0)
    }

    async fn last_leaf(&self) -> anyhow::Result<Option<LeafQueryData<SeqTypes>>> {
        Ok(None)
    }
}

#[derive(Derivative, From)]
#[derivative(Clone(bound = ""), Debug(bound = "D: Debug"))]
pub(crate) struct ApiEventConsumer<D> {
    inner: Arc<StorageState<D>>,
}

#[async_trait]
impl<D> EventConsumer for ApiEventConsumer<D>
where
    D: SequencerDataSource + Debug + Send + Sync + 'static,
{
    async fn handle_event(&self, event: &CoordinatorEvent<SeqTypes>) -> anyhow::Result<()> {
        // The ingester stores decided blocks, from the consensus store.
        if matches!(event, CoordinatorEvent::NewDecide { .. }) {
            return Ok(());
        }
        if let Err(height) = self.inner.update(event).await {
            bail!("failed to update API state after {height}: {event:?}",);
        }
        Ok(())
    }
}

#[async_trait]
impl<D> DecideSink for ApiEventConsumer<D>
where
    D: SequencerDataSource + UpdateAvailabilityData<SeqTypes> + Debug + Send + Sync + 'static,
{
    /// Stores the state certificate before the block, so that a block this node holds always
    /// has its certificate; storing either again is harmless.
    async fn append(
        &self,
        info: BlockInfo<SeqTypes>,
        state_cert: Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
    ) -> anyhow::Result<()> {
        if let Some(cert) = state_cert {
            self.inner.insert_state_cert(*cert.epoch, cert).await?;
        }
        self.inner.append(info).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        Ok(NodeDataSource::block_height(&*self.inner).await? as u64)
    }

    async fn last_leaf(&self) -> anyhow::Result<Option<LeafQueryData<SeqTypes>>> {
        let Some(last) = NodeDataSource::block_height(&*self.inner)
            .await?
            .checked_sub(1)
        else {
            return Ok(None);
        };
        match AvailabilityDataSource::get_leaf(&*self.inner, last)
            .await
            .try_resolve()
        {
            Ok(leaf) => Ok(Some(leaf)),
            Err(_) => bail!("leaf {last} is counted in the block height but not stored"),
        }
    }
}

/// A [`DecideSink`] as the ingester writes to it.
pub(crate) struct IngestSink(pub Arc<dyn DecideSink>);

/// Writes each block to the sink, then has the API service's [`ChainView`] take it in.
pub(crate) struct TrackedSink<P> {
    pub(crate) sink: Arc<dyn DecideSink>,
    pub(crate) view: Arc<ChainView<P>>,
}

#[async_trait]
impl<P: SequencerPersistence> ingester::Sink<SeqTypes> for TrackedSink<P> {
    async fn append(&self, block: ingester::Block<SeqTypes>) -> anyhow::Result<()> {
        let (info, state_cert) = block_info(block)?;
        let leaf = info.leaf.clone();
        self.sink.append(info, state_cert).await?;
        self.view.track(&leaf).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        self.sink.block_height().await
    }
}

#[async_trait]
impl ingester::Sink<SeqTypes> for IngestSink {
    async fn append(&self, block: ingester::Block<SeqTypes>) -> anyhow::Result<()> {
        let (info, state_cert) = block_info(block)?;
        self.0.append(info, state_cert).await
    }

    async fn block_height(&self) -> anyhow::Result<u64> {
        self.0.block_height().await
    }
}

/// What the query service stores for `block`, and the state certificate that goes with it.
fn block_info(
    block: ingester::Block<SeqTypes>,
) -> anyhow::Result<(
    BlockInfo<SeqTypes>,
    Option<LightClientStateUpdateCertificateV2<SeqTypes>>,
)> {
    let header = block.leaf.block_header().clone();
    let payload = block.payload.map(|bytes| {
        let payload = Payload::from_bytes(&bytes, header.metadata());
        BlockQueryData::new(header.clone(), payload)
    });
    let vid_common = block
        .vid_common
        .map(|common| VidCommonQueryData::new(header, common));
    let leaf = LeafQueryData::new(block.leaf, block.cert1)?;
    let mut info = BlockInfo::new(leaf, payload, vid_common, None);
    if let Some(cert2) = block.cert2 {
        info = info.with_cert2(cert2);
    }
    Ok((info, block.state_cert))
}
