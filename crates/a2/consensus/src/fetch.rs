use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
    time::Duration,
};

use async_trait::async_trait;
use consensus_store::Bytes;
use hotshot_types::{
    data::{VidCommitment2, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    traits::{
        BlockPayload,
        block_contents::{BlockHeader, EncodeBytes},
        node_implementation::NodeType,
    },
};
use tokio::task::{AbortHandle, JoinSet};
use tracing::{debug, error, warn};

use crate::{
    consensus::Consensus,
    vid::{ObtainedPayload, expected_vid_param, matches_commitment},
};

/// Where a node finds the payloads it is missing: its deployment's store, which acknowledged each
/// payload before the node voted vote2, and other deployments' stores.
#[async_trait]
pub trait PayloadSource: Send + Sync {
    /// The payload with VID commitment `commitment`, as consensus encodes payloads, if a store has
    /// it. It is checked against the commitment, so a source need not be trusted.
    async fn payload(&self, commitment: VidCommitment2) -> anyhow::Result<Option<Bytes>>;

    /// Whether the node's own deployment's store holds the payload with VID commitment
    /// `commitment` durably, as it holds every payload a storage instance acknowledged, whether
    /// or not it can serve it yet.
    async fn held(&self, commitment: VidCommitment2) -> anyhow::Result<bool>;
}

/// How long to wait before asking the source again for a payload it did not have. A fresh payload
/// reaches the stores shortly after the nodes that have it vote.
const RETRY_DELAY: Duration = Duration::from_millis(500);

/// Payload recovery: a payload missing from memory is asked of the [`PayloadSource`] until it
/// has it.
///
/// Asking stops once the payload is obtained, this way or any other, when the node's view times
/// out, or when a decide passes the payload's view. A timeout makes consensus ask again for the
/// payloads it still needs.
pub struct Fetcher<T: NodeType> {
    /// Where payloads are asked for, if anywhere.
    source: Option<Arc<dyn PayloadSource>>,

    /// The payloads being asked for, with the tasks that ask.
    asking: HashMap<(ViewNumber, VidCommitment2), AbortHandle>,

    /// The tasks that ask, each ending with its payload, checked against its commitment.
    tasks: JoinSet<ObtainedPayload<T>>,
}

impl<T: NodeType> Fetcher<T> {
    pub fn new() -> Self {
        Self {
            source: None,
            asking: HashMap::new(),
            tasks: JoinSet::new(),
        }
    }

    /// Asks `source` for each missing payload.
    pub fn with_source(mut self, source: Arc<dyn PayloadSource>) -> Self {
        self.source = Some(source);
        self
    }

    /// The next payload the source had.
    pub async fn next(&mut self) -> Option<ObtainedPayload<T>> {
        loop {
            match self.tasks.join_next().await? {
                Ok(payload) => return Some(payload),
                Err(err) if err.is_panic() => error!(%err, "asking for a payload panicked"),
                Err(_) => {},
            }
        }
    }

    /// Asks the source whether the node's own deployment's store holds each of `payloads`, by
    /// the view their block was proposed in and their commitment, until it answers for all of
    /// them, and yields those it does not hold. `None` without a source.
    pub fn unheld(
        &self,
        payloads: BTreeSet<(ViewNumber, VidCommitment2)>,
    ) -> Option<impl Future<Output = BTreeSet<(ViewNumber, VidCommitment2)>> + Send + 'static> {
        let Some(source) = self.source.clone() else {
            warn!("no source to ask whether the store holds the payloads of restored locks");
            return None;
        };
        Some(async move {
            let mut unheld = BTreeSet::new();
            for (view, commitment) in payloads {
                loop {
                    match source.held(commitment).await {
                        Ok(true) => break,
                        Ok(false) => {
                            unheld.insert((view, commitment));
                            break;
                        },
                        Err(err) => {
                            warn!(%view, "asking whether the store holds a payload failed: {err:#}")
                        },
                    }
                    tokio::time::sleep(RETRY_DELAY).await;
                }
            }
            unheld
        })
    }

    /// Asks the source for the payload with commitment `commitment` of the block proposed in
    /// `view` until it has it, unless it is being asked already.
    pub fn request(
        &mut self,
        view: ViewNumber,
        commitment: VidCommitment2,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) {
        if self.asking.contains_key(&(view, commitment)) {
            return;
        }
        let Some(source) = self.source.clone() else {
            debug!(%view, "no source to fetch the payload from");
            return;
        };
        let Some(check) = Check::of(view, commitment, consensus, membership) else {
            warn!(%view, "cannot check a fetched payload without its proposal and stake table");
            return;
        };
        let task = self.tasks.spawn(async move {
            loop {
                match source.payload(commitment).await {
                    Ok(Some(data)) => match check.run(data.to_vec()).await {
                        Some(payload) => return payload,
                        None => warn!(%view, "a payload from the source does not match"),
                    },
                    Ok(None) => debug!(%view, "no store has the payload yet"),
                    Err(err) => debug!(%view, "asking the source for a payload failed: {err:#}"),
                }
                tokio::time::sleep(RETRY_DELAY).await;
            }
        });
        self.asking.insert((view, commitment), task);
    }

    /// Stops asking for a payload this node now has.
    pub fn obtained(&mut self, view: ViewNumber, commitment: VidCommitment2) {
        if let Some(task) = self.asking.remove(&(view, commitment)) {
            task.abort();
        }
    }

    /// Stops asking for every payload, once the node's view timed out.
    pub fn view_timed_out(&mut self) {
        for (_, task) in self.asking.drain() {
            task.abort();
        }
    }

    /// Stops asking for the payloads of views before `view`, that of a block just decided.
    ///
    /// The decided block's own payload is still asked for: it is needed to vote on the block's
    /// children, and a decide can follow the request at once.
    pub fn gc(&mut self, view: ViewNumber) {
        self.asking.retain(|(v, _), task| {
            let keep = *v >= view;
            if !keep {
                task.abort();
            }
            keep
        });
    }
}

impl<T: NodeType> Default for Fetcher<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// What a payload of the block proposed in `view` must match, from the block's proposal.
#[derive(Clone)]
struct Check<T: NodeType> {
    view: ViewNumber,
    epoch: hotshot_types::data::EpochNumber,
    commitment: VidCommitment2,
    param: hotshot_types::vid::avidm_gf2::AvidmGf2Param,
    metadata: <T::BlockPayload as BlockPayload<T>>::Metadata,
}

impl<T: NodeType> Check<T> {
    /// The check of a payload with commitment `commitment`, if this node holds a proposal of it
    /// and the stake table of its epoch.
    fn of(
        view: ViewNumber,
        commitment: VidCommitment2,
        consensus: &Consensus<T>,
        membership: &EpochMembershipCoordinator<T>,
    ) -> Option<Self> {
        let proposal = consensus.proposal_with_payload(view, commitment)?;
        Some(Self {
            view,
            epoch: proposal.epoch,
            commitment,
            param: expected_vid_param(membership, proposal.epoch)?,
            metadata: proposal.block_header.metadata().clone(),
        })
    }

    /// The payload `data` encodes, if it matches the commitment.
    async fn run(&self, data: Vec<u8>) -> Option<ObtainedPayload<T>> {
        let this = self.clone();
        tokio::task::spawn_blocking(move || {
            if !matches_commitment(
                this.view,
                &this.param,
                &this.metadata.encode(),
                &data,
                &this.commitment,
            ) {
                return None;
            }
            let payload = T::BlockPayload::from_bytes(&data, &this.metadata);
            let tx_commitments = payload.transaction_commitments(&this.metadata);
            Some(ObtainedPayload {
                view: this.view,
                epoch: this.epoch,
                payload_commitment: this.commitment,
                payload,
                metadata: this.metadata,
                tx_commitments,
            })
        })
        .await
        .ok()
        .flatten()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    use async_trait::async_trait;
    use consensus_store::Bytes;
    use hotshot_example_types::{
        block_types::{TestBlockHeader, TestBlockPayload, TestMetadata, TestTransaction},
        node_types::{TEST_VERSIONS, TestTypes},
    };
    use hotshot_types::{
        data::{EpochNumber, Leaf2, VidCommitment, VidCommitment2, ViewNumber, ns_table},
        traits::block_contents::EncodeBytes,
        vid::avidm_gf2::AvidmGf2Scheme,
    };

    use super::{Fetcher, PayloadSource, RETRY_DELAY};
    use crate::{
        tests::common::utils::{ConsensusHarness, TestData},
        vid::expected_vid_param,
    };

    fn view() -> ViewNumber {
        ViewNumber::new(1)
    }

    /// A payload, and a proposal at view 1 that commits to it.
    ///
    /// The commitment is computed the way a disperser computes it, from the
    /// committee's erasure parameters, so `response` derives the same one from
    /// the proposal and the stake table.
    async fn payload_and_consensus() -> (Vec<u8>, VidCommitment2, ConsensusHarness) {
        let mut harness = ConsensusHarness::new(0).await;
        let test_data = TestData::new(1).await;

        let metadata = TestMetadata {
            num_transactions: 1,
        };
        let payload = TestBlockPayload {
            transactions: vec![TestTransaction::new(vec![1, 2, 3])],
        };
        let bytes = payload.encode().to_vec();

        let param = expected_vid_param(&harness.membership_coordinator, EpochNumber::genesis())
            .expect("committee resolves");
        let ns_table = ns_table::parse_ns_table(bytes.len(), &metadata.encode());
        let (commitment, _) = AvidmGf2Scheme::commit(&param, &bytes, ns_table).expect("commit");

        let template = &test_data.views[0];
        let parent_leaf: Leaf2<TestTypes> = template.proposal.data.clone().into();
        let mut proposal = template.proposal.data.clone();
        proposal.block_header = TestBlockHeader::new(
            &parent_leaf,
            VidCommitment::V2(commitment),
            template
                .proposal
                .data
                .block_header
                .builder_commitment
                .clone(),
            metadata,
            TEST_VERSIONS.test.base,
        );

        harness.force_set_proposal(view(), proposal);
        (bytes, commitment, harness)
    }

    /// A source that has a payload from its `misses + 1`th answer on, and counts the questions.
    struct Source {
        payload: (VidCommitment2, Vec<u8>),
        misses: usize,
        asked: AtomicUsize,
    }

    impl Source {
        fn new(commitment: VidCommitment2, data: Vec<u8>, misses: usize) -> Arc<Self> {
            Arc::new(Self {
                payload: (commitment, data),
                misses,
                asked: AtomicUsize::new(0),
            })
        }
    }

    #[async_trait]
    impl PayloadSource for Source {
        async fn payload(&self, commitment: VidCommitment2) -> anyhow::Result<Option<Bytes>> {
            let asked = self.asked.fetch_add(1, Ordering::Relaxed);
            let (held, data) = &self.payload;
            Ok((asked >= self.misses && *held == commitment).then(|| Bytes::from(data.clone())))
        }

        async fn held(&self, commitment: VidCommitment2) -> anyhow::Result<bool> {
            Ok(self.payload.0 == commitment)
        }
    }

    fn request(
        fetcher: &mut Fetcher<TestTypes>,
        commitment: VidCommitment2,
        harness: &ConsensusHarness,
    ) {
        fetcher.request(
            view(),
            commitment,
            &harness.consensus,
            &harness.membership_coordinator,
        );
    }

    /// The source is asked again until it has the payload, which is yielded once it re-commits.
    #[tokio::test]
    async fn the_source_is_asked_until_it_has_the_payload() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let source = Source::new(commitment, bytes, 2);
        let mut fetcher = Fetcher::new().with_source(source.clone());

        request(&mut fetcher, commitment, &harness);
        let obtained = fetcher.next().await.expect("the source has it in the end");
        assert_eq!(obtained.payload_commitment, commitment);
        assert_eq!(source.asked.load(Ordering::Relaxed), 3);
    }

    /// A payload that does not re-commit is not yielded, and the source is asked again.
    #[tokio::test]
    async fn a_payload_that_does_not_recommit_is_asked_again() {
        let (mut bytes, commitment, harness) = payload_and_consensus().await;
        bytes.push(0);
        let source = Source::new(commitment, bytes, 0);
        let mut fetcher = Fetcher::new().with_source(source.clone());

        request(&mut fetcher, commitment, &harness);
        let yielded = tokio::time::timeout(RETRY_DELAY * 3, fetcher.next()).await;
        assert!(yielded.is_err(), "nothing is yielded");
        assert!(source.asked.load(Ordering::Relaxed) >= 2);
    }

    /// A payload being asked for is not asked for again.
    #[tokio::test]
    async fn a_payload_is_asked_for_once() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let source = Source::new(commitment, bytes, 0);
        let mut fetcher = Fetcher::new().with_source(source.clone());

        request(&mut fetcher, commitment, &harness);
        request(&mut fetcher, commitment, &harness);
        assert!(fetcher.next().await.is_some());
        assert!(fetcher.next().await.is_none());
        assert_eq!(source.asked.load(Ordering::Relaxed), 1);
    }

    /// A timeout, a decide past the view, and the payload arriving otherwise each stop the asking,
    /// after which the payload can be asked for again.
    #[tokio::test]
    async fn asking_stops_on_a_timeout_a_decide_or_the_payload() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let stops: [fn(&mut Fetcher<TestTypes>, VidCommitment2); 3] = [
            |fetcher, _| fetcher.view_timed_out(),
            |fetcher, _| fetcher.gc(view() + 1),
            |fetcher, commitment| fetcher.obtained(view(), commitment),
        ];
        for stop in stops {
            let source = Source::new(commitment, bytes.clone(), usize::MAX);
            let mut fetcher = Fetcher::new().with_source(source);
            request(&mut fetcher, commitment, &harness);
            stop(&mut fetcher, commitment);
            assert!(fetcher.next().await.is_none());
            assert!(fetcher.asking.is_empty());

            request(&mut fetcher, commitment, &harness);
            assert_eq!(fetcher.asking.len(), 1);
        }
    }

    /// A decide keeps asking for the decided block's own payload, which its children still need.
    #[tokio::test]
    async fn a_decide_keeps_asking_for_its_own_payload() {
        let (bytes, commitment, harness) = payload_and_consensus().await;
        let source = Source::new(commitment, bytes, usize::MAX);
        let mut fetcher = Fetcher::new().with_source(source);

        request(&mut fetcher, commitment, &harness);
        fetcher.gc(view());
        assert!(fetcher.asking.contains_key(&(view(), commitment)));
    }
}
