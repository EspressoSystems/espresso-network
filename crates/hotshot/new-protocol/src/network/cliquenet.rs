use std::collections::{HashMap, HashSet};

pub use cliquenet::Config as CliquenetConfig;
use cliquenet::{
    NetAddr, NetworkError as CliquenetError, Role, Slot, noise::Protocol, x25519::PublicKey,
};
use hotshot_types::{
    PeerConnectInfo,
    data::{EpochNumber, ViewNumber},
    epoch_membership::EpochMembershipCoordinator,
    message::{EXTERNAL_MESSAGE_VERSION, MessageKind, UpgradeLock},
    traits::{
        metrics::{Counter, CounterFamily, Gauge, GaugeFamily, Metrics},
        node_implementation::NodeType,
    },
    x25519::Keypair,
};
use parking_lot::RwLock;
use tracing::{error, info};

use crate::{
    message::{Message, MessageType, Unchecked, Validated},
    network::{Network, NetworkError, PeerRole},
};

pub struct Cliquenet<T: NodeType> {
    my_keys: (T::SignatureKey, PublicKey),
    inner: cliquenet::Network,
    peers: HashMap<T::SignatureKey, PeerConnectInfo>,
    epoch: EpochNumber,
    upgrade_lock: UpgradeLock<T>,
}

impl<T: NodeType> Cliquenet<T> {
    pub async fn create<A, P>(
        name: &str,
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
        let network = cliquenet::Network::create(config.with_metrics(metrics))
            .await
            .map_err(to_network_error)?;

        let peers: HashMap<_, _> = parties.into_iter().collect();

        info!(peers = %peers.len(), "cliquenet created");

        Ok(Self {
            my_keys: (signing_key, public_key),
            inner: network,
            peers,
            epoch: EpochNumber::new(0),
            upgrade_lock,
        })
    }
}

impl<T: NodeType> Network<T> for Cliquenet<T> {
    type PeerData = (PublicKey, NetAddr);

    fn unicast_raw(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: Vec<u8>,
    ) -> Result<(), NetworkError> {
        let target = if *to == self.my_keys.0 {
            self.my_keys.1
        } else if let Some(info) = self.peers.get(to) {
            info.x25519_key.into()
        } else {
            error!(peer = %to, "unicast target not found");
            return Ok(());
        };
        self.inner
            .unicast(Slot::new(*v), target, m)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn unicast(
        &mut self,
        v: ViewNumber,
        to: &T::SignatureKey,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.unicast_raw(v, to, bytes)
    }

    fn multicast_raw(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: Vec<u8>,
    ) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for t in to {
            if let Some(info) = self.peers.get(t) {
                targets.push(info.x25519_key.into())
            } else if *t == self.my_keys.0 {
                targets.push(self.my_keys.1)
            } else {
                error!(peer = %t, "multicast target not found");
            }
        }
        self.inner
            .multicast(Slot::new(*v), targets, m)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn multicast(
        &mut self,
        v: ViewNumber,
        to: Vec<&T::SignatureKey>,
        m: &Message<T, Validated>,
    ) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.multicast_raw(v, to, bytes)
    }

    fn broadcast_raw(&mut self, v: ViewNumber, m: Vec<u8>) -> Result<(), NetworkError> {
        self.inner
            .broadcast(Slot::new(*v), m)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn broadcast(&mut self, v: ViewNumber, m: &Message<T, Validated>) -> Result<(), NetworkError> {
        let bytes = self.serialize(m)?;
        self.broadcast_raw(v, bytes)
    }

    async fn receive(&mut self) -> Result<Message<T, Unchecked>, NetworkError> {
        let (src, bytes) = self
            .inner
            .receive()
            .await
            .ok_or_else(|| NetworkError::Critical("cliquenet has shutdown".into()))?;
        let m = self.deserialize(&bytes)?;
        let k = self.peers.get(&m.sender).map(|info| info.x25519_key);
        let s = src.into();
        if Some(s) != k {
            return Err(NetworkError::InvalidSender {
                sender: m.sender.to_string(),
                configured: k,
                actual: s,
            });
        }
        Ok(m)
    }

    async fn shutdown(&mut self) {
        if let Ok(done) = self.inner.shutdown() {
            done.await
        }
    }

    fn gc(&mut self, v: ViewNumber) -> Result<(), NetworkError> {
        self.inner.gc(Slot::new(*v)).map_err(to_network_error)
    }

    fn add_peers(
        &mut self,
        r: PeerRole,
        ps: Vec<(T::SignatureKey, Self::PeerData)>,
    ) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for (k, (x, a)) in ps {
            self.peers.insert(
                k,
                PeerConnectInfo {
                    x25519_key: x.into(),
                    p2p_addr: a.clone(),
                },
            );
            targets.push((x, a))
        }
        self.inner
            .add_peers(map_peer_role(r), targets)
            .map_err(to_network_error)?;
        Ok(())
    }

    fn remove_peers(&mut self, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for k in ps {
            if let Some(info) = self.peers.remove(k) {
                targets.push(info.x25519_key.into())
            }
        }
        self.inner.remove_peers(targets).map_err(to_network_error)?;
        Ok(())
    }

    fn assign_role(&mut self, r: PeerRole, ps: Vec<&T::SignatureKey>) -> Result<(), NetworkError> {
        let mut targets = Vec::new();
        for k in ps {
            if let Some(info) = self.peers.get(k) {
                targets.push(info.x25519_key.into())
            }
        }
        self.inner
            .assign_peers(map_peer_role(r), targets)
            .map_err(to_network_error)?;
        Ok(())
    }

    /// Update peers on every epoch change.
    ///
    /// For any given epoch `e` we collect the validators of `e`, `e-1` and
    /// `e+1` from the stake tables and merge their connection information.
    ///
    /// We keep validators that were in `e-1` but not in `e` for one additional
    /// epoch and eagerly connect to new validators of `e+1`.
    fn apply_epoch(
        &mut self,
        epoch: EpochNumber,
        coord: &EpochMembershipCoordinator<T>,
    ) -> Result<(), NetworkError> {
        if epoch <= self.epoch {
            info!(%epoch, ours = %self.epoch, "epoch already seen");
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
                if Some(new_info) != self.peers.get(k) {
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
        for (k, info) in &self.peers {
            if !(retained.contains(k) || wanted.contains(k)) {
                info!(%epoch, peer = %k, "removing network peer");
                to_del.push((k.clone(), info.clone()));
            }
        }

        for (k, _) in &to_del {
            self.peers.remove(k);
        }
        for (k, info) in &to_add {
            self.peers.insert(k.clone(), info.clone());
        }

        let add_targets: Vec<(PublicKey, NetAddr)> = to_add
            .iter()
            .map(|(_, i)| (i.x25519_key.into(), i.p2p_addr.clone()))
            .collect();
        let del_targets: Vec<PublicKey> = to_del.iter().map(|(_, i)| i.x25519_key.into()).collect();

        if let Err(err) = self.inner.add_peers(Role::Active, add_targets) {
            error!(%epoch, %err, "network down; could not add peers to network");
            return Err(to_network_error(err));
        }

        if let Err(err) = self.inner.remove_peers(del_targets) {
            error!(%epoch, %err, "network down; could not remove peers from network");
            return Err(to_network_error(err));
        }

        info!(%epoch, peers = %self.peers.len());

        self.epoch = epoch;
        Ok(())
    }
}

impl<T: NodeType> Cliquenet<T> {
    fn serialize(&self, m: &Message<T, Validated>) -> Result<Vec<u8>, NetworkError> {
        if let MessageType::External(bytes) = &m.message_type {
            return Ok(bytes.clone());
        }
        self.upgrade_lock
            .serialize(m)
            .map_err(|e| NetworkError::Io(format!("serialization error: {e}").into()))
    }

    fn deserialize(&self, bytes: &[u8]) -> Result<Message<T, Unchecked>, NetworkError> {
        match self
            .upgrade_lock
            .deserialize::<Message<T, Unchecked>>(bytes)
        {
            Ok((m, v)) => {
                if v == EXTERNAL_MESSAGE_VERSION && !m.is_external() {
                    let e = "received a non-external message with version 0.0".to_string();
                    return Err(NetworkError::Io(e.into()));
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
                Err(NetworkError::Io(primary_err.to_string().into()))
            },
        }
    }
}

fn map_peer_role(r: PeerRole) -> Role {
    match r {
        PeerRole::Active => Role::Active,
        PeerRole::Passive => Role::Passive,
    }
}

fn to_network_error(e: CliquenetError) -> NetworkError {
    match e {
        e @ CliquenetError::Bind(..) => NetworkError::Critical(e.into()),
        e @ CliquenetError::ChannelClosed => NetworkError::Critical(e.into()),
        e @ CliquenetError::BudgetClosed => NetworkError::Critical(e.into()),
        e => NetworkError::Io(e.into()),
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
