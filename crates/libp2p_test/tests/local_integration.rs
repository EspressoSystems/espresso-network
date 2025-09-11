//! Local integration test: spawns 1 sender and 3 receivers in-process

use std::{str::FromStr, sync::Arc, time::Duration};

use hotshot_example_types::node_types::TestTypes;
use hotshot_types::{
    signature_key::{BLSPrivKey, BLSPubKey},
    traits::signature_key::SignatureKey,
};
use libp2p::Multiaddr;
use libp2p_identity::{ed25519, ed25519::SecretKey, Keypair, PeerId};
use libp2p_test::{
    config::{Libp2pTest, TransportProtocol},
    run_receiver, run_sender, AppConfig,
};
use tokio::{sync::Barrier, task::JoinHandle, time::sleep};
use tracing::{error, info};

fn make_listen_string(port: u64, protocol: &TransportProtocol) -> String {
    match protocol {
        TransportProtocol::Tcp { .. } => format!("/ip4/127.0.0.1/tcp/{port}"),
        TransportProtocol::Quic => format!("/ip4/127.0.0.1/udp/{port}/quic-v1"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_libp2p_test() {
    local_sender_and_receivers(None).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_tcp_ping_test() {
    local_sender_and_receivers(Some(Libp2pTest::Ping {
        transport_protocol: TransportProtocol::default(),
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_quic_ping_test() {
    local_sender_and_receivers(Some(Libp2pTest::Ping {
        transport_protocol: TransportProtocol::Quic,
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_tcp_rr_test() {
    local_sender_and_receivers(Some(Libp2pTest::RequestResponse {
        transport_protocol: TransportProtocol::default(),
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_quic_rr_test() {
    local_sender_and_receivers(Some(Libp2pTest::RequestResponse {
        transport_protocol: TransportProtocol::Quic,
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_tcp_gossip_test() {
    local_sender_and_receivers(Some(Libp2pTest::Gossipsub {
        transport_protocol: TransportProtocol::default(),
    }))
    .await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn local_quic_gossip_test() {
    local_sender_and_receivers(Some(Libp2pTest::Gossipsub {
        transport_protocol: TransportProtocol::Quic,
    }))
    .await;
}

async fn local_sender_and_receivers(maybe_libp2p_test: Option<Libp2pTest>) {
    tracing_subscriber::fmt::init();
    let base_port = 9000;
    let peers: Vec<_> = (base_port..base_port + 4)
        .map(|port| {
            let (_, private_key) = BLSPubKey::generated_from_seed_indexed([0; 32], port);
            let listen = Multiaddr::from_str(&make_listen_string(
                port,
                maybe_libp2p_test
                    .as_ref()
                    .map(|t| t.transport_protocol())
                    .unwrap_or(&TransportProtocol::Quic),
            ))
            .unwrap();
            info!("listen address: {}", listen);
            let mut listen_clone = listen.clone();
            while let Some(protocol) = listen_clone.pop() {
                info!("listening protocol: {}", protocol);
            }
            (private_key, listen)
        })
        .collect();
    let barrier = Arc::new(Barrier::new(peers.len()));
    let mut handles: Vec<JoinHandle<()>> = Vec::new();
    let peer_ids: Vec<_> = peers[..]
        .iter()
        .map(|(key, addr)| {
            (
                peer_id_from_priv_key(key),
                Multiaddr::from_str(
                    &addr
                        .clone()
                        .to_string()
                        .replace("ip4", "dns4")
                        .replace("127.0.0.1", "localhost"),
                )
                .unwrap(),
            )
        })
        .collect();

    // Spawn 3 receivers
    for (i, (private_key, addr)) in peers.iter().enumerate().skip(1) {
        let barrier = barrier.clone();
        let mut receiver_peers = peer_ids.clone();
        receiver_peers.remove(i);
        handles.push(tokio::spawn({
            let private_key = private_key.to_tagged_base64().unwrap();
            let addr = addr.clone();
            let maybe_libp2p_test_clone = maybe_libp2p_test.clone();
            async move {
                let config = AppConfig {
                    listen: addr,
                    private_key,
                    peers: receiver_peers,
                    send_mode: false,
                    message: None,
                    libp2p_test: maybe_libp2p_test_clone,
                };
                barrier.wait().await;
                info!(
                    "Spawning simple node with config:\n{}",
                    toml::to_string(&config).unwrap()
                );
                if let Err(e) = run_receiver::<TestTypes>(config).await {
                    error!("Receiver error: {}", e);
                }
            }
        }));
    }

    // Spawn sender
    let mut sender_peers: Vec<_> = peer_ids.clone();
    sender_peers.remove(0);
    let barrier = barrier.clone();
    handles.push(tokio::spawn(async move {
        let config = AppConfig {
            listen: peers[0].1.clone(),
            private_key: peers[0].0.to_tagged_base64().unwrap(),
            peers: sender_peers,
            send_mode: true,
            message: Some("test-message".to_string()),
            libp2p_test: maybe_libp2p_test,
        };
        barrier.wait().await;
        info!(
            "Spawning simple node with config:\n{}",
            toml::to_string(&config).unwrap()
        );
        if let Err(e) = run_sender::<TestTypes>(config).await {
            error!("Sender error: {}", e);
        }
    }));

    // Sleep to let the test run
    sleep(Duration::from_secs(60)).await;

    // Abort all tasks and finish
    for h in handles.into_iter() {
        h.abort();
    }
}

fn peer_id_from_priv_key(key: &BLSPrivKey) -> PeerId {
    let derived_key = blake3::derive_key("libp2p key", &key.to_bytes());
    let derived_key = SecretKey::try_from_bytes(derived_key).unwrap();

    // Create an `ed25519` keypair from the derived key
    let keypair: Keypair = ed25519::Keypair::from(derived_key).into();
    keypair.public().to_peer_id()
}
