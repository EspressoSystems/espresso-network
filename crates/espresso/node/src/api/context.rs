//! [`ApiState`](super::ApiState) reaches consensus and node-wide state only through
//! [`ApiContext`], so the same API modules can be served by a validator ([`ValidatorApi`]) or by
//! a node that follows the chain without taking part in consensus.

use std::{collections::HashMap, sync::Arc, time::Duration};

use ::light_client::{
    LightClient,
    client::{FallbackClient, QueryServiceClient},
    storage::SqlStorage,
};
use async_lock::RwLock;
use async_trait::async_trait;
use committable::Commitment;
use espresso_types::{
    Leaf2, NodeState, PubKey, SeqTypes, Transaction, ValidatedState,
    v0::traits::{MembershipPersistence, SequencerPersistence, StateCertPersistence},
};
use futures::future::BoxFuture;
use hotshot_query_service::availability::VidCommonQueryData;
use hotshot_types::{
    ValidatorConfig,
    data::{EpochNumber, VidShare},
    epoch_membership::EpochMembershipCoordinator,
    message::UpgradeLock,
    network::NetworkConfig,
};

use crate::{
    SequencerContext,
    api::update::{DecideSink, IngestSink, TrackedSink},
    chain_view::ChainView,
    context::TaskList,
    follower::FollowerHandle,
    state_signature::StateSignatureMemStorage,
};

pub type NodeLightClient = LightClient<SqlStorage, FallbackClient<QueryServiceClient>>;

pub type Delta = <ValidatedState as hotshot_types::traits::ValidatedState<SeqTypes>>::Delta;

#[async_trait]
pub trait ConsensusSource: Send + Sync + 'static {
    async fn decided_leaf(&self) -> Leaf2;
    async fn decided_state(&self) -> Option<Arc<ValidatedState>>;
    async fn current_epoch(&self) -> Option<EpochNumber>;
    async fn membership_coordinator(&self) -> EpochMembershipCoordinator<SeqTypes>;
    async fn upgrade_lock(&self) -> UpgradeLock<SeqTypes>;
    /// The commitment the accepting node reports, which the submit API returns.
    async fn submit_transaction(&self, tx: Transaction) -> anyhow::Result<Commitment<Transaction>>;
    async fn current_proposal_participation(&self) -> HashMap<PubKey, f64>;
    async fn proposal_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64>;
    async fn current_vote_participation(&self) -> HashMap<PubKey, f64>;
    async fn vote_participation(&self, epoch: EpochNumber) -> HashMap<PubKey, f64>;
}

pub trait ApiContext: Send + Sync + 'static {
    fn consensus(&self) -> Arc<dyn ConsensusSource>;
    fn membership(&self) -> Arc<dyn MembershipPersistence>;
    fn state_certs(&self) -> Arc<dyn StateCertPersistence>;
    fn node_state(&self) -> NodeState;
    fn network_config(&self) -> NetworkConfig<SeqTypes>;
    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>>;
    fn state_signatures(&self) -> Option<Arc<RwLock<StateSignatureMemStorage>>>;

    /// Where this node serves the ingester API, if it runs consensus.
    fn consensus_api_addr(&self) -> Option<std::net::SocketAddr> {
        None
    }

    /// What the ingester writes the blocks of this node's consensus to: `sink`, followed by
    /// whatever else the API service keeps up with the blocks it stores.
    fn ingest_sink(&self, sink: Arc<dyn DecideSink>) -> Arc<dyn ingester::Sink<SeqTypes>> {
        Arc::new(IngestSink(sink))
    }

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

/// The API context of a validator: the API service's own view of the chain, which it keeps up
/// with the blocks it ingests from this node's consensus, and the few things only consensus has.
#[derive(Clone)]
pub(crate) struct ValidatorApi<P: SequencerPersistence> {
    pub(crate) node: SequencerContext<P>,
    pub(crate) api: FollowerHandle<P>,
    pub(crate) view: Arc<ChainView<P>>,
}

#[cfg(any(test, feature = "testing"))]
impl<P: SequencerPersistence> ValidatorApi<P> {
    /// The API service of a node that starts at genesis, sharing the node's storage.
    pub(crate) async fn at_genesis(
        node: SequencerContext<P>,
        upgrade: versions::Upgrade,
    ) -> anyhow::Result<Self> {
        let (api, view) = crate::follower::api_service(
            node.node_state(),
            node.persistence(),
            node.network_config(),
            upgrade,
            EpochNumber::genesis(),
            crate::follower::SubmitTo::Consensus(node.consensus_api_addr()),
        )
        .await?;
        Ok(Self { node, api, view })
    }
}

impl<P: SequencerPersistence> ApiContext for ValidatorApi<P> {
    fn consensus(&self) -> Arc<dyn ConsensusSource> {
        self.api.consensus()
    }

    fn membership(&self) -> Arc<dyn MembershipPersistence> {
        self.api.membership()
    }

    fn state_certs(&self) -> Arc<dyn StateCertPersistence> {
        self.api.state_certs()
    }

    fn node_state(&self) -> NodeState {
        self.api.node_state()
    }

    fn network_config(&self) -> NetworkConfig<SeqTypes> {
        self.api.network_config()
    }

    fn validator_config(&self) -> Option<&ValidatorConfig<SeqTypes>> {
        Some(self.node.validator_config())
    }

    fn state_signatures(&self) -> Option<Arc<RwLock<StateSignatureMemStorage>>> {
        Some(self.node.state_signatures())
    }

    fn consensus_api_addr(&self) -> Option<std::net::SocketAddr> {
        Some(self.node.consensus_api_addr())
    }

    fn ingest_sink(&self, sink: Arc<dyn DecideSink>) -> Arc<dyn ingester::Sink<SeqTypes>> {
        Arc::new(TrackedSink {
            sink,
            view: self.view.clone(),
        })
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
            self.node.request_response_protocol.clone(),
            block_number,
            vid_common,
            timeout,
        )
    }

    fn with_task_list(self, tasks: TaskList) -> Self {
        Self {
            node: self.node.with_task_list(tasks),
            ..self
        }
    }
}
