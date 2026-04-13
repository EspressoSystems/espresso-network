use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::StreamExt;
use hotshot::{traits::implementations::Cliquenet, types::BLSPubKey};
use hotshot_example_types::node_types::{TEST_VERSIONS, TestTypes};
use hotshot_new_protocol::{
    helpers::upgrade_lock,
    message::{ConsensusMessage, Message, MessageType, Validated},
};
use hotshot_testing::view_generator::TestViewGenerator;
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{VidDisperse, VidDisperseShare2, ViewNumber},
    traits::{metrics::NoMetrics, network::ConnectedNetwork, signature_key::SignatureKey},
    x25519::Keypair,
};
use tracing::info;

use crate::config::OrchestratorConfig;

/// Run the orchestrator: connect to network, wait for peers, inject genesis proposal.
pub async fn run(cfg: OrchestratorConfig) -> Result<()> {
    info!(total_nodes = cfg.total_nodes, "orchestrator starting");

    // Use a separate key identity for the orchestrator (index = total_nodes).
    let orch_index = cfg.total_nodes as u64;
    let (_orch_pk, orch_sk) = BLSPubKey::generated_from_seed_indexed([cfg.seed; 32], orch_index);
    let orch_pk = BLSPubKey::from_private(&orch_sk);
    let orch_keypair = Keypair::derive_from::<BLSPubKey>(&orch_sk);

    let bind_addr: NetAddr = cfg
        .bind_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address '{}': {e}", cfg.bind_addr))?;

    // Build peer list (all consensus nodes).
    let mut parties = Vec::new();
    for (i, addr_str) in cfg.peers.iter().enumerate() {
        let (_peer_pk, peer_sk) = BLSPubKey::generated_from_seed_indexed([cfg.seed; 32], i as u64);
        let peer_pk = BLSPubKey::from_private(&peer_sk);
        let peer_keypair = Keypair::derive_from::<BLSPubKey>(&peer_sk);
        let peer_addr: NetAddr = addr_str
            .parse()
            .map_err(|e| anyhow::anyhow!("invalid peer address '{addr_str}': {e}"))?;
        parties.push((
            peer_pk,
            PeerConnectInfo {
                x25519_key: peer_keypair.public_key(),
                p2p_addr: peer_addr,
            },
        ));
    }

    let net = Cliquenet::create(
        "orchestrator",
        orch_pk,
        orch_keypair,
        bind_addr,
        parties,
        Box::new(NoMetrics),
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to create cliquenet: {e}"))?;

    // Give nodes time to start up and connect.
    info!("waiting for nodes to connect...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Generate test data for view 1 using the same membership the nodes will use.
    let membership = crate::membership::make_membership(cfg.total_nodes).await;
    let keys = key_map(cfg.total_nodes, cfg.seed);
    let node_key_map = Arc::new(keys.clone());

    let mut generator =
        TestViewGenerator::generate(membership.clone(), node_key_map, TEST_VERSIONS.vid2);

    // Generate view 1 data.
    let gen_view = (&mut generator)
        .next()
        .await
        .expect("should generate view 1");

    let leader_pk = gen_view.leader_public_key;
    info!(%leader_pk, "view 1 leader identified, sending genesis proposals");

    // Extract VID disperse and shares.
    let VidDisperse::V2(vid_disperse) = gen_view.vid_disperse.data.clone() else {
        anyhow::bail!("expected V2 VID disperse");
    };

    let vid_shares: Vec<VidDisperseShare2<TestTypes>> = vid_disperse
        .shares
        .iter()
        .map(|(key, share)| VidDisperseShare2 {
            view_number: vid_disperse.view_number,
            epoch: vid_disperse.epoch,
            target_epoch: vid_disperse.target_epoch,
            payload_commitment: vid_disperse.payload_commitment,
            share: share.clone(),
            recipient_key: *key,
            common: vid_disperse.common.clone(),
        })
        .collect();

    // Convert the quorum proposal into the new protocol's proposal format.
    let inner_proposal = hotshot_types::message::Proposal {
        data: gen_view.quorum_proposal.data.clone().into(),
        signature: gen_view.quorum_proposal.signature.clone(),
        _pd: std::marker::PhantomData,
    };

    // Send a proposal message to each node with its specific VID share.
    // We construct as `Validated` and serialize — the `Validated`/`Unchecked` marker
    // is a zero-sized PhantomData with `#[serde(skip)]`, so the bytes are identical
    // and will deserialize as `Unchecked` on the receiving node.
    let lock = upgrade_lock::<TestTypes>();
    for vid_share in &vid_shares {
        let proposal_msg = hotshot_new_protocol::message::ProposalMessage::validated(
            inner_proposal.clone(),
            vid_share.clone(),
        );
        let message: Message<TestTypes, Validated> = Message {
            sender: leader_pk,
            message_type: MessageType::Consensus(ConsensusMessage::Proposal(proposal_msg)),
        };

        let bytes = lock
            .serialize(&message)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        net.direct_message(ViewNumber::new(1), bytes, vid_share.recipient_key)
            .await
            .map_err(|e| anyhow::anyhow!("failed to send proposal: {e}"))?;
    }

    info!("genesis proposal broadcast complete, consensus should start");
    info!(
        target_views = cfg.target_views,
        "orchestrator waiting for benchmark to complete (Ctrl+C to stop)"
    );

    tokio::signal::ctrl_c().await?;
    info!("orchestrator shutting down");

    Ok(())
}

fn key_map(
    num_nodes: usize,
    seed: u8,
) -> std::collections::BTreeMap<BLSPubKey, hotshot::types::BLSPrivKey> {
    let mut map = std::collections::BTreeMap::new();
    for i in 0..num_nodes {
        let (public_key, private_key) =
            BLSPubKey::generated_from_seed_indexed([seed; 32], i as u64);
        map.insert(public_key, private_key);
    }
    map
}
