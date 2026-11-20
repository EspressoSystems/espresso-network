//! The API services a gap filler asks: those the validators of the current epoch name in their
//! signed service records.
//!
//! Records change only when the epoch does, so the cache is refreshed once per epoch. It asks the
//! API service's own consensus node, which collects the records, and while that has none, the
//! bootstrap API services, which list the records their consensus nodes hold. A record counts only
//! if it is for the epoch, is signed by its signer, and its signer is staked in the epoch, whoever
//! sent it. Until the cache holds records, the bootstrap URLs are the peers. The API service's own
//! URL is never a peer.

use std::{fmt, net::SocketAddr};

use consensus::discovery::SignedServiceRecord;
use espresso_types::SeqTypes;
use hotshot_types::data::EpochNumber;
use rand::seq::SliceRandom;
use tonic::transport::Channel;
use url::Url;

use crate::{
    Membership,
    discovery::{consensus_channel, from_listed, service_records},
    gaps::endpoint,
    proto::{GetPeersRequest, discovery_service_client::DiscoveryServiceClient},
};

pub struct Peers {
    /// The channel to the API service's consensus node.
    consensus: Channel,

    bootstrap: Vec<Peer>,

    /// The URL other services reach this API service at, left out of the peers.
    own: Option<Url>,

    /// The peers the records of `epoch` name.
    peers: Vec<Peer>,
    epoch: Option<EpochNumber>,
}

/// An API service to ask, with a channel to it that connects on first use and reconnects when
/// the connection fails, so requests share one connection.
#[derive(Clone, Debug)]
pub struct Peer {
    pub url: Url,
    pub(crate) channel: Channel,
}

impl Peer {
    fn new(url: Url) -> anyhow::Result<Self> {
        let channel = endpoint(&url)?.connect_lazy();
        Ok(Self { url, channel })
    }
}

impl fmt::Display for Peer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.url.fmt(f)
    }
}

impl Peers {
    pub fn new(consensus_api: SocketAddr, bootstrap: Vec<Url>, own: Option<Url>) -> Self {
        let bootstrap = bootstrap
            .into_iter()
            .filter(|url| Some(url) != own.as_ref())
            .filter_map(|url| {
                Peer::new(url.clone())
                    .inspect_err(|err| tracing::warn!(%url, "a bad bootstrap URL: {err:#}"))
                    .ok()
            })
            .collect();
        Self {
            consensus: consensus_channel(consensus_api),
            bootstrap,
            own,
            peers: Vec::new(),
            epoch: None,
        }
    }

    /// The peers, in random order, so that the load spreads and no peer is always asked first.
    pub fn shuffled(&self) -> Vec<Peer> {
        let mut peers = match self.peers.is_empty() {
            true => self.bootstrap.clone(),
            false => self.peers.clone(),
        };
        peers.shuffle(&mut rand::thread_rng());
        peers
    }

    /// Takes the records of `epoch`, unless the cache holds them already. Keeps the cache as it
    /// is if nobody has valid records of `epoch` yet, to be asked again on the next call.
    pub async fn refresh(&mut self, epoch: EpochNumber, membership: &Membership) {
        if self.epoch == Some(epoch) {
            return;
        }
        let mut urls = match service_records(self.consensus.clone(), Some(*epoch)).await {
            Ok((_, records)) => valid_urls(records, epoch, membership),
            Err(status) => {
                tracing::debug!(%epoch, %status, "no records from consensus");
                Vec::new()
            },
        };
        if urls.is_empty() {
            urls = self.ask_bootstrap(epoch, membership).await;
        }
        if urls.is_empty() {
            return;
        }
        tracing::info!(%epoch, peers = urls.len(), "peers of the epoch");
        let peers = urls
            .into_iter()
            .filter(|url| Some(url) != self.own.as_ref())
            .filter_map(|url| {
                match self
                    .peers
                    .iter()
                    .chain(&self.bootstrap)
                    .find(|peer| peer.url == url)
                {
                    // A peer known already keeps its connection.
                    Some(peer) => Some(peer.clone()),
                    None => Peer::new(url.clone())
                        .inspect_err(|err| tracing::debug!(%url, "a bad peer URL: {err:#}"))
                        .ok(),
                }
            })
            .collect();
        (self.peers, self.epoch) = (peers, Some(epoch));
    }

    /// The URLs of the valid records of `epoch` the first bootstrap API service with any lists.
    async fn ask_bootstrap(&self, epoch: EpochNumber, membership: &Membership) -> Vec<Url> {
        for peer in &self.bootstrap {
            match listed_records(peer, epoch).await {
                Ok(records) => {
                    let urls = valid_urls(records, epoch, membership);
                    if !urls.is_empty() {
                        return urls;
                    }
                },
                Err(err) => tracing::debug!(%peer, %epoch, "no records: {err:#}"),
            }
        }
        Vec::new()
    }
}

/// The records `peer` lists for `epoch`; those that do not decode are left out.
async fn listed_records(
    peer: &Peer,
    epoch: EpochNumber,
) -> anyhow::Result<Vec<SignedServiceRecord<SeqTypes>>> {
    let mut client = DiscoveryServiceClient::new(peer.channel.clone());
    let response = client
        .get_peers(GetPeersRequest {
            epoch: Some(*epoch),
        })
        .await?
        .into_inner();
    Ok(response
        .peers
        .into_iter()
        .filter_map(|listed| {
            from_listed(listed)
                .inspect_err(|err| tracing::debug!(%peer, "an undecodable record: {err:#}"))
                .ok()
        })
        .collect())
}

/// The distinct URLs of the `records` of `epoch` that are signed by a validator staked in it.
fn valid_urls(
    records: Vec<SignedServiceRecord<SeqTypes>>,
    epoch: EpochNumber,
    membership: &Membership,
) -> Vec<Url> {
    let Some(table) = membership.stake_table(epoch) else {
        tracing::debug!(%epoch, "no stake table to check the records against");
        return Vec::new();
    };
    let mut urls = Vec::new();
    for record in records {
        let staked = table
            .0
            .iter()
            .any(|peer| peer.stake_table_entry.stake_key == record.signer);
        if record.record.epoch != epoch || !staked || !record.is_valid() {
            tracing::debug!(signer = %record.signer, "a record that does not check out");
            continue;
        }
        match record.record.api_url.parse::<Url>() {
            Ok(url) if !urls.contains(&url) => urls.push(url),
            Ok(_) => {},
            Err(err) => tracing::debug!(%err, "a record with a bad API URL"),
        }
    }
    urls
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use consensus::discovery::ServiceRecord;

    use super::*;
    use crate::{discovery::listed, membership::testing::*};

    fn url(port: u16) -> Url {
        format!("http://127.0.0.1:{port}").parse().unwrap()
    }

    /// A record of `epoch` naming `api_url`, signed by the test validator `signer`.
    fn record(signer: u64, epoch: u64, api_url: &Url) -> SignedServiceRecord<SeqTypes> {
        let (public, private) = bls_key(signer);
        ServiceRecord {
            epoch: EpochNumber::new(epoch),
            api_url: api_url.to_string(),
            api_key: None,
            sequence: 1,
        }
        .sign(public, &private)
        .unwrap()
    }

    /// Without records the bootstrap URLs are the peers, and the service's own URL is never one.
    #[tokio::test]
    async fn uses_the_bootstrap_urls_but_not_its_own() {
        let peers = Peers::new(
            "127.0.0.1:1".parse().unwrap(),
            vec![url(1), url(2), url(3)],
            Some(url(3)),
        );
        let shuffled: BTreeSet<_> = peers.shuffled().into_iter().map(|peer| peer.url).collect();
        assert_eq!(shuffled, BTreeSet::from([url(1), url(2)]));
    }

    /// Only records of the epoch, signed by a validator staked in it, name peers, however they
    /// were listed.
    #[test]
    fn takes_only_valid_records_of_the_epoch() {
        let membership = membership();
        let epoch = FIRST_EPOCH + 1;
        let mut forged = record(1, epoch, &url(11));
        forged.record.api_url = url(12).to_string();
        let records = [
            record(0, epoch, &url(10)),
            record(0, epoch, &url(10)),
            forged,
            record(2, epoch + 1, &url(13)),
            record(SIGNERS + 1, epoch, &url(14)),
        ]
        .into_iter()
        .map(|record| from_listed(listed(record)).unwrap())
        .collect();
        let urls = valid_urls(records, EpochNumber::new(epoch), &membership);
        assert_eq!(urls, vec![url(10)]);
    }
}
