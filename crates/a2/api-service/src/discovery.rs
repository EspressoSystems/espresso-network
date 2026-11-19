//! The discovery service: the API services of the validators, as their signed records name them.

use std::net::SocketAddr;

use anyhow::Context;
use consensus::discovery::{ServiceRecord, SignedServiceRecord};
use consensus_api::{DecideLogClient, proto::ServiceRecordsRequest};
use espresso_types::SeqTypes;
use hotshot_types::data::EpochNumber;
use tagged_base64::TaggedBase64;
use tonic::{Code, Request, Response, Status};

use crate::{
    ApiService,
    proto::{self, discovery_service_server::DiscoveryService},
};

#[tonic::async_trait]
impl DiscoveryService for ApiService {
    /// The records consensus holds for the epoch, as it received them.
    async fn get_peers(
        &self,
        request: Request<proto::GetPeersRequest>,
    ) -> Result<Response<proto::PeersResponse>, Status> {
        let (epoch, records) =
            service_records(self.consensus_api, request.into_inner().epoch).await?;
        Ok(Response::new(proto::PeersResponse {
            peers: records.into_iter().map(listed).collect(),
            epoch,
        }))
    }
}

/// The valid records consensus at `consensus_api` holds for `epoch`, or for the latest epoch with
/// records, with that epoch.
pub(crate) async fn service_records(
    consensus_api: SocketAddr,
    epoch: Option<u64>,
) -> Result<(u64, Vec<SignedServiceRecord<SeqTypes>>), Status> {
    let mut client = DecideLogClient::connect(format!("http://{consensus_api}"))
        .await
        .map_err(|err| Status::unavailable(format!("cannot reach consensus: {err}")))?;
    let response = client
        .get_service_records(ServiceRecordsRequest { epoch })
        .await
        .map_err(|status| match status.code() {
            Code::NotFound => Status::not_found("no service records for the epoch"),
            _ => Status::unavailable(format!("asking consensus for records: {status}")),
        })?
        .into_inner();
    let records = response
        .records
        .iter()
        .filter_map(|bytes| {
            bincode::deserialize::<SignedServiceRecord<SeqTypes>>(bytes)
                .inspect_err(|err| tracing::warn!(%err, "undecodable service record"))
                .ok()
        })
        .filter(SignedServiceRecord::is_valid)
        .collect();
    Ok((response.epoch, records))
}

/// The record `listed` describes, as [`listed`] lists it; its signature is not checked here.
pub(crate) fn from_listed(
    listed: proto::ServiceRecord,
) -> anyhow::Result<SignedServiceRecord<SeqTypes>> {
    let api_key = listed
        .api_key
        .map(|key| <[u8; 32]>::try_from(key.as_slice()))
        .transpose()
        .context("an API key that is not 32 bytes")?;
    Ok(SignedServiceRecord {
        record: ServiceRecord {
            epoch: EpochNumber::new(listed.epoch),
            api_url: listed.api_url,
            api_key,
            sequence: listed.sequence,
        },
        signer: listed.signer.parse().context("the signer")?,
        signature: listed.signature.parse().context("the signature")?,
    })
}

/// `signed` as clients receive it; they check the signature against the stake table of the
/// record's epoch themselves.
pub(crate) fn listed(signed: SignedServiceRecord<SeqTypes>) -> proto::ServiceRecord {
    proto::ServiceRecord {
        signer: signed.signer.to_string(),
        epoch: *signed.record.epoch,
        signed_message: signed.record.signed_bytes(),
        api_url: signed.record.api_url,
        api_key: signed.record.api_key.map(|key| key.to_vec()),
        sequence: signed.record.sequence,
        signature: TaggedBase64::from(signed.signature).to_string(),
    }
}
