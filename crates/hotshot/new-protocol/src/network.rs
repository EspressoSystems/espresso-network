use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub use cliquenet::{Config as CliquenetConfig, NetAddr, Role};
use cliquenet::{NetworkReceiver, NetworkSender, Slot, noise::Protocol, x25519::PublicKey};
use hotshot_types::{
    PeerConnectInfo,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{EXTERNAL_MESSAGE_VERSION, MessageKind, UpgradeLock},
    traits::{
        metrics::{Counter, CounterFamily, Gauge, GaugeFamily, Metrics},
        node_implementation::NodeType,
    },
    x25519::{self, Keypair},
};
use hotshot_utils::anytrace;
use parking_lot::RwLock;
use tracing::{error, info};

use crate::message::{Message, MessageType, Unchecked, Validated};

#[derive(Debug)]
pub struct Cliquenet<T: NodeType> {
    inner: Sender<T>,
    receiver: NetworkReceiver,
}

#[derive(Debug, Clone)]
pub struct Sender<T: NodeType> {
    my_keys: (T::SignatureKey, PublicKey),
    sender: NetworkSender,
    shared: Arc<RwLock<Shared<T::SignatureKey>>>,
    upgrade_lock: UpgradeLock<T>,
}

#[derive(Debug)]
struct Shared<K> {
    peers: HashMap<K, PeerConnectInfo>,
    epoch: EpochNumber,
}

impl<T: NodeType> Cliquenet<T> {
    pub async fn create<A, P, S>(
        name: S,
        signing_key: T::SignatureKey,
        keypair: Keypair,
        addr: A,
        parties: P,
        upgrade_lock: UpgradeLock<T>,
        metrics: Box<dyn Metrics>,
    ) -> Result<Self, NetworkError>
    where
        A: Into<cliquenet::NetAddr>,
        P: IntoIterator<Item = (T::SignatureKey, PeerConnectInfo)>,
        S: Into<String>,
    {
        let parties: HashMap<T::SignatureKey, PeerConnectInfo> = parties.into_iter().collect();

        let cfg = cliquenet::Config::builder()
            .name(name)
            .keypair(keypair.into())
            .bind(addr.into())
            .parties(
                parties
                    .values()
                    .map(|info| (info.x25519_key.into(), info.p2p_addr.clone())),
            )
            .noise_protocols([(1.into(), Protocol::IK_25519_AesGcm_Blake2s)])
            .max_message_size(std::num::NonZeroUsize::new(100 * 1024 * 1024).expect("100 MiB > 0"))
            .build();

        Self::create_with_config(signing_key, upgrade_lock, cfg, parties, metrics).await
    }

    pub(crate) async fn create_with_config<P>(
        signing_key: T::SignatureKey,
        upgrade_lock: UpgradeLock<T>,
        config: cliquenet::Config,
        parties: P,
        metrics: Box<dyn Metrics>,
    ) -> Result<Self, NetworkError>
    where
        P: IntoIterator<Item = (T::SignatureKey, PeerConnectInfo)>,
    {
        let public_key = config.public_key();
        let metrics = CliquenetMetrics::new(metrics);
        let network = cliquenet::Network::create(config.with_metrics(metrics)).await?;
        let peers: HashMap<_, _> = parties.into_iter().collect();

        info!(peers = %peers.len(), "cliquenet created");

        let (send, recv) = network.split_into();

        Ok(Self {
            inner: Sender {
                my_keys: (signing_key, public_key),
                sender: send,
                shared: Arc::new(RwLock::new(Shared {
                    peers,
                    epoch: EpochNumber::new(0),
                })),
                upgrade_lock,
            },
            receiver: recv,
        })
    }

    pub fn sender(&self) -> &Sender<T> {
        &self.inner
    }

    pub async fn receive(&mut self) -> Result<Message<T, Unchecked>, NetworkError> {
        let (src, bytes) = self
            .receiver
            .receive()
            .await
            .ok_or(cliquenet::NetworkError::ChannelClosed)?;
        let msg = self.deserialize(&bytes)?;
        let key = self
            .inner
            .shared
            .read()
            .peers
            .get(&msg.sender)
            .map(|info| info.x25519_key)
            .or_else(|| {
                (msg.sender == self.inner.my_keys.0).then_some(self.inner.my_keys.1.into())
            });
        if Some(src.into()) != key {
            return Err(NetworkError::InvalidSender {
                msg: key,
                src: src.into(),
            });
        }
        Ok(msg)
    }

    pub async fn shutdown(&mut self) {
        if let Ok(done) = self.inner.sender.shutdown() {
            done.await
        }
    }

    pub fn gc(&mut self, v: ViewNumber) -> Result<(), NetworkError> {
        self.inner.sender.gc(Slot::new(*v))?;
        Ok(())
    }

    pub fn add_peers(
        &mut self,
        r: Role,
        ps: Vec<(T::SignatureKey, (PublicKey, NetAddr))>,
    ) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        {
            let mut shared = self.inner.shared.write();
            for (k, (x, a)) in ps {
                shared.peers.insert(
                    k,
                    PeerConnectInfo {
                        x25519_key: x.into(),
                        p2p_addr: a.clone(),
                    },
                );
                targets.push((x, a))
            }
        }
        self.inner.sender.add_peers(r, targets)?;
        Ok(())
    }

    pub fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        {
            let mut shared = self.inner.shared.write();
            for k in ps {
                if let Some(info) = shared.peers.remove(k) {
                    targets.push(info.x25519_key.into())
                }
            }
        }
        self.inner.sender.remove_peers(targets)?;
        Ok(())
    }

    pub fn assign_role(&mut self, r: Role, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        {
            let shared = self.inner.shared.read();
            for k in ps {
                if let Some(info) = shared.peers.get(k) {
                    targets.push(info.x25519_key.into())
                }
            }
        }
        self.inner.sender.assign_peers(r, targets)?;
        Ok(())
    }

    /// Update peers on every epoch change.
    ///
    /// For any given epoch `e` we collect the validators of `e`, `e-1` and
    /// `e+1` from the stake tables and merge their connection information.
    ///
    /// We keep validators that were in `e-1` but not in `e` for one additional
    /// epoch and eagerly connect to new validators of `e+1`.
    pub fn apply_epoch(
        &mut self,
        epoch: EpochNumber,
        coord: &EpochMembershipCoordinator<T>,
    ) -> Result<(), NetworkError> {
        let ours = self.inner.shared.read().epoch;
        if epoch <= ours {
            info!(%epoch, %ours, "epoch already seen");
            return Ok(());
        }

        // Validators of the new epoch.
        let Some(curr_infos) = coord.epoch_peers(Some(epoch)) else {
            error!(%epoch, "no stake table available");
            return Ok(());
        };

        // Validators leaving are retained as peers for one additional epoch.
        let prev_infos = if *epoch > 0 {
            coord.epoch_peers(Some(epoch - 1)).unwrap_or_else(|| {
                info!(%epoch, "previous epoch's stake table unavailable");
                HashMap::new()
            })
        } else {
            HashMap::new()
        };

        // Validators joining in the next epoch are connected to early.
        let next_infos = coord.epoch_peers(Some(epoch + 1)).unwrap_or_else(|| {
            info!(%epoch, "next epoch's stake table not available");
            HashMap::new()
        });

        // Since connection information may be updated, we need to merge them,
        // preferring the newest epoch's data, i.e. `next(curr(prev))`.
        let mut merged_infos = prev_infos.clone();
        for (k, v) in curr_infos.iter().chain(&next_infos) {
            merged_infos.insert(k.clone(), v.clone());
        }

        let wanted: HashSet<T::SignatureKey> = curr_infos
            .keys()
            .chain(next_infos.keys())
            .cloned()
            .collect();

        let retained: HashSet<T::SignatureKey> = curr_infos
            .keys()
            .chain(prev_infos.keys())
            .cloned()
            .collect();

        let mut to_add: Vec<(T::SignatureKey, PeerConnectInfo)> = Vec::new();
        let mut to_del: Vec<(T::SignatureKey, PeerConnectInfo)> = Vec::new();

        for k in &wanted {
            if let Some(Some(new_info)) = merged_infos.get(k) {
                if Some(new_info) != self.inner.shared.read().peers.get(k) {
                    info!(%epoch, peer = %k, "adding/updating network peer");
                    to_add.push((k.clone(), new_info.clone()));
                } else {
                    info!(%epoch, peer = %k, "peer unchanged");
                }
            } else {
                info!(%epoch, peer = %k, "ignoring peer without connection info");
            }
        }

        // Remove peers that have left both the current and previous epochs.
        for (k, info) in &self.inner.shared.read().peers {
            if !(retained.contains(k) || wanted.contains(k)) {
                info!(%epoch, peer = %k, "removing network peer");
                to_del.push((k.clone(), info.clone()));
            }
        }

        {
            let peers = &mut self.inner.shared.write().peers;
            for (k, _) in &to_del {
                peers.remove(k);
            }
            for (k, info) in &to_add {
                peers.insert(k.clone(), info.clone());
            }
        }

        let add_targets: Vec<(PublicKey, NetAddr)> = to_add
            .iter()
            .map(|(_, i)| (i.x25519_key.into(), i.p2p_addr.clone()))
            .collect();
        let del_targets: Vec<PublicKey> = to_del.iter().map(|(_, i)| i.x25519_key.into()).collect();

        if let Err(err) = self.inner.sender.add_peers(Role::Active, add_targets) {
            error!(%epoch, %err, "network down; could not add peers to network");
            return Err(err.into());
        }

        if let Err(err) = self.inner.sender.remove_peers(del_targets) {
            error!(%epoch, %err, "network down; could not remove peers from network");
            return Err(err.into());
        }

        info!(%epoch, peers = %self.inner.shared.read().peers.len());

        self.inner.shared.write().epoch = epoch;

        Ok(())
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message<T, Unchecked>, NetworkError> {
        match self
            .inner
            .upgrade_lock
            .deserialize::<Message<T, Unchecked>>(bytes)
        {
            Ok((m, v)) => {
                if v == EXTERNAL_MESSAGE_VERSION && !m.is_external() {
                    let e = anytrace::warn!("received a non-external message with version 0.0");
                    return Err(NetworkError::Serialize(e));
                }
                Ok(m)
            },
            Err(primary_err) => {
                // Fallback: bytes may be a hotshot-types `Message<T>` carrying
                // an `External` payload (this is how `Leaf2Fetcher` in the
                // membership layer frames leaf-catchup requests/responses).
                // If so, surface it as `MessageType::External` so the
                // Coordinator can route it to the membership external
                // channel just like a native new-protocol external message.
                if let Ok((_v, hs_msg)) =
                    versions::decode::<hotshot_types::message::Message<T>>(bytes)
                    && let MessageKind::External(data) = hs_msg.kind
                {
                    return Ok(Message {
                        sender: hs_msg.sender,
                        message_type: MessageType::External(data),
                    });
                }
                Err(NetworkError::Serialize(primary_err))
            },
        }
    }
}

impl<T: NodeType> Sender<T> {
    pub fn unicast(
        &self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let target = if *to == self.my_keys.0 {
            self.my_keys.1
        } else if let Some(info) = self.shared.read().peers.get(to) {
            info.x25519_key.into()
        } else {
            error!(peer = %to, "unicast target not found");
            return Ok(());
        };
        let bytes = self.serialize(m)?;
        self.sender.unicast(Slot::new(*v), target, bytes)?;
        Ok(())
    }

    pub fn multicast(
        &self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        let mut targets = Vec::new();
        {
            let shared = self.shared.read();
            for t in to {
                if let Some(info) = shared.peers.get(t) {
                    targets.push(info.x25519_key.into())
                } else if *t == self.my_keys.0 {
                    targets.push(self.my_keys.1)
                } else {
                    error!(peer = %t, "multicast target not found");
                }
            }
        }
        self.sender.multicast(Slot::new(*v), targets, bytes)?;
        Ok(())
    }

    pub fn broadcast(&self, v: ViewNumber, m: &Message<T, Validated>) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.sender.broadcast(Slot::new(*v), bytes)?;
        Ok(())
    }

    fn serialize(&self, m: &Message<T, Validated>) -> Result<Vec<u8>, NetworkError> {
        if let MessageType::External(bytes) = &m.message_type {
            return Ok(bytes.clone());
        }
        let v = self.upgrade_lock.serialize(m)?;
        Ok(v)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("cliquenet: {0}")]
    Cliquenet(#[from] cliquenet::NetworkError),

    #[error("serialization: {0}")]
    Serialize(#[from] anytrace::Error),

    #[error("message sender {msg:?} != message source {src}")]
    InvalidSender {
        msg: Option<x25519::PublicKey>,
        src: x25519::PublicKey,
    },
}

impl NetworkError {
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            Self::Cliquenet(
                cliquenet::NetworkError::Bind(..)
                    | cliquenet::NetworkError::ChannelClosed
                    | cliquenet::NetworkError::BudgetClosed
            )
        )
    }
}

struct CliquenetMetrics {
    metrics: Box<dyn Metrics>,
    gauges: RwLock<Gauges>,
    counters: RwLock<Counters>,
}

#[derive(Default)]
struct Gauges {
    gauges: HashMap<PublicKey, HashMap<String, Box<dyn Gauge>>>,
    family: HashMap<String, Box<dyn GaugeFamily>>,
}

#[derive(Default)]
struct Counters {
    counters: HashMap<PublicKey, HashMap<String, Box<dyn Counter>>>,
    family: HashMap<String, Box<dyn CounterFamily>>,
}

impl CliquenetMetrics {
    pub fn new(m: Box<dyn Metrics>) -> Self {
        Self {
            metrics: m.subgroup("cliquenet".to_string()),
            gauges: RwLock::new(Gauges::default()),
            counters: RwLock::new(Counters::default()),
        }
    }
}

// In here we lazily create counters and gauges based on their labels.
// If not found, we create a family using the label, e.g. "connect_attempts",
// indexed by the peer (key). Afterwards we create the actual counter or gauge,
// and update its value. On the next call, the metric would be found and
// updated right away.
impl cliquenet::Metrics for CliquenetMetrics {
    fn set(&self, key: &PublicKey, label: &str, val: usize) {
        if let Some(g) = self
            .gauges
            .read()
            .gauges
            .get(key)
            .and_then(|m| m.get(label))
        {
            return g.set(val);
        }

        let mut gauges = self.gauges.write();

        // Check again, in case a concurrent write has created the gauge:
        if let Some(g) = gauges.gauges.get(key).and_then(|m| m.get(label)) {
            return g.set(val);
        }

        let g = gauges
            .family
            .entry(label.to_string())
            .or_insert_with(|| {
                self.metrics
                    .gauge_family(label.to_string(), vec!["peer".to_string()])
            })
            .create(vec![key.to_string()]);

        gauges
            .gauges
            .entry(*key)
            .or_default()
            .entry(label.to_string())
            .or_insert(g)
            .set(val)
    }

    fn add(&self, key: &PublicKey, label: &str, val: usize) {
        if let Some(c) = self
            .counters
            .read()
            .counters
            .get(key)
            .and_then(|m| m.get(label))
        {
            return c.add(val);
        }

        let mut counters = self.counters.write();

        // Check again, in case a concurrent write has created the counter:
        if let Some(c) = counters.counters.get(key).and_then(|m| m.get(label)) {
            return c.add(val);
        }

        let c = counters
            .family
            .entry(label.to_string())
            .or_insert_with(|| {
                self.metrics
                    .counter_family(label.to_string(), vec!["peer".to_string()])
            })
            .create(vec![key.to_string()]);

        counters
            .counters
            .entry(*key)
            .or_default()
            .entry(label.to_string())
            .or_insert(c)
            .add(val)
    }

    fn del(&self, key: &PublicKey) {
        let key_string = key.to_string();

        {
            let mut gauges = self.gauges.write();
            for (label, _) in gauges.gauges.remove(key).into_iter().flatten() {
                if let Some(f) = gauges.family.get(&label) {
                    f.destroy(&[&key_string]);
                }
            }
        }

        {
            let mut counters = self.counters.write();
            for (label, _) in counters.counters.remove(key).into_iter().flatten() {
                if let Some(f) = counters.family.get(&label) {
                    f.destroy(&[&key_string]);
                }
            }
        }
    }
}
