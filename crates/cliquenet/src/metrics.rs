use std::{collections::HashMap, fmt::Display, hash::Hash, sync::Arc, time::Duration};

use hotshot_types::traits::metrics::{Counter, CounterFamily, Gauge, GaugeFamily, Metrics};

const CONNECT_ATTEMPTS: &str = "connect_attempts";
const LATENCY: &str = "latency_ms";
const PEER_OQUEUE_CAP: &str = "peer_oqueue_cap";
const PEER_IQUEUE_CAP: &str = "peer_iqueue_cap";

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct NetworkMetrics<K> {
    pub connections: Box<dyn Gauge>,
    pub iqueue: Box<dyn Gauge>,
    pub oqueue: Box<dyn Gauge>,
    peer_counter_fams: HashMap<&'static str, Arc<dyn CounterFamily>>,
    peer_gauge_fams: HashMap<&'static str, Arc<dyn GaugeFamily>>,
    connects: HashMap<K, Box<dyn Counter>>,
    latencies: HashMap<K, Box<dyn Gauge>>,
    peer_oqueues: HashMap<K, Box<dyn Gauge>>,
    peer_iqueues: HashMap<K, Box<dyn Gauge>>,
}

impl<K> NetworkMetrics<K>
where
    K: Display + Eq + Hash + Clone,
{
    pub fn new<P>(label: &str, metrics: &dyn Metrics, parties: P) -> Self
    where
        P: IntoIterator<Item = K>,
    {
        let group = metrics.subgroup(format!("cliquenet_{label}"));

        let peers = vec!["peers".into()];

        let mut cf: HashMap<&'static str, Arc<dyn CounterFamily>> = HashMap::new();
        cf.insert(
            CONNECT_ATTEMPTS,
            group
                .counter_family(CONNECT_ATTEMPTS.into(), peers.clone())
                .into(),
        );

        let mut gf: HashMap<&'static str, Arc<dyn GaugeFamily>> = HashMap::new();
        gf.insert(
            LATENCY,
            group.gauge_family(LATENCY.into(), peers.clone()).into(),
        );
        gf.insert(
            PEER_OQUEUE_CAP,
            group
                .gauge_family(PEER_OQUEUE_CAP.into(), peers.clone())
                .into(),
        );
        gf.insert(
            PEER_IQUEUE_CAP,
            group.gauge_family(PEER_IQUEUE_CAP.into(), peers).into(),
        );

        let connects = peer_counters(&*cf[CONNECT_ATTEMPTS], parties);

        Self {
            connections: group.create_gauge("connections".into(), None),
            iqueue: group.create_gauge("iqueue_cap".into(), None),
            oqueue: group.create_gauge("oqueue_cap".into(), None),
            latencies: peer_gauges(&*gf[LATENCY], connects.keys().cloned()),
            peer_oqueues: peer_gauges(&*gf[PEER_OQUEUE_CAP], connects.keys().cloned()),
            peer_iqueues: peer_gauges(&*gf[PEER_IQUEUE_CAP], connects.keys().cloned()),
            connects,
            peer_counter_fams: cf,
            peer_gauge_fams: gf,
        }
    }

    pub fn add_connect_attempt(&self, k: &K) {
        if let Some(c) = self.connects.get(k) {
            c.add(1)
        }
    }

    pub fn set_latency(&self, k: &K, d: Duration) {
        if let Some(g) = self.latencies.get(k) {
            g.set(d.as_millis() as usize)
        }
    }

    pub fn set_peer_oqueue_cap(&self, k: &K, n: usize) {
        if let Some(g) = self.peer_oqueues.get(k) {
            g.set(n)
        }
    }

    pub fn set_peer_iqueue_cap(&self, k: &K, n: usize) {
        if let Some(g) = self.peer_iqueues.get(k) {
            g.set(n)
        }
    }

    pub fn add_parties<P>(&mut self, parties: P)
    where
        P: IntoIterator<Item = K>,
    {
        for k in parties {
            if !self.connects.contains_key(&k) {
                let c = self.peer_counter_fams[CONNECT_ATTEMPTS].create(vec![k.to_string()]);
                self.connects.insert(k.clone(), c);
            }
            if !self.latencies.contains_key(&k) {
                let g = self.peer_gauge_fams[LATENCY].create(vec![k.to_string()]);
                self.latencies.insert(k.clone(), g);
            }
            if !self.peer_oqueues.contains_key(&k) {
                let g = self.peer_gauge_fams[PEER_OQUEUE_CAP].create(vec![k.to_string()]);
                self.peer_oqueues.insert(k.clone(), g);
            }
            if !self.peer_iqueues.contains_key(&k) {
                let g = self.peer_gauge_fams[PEER_IQUEUE_CAP].create(vec![k.to_string()]);
                self.peer_iqueues.insert(k, g);
            }
        }
    }

    pub fn remove_parties<'a, P>(&mut self, parties: P)
    where
        P: IntoIterator<Item = &'a K>,
        K: 'a,
    {
        // TODO: Counters and gauges should be de-registered.
        for k in parties {
            self.connects.remove(k);
            self.latencies.remove(k);
            self.peer_oqueues.remove(k);
            self.peer_iqueues.remove(k);
        }
    }
}

fn peer_counters<P, K>(fam: &dyn CounterFamily, peers: P) -> HashMap<K, Box<dyn Counter>>
where
    P: IntoIterator<Item = K>,
    K: Display + Eq + Hash + Clone,
{
    peers
        .into_iter()
        .map(|k| {
            let c = fam.create(vec![k.to_string()]);
            (k, c)
        })
        .collect()
}

fn peer_gauges<P, K>(fam: &dyn GaugeFamily, peers: P) -> HashMap<K, Box<dyn Gauge>>
where
    P: IntoIterator<Item = K>,
    K: Display + Eq + Hash + Clone,
{
    peers
        .into_iter()
        .map(|k| {
            let c = fam.create(vec![k.to_string()]);
            (k, c)
        })
        .collect()
}
