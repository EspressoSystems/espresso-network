use std::{sync::Arc, time::Duration};

use anyhow::Context;
use async_channel::{Receiver, Sender};
use async_lock::RwLock;
use clap::Parser;
use committable::Commitment;
use derivative::Derivative;
use espresso_types::{parse_duration, v0::traits::SequencerPersistence, PubKey, ValidatedState};
use futures::stream::StreamExt;
use hotshot::types::EventType;
use hotshot_types::{
    data::{Leaf2, ViewNumber},
    traits::{
        metrics::{Counter, Gauge, Metrics},
        network::ConnectedNetwork,
        node_implementation::{ConsensusTime, Versions},
        ValidatedState as _,
    },
    utils::{View, ViewInner},
};
use tokio::time::{sleep, timeout};
use tracing::Instrument;

use crate::{
    context::{Consensus, TaskList},
    SeqTypes,
};

#[derive(Clone, Copy, Debug, Parser)]
pub struct ProposalFetcherConfig {
    #[arg(
        long = "proposal-fetcher-num-workers",
        env = "ESPRESSO_SEQUENCER_PROPOSAL_FETCHER_NUM_WORKERS",
        default_value = "2"
    )]
    pub num_workers: usize,

    #[arg(
        long = "proposal-fetcher-fetch-timeout",
        env = "ESPRESSO_SEQUENCER_PROPOSAL_FETCHER_FETCH_TIMEOUT",
        default_value = "2s",
        value_parser = parse_duration,
    )]
    pub fetch_timeout: Duration,
}

impl Default for ProposalFetcherConfig {
    fn default() -> Self {
        Self::parse_from(std::iter::empty::<String>())
    }
}

impl ProposalFetcherConfig {
    pub(crate) fn spawn<N, P, V>(
        self,
        tasks: &mut TaskList,
        consensus: Arc<RwLock<Consensus<N, P, V>>>,
        persistence: Arc<P>,
        metrics: &(impl Metrics + ?Sized),
    ) where
        N: ConnectedNetwork<PubKey>,
        P: SequencerPersistence,
        V: Versions,
    {
        let (sender, receiver) = async_channel::unbounded();
        let fetcher = ProposalFetcher {
            sender,
            consensus,
            persistence,
            cfg: self,
            metrics: ProposalFetcherMetrics::new(metrics),
        };

        tasks.spawn("proposal scanner", fetcher.clone().scan());
        for i in 0..self.num_workers {
            tasks.spawn(
                format!("proposal fetcher {i}"),
                fetcher.clone().fetch(receiver.clone()),
            );
        }
    }
}

#[derive(Clone, Debug)]
struct ProposalFetcherMetrics {
    fetched: Arc<dyn Counter>,
    failed: Arc<dyn Counter>,
    queue_len: Arc<dyn Gauge>,
    last_seen: Arc<dyn Gauge>,
    last_fetched: Arc<dyn Gauge>,
}

impl ProposalFetcherMetrics {
    fn new(metrics: &(impl Metrics + ?Sized)) -> Self {
        let metrics = metrics.subgroup("proposal_fetcher".into());
        Self {
            fetched: metrics.create_counter("fetched".into(), None).into(),
            failed: metrics.create_counter("failed".into(), None).into(),
            queue_len: metrics.create_gauge("queue_len".into(), None).into(),
            last_seen: metrics
                .create_gauge("last_seen".into(), Some("view".into()))
                .into(),
            last_fetched: metrics
                .create_gauge("last_fetched".into(), Some("view".into()))
                .into(),
        }
    }
}

type Request = (ViewNumber, Commitment<Leaf2<SeqTypes>>);

#[derive(Derivative)]
#[derivative(Clone(bound = ""), Debug(bound = ""))]
struct ProposalFetcher<N, P, V>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    V: Versions,
{
    sender: Sender<Request>,
    #[derivative(Debug = "ignore")]
    consensus: Arc<RwLock<Consensus<N, P, V>>>,
    #[derivative(Debug = "ignore")]
    persistence: Arc<P>,
    cfg: ProposalFetcherConfig,
    metrics: ProposalFetcherMetrics,
}

impl<N, P, V> ProposalFetcher<N, P, V>
where
    N: ConnectedNetwork<PubKey>,
    P: SequencerPersistence,
    V: Versions,
{
    #[tracing::instrument(skip_all)]
    async fn scan(self) {
        let mut events = self.consensus.read().await.event_stream();
        while let Some(event) = events.next().await {
            let EventType::QuorumProposal { proposal, .. } = event.event else {
                continue;
            };
            // Whenever we see a quorum proposal, ensure we have the chain of proposals stretching back
            // to the anchor. This allows state replay from the decided state.
            let parent_view = proposal.data.justify_qc().view_number;
            let parent_leaf = proposal.data.justify_qc().data.leaf_commit;
            self.request((parent_view, parent_leaf)).await;
        }
    }

    #[tracing::instrument(skip_all)]
    async fn fetch(self, receiver: Receiver<(ViewNumber, Commitment<Leaf2<SeqTypes>>)>) {
        let mut receiver = std::pin::pin!(receiver);
        while let Some(req) = receiver.next().await {
            self.fetch_request(req).await;
        }
    }

    async fn request(&self, req: Request) {
        self.sender.send(req).await.ok();
        self.metrics.queue_len.set(self.sender.len());
        self.metrics.last_seen.set(req.0.u64() as usize);
    }

    async fn fetch_request(&self, (view, leaf): Request) {
        let span = tracing::warn_span!("fetch proposal", ?view, %leaf);
        let res: anyhow::Result<()> = async {
            let anchor_view = self
                .persistence
                .load_anchor_view()
                .await
                .context("loading anchor view")?;
            if view <= anchor_view {
                tracing::debug!(?anchor_view, "skipping already-decided proposal");
                return Ok(());
            }

            match self.persistence.load_quorum_proposal(view).await {
                Ok(proposal) => {
                    // If we already have the proposal in storage, keep traversing the chain to its
                    // parent.
                    let view = proposal.data.justify_qc().view_number;
                    let leaf = proposal.data.justify_qc().data.leaf_commit;
                    self.request((view, leaf)).await;
                    return Ok(());
                },
                Err(err) => {
                    tracing::info!("proposal missing from storage; fetching from network: {err:#}");
                },
            }

            let future = self.consensus.read().await.request_proposal(view, leaf)?;
            let proposal = timeout(self.cfg.fetch_timeout, future)
                .await
                .context("timed out fetching proposal")?
                .context("error fetching proposal")?;
            self.persistence
                .append_quorum_proposal2(&proposal)
                .await
                .context("error saving fetched proposal")?;

            // Add the fetched leaf to HotShot state, so consensus can make use of it.
            let leaf = Leaf2::from_quorum_proposal(&proposal.data);
            let handle = self.consensus.read().await;
            let consensus = handle.consensus();
            let mut consensus = consensus.write().await;
            if matches!(
                consensus.validated_state_map().get(&view),
                None | Some(View {
                    // Replace a Da-only view with a Leaf view, which has strictly more information.
                    view_inner: ViewInner::Da { .. }
                })
            ) {
                let state = Arc::new(ValidatedState::from_header(leaf.block_header()));
                if let Err(err) = consensus.update_leaf(leaf, state, None) {
                    tracing::warn!("unable to update leaf: {err:#}");
                }
            }

            self.metrics.last_fetched.set(view.u64() as usize);
            self.metrics.fetched.add(1);

            Ok(())
        }
        .instrument(span)
        .await;
        if let Err(err) = res {
            tracing::warn!("failed to fetch proposal: {err:#}");
            self.metrics.failed.add(1);

            // Avoid busy loop when operations are failing.
            sleep(Duration::from_secs(1)).await;

            // If we fail fetching the proposal, don't let it clog up the fetching task. Just push
            // it back onto the queue and move onto the next proposal.
            self.request((view, leaf)).await;
        }
    }
}
