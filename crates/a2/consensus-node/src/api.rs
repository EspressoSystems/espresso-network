//! The consensus node's API, for its API service: the decide log for the ingester, and the node
//! service for submissions. It listens only on the private subnet.

use std::{net::SocketAddr, sync::Arc};

use async_trait::async_trait;
use bytes::Bytes;
use consensus::{
    block::SubmitError as BlockSubmitError,
    client::{ClientApi, QueryError},
    ingest::{IngestSource, StakeTables},
};
use consensus_api::{Node, SubmitError};
use espresso_types::SeqTypes;
use hotshot_types::{
    data::EpochNumber,
    epoch_membership::EpochMembershipCoordinator,
    stake_table::HSStakeTable,
    traits::election::{Membership as _, MembershipSnapshot as _},
};

use crate::store::NodeStore;

/// Binds the consensus node's API to `addr`: the decide log over `store`, with the stake tables
/// of `membership`, for the API service's store, and the node service over the coordinator behind
/// `client` for the API service. Returns the address
/// it is bound to, which differs from `addr` for port 0, with the server, which runs until it is
/// dropped or fails.
pub fn bind(
    store: NodeStore,
    epoch_height: u64,
    membership: EpochMembershipCoordinator<SeqTypes>,
    client: ClientApi<SeqTypes>,
    addr: SocketAddr,
) -> std::io::Result<(
    SocketAddr,
    impl Future<Output = Result<(), tonic::transport::Error>> + Send,
)> {
    // Without TCP_NODELAY a small answer waits for the client's delayed acknowledgement.
    let incoming = tonic::transport::server::TcpIncoming::bind(addr)?.with_nodelay(Some(true));
    let bound = incoming.local_addr()?;
    let server = tonic::transport::Server::builder()
        .add_service(consensus_api::server(IngestSource::new(
            store,
            epoch_height,
            Arc::new(MembershipStakeTables { membership }),
        )))
        .add_service(consensus_api::node_server(Arc::new(NodeService { client })))
        .serve_with_incoming(incoming);
    Ok((bound, server))
}

/// The stake tables of the node's membership, each as the list of its validators.
struct MembershipStakeTables {
    membership: EpochMembershipCoordinator<SeqTypes>,
}

#[async_trait]
impl StakeTables for MembershipStakeTables {
    async fn stake_table(&self, epoch: EpochNumber) -> anyhow::Result<Option<Bytes>> {
        let Some(snapshot) = self.membership.membership().snapshot(epoch) else {
            return Ok(None);
        };
        let table = HSStakeTable::from_iter(snapshot.stake_table());
        Ok(Some(bincode::serialize(&table.0)?.into()))
    }
}

/// The node service: what only consensus can do for the API service.
struct NodeService {
    client: ClientApi<SeqTypes>,
}

#[async_trait]
impl Node for NodeService {
    async fn submit_transaction(&self, transaction: Bytes) -> Result<(), SubmitError> {
        let transaction = bincode::deserialize(&transaction)
            .map_err(|err| SubmitError::Invalid(format!("undecodable transaction: {err}")))?;
        match self.client.submit_transaction(transaction).await {
            Ok(()) => Ok(()),
            Err(QueryError::Rejected(rejection @ BlockSubmitError::TooLarge { .. })) => {
                Err(SubmitError::Invalid(rejection.to_string()))
            },
            Err(QueryError::Rejected(rejection @ BlockSubmitError::RetryBufferFull)) => {
                Err(SubmitError::Overloaded(rejection.to_string()))
            },
            Err(err) => Err(SubmitError::Internal(err.into())),
        }
    }
}
