//! The API services a node without consensus of its own reads the chain from.
//!
//! The set starts from the bootstrap URLs the operator configures and adds the API services of the
//! validators, which it learns from the discovery records an API service lists (`GetPeers`, also
//! served at `/v2/node/peers`). It checks each record against the stake table of the record's epoch, so the
//! service that lists the records need not be trusted. The bootstrap URLs stay in the set, first,
//! so a network that sends no records still leaves the node its configured services.

use std::{sync::Arc, time::Duration};

use ::light_client::client::{ClientList, QueryServiceClient};
use anyhow::{Context, bail};
use espresso_api::proto::{self, node_service_client::NodeServiceClient};
use espresso_types::{PubKey, SeqTypes};
use hotshot_new_protocol::discovery::{ServiceRecord, SignedServiceRecord};
use hotshot_query_service::availability::LeafQueryData;
use hotshot_types::{data::EpochNumber, epoch_membership::EpochMembershipCoordinator};
use parking_lot::RwLock;
use tagged_base64::TaggedBase64;
use tokio::sync::watch;
use tonic::transport::{ClientTlsConfig, Endpoint};
use url::Url;

/// How often the set asks for the records anew. Records change at most once per epoch.
const REFRESH_INTERVAL: Duration = Duration::from_secs(60);

/// How long one request for the records may take.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// The API services to read from, bootstrap URLs first.
#[derive(Debug)]
pub(crate) struct PeerSet {
    bootstrap: Vec<Url>,
    urls: RwLock<Arc<Vec<Url>>>,
    clients: ClientList<QueryServiceClient>,
}

impl PeerSet {
    pub(crate) fn new(bootstrap: Vec<Url>) -> Arc<Self> {
        let clients = ClientList::new(
            bootstrap
                .iter()
                .cloned()
                .map(QueryServiceClient::new)
                .collect(),
        );
        Arc::new(Self {
            urls: RwLock::new(Arc::new(bootstrap.clone())),
            bootstrap,
            clients,
        })
    }

    pub(crate) fn urls(&self) -> Arc<Vec<Url>> {
        self.urls.read().clone()
    }

    /// The clients of the set's services, which follow the set as it changes.
    pub(crate) fn clients(&self) -> ClientList<QueryServiceClient> {
        self.clients.clone()
    }

    /// Keeps the set current with the records of the epoch of the newest block in `decided`, the
    /// node's own chain, whose stake table `coordinator` holds. A node that has stored no block
    /// waits for one. Never returns.
    pub(crate) async fn run(
        self: Arc<Self>,
        coordinator: EpochMembershipCoordinator<SeqTypes>,
        decided: watch::Receiver<Option<LeafQueryData<SeqTypes>>>,
        epoch_height: u64,
    ) {
        loop {
            let epoch = decided
                .borrow()
                .as_ref()
                .and_then(|leaf| leaf.leaf().epoch(epoch_height));
            if let Some(epoch) = epoch
                && let Err(err) = self.refresh(&coordinator, epoch).await
            {
                tracing::warn!(%epoch, "refreshing the API services to read from failed: {err:#}");
            }
            tokio::time::sleep(REFRESH_INTERVAL).await;
        }
    }

    /// Asks the services the set holds for the records of `epoch`, in order, until one answers
    /// with records the node can check.
    async fn refresh(
        &self,
        coordinator: &EpochMembershipCoordinator<SeqTypes>,
        epoch: EpochNumber,
    ) -> anyhow::Result<()> {
        let membership = coordinator
            .membership_for_epoch(epoch)
            .map_err(|err| anyhow::anyhow!("the stake table of epoch {epoch}: {err}"))?;
        let mut last_err = anyhow::anyhow!("no API service to ask for records");
        for url in self.urls().iter() {
            match fetch_records(url, epoch).await {
                Ok(response) => {
                    let records =
                        verified_records(response, epoch, |signer| membership.has_stake(signer));
                    self.set_discovered(
                        records
                            .iter()
                            .filter_map(|record| record.record.api_url.parse().ok()),
                    );
                    return Ok(());
                },
                Err(err) => last_err = err.context(format!("asking {url}")),
            }
        }
        Err(last_err)
    }

    /// The bootstrap URLs, then those of `discovered` that are not among them. An unchanged set
    /// keeps its clients and their connections.
    fn set_discovered(&self, discovered: impl IntoIterator<Item = Url>) {
        let mut urls = self.bootstrap.clone();
        for url in discovered {
            if !urls.contains(&url) {
                urls.push(url);
            }
        }
        if **self.urls.read() == urls {
            return;
        }
        self.clients
            .replace(urls.iter().cloned().map(QueryServiceClient::new).collect());
        *self.urls.write() = Arc::new(urls);
    }
}

/// The record `signed` as `/v2/node/peers` lists it.
pub(crate) fn listed_record(signed: SignedServiceRecord<SeqTypes>) -> proto::ServiceRecord {
    proto::ServiceRecord {
        signer: Some(proto::BlsPublicKey {
            key: signed.signer.to_string(),
        }),
        epoch: *signed.record.epoch,
        signed_message: signed.record.signed_bytes(),
        api_url: signed.record.api_url,
        api_key: signed.record.api_key.map(|key| key.to_vec()),
        sequence: signed.record.sequence,
        signature: TaggedBase64::from(signed.signature).to_string(),
    }
}

/// Asks the API service at `url` for the records of `epoch`, over gRPC, which shares the port of
/// the HTTP API. Binary protobuf skips fields this node does not know, so a service on a newer
/// release still answers it.
async fn fetch_records(url: &Url, epoch: EpochNumber) -> anyhow::Result<proto::PeersResponse> {
    let mut endpoint = Endpoint::from_shared(url.to_string())?
        .connect_timeout(REQUEST_TIMEOUT)
        .timeout(REQUEST_TIMEOUT);
    if url.scheme() == "https" {
        endpoint = endpoint.tls_config(ClientTlsConfig::new().with_webpki_roots())?;
    }
    let mut client = NodeServiceClient::new(endpoint.connect().await?);
    let response = client
        .get_peers(proto::GetPeersRequest {
            epoch: Some(*epoch),
        })
        .await?;
    Ok(response.into_inner())
}

/// The records of `response` that are of `epoch`, validly signed, and from a validator that
/// `staked` says is staked in it. Each record stands on its own signature, so a bad one costs only
/// itself.
fn verified_records(
    response: proto::PeersResponse,
    epoch: EpochNumber,
    staked: impl Fn(&PubKey) -> bool,
) -> Vec<SignedServiceRecord<SeqTypes>> {
    let mut records = Vec::new();
    for listed in response.peers {
        let signed = match decode(listed) {
            Ok(signed) => signed,
            Err(err) => {
                tracing::warn!("dropping an undecodable record: {err:#}");
                continue;
            },
        };
        if signed.record.epoch != epoch || !signed.is_valid() || !staked(&signed.signer) {
            tracing::warn!(
                signer = %signed.signer,
                record_epoch = %signed.record.epoch,
                %epoch,
                "dropping a record of another epoch, an invalid signature or an unstaked signer"
            );
            continue;
        }
        records.push(signed);
    }
    records
}

fn decode(listed: proto::ServiceRecord) -> anyhow::Result<SignedServiceRecord<SeqTypes>> {
    let Some(signer) = listed.signer else {
        bail!("a record without a signer");
    };
    let signer: PubKey = TaggedBase64::parse(&signer.key)
        .context("the signer of a record")?
        .try_into()
        .map_err(|err| anyhow::anyhow!("the signer of a record: {err}"))?;
    let signature = TaggedBase64::parse(&listed.signature)
        .context("the signature of a record")?
        .try_into()
        .map_err(|err| anyhow::anyhow!("the signature of a record: {err}"))?;
    let api_key = listed
        .api_key
        .map(|key| key.try_into())
        .transpose()
        .map_err(|_| anyhow::anyhow!("an API key of a record that is not 32 bytes"))?;
    Ok(SignedServiceRecord {
        record: ServiceRecord {
            epoch: EpochNumber::new(listed.epoch),
            api_url: listed.api_url,
            api_key,
            sequence: listed.sequence,
        },
        signer,
        signature,
    })
}

#[cfg(test)]
mod tests {
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey};

    use super::*;

    fn signed(index: u64, epoch: u64, url: &str) -> SignedServiceRecord<SeqTypes> {
        let (public, private) = BLSPubKey::generated_from_seed_indexed([0; 32], index);
        ServiceRecord {
            epoch: EpochNumber::new(epoch),
            api_url: url.into(),
            api_key: Some([index as u8; 32]),
            sequence: 1,
        }
        .sign::<SeqTypes>(public, &private)
        .unwrap()
    }

    fn listed(epoch: u64, records: Vec<SignedServiceRecord<SeqTypes>>) -> proto::PeersResponse {
        proto::PeersResponse {
            peers: records.into_iter().map(listed_record).collect(),
            epoch,
        }
    }

    fn everyone_staked(_: &PubKey) -> bool {
        true
    }

    const EPOCH: EpochNumber = EpochNumber::new(5);

    #[test]
    fn a_listed_record_decodes_to_itself() {
        let records = vec![signed(0, 5, "http://a/"), signed(1, 5, "http://b/")];
        let decoded = verified_records(listed(5, records.clone()), EPOCH, everyone_staked);
        assert_eq!(decoded, records);
    }

    #[test]
    fn drops_a_record_of_another_epoch() {
        let kept = signed(0, 5, "http://a/");
        let response = listed(5, vec![kept.clone(), signed(1, 4, "http://b/")]);
        assert_eq!(
            verified_records(response, EPOCH, everyone_staked),
            vec![kept]
        );
    }

    #[test]
    fn drops_a_record_its_signer_did_not_sign() {
        let kept = signed(1, 5, "http://b/");
        let mut response = listed(5, vec![signed(0, 5, "http://a/"), kept.clone()]);
        response.peers[0].api_url = "http://evil/".into();
        assert_eq!(
            verified_records(response, EPOCH, everyone_staked),
            vec![kept]
        );
    }

    #[test]
    fn drops_a_record_of_an_unstaked_signer() {
        let kept = signed(0, 5, "http://a/");
        let response = listed(5, vec![kept.clone(), signed(1, 5, "http://b/")]);
        let staked = |key: &PubKey| *key == kept.signer;
        assert_eq!(verified_records(response, EPOCH, staked), vec![kept]);
    }

    #[test]
    fn keeps_the_bootstrap_urls_first() {
        let bootstrap: Url = "http://boot/".parse().unwrap();
        let peers = PeerSet::new(vec![bootstrap.clone()]);
        let a: Url = "http://a/".parse().unwrap();
        peers.set_discovered([a.clone(), bootstrap.clone(), a.clone()]);
        assert_eq!(*peers.urls(), vec![bootstrap.clone(), a]);
        peers.set_discovered([]);
        assert_eq!(*peers.urls(), vec![bootstrap]);
    }
}
