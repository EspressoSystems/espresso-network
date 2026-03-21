use std::{
    collections::{BTreeMap, HashMap},
    marker::PhantomData,
};

use hotshot_types::{
    data::ViewNumber,
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    simple_vote::HasEpoch,
    traits::node_implementation::NodeType,
    vote::{Certificate, Vote, VoteAccumulator},
};
use tokio::{
    spawn,
    sync::mpsc::{self},
};

pub(super) struct VoteCollectionTask<TYPES: NodeType, V, C> {
    per_view: BTreeMap<ViewNumber, mpsc::Sender<V>>,
    rx: mpsc::Receiver<V>,
    epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
    upgrade_lock: UpgradeLock<TYPES>,
    internal_tx: mpsc::Sender<(ViewNumber, C)>,
    internal_rx: mpsc::Receiver<(ViewNumber, C)>,
}

impl<TYPES, V, C> VoteCollectionTask<TYPES, V, C>
where
    TYPES: NodeType,
    V: Vote<TYPES> + HasEpoch + Send + Sync + 'static,
    C: Certificate<TYPES, V::Commitment, Voteable = V::Commitment> + Send + Sync + 'static,
{
    pub fn new(
        rx: mpsc::Receiver<V>,
        epoch_membership_coordinator: EpochMembershipCoordinator<TYPES>,
        upgrade_lock: UpgradeLock<TYPES>,
    ) -> Self {
        let (internal_tx, internal_rx) = mpsc::channel(100);
        Self {
            per_view: BTreeMap::new(),
            rx,
            epoch_membership_coordinator,
            upgrade_lock,
            internal_tx,
            internal_rx,
        }
    }

    pub async fn run(mut self, on_cert: impl Fn(C) -> CertFut + Send + 'static) {
        loop {
            tokio::select! {
                Some(vote) = self.rx.recv() => {
                    let view = vote.view_number();
                    let tx = self.per_view.entry(view).or_insert_with(|| {
                        let (tx, rx) = mpsc::channel(100);
                        let accumulator = VoteAccumulator {
                            vote_outcomes: HashMap::new(),
                            signers: HashMap::new(),
                            phantom: PhantomData,
                            upgrade_lock: self.upgrade_lock.clone(),
                        };
                        let membership_coordinator = self.epoch_membership_coordinator.clone();
                        let internal_tx = self.internal_tx.clone();
                        spawn(Self::run_per_view(view, rx, accumulator, membership_coordinator, internal_tx));
                        tx
                    });
                    let _ = tx.send(vote).await;
                },
                Some((view, cert)) = self.internal_rx.recv() => {
                    self.per_view.remove(&view);
                    on_cert(cert).await;
                },
                else => break,
            }
        }
    }

    async fn run_per_view(
        view: ViewNumber,
        mut rx: mpsc::Receiver<V>,
        mut accumulator: VoteAccumulator<TYPES, V, C>,
        membership_coordinator: EpochMembershipCoordinator<TYPES>,
        internal_tx: mpsc::Sender<(ViewNumber, C)>,
    ) {
        while let Some(vote) = rx.recv().await {
            let epoch = vote.epoch();
            let Ok(membership) = membership_coordinator.membership_for_epoch(epoch).await else {
                continue;
            };
            if let Some(cert) = accumulator.accumulate(&vote, membership).await {
                let _ = internal_tx.send((view, cert)).await;
                return;
            }
        }
    }
}

pub(super) type CertFut = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;
