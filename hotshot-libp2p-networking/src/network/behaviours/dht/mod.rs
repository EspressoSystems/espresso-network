// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

/// Task for doing bootstraps at a regular interval
pub mod bootstrap;
use std::{collections::HashMap, marker::PhantomData, num::NonZeroUsize, time::Duration};

/// a local caching layer for the DHT key value pairs
use futures::{
    channel::{mpsc, oneshot::Sender},
    SinkExt,
};
use hotshot_types::traits::signature_key::SignatureKey;
use lazy_static::lazy_static;
use libp2p::kad::{
    /* handler::KademliaHandlerIn, */ store::MemoryStore, BootstrapOk, GetClosestPeersOk,
    GetRecordOk, GetRecordResult, ProgressStep, PutRecordResult, QueryId, QueryResult, Record,
};
use libp2p::kad::{
    store::RecordStore, Behaviour as KademliaBehaviour, BootstrapError, Event as KademliaEvent,
};
use libp2p_identity::PeerId;
use store::{
    persistent::{DhtPersistentStorage, PersistentStore},
    validated::ValidatedStore,
};
use tokio::{spawn, sync::mpsc::UnboundedSender, time::sleep};
use tracing::{debug, error, warn};

/// Additional DHT record functionality
pub mod record;

/// Additional DHT store functionality
pub mod store;

/// the number of nodes required to get an answer from
/// in order to trust that the answer is correct when retrieving from the DHT
pub(crate) const NUM_REPLICATED_TO_TRUST: usize = 2;

lazy_static! {
    /// the maximum number of nodes to query in the DHT at any one time
    static ref MAX_DHT_QUERY_SIZE: NonZeroUsize = NonZeroUsize::new(50).unwrap();
}

use super::exponential_backoff::ExponentialBackoff;
use crate::network::{ClientRequest, NetworkEvent};

/// Behaviour wrapping libp2p's kademlia
/// included:
/// - publishing API
/// - Request API
/// - bootstrapping into the network
/// - peer discovery
#[derive(Debug)]
pub struct DHTBehaviour<K: SignatureKey + 'static, D: DhtPersistentStorage> {
    /// in progress queries for nearby peers
    pub in_progress_get_closest_peers: HashMap<QueryId, Sender<()>>,
    /// List of in-progress get requests
    in_progress_record_queries: HashMap<QueryId, KadGetQuery>,
    /// The list of in-progress get requests by key
    outstanding_dht_query_keys: HashMap<Vec<u8>, QueryId>,
    /// List of in-progress put requests
    in_progress_put_record_queries: HashMap<QueryId, KadPutQuery>,
    /// State of bootstrapping
    pub bootstrap_state: Bootstrap,
    /// the peer id (useful only for debugging right now)
    pub peer_id: PeerId,
    /// replication factor
    pub replication_factor: NonZeroUsize,
    /// Sender to retry requests.
    retry_tx: Option<UnboundedSender<ClientRequest>>,
    /// Sender to the bootstrap task
    bootstrap_tx: Option<mpsc::Sender<bootstrap::InputEvent>>,

    /// Phantom type for the key and persistent storage
    phantom: PhantomData<(K, D)>,
}

/// State of bootstrapping
#[derive(Debug, Clone)]
pub struct Bootstrap {
    /// State of bootstrap
    pub state: State,
    /// Retry timeout
    pub backoff: ExponentialBackoff,
}

/// State used for random walk and bootstrapping
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum State {
    /// Not in progress
    NotStarted,
    /// In progress
    Started,
}

/// DHT event enum
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DHTEvent {
    /// Only event tracked currently is when we successfully bootstrap into the network
    IsBootstrapped,
}

impl<K: SignatureKey + 'static, D: DhtPersistentStorage> DHTBehaviour<K, D> {
    /// Give the handler a way to retry requests.
    pub fn set_retry(&mut self, tx: UnboundedSender<ClientRequest>) {
        self.retry_tx = Some(tx);
    }
    /// Sets a sender to bootstrap task
    pub fn set_bootstrap_sender(&mut self, tx: mpsc::Sender<bootstrap::InputEvent>) {
        self.bootstrap_tx = Some(tx);
    }
    /// Create a new DHT behaviour
    #[must_use]
    pub fn new(pid: PeerId, replication_factor: NonZeroUsize) -> Self {
        // needed because otherwise we stay in client mode when testing locally
        // and don't publish keys stuff
        // e.g. dht just doesn't work. We'd need to add mdns and that doesn't seem worth it since
        // we won't have a local network
        // <https://github.com/libp2p/rust-libp2p/issues/4194>
        Self {
            peer_id: pid,
            in_progress_record_queries: HashMap::default(),
            in_progress_put_record_queries: HashMap::default(),
            outstanding_dht_query_keys: HashMap::default(),
            bootstrap_state: Bootstrap {
                state: State::NotStarted,
                backoff: ExponentialBackoff::new(2, Duration::from_secs(1)),
            },
            in_progress_get_closest_peers: HashMap::default(),
            replication_factor,
            retry_tx: None,
            bootstrap_tx: None,
            phantom: PhantomData,
        }
    }

    /// print out the routing table to stderr
    pub fn print_routing_table(
        &mut self,
        kadem: &mut KademliaBehaviour<PersistentStore<ValidatedStore<MemoryStore, K>, D>>,
    ) {
        let mut err = format!("KBUCKETS: PID: {:?}, ", self.peer_id);
        let v = kadem.kbuckets().collect::<Vec<_>>();
        for i in v {
            for j in i.iter() {
                let s = format!(
                    "node: key: {:?}, val {:?}, status: {:?}",
                    j.node.key, j.node.value, j.status
                );
                err.push_str(&s);
            }
        }
        error!("{:?}", err);
    }

    /// Get the replication factor for queries
    #[must_use]
    pub fn replication_factor(&self) -> NonZeroUsize {
        self.replication_factor
    }
    /// Publish a key/value to the kv store.
    /// Once replicated upon all nodes, the caller is notified over
    /// `chan`
    pub fn put_record(&mut self, id: QueryId, query: KadPutQuery) {
        self.in_progress_put_record_queries.insert(id, query);
    }

    /// Retrieve a value for a key from the DHT.
    pub fn get_record(
        &mut self,
        key: Vec<u8>,
        chans: Vec<Sender<Vec<u8>>>,
        factor: NonZeroUsize,
        backoff: ExponentialBackoff,
        retry_count: u8,
        kad: &mut KademliaBehaviour<PersistentStore<ValidatedStore<MemoryStore, K>, D>>,
    ) {
        // noop
        if retry_count == 0 {
            return;
        }

        // Check the cache before making the (expensive) query
        if let Some(entry) = kad.store_mut().get(&key.clone().into()) {
            // The key already exists in the cache, send the value to all channels
            for chan in chans {
                if chan.send(entry.value.clone()).is_err() {
                    warn!("Get DHT: channel closed before get record request result could be sent");
                }
            }
        } else {
            // Check if the key is already being queried
            if let Some(qid) = self.outstanding_dht_query_keys.get(&key) {
                // The key was already being queried. Add the channel to the existing query
                // Try to get the query from the query id
                let Some(query) = self.in_progress_record_queries.get_mut(qid) else {
                    warn!("Get DHT: outstanding query not found");
                    return;
                };

                // Add the channel to the existing query
                query.notify.extend(chans);
            } else {
                // The key was not already being queried and was not in the cache. Start a new query.
                let qid = kad.get_record(key.clone().into());
                let query = KadGetQuery {
                    backoff,
                    progress: DHTProgress::InProgress(qid),
                    notify: chans,
                    num_replicas: factor,
                    key: key.clone(),
                    retry_count: retry_count - 1,
                    records: HashMap::default(),
                };

                // Add the key to the outstanding queries and in-progress queries
                self.outstanding_dht_query_keys.insert(key, qid);
                self.in_progress_record_queries.insert(qid, query);
            }
        }
    }

    /// Spawn a task which will retry the query after a backoff.
    fn retry_get(&self, mut query: KadGetQuery) {
        let Some(tx) = self.retry_tx.clone() else {
            return;
        };
        let req = ClientRequest::GetDHT {
            key: query.key,
            notify: query.notify,
            retry_count: query.retry_count,
        };
        let backoff = query.backoff.next_timeout(false);
        spawn(async move {
            sleep(backoff).await;
            let _ = tx.send(req);
        });
    }

    /// Spawn a task which will retry the query after a backoff.
    fn retry_put(&self, mut query: KadPutQuery) {
        let Some(tx) = self.retry_tx.clone() else {
            return;
        };
        let req = ClientRequest::PutDHT {
            key: query.key,
            value: query.value,
            notify: query.notify,
        };
        spawn(async move {
            sleep(query.backoff.next_timeout(false)).await;
            let _ = tx.send(req);
        });
    }

    /// update state based on recv-ed get query
    fn handle_get_query(
        &mut self,
        store: &mut PersistentStore<ValidatedStore<MemoryStore, K>, D>,
        record_results: GetRecordResult,
        id: QueryId,
        mut last: bool,
    ) {
        let num = match self.in_progress_record_queries.get_mut(&id) {
            Some(query) => match record_results {
                Ok(results) => match results {
                    GetRecordOk::FoundRecord(record) => {
                        match query.records.entry(record.record.value) {
                            std::collections::hash_map::Entry::Occupied(mut o) => {
                                let num_entries = o.get_mut();
                                *num_entries += 1;
                                *num_entries
                            },
                            std::collections::hash_map::Entry::Vacant(v) => {
                                v.insert(1);
                                1
                            },
                        }
                    },
                    GetRecordOk::FinishedWithNoAdditionalRecord {
                        cache_candidates: _,
                    } => {
                        tracing::debug!("GetRecord Finished with No Additional Record");
                        last = true;
                        0
                    },
                },
                Err(err) => {
                    warn!("Error in Kademlia query: {:?}", err);
                    0
                },
            },
            None => {
                // We already finished the query (or it's been cancelled). Do nothing and exit the
                // function.
                return;
            },
        };

        // if the query has completed and we need to retry
        // or if the query has enough replicas to return to the client
        // trigger retry or completion logic
        if num >= NUM_REPLICATED_TO_TRUST || last {
            if let Some(KadGetQuery {
                backoff,
                progress,
                notify,
                num_replicas,
                key,
                retry_count,
                records,
            }) = self.in_progress_record_queries.remove(&id)
            {
                // Remove the key from the outstanding queries so we are in sync
                self.outstanding_dht_query_keys.remove(&key);

                // `notify` is all channels that are still open
                let notify = notify
                    .into_iter()
                    .filter(|n| !n.is_canceled())
                    .collect::<Vec<_>>();

                // If all are closed, we can exit
                if notify.is_empty() {
                    return;
                }

                // NOTE case where multiple nodes agree on different
                // values is not handled because it can't be hit.
                // We optimistically choose whichever record returns the most trusted entries first

                // iterate through the records and find an value that has enough replicas
                // to trust the value
                if let Some((r, _)) = records
                    .into_iter()
                    .find(|(_, v)| *v >= NUM_REPLICATED_TO_TRUST)
                {
                    let record = Record {
                        key: key.into(),
                        value: r.clone(),
                        publisher: None,
                        expires: None,
                    };

                    // Only return the record if we can store it (validation passed)
                    if store.put(record).is_ok() {
                        // Send the record to all channels that are still open
                        for n in notify {
                            if n.send(r.clone()).is_err() {
                                warn!("Get DHT: channel closed before get record request result could be sent");
                            }
                        }
                    } else {
                        error!("Failed to store record in local store");
                    }
                }
                // disagreement => query more nodes
                else {
                    // there is some internal disagreement or not enough nodes returned
                    // Initiate new query that hits more replicas
                    if retry_count > 0 {
                        let new_retry_count = retry_count - 1;
                        warn!("Get DHT: Internal disagreement for get dht request {:?}! requerying with more nodes. {:?} retries left", progress, new_retry_count);
                        let new_factor = NonZeroUsize::max(
                            NonZeroUsize::new(num_replicas.get() + 1).unwrap_or(num_replicas),
                            *MAX_DHT_QUERY_SIZE,
                        );
                        self.retry_get(KadGetQuery {
                            backoff,
                            progress: DHTProgress::NotStarted,
                            notify,
                            num_replicas: new_factor,
                            key,
                            retry_count: new_retry_count,
                            records: HashMap::default(),
                        });
                    }
                    warn!("Get DHT: Internal disagreement for get dht request {:?}! Giving up because out of retries. ", progress);
                }
            }
        }
    }

    /// Update state based on put query
    fn handle_put_query(&mut self, record_results: PutRecordResult, id: QueryId) {
        if let Some(mut query) = self.in_progress_put_record_queries.remove(&id) {
            // dropped so we handle further
            if query.notify.is_canceled() {
                return;
            }

            match record_results {
                Ok(_) => {
                    if query.notify.send(()).is_err() {
                        warn!("Put DHT: client channel closed before put record request could be sent");
                    }
                },
                Err(e) => {
                    query.progress = DHTProgress::NotStarted;
                    query.backoff.start_next(false);

                    warn!(
                        "Put DHT: error performing put: {:?}. Retrying on pid {:?}.",
                        e, self.peer_id
                    );
                    // push back onto the queue
                    self.retry_put(query);
                },
            }
        } else {
            warn!("Put DHT: completed DHT query that is no longer tracked.");
        }
    }

    /// Send that the bootstrap succeeded
    fn finish_bootstrap(&mut self) {
        if let Some(mut tx) = self.bootstrap_tx.clone() {
            spawn(async move { tx.send(bootstrap::InputEvent::BootstrapFinished).await });
        }
    }
    #[allow(clippy::too_many_lines)]
    /// handle a DHT event
    pub fn dht_handle_event(
        &mut self,
        event: KademliaEvent,
        store: &mut PersistentStore<ValidatedStore<MemoryStore, K>, D>,
    ) -> Option<NetworkEvent> {
        match event {
            KademliaEvent::OutboundQueryProgressed {
                result: QueryResult::PutRecord(record_results),
                id,
                step: ProgressStep { last, .. },
                ..
            } => {
                if last {
                    self.handle_put_query(record_results, id);
                }
            },
            KademliaEvent::OutboundQueryProgressed {
                result: QueryResult::GetClosestPeers(r),
                id: query_id,
                stats: _,
                step: ProgressStep { last: true, .. },
                ..
            } => match r {
                Ok(GetClosestPeersOk { key, peers: _ }) => {
                    if let Some(chan) = self.in_progress_get_closest_peers.remove(&query_id) {
                        if chan.send(()).is_err() {
                            warn!("DHT: finished query but client was no longer interested");
                        };
                    };
                    debug!("Successfully got closest peers for key {:?}", key);
                },
                Err(e) => {
                    if let Some(chan) = self.in_progress_get_closest_peers.remove(&query_id) {
                        let _: Result<_, _> = chan.send(());
                    };
                    warn!("Failed to get closest peers: {:?}", e);
                },
            },
            KademliaEvent::OutboundQueryProgressed {
                result: QueryResult::GetRecord(record_results),
                id,
                step: ProgressStep { last, .. },
                ..
            } => {
                self.handle_get_query(store, record_results, id, last);
            },
            KademliaEvent::OutboundQueryProgressed {
                result:
                    QueryResult::Bootstrap(Ok(BootstrapOk {
                        peer: _,
                        num_remaining,
                    })),
                step: ProgressStep { last: true, .. },
                ..
            } => {
                if num_remaining == 0 {
                    self.finish_bootstrap();
                } else {
                    debug!("Bootstrap in progress, {} nodes remaining", num_remaining);
                }
                return Some(NetworkEvent::IsBootstrapped);
            },
            KademliaEvent::OutboundQueryProgressed {
                result: QueryResult::Bootstrap(Err(e)),
                ..
            } => {
                let BootstrapError::Timeout { num_remaining, .. } = e;
                if num_remaining.is_none() {
                    error!("Failed to bootstrap: {:?}", e);
                }
                self.finish_bootstrap();
            },
            KademliaEvent::RoutablePeer { peer, address: _ } => {
                debug!("Found routable peer {:?}", peer);
            },
            KademliaEvent::PendingRoutablePeer { peer, address: _ } => {
                debug!("Found pending routable peer {:?}", peer);
            },
            KademliaEvent::UnroutablePeer { peer } => {
                debug!("Found unroutable peer {:?}", peer);
            },
            KademliaEvent::RoutingUpdated {
                peer: _,
                is_new_peer: _,
                addresses: _,
                bucket_range: _,
                old_peer: _,
            } => {
                debug!("Routing table updated");
            },
            e @ KademliaEvent::OutboundQueryProgressed { .. } => {
                debug!("Not handling dht event {:?}", e);
            },
            e => {
                debug!("New unhandled swarm event: {e:?}");
            },
        }
        None
    }
}

/// Metadata holder for get query
#[derive(Debug)]
pub(crate) struct KadGetQuery {
    /// Exponential retry backoff
    pub(crate) backoff: ExponentialBackoff,
    /// progress through DHT query
    pub(crate) progress: DHTProgress,
    /// The channels to notify of the result
    pub(crate) notify: Vec<Sender<Vec<u8>>>,
    /// number of replicas required to replicate over
    pub(crate) num_replicas: NonZeroUsize,
    /// the key to look up
    pub(crate) key: Vec<u8>,
    /// the number of remaining retries before giving up
    pub(crate) retry_count: u8,
    /// already received records
    pub(crate) records: HashMap<Vec<u8>, usize>,
}

/// Metadata holder for get query
#[derive(Debug)]
pub struct KadPutQuery {
    /// Exponential retry backoff
    pub(crate) backoff: ExponentialBackoff,
    /// progress through DHT query
    pub(crate) progress: DHTProgress,
    /// notify client of result
    pub(crate) notify: Sender<()>,
    /// the key to put
    pub(crate) key: Vec<u8>,
    /// the value to put
    pub(crate) value: Vec<u8>,
}

/// represents progress through DHT
#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub enum DHTProgress {
    /// The query has been started
    InProgress(QueryId),
    /// The query has not been started
    NotStarted,
}
