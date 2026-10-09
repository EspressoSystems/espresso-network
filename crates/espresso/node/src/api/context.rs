//! [`ApiState`](super::ApiState) reaches consensus and node-wide state only through
//! [`ApiContext`], so the same API modules can be served by a validator ([`SequencerContext`])
//! or by a node that follows the chain without taking part in consensus.

use std::{collections::HashMap, future::Future, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    storage::SqliteStorage,
};
use anyhow::Context as _;
use async_lock::RwLock;
use async_trait::async_trait;
use committable::{Commitment, Committable};
use espresso_api::error::{ConsensusUnavailable, SubmitError};
use espresso_types::{
    Leaf2, NodeState, PubKey, SeqTypes, Transaction, ValidatedState,
    v0::traits::SequencerPersistence,
};
use futures::future::BoxFuture;
use hotshot_new_protocol::{
    block,
    client::{ClientApi, QueryError},
    state::UpdateLeaf,
};
use hotshot_query_service::availability::VidCommonQueryData;
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
    utils::StateAndDelta,
};
use tokio::sync::watch;

use crate::{
    SequencerContext, context::TaskList, coordinator_task::Status,
    state_signature::StateSignatureMemStorage,
};

pub type NodeLightClient = LightClient<SqliteStorage, FallbackClient<QueryServiceClient>>;

pub type Delta = <ValidatedState as hotshot_types::traits::ValidatedState<SeqTypes>>::Delta;

/// Reads fail with [`ConsensusUnavailable`] when consensus cannot answer, so the API can tell "not
/// running" apart from "no value" and report it as 503.
#[async_trait]
pub trait ConsensusSource: Send + Sync + 'static {
    async fn decided_leaf(&self) -> Result<Leaf2, ConsensusUnavailable>;
    async fn decided_state(&self) -> Result<Option<Arc<ValidatedState>>, ConsensusUnavailable>;
    async fn state(
        &self,
        view: ViewNumber,
    ) -> Result<Option<Arc<ValidatedState>>, ConsensusUnavailable>;
    async fn state_and_delta(
        &self,
        view: ViewNumber,
    ) -> Result<StateAndDelta<SeqTypes>, ConsensusUnavailable>;
    async fn undecided_leaves(&self) -> Result<Vec<Leaf2>, ConsensusUnavailable>;
    async fn current_epoch(&self) -> Result<Option<EpochNumber>, ConsensusUnavailable>;
    /// The commitment the accepting node reports, which the submit API returns.
    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<Commitment<Transaction>>;
    /// How catchup pushes a state it recovered from storage back into memory.
    async fn update_leaf(
        &self,
        leaf: Leaf2,
        state: Arc<ValidatedState>,
        delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()>;
    async fn current_proposal_participation(
        &self,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable>;
    async fn proposal_participation(
        &self,
        epoch: EpochNumber,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable>;
    async fn current_vote_participation(
        &self,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable>;
    async fn vote_participation(
        &self,
        epoch: EpochNumber,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable>;
}

pub trait ApiContext: Clone + Send + Sync + 'static {
    type Persistence: SequencerPersistence;

    fn consensus(&self) -> Arc<dyn ConsensusSource>;
    fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes>;
    fn upgrade_lock(&self) -> UpgradeLock<SeqTypes>;
    fn persistence(&self) -> Arc<Self::Persistence>;
    fn node_state(&self) -> NodeState;
    fn network_config(&self) -> NetworkConfig<SeqTypes>;
    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>>;
    fn state_signatures(&self) -> Option<Arc<RwLock<StateSignatureMemStorage>>>;

    /// A light client the node already runs, for the query service to fetch through.
    fn light_client(&self) -> Option<Arc<NodeLightClient>>;
    fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common: VidCommonQueryData<SeqTypes>,
        timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>>;
    /// [`Options::serve`](super::Options::serve) hands a clone of the context to the API before
    /// calling this, so the tasks attached here must be shared with that clone: [`TaskList`] is
    /// that shared handle, and dropping any clone of it aborts every task.
    fn with_task_list(self, tasks: TaskList) -> Self
    where
        Self: Sized;
}

/// A validator's consensus: the coordinator, read through its [`ClientApi`].
#[derive(Clone)]
pub(crate) struct CoordinatorConsensus {
    pub(crate) client_api: ClientApi<SeqTypes>,
    pub(crate) status: watch::Receiver<Status>,
}

impl CoordinatorConsensus {
    /// Fails at once while the coordinator is not running, instead of queueing a query behind a
    /// coordinator that has not started or will never answer.
    fn running(&self) -> Result<(), ConsensusUnavailable> {
        let status = *self.status.borrow();
        match status {
            Status::NotStarted => Err(ConsensusUnavailable::NotStarted),
            Status::Stopped => Err(ConsensusUnavailable::Stopped),
            Status::Running => Ok(()),
        }
    }

    async fn ask<A>(
        &self,
        query: impl Future<Output = Result<A, QueryError>>,
    ) -> Result<A, ConsensusUnavailable> {
        self.running()?;
        query
            .await
            .map_err(|err| ConsensusUnavailable::Failed(Box::new(err)))
    }
}

#[async_trait]
impl ConsensusSource for CoordinatorConsensus {
    async fn decided_leaf(&self) -> Result<Leaf2, ConsensusUnavailable> {
        self.ask(self.client_api.decided_leaf()).await
    }

    async fn decided_state(&self) -> Result<Option<Arc<ValidatedState>>, ConsensusUnavailable> {
        self.ask(self.client_api.decided_state()).await
    }

    async fn state(
        &self,
        view: ViewNumber,
    ) -> Result<Option<Arc<ValidatedState>>, ConsensusUnavailable> {
        self.ask(self.client_api.state(view)).await
    }

    async fn state_and_delta(
        &self,
        view: ViewNumber,
    ) -> Result<StateAndDelta<SeqTypes>, ConsensusUnavailable> {
        self.ask(self.client_api.state_and_delta(view)).await
    }

    async fn undecided_leaves(&self) -> Result<Vec<Leaf2>, ConsensusUnavailable> {
        self.ask(self.client_api.undecided_leaves()).await
    }

    async fn current_epoch(&self) -> Result<Option<EpochNumber>, ConsensusUnavailable> {
        self.ask(self.client_api.current_epoch()).await
    }

    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<Commitment<Transaction>> {
        let commitment = tx.commit();
        self.running()?;
        self.client_api
            .submit_transaction(tx)
            .await
            .map_err(|err| match err {
                QueryError::Rejected(rejection @ block::SubmitError::TooLarge { .. }) => {
                    SubmitError::Invalid(rejection.to_string()).into()
                },
                QueryError::Rejected(rejection @ block::SubmitError::RetryBufferFull) => {
                    SubmitError::Overloaded(rejection.to_string()).into()
                },
                err => anyhow::Error::new(ConsensusUnavailable::Failed(Box::new(err)))
                    .context("failed to submit transaction to the coordinator"),
            })?;
        Ok(commitment)
    }

    async fn update_leaf(
        &self,
        leaf: Leaf2,
        state: Arc<ValidatedState>,
        delta: Option<Arc<Delta>>,
    ) -> anyhow::Result<()> {
        let update = UpdateLeaf {
            view: leaf.view_number(),
            leaf,
            state,
            delta,
        };
        self.ask(self.client_api.update_leaf(update))
            .await
            .context("failed to update a leaf in the coordinator")
    }

    async fn current_proposal_participation(
        &self,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable> {
        self.ask(self.client_api.proposal_participation(None)).await
    }

    async fn proposal_participation(
        &self,
        epoch: EpochNumber,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable> {
        self.ask(self.client_api.proposal_participation(Some(epoch)))
            .await
    }

    async fn current_vote_participation(
        &self,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable> {
        self.ask(self.client_api.vote_participation(None)).await
    }

    async fn vote_participation(
        &self,
        epoch: EpochNumber,
    ) -> Result<HashMap<PubKey, f64>, ConsensusUnavailable> {
        self.ask(self.client_api.vote_participation(Some(epoch)))
            .await
    }
}

impl<P> ApiContext for SequencerContext<P>
where
    P: SequencerPersistence,
{
    type Persistence = P;

    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        Arc::new(SequencerContext::consensus(self))
    }

    fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes> {
        SequencerContext::membership_coordinator(self).clone()
    }

    fn upgrade_lock(&self) -> UpgradeLock<SeqTypes> {
        SequencerContext::upgrade_lock(self).clone()
    }

    fn persistence(&self) -> Arc<P> {
        SequencerContext::persistence(self)
    }

    fn node_state(&self) -> NodeState {
        SequencerContext::node_state(self)
    }

    fn network_config(&self) -> NetworkConfig<SeqTypes> {
        SequencerContext::network_config(self)
    }

    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>> {
        Some(SequencerContext::validator_config(self))
    }

    fn state_signatures(&self) -> Option<Arc<RwLock<StateSignatureMemStorage>>> {
        Some(SequencerContext::state_signatures(self))
    }

    fn light_client(&self) -> Option<Arc<NodeLightClient>> {
        None
    }

    fn request_vid_shares(
        &self,
        block_number: u64,
        vid_common: VidCommonQueryData<SeqTypes>,
        timeout: Duration,
    ) -> BoxFuture<'static, anyhow::Result<Vec<VidShare>>> {
        super::request_vid_shares(
            self.request_response_protocol.clone(),
            block_number,
            vid_common,
            timeout,
        )
    }

    fn with_task_list(self, tasks: TaskList) -> Self {
        SequencerContext::with_task_list(self, tasks)
    }
}
