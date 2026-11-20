//! The submit service: hands transactions to the consensus node.

use committable::Committable;
use consensus_api::{ConsensusNodeClient, proto::SubmitTransactionRequest as ConsensusRequest};
use espresso_api::proto::{self, submit_service_server::SubmitService};
use espresso_types::Transaction;
use tonic::{Code, Request, Response, Status};

use crate::{
    ApiService,
    request::{namespace, required},
};

#[tonic::async_trait]
impl SubmitService for ApiService {
    async fn submit_transaction(
        &self,
        request: Request<proto::SubmitTransactionRequest>,
    ) -> Result<Response<proto::SubmitTransactionResponse>, Status> {
        let r = request.into_inner();
        let transaction =
            Transaction::new(namespace(r.namespace)?, required(r.payload, "payload")?);
        let hash = transaction.commit();
        let mut client = ConsensusNodeClient::new(self.consensus.clone());
        let encoded = bincode::serialize(&transaction)
            .map_err(|err| Status::internal(format!("encoding the transaction: {err}")))?;
        client
            .submit_transaction(ConsensusRequest {
                transaction: encoded,
            })
            .await
            .map_err(|status| match status.code() {
                // Consensus refused it or has no room: the client may act on either.
                Code::InvalidArgument | Code::ResourceExhausted => status,
                _ => Status::unavailable(format!("submitting to consensus: {status}")),
            })?;
        Ok(Response::new(proto::SubmitTransactionResponse {
            hash: hash.to_string(),
        }))
    }
}
