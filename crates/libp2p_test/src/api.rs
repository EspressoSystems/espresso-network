use std::{
    collections::{HashMap, HashSet},
    num::NonZeroUsize,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Result;
use bimap::BiMap;
use hotshot_libp2p_networking::network::{
    behaviours::dht::store::persistent::DhtNoPersistence, spawn_network_node, NetworkEvent,
    NetworkNodeConfigBuilder,
};
use hotshot_types::{signature_key::BLSPrivKey, traits::node_implementation::NodeType};
use libp2p::{
    futures::StreamExt, noise, ping, ping::Behaviour, swarm::SwarmEvent, tcp, yamux, Swarm,
};
use libp2p_identity::{ed25519, ed25519::SecretKey, Keypair};
use parking_lot::Mutex;
use tokio::time::sleep;
use tracing::{error, info};

use crate::config::{AppConfig, PingProtocol};

pub const NODE_ID: usize = 0;

pub async fn run_sender<T: NodeType>(config: AppConfig) -> Result<()> {
    info!("Starting as sender");
    if config.ping.is_some() {
        run_ping_sender(config).await
    } else {
        run_libp2p_sender::<T>(config).await
    }
}

pub async fn run_receiver<T: NodeType>(config: AppConfig) -> Result<()> {
    info!("Starting as receiver");
    if config.ping.is_some() {
        run_ping_receiver(config).await
    } else {
        run_libp2p_receiver::<T>(config).await
    }
}

pub async fn spawn_simple_node<T: NodeType>(
    config: &AppConfig,
) -> Result<(
    hotshot_libp2p_networking::network::NetworkNodeHandle<T>,
    hotshot_libp2p_networking::network::NetworkNodeReceiver,
)> {
    let libp2p_keypair = keypair_from_priv_key(&config.private_key.clone().try_into()?)?;
    let dht = DhtNoPersistence;
    let consensus_key_to_pid_map = Arc::new(Mutex::new(BiMap::new()));
    let replication_factor = NonZeroUsize::new((2 * config.peers.len()).div_ceil(3)).unwrap();
    let network_config = NetworkNodeConfigBuilder::default()
        .keypair(libp2p_keypair)
        .replication_factor(replication_factor)
        .bind_address(Some(config.listen.clone()))
        .to_connect_addrs(HashSet::from_iter(config.clone().peers.into_iter()))
        .republication_interval(None)
        .build()
        .expect("Failed to build network node config");
    let peers = network_config.to_connect_addrs.clone();
    let _peers_num = peers.len();
    let (receiver, handle) =
        spawn_network_node(network_config, dht, consensus_key_to_pid_map, NODE_ID).await?;
    // handle.wait_to_connect(peers_num, NODE_ID).await?;
    // info!(
    //     "Connected to {} peers, node: {}",
    //     peers_num,
    //     config.listen.to_string()
    // );
    Ok((handle, receiver))
}

pub fn keypair_from_priv_key(private_key: &BLSPrivKey) -> Result<Keypair> {
    let derived_key = blake3::derive_key("libp2p key", &private_key.to_bytes());
    let derived_key = SecretKey::try_from_bytes(derived_key)?;
    Ok(ed25519::Keypair::from(derived_key).into())
}

pub async fn run_ping_sender(config: AppConfig) -> Result<()> {
    let peers_map: HashMap<_, _> = config.peers.clone().into_iter().collect();
    let mut swarm = spawn_ping_swarm(&config)?;
    let _ = swarm.listen_on(config.listen)?;
    loop {
        for (peer_id, addr) in config.peers.iter() {
            sleep(Duration::from_secs(1)).await;
            info!("Dialing {}", addr.to_string());
            if let Err(e) = swarm.dial(addr.clone()) {
                error!("Failed to ping to {}: {}", addr, e);
            }
            let mut retries = 5;
            let peer = loop {
                match swarm.select_next_some().await {
                    SwarmEvent::Behaviour(event) => {
                        let peer_addr = peers_map.get(&event.peer).unwrap();
                        info!(
                            "Time reported by ping for {}: {:?}",
                            peer_addr.to_string(),
                            event.result
                        );
                        if peer_id == &event.peer {
                            retries -= 1;
                        }
                        if retries == 0 {
                            break event.peer;
                        }
                    },
                    e => {
                        info!("Received unexpected swarm event: {e:?}");
                    },
                }
            };
            info!("Disconnecting from {}", addr.to_string());
            let _ = swarm.disconnect_peer_id(peer);
        }
    }
}

pub fn spawn_ping_swarm(config: &AppConfig) -> Result<Swarm<Behaviour>> {
    let libp2p_keypair = keypair_from_priv_key(&config.private_key.clone().try_into()?)?;
    let ping_config = ping::Config::new().with_interval(Duration::from_secs(1));
    match config.ping.as_ref() {
        Some(PingProtocol::Quic) => {
            Ok(libp2p::SwarmBuilder::with_existing_identity(libp2p_keypair)
                .with_tokio()
                .with_quic()
                .with_dns()?
                .with_behaviour(|_| ping::Behaviour::new(ping_config))?
                .with_swarm_config(|cfg| {
                    cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
                })
                .build())
        },
        Some(PingProtocol::Tcp { auth: _, mplex: _ }) => {
            Ok(libp2p::SwarmBuilder::with_existing_identity(libp2p_keypair)
                .with_tokio()
                .with_tcp(
                    tcp::Config::default(),
                    noise::Config::new,
                    yamux::Config::default,
                )?
                .with_dns()?
                .with_behaviour(|_| ping::Behaviour::new(ping_config))?
                .with_swarm_config(|cfg| {
                    cfg.with_idle_connection_timeout(Duration::from_secs(u64::MAX))
                })
                .build())
        },
        None => Err(anyhow::anyhow!("No ping protocol specified")),
    }
}

pub async fn run_ping_receiver(config: AppConfig) -> Result<()> {
    let mut swarm = spawn_ping_swarm(&config)?;
    let _ = swarm.listen_on(config.listen)?;
    loop {
        match swarm.select_next_some().await {
            SwarmEvent::Behaviour(event) => {
                info!("Received ping: {event:?}");
            },
            e => {
                info!("Received unexpected swarm event: {e:?}");
            },
        }
    }
}

pub async fn run_libp2p_receiver<T: NodeType>(config: AppConfig) -> Result<()> {
    let (handle, mut receiver) = spawn_simple_node::<T>(&config).await?;
    loop {
        match receiver.recv().await {
            Ok(ev) => {
                if let hotshot_libp2p_networking::network::NetworkEvent::DirectRequest(
                    _,
                    _peer_id,
                    chan,
                ) = ev
                {
                    let reply = config.listen.to_string();
                    handle.direct_response(chan, reply.as_bytes())?;
                    info!("Received and replied with {reply}");
                } else {
                    info!("Receiver received unexpected event: {ev:?}");
                }
            },
            Err(e) => {
                error!("Receiver error: {:?}", e);
                break;
            },
        }
    }
    Ok(())
}

pub async fn run_libp2p_sender<T: NodeType>(config: AppConfig) -> Result<()> {
    let (handle, mut receiver) = spawn_simple_node::<T>(&config).await?;
    let msg = config.message.clone().unwrap_or_default().into_bytes();
    loop {
        sleep(Duration::from_secs(1)).await;
        let mut roundtrips = Vec::new();
        for (peer_id, addr) in config.peers.iter() {
            info!("Sending request to {}", addr.to_string());
            let start = Instant::now();
            if let Err(e) = handle.direct_request_no_serialize(*peer_id, msg.clone()) {
                error!("Failed to send request to {}: {}", peer_id, e);
            }
            loop {
                match receiver.recv().await {
                    Ok(NetworkEvent::DirectResponse(_, pid)) if &pid == peer_id => {
                        let elapsed = start.elapsed();
                        roundtrips.push((addr.to_string(), elapsed));
                        info!(
                            "Reply from {}: {} in {:?}",
                            peer_id,
                            addr.to_string(),
                            elapsed
                        );
                        break;
                    },
                    Ok(ev) => {
                        info!("Sender received unexpected event: {ev:?}");
                    },
                    Err(e) => {
                        error!("Receiver error: {:?}", e);
                        break;
                    },
                }
            }
        }
        for (sender, elapsed) in roundtrips {
            println!("Reply from {sender}: {elapsed:?}");
        }
    }
}
