//! The API services a gap filler asks: those the validators of the current epoch name in their
//! signed service records.
//!
//! Records change only when the epoch does, so the cache is refreshed once per epoch. It takes the
//! records the [`Publisher`](crate::publish::Publisher) stored from the deployment's consensus
//! node, and while there are none, those the bootstrap API services list. A record counts only if
//! it is for the epoch, is signed by its signer, and its signer is staked in the epoch, whoever
//! sent it. Until the cache holds records, the bootstrap URLs are the peers. The deployment's own
//! URL is never a peer.

use std::fmt;

use chain_api::{
    Tables,
    discovery::from_listed,
    proto::{GetPeersRequest, discovery_service_client::DiscoveryServiceClient},
};
use consensus_api::discovery::SignedServiceRecord;
use espresso_types::SeqTypes;
use hotshot_types::data::EpochNumber;
use rand::seq::SliceRandom;
use tonic::transport::Channel;
use url::Url;

use crate::{gaps::endpoint, membership::Membership};

pub struct Peers {
    /// Where the records of the deployment's consensus node are stored.
    tables: Tables,

    bootstrap: Vec<Peer>,

    /// The URL other services reach this deployment's API at, left out of the peers.
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
    pub fn new(tables: Tables, bootstrap: Vec<Url>, own: Option<Url>) -> Self {
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
            tables,
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

    /// Takes the stored records of `epoch`, which the publisher completes as validators send
    /// them, so they are read again on every call. Asks the bootstrap services once per epoch
    /// while none are stored. Keeps the cache as it is if nobody has valid records of `epoch` yet,
    /// to be asked again on the next call.
    pub async fn refresh(&mut self, epoch: EpochNumber, membership: &Membership) {
        let stored = match self.tables.service_records(Some(epoch)).await {
            Ok(Some((_, records))) => self.urls(&valid_records(records, epoch, membership)),
            Ok(None) => Vec::new(),
            Err(err) => {
                tracing::warn!(%epoch, "reading the stored records failed: {err:#}");
                Vec::new()
            },
        };
        let urls = match (stored.is_empty(), self.epoch == Some(epoch)) {
            (false, _) => stored,
            (true, true) => return,
            (true, false) => self.ask_bootstrap(epoch, membership).await,
        };
        let current: Vec<_> = self.peers.iter().map(|peer| &peer.url).collect();
        if urls.is_empty() || (self.epoch == Some(epoch) && urls.iter().eq(current)) {
            return;
        }
        tracing::info!(%epoch, peers = urls.len(), "peers of the epoch");
        let peers = urls
            .into_iter()
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

    /// The distinct URLs the `records` name, but for the deployment's own.
    fn urls(&self, records: &[SignedServiceRecord<SeqTypes>]) -> Vec<Url> {
        let mut urls = Vec::new();
        for record in records {
            match record.record.api_url.parse::<Url>() {
                Ok(url) if Some(&url) == self.own.as_ref() || urls.contains(&url) => {},
                Ok(url) => urls.push(url),
                Err(err) => tracing::debug!(%err, "a record with a bad API URL"),
            }
        }
        urls
    }

    /// The URLs of the valid records of `epoch` the first bootstrap API service with any lists.
    async fn ask_bootstrap(&self, epoch: EpochNumber, membership: &Membership) -> Vec<Url> {
        for peer in &self.bootstrap {
            match listed_records(peer, epoch).await {
                Ok(records) => {
                    let urls = self.urls(&valid_records(records, epoch, membership));
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

/// The `records` of `epoch` that are signed by a validator staked in it.
pub(crate) fn valid_records(
    records: Vec<SignedServiceRecord<SeqTypes>>,
    epoch: EpochNumber,
    membership: &Membership,
) -> Vec<SignedServiceRecord<SeqTypes>> {
    let Some(table) = membership.stake_table(epoch) else {
        tracing::debug!(%epoch, "no stake table to check the records against");
        return Vec::new();
    };
    records
        .into_iter()
        .filter(|record| {
            let staked = table
                .0
                .iter()
                .any(|peer| peer.stake_table_entry.stake_key == record.signer);
            let valid = record.record.epoch == epoch && staked && record.is_valid();
            if !valid {
                tracing::debug!(signer = %record.signer, "a record that does not check out");
            }
            valid
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use chain_api::discovery::listed;
    use chain_store::{ChainStore, mem::MemBackend};
    use consensus_api::discovery::ServiceRecord;

    use super::*;
    use crate::membership::testing::*;

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
            Tables::new(ChainStore::new(MemBackend::default())),
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
        let peers = Peers::new(
            Tables::new(ChainStore::new(MemBackend::default())),
            Vec::new(),
            None,
        );
        let urls = peers.urls(&valid_records(
            records,
            EpochNumber::new(epoch),
            &membership,
        ));
        assert_eq!(urls, vec![url(10)]);
    }

    /// Records the publisher stores later in the epoch replace those taken earlier, and the
    /// deployment's own URL is left out of them.
    #[tokio::test]
    async fn takes_records_stored_later_in_the_epoch() {
        let membership = membership();
        let epoch = EpochNumber::new(FIRST_EPOCH + 1);
        let tables = Tables::new(ChainStore::new(MemBackend::default()));
        let mut peers = Peers::new(tables.clone(), Vec::new(), Some(url(12)));
        let store = |signers: &[u64]| {
            let records: Vec<_> = signers
                .iter()
                .map(|&signer| record(signer, *epoch, &url(10 + signer as u16)))
                .collect();
            let tables = tables.clone();
            async move {
                tables
                    .put_service_records(epoch, &records, epoch)
                    .await
                    .unwrap()
            }
        };
        let urls = |peers: &Peers| -> BTreeSet<_> {
            peers.shuffled().into_iter().map(|peer| peer.url).collect()
        };

        store(&[0]).await;
        peers.refresh(epoch, &membership).await;
        assert_eq!(urls(&peers), BTreeSet::from([url(10)]));
        store(&[0, 1, 2]).await;
        peers.refresh(epoch, &membership).await;
        assert_eq!(urls(&peers), BTreeSet::from([url(10), url(11)]));
    }
}
