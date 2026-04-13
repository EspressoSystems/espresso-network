use std::{collections::BTreeMap, time::Duration};

use anyhow::Result;
use committable::Committable;
use hotshot::{
    traits::{BlockPayload, implementations::Cliquenet},
    types::{BLSPrivKey, BLSPubKey},
};
use hotshot_example_types::{
    block_types::TestBlockPayload,
    node_types::TestTypes,
    state_types::{TestInstanceState, TestValidatedState},
};
use hotshot_new_protocol::{
    helpers::upgrade_lock,
    message::{ConsensusMessage, Message, MessageType, Proposal, ProposalMessage, Validated},
};
use hotshot_testing::helpers::build_cert;
use hotshot_types::{
    PeerConnectInfo,
    addr::NetAddr,
    data::{EpochNumber, Leaf2, VidDisperse, VidDisperseShare2, ViewNumber},
    message::Proposal as SignedProposal,
    simple_certificate::QuorumCertificate2,
    simple_vote::{QuorumData2, QuorumVote2},
    traits::{
        block_contents::BlockHeader, metrics::NoMetrics, network::ConnectedNetwork,
        signature_key::SignatureKey,
    },
    x25519::Keypair,
};
use tracing::info;

use crate::config::OrchestratorConfig;

/// Run the orchestrator: connect to network, wait for peers, inject genesis proposal.
pub async fn run(cfg: OrchestratorConfig) -> Result<()> {
    info!(total_nodes = cfg.total_nodes, "orchestrator starting");

    // Use a separate key identity for the orchestrator (index = total_nodes).
    let orch_index = cfg.total_nodes as u64;
    let (_orch_pk, orch_sk) =
        <BLSPubKey as SignatureKey>::generated_from_seed_indexed([cfg.seed; 32], orch_index);
    let orch_pk = BLSPubKey::from_private(&orch_sk);
    let orch_keypair = Keypair::derive_from::<BLSPubKey>(&orch_sk);

    let bind_addr: NetAddr = cfg
        .bind_addr
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid bind address '{}': {e}", cfg.bind_addr))?;

    // Build peer list (all consensus nodes).
    let mut parties = Vec::new();
    for (i, addr_str) in cfg.peers.iter().enumerate() {
        let (_peer_pk, peer_sk) =
            <BLSPubKey as SignatureKey>::generated_from_seed_indexed([cfg.seed; 32], i as u64);
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
        "bench",
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

    // Build membership (same as nodes use).
    let membership = crate::membership::make_membership(cfg.total_nodes, cfg.seed).await;

    // Build key map for TestViewGenerator.
    let key_map: BTreeMap<BLSPubKey, BLSPrivKey> = (0..cfg.total_nodes as u64)
        .map(|i| {
            let (pk, sk) = BLSPubKey::generated_from_seed_indexed([0u8; 32], i);
            (pk, sk)
        })
        .collect();

    let instance = TestInstanceState::default();
    let validated_state = TestValidatedState::default();
    let lock = upgrade_lock::<TestTypes>();

    // Create genesis leaf.
    let genesis_leaf = Leaf2::<TestTypes>::genesis(
        &validated_state,
        &instance,
        hotshot_example_types::node_types::TEST_VERSIONS.test.base,
    )
    .await;
    let genesis_leaf_commit = <Leaf2<TestTypes> as Committable>::commit(&genesis_leaf);

    // Determine view-1 leader.
    let view_1 = ViewNumber::new(1);
    let epoch = EpochNumber::genesis();
    let epoch_membership = membership
        .membership_for_epoch(Some(epoch))
        .await
        .map_err(|e| anyhow::anyhow!("failed to get epoch membership: {e}"))?;
    let leader_pk: BLSPubKey = epoch_membership
        .leader(view_1)
        .await
        .map_err(|e| anyhow::anyhow!("failed to determine view-1 leader: {e}"))?;

    let leader_sk = key_map
        .get(&leader_pk)
        .ok_or_else(|| anyhow::anyhow!("view-1 leader not found in key map"))?;

    info!(%leader_pk, "view 1 leader identified");

    // Build genesis justify QC (Certificate1 over genesis leaf at view 0).
    let qc_data = QuorumData2::<TestTypes> {
        leaf_commit: genesis_leaf_commit,
        epoch: Some(epoch),
        block_number: Some(BlockHeader::<TestTypes>::block_number(
            genesis_leaf.block_header(),
        )),
    };
    let justify_qc: QuorumCertificate2<TestTypes> = build_cert::<
        TestTypes,
        QuorumData2<TestTypes>,
        QuorumVote2<TestTypes>,
        QuorumCertificate2<TestTypes>,
    >(
        qc_data,
        &epoch_membership,
        ViewNumber::genesis(),
        &leader_pk,
        leader_sk,
        &lock,
    )
    .await;

    // Create empty payload for view 1.
    let (payload, metadata) = <TestBlockPayload as BlockPayload<TestTypes>>::from_transactions(
        [],
        &validated_state,
        &instance,
    )
    .await
    .map_err(|e| anyhow::anyhow!("failed to create empty payload: {e}"))?;

    // Compute VID disperse for view-1 payload.
    let vid_result = VidDisperse::calculate_vid_disperse(
        &payload,
        &membership,
        view_1,
        Some(epoch),
        Some(epoch),
        &metadata,
        &lock,
    )
    .await
    .map_err(|e| anyhow::anyhow!("VID disperse failed: {e}"))?;

    let VidDisperse::V2(vid_disperse) = vid_result.disperse else {
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

    // Build the view-1 header with the VID commitment from the disperse above.
    let payload_commitment =
        hotshot_types::data::VidCommitment::V2(vid_disperse.payload_commitment);
    let builder_commitment =
        <TestBlockPayload as BlockPayload<TestTypes>>::builder_commitment(&payload, &metadata);

    use hotshot_example_types::block_types::TestBlockHeader;
    let version = lock.version_infallible(view_1);
    let header = TestBlockHeader::new(
        &genesis_leaf,
        payload_commitment,
        builder_commitment,
        metadata,
        version,
    );

    // Build the new-protocol Proposal.
    let proposal = Proposal::<TestTypes> {
        block_header: header,
        view_number: view_1,
        epoch,
        justify_qc,
        next_epoch_justify_qc: None,
        upgrade_certificate: None,
        view_change_evidence: None,
        next_drb_result: None,
        state_cert: None,
    };

    // Sign the proposal (leader signs the leaf commitment).
    let leaf: Leaf2<TestTypes> = proposal.clone().into();
    let leaf_commit = <Leaf2<TestTypes> as Committable>::commit(&leaf);
    let signature = BLSPubKey::sign(leader_sk, leaf_commit.as_ref())
        .map_err(|e| anyhow::anyhow!("failed to sign proposal: {e}"))?;

    let signed_proposal = SignedProposal {
        data: proposal,
        signature,
        _pd: std::marker::PhantomData,
    };

    info!("sending genesis proposals to {} nodes", vid_shares.len());

    // Send a proposal message to each node with its specific VID share.
    for vid_share in &vid_shares {
        let proposal_msg = ProposalMessage::validated(signed_proposal.clone(), vid_share.clone());
        let message: Message<TestTypes, Validated> = Message {
            sender: leader_pk,
            message_type: MessageType::Consensus(ConsensusMessage::Proposal(proposal_msg)),
        };

        let bytes = lock
            .serialize(&message)
            .map_err(|e| anyhow::anyhow!("serialization failed: {e}"))?;
        net.direct_message(view_1, bytes, vid_share.recipient_key)
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
