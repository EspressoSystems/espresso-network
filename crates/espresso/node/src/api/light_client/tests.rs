use std::{marker::PhantomData, sync::Mutex};

use axum::{Router, extract::Path, http::HeaderMap, routing::get};
use committable::Committable;
use disco_types::error::Error;
use espresso_types::BLOCK_MERKLE_TREE_HEIGHT;
use futures::future::join_all;
use hotshot_query_service::{
    availability::{BlockQueryData, TransactionIndex, VidCommonQueryData},
    data_source::{
        Transaction,
        storage::{AvailabilityStorage, UpdateAvailabilityStorage, pruning::PrunerCfg},
    },
    fetching::provider::TrustedQueryServiceProvider,
    merklized_state::{MerklizedState, UpdateStateData},
};
use hotshot_types::{data::ViewNumber, simple_vote::Vote2Data};
use http_wire::{ServerError, respond};
use itertools::Itertools;
use jf_merkle_tree_compat::{AppendableMerkleTreeScheme, ToTraversalPath};
use light_client::{
    consensus::leaf::{FinalityProof, LeafProofHint},
    testing::{
        AlwaysTrueQuorum, TestClient, VersionCheckQuorum, custom_leaf_chain_with_upgrade,
        custom_leaf_chain_with_upgrades, leaf_chain, leaf_chain_with_upgrade,
    },
};
use tokio::{
    net::TcpListener,
    time::{sleep, timeout},
};
use vbs::version::Version;
use versions::{
    DRB_AND_HEADER_UPGRADE_VERSION, EPOCH_REWARD_VERSION, EPOCH_VERSION, FEE_VERSION,
    NEW_PROTOCOL_VERSION, Upgrade,
};

use super::{
    qs::{Config, SqlStorage},
    sql::DataSource,
    *,
};
use crate::api::data_source::{
    Provider, SequencerDataSource, testing::TestableSequencerDataSource,
};

const CHAIN_LIMIT: usize = 500;

fn cert2_for_leaf(leaf: &LeafQueryData<SeqTypes>) -> espresso_types::Certificate2<SeqTypes> {
    let data = Vote2Data {
        leaf_commit: leaf.leaf().commit(),
        epoch: leaf.qc().data.epoch.unwrap(),
        block_number: leaf.height(),
    };
    espresso_types::Certificate2::new(
        data.clone(),
        data.commit(),
        leaf.leaf().view_number(),
        None,
        PhantomData,
    )
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_two_chain() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Insert some leaves, forming a chain.
    let leaves = leaf_chain(1..=3, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.insert_leaf(&leaves[1]).await.unwrap();
        tx.insert_leaf(&leaves[2]).await.unwrap();
        tx.commit().await.unwrap();
    }

    // Ask for the first leaf; it is proved finalized by the chain formed along with the second.
    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_finalized() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Insert a single leaf. We will not be able to provide proofs ending in a leaf chain, but
    // we can return a leaf if the leaf after it is already known to be finalized.
    let leaves = leaf_chain(1..=2, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_finalized_assumption(
        &ds,
        leaves[0].clone(),
        2,
        Duration::MAX,
        CHAIN_LIMIT,
    )
    .await
    .unwrap();
    assert_eq!(
        proof
            .verify(LeafProofHint::assumption(leaves[1].leaf()))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_finalized_assumption() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    let leaves = leaf_chain(1..=2, NEW_PROTOCOL_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_finalized_assumption(
        &ds,
        leaves[0].clone(),
        2,
        Duration::MAX,
        CHAIN_LIMIT,
    )
    .await
    .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::Assumption));
    assert_eq!(
        proof
            .verify(LeafProofHint::assumption(leaves[1].leaf()))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_cert2() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    let leaves = leaf_chain(1..=2, NEW_PROTOCOL_VERSION).await;
    let cert2_leaf = &leaves[1];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.insert_leaf(cert2_leaf).await.unwrap();
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_cert2_chain_limit() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // A proof for the first leaf must extend to the nearest cert2, three leaves away.
    let leaves = leaf_chain(1..=4, NEW_PROTOCOL_VERSION).await;
    let cert2_leaf = &leaves[3];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    let err = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::MAX, 2)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    let proof = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::MAX, 3)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_new_protocol_hint_vs_cert2() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    let leaves = leaf_chain(1..=5, NEW_PROTOCOL_VERSION).await;
    let cert2_leaf = &leaves[1];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    // The cert2 at height 2 beats the hint at height 5, so the hint is ignored.
    let proof = get_leaf_proof(&ds, leaves[0].clone(), Some(5), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );

    // A hint at height 3 ties the cert2 proof length, so the hint wins.
    let proof = get_leaf_proof(&ds, leaves[0].clone(), Some(3), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::Assumption));
    assert_eq!(
        proof
            .verify(LeafProofHint::assumption(leaves[2].leaf()))
            .await
            .unwrap(),
        leaves[0]
    );

    // With no cert2 at or after the requested height, the hint is honored.
    let proof = get_leaf_proof(&ds, leaves[2].clone(), Some(5), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::Assumption));
    assert_eq!(
        proof
            .verify(LeafProofHint::assumption(leaves[4].leaf()))
            .await
            .unwrap(),
        leaves[2]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_distant_hint_falls_through() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Legacy leaves 1-3 upgrade to the new protocol at height 4, with a cert2 at height 5, so
    // both fall-through paths have a bounded proof available.
    let leaves = leaf_chain_with_upgrade(
        1..=5,
        4,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
    )
    .await;
    let cert2_leaf = &leaves[4];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    // A hint more than `chain_limit` past a legacy leaf is ignored in favor of a QC chain.
    let proof = get_leaf_proof(
        &ds,
        leaves[0].clone(),
        Some(1000),
        Duration::MAX,
        CHAIN_LIMIT,
    )
    .await
    .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );

    // A hint more than `chain_limit` past a new-protocol leaf is ignored in favor of cert2.
    let proof = get_leaf_proof(
        &ds,
        leaves[3].clone(),
        Some(1000),
        Duration::MAX,
        CHAIN_LIMIT,
    )
    .await
    .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[3]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_qc_chain_chain_limit() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Proving the first leaf finalized requires walking the two subsequent leaves.
    let leaves = leaf_chain(1..=3, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    let err = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, 1)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, 2)
        .await
        .unwrap();
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_finalized_hint_too_far() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    let leaves = leaf_chain(1..=2, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let err =
        get_leaf_proof_with_finalized_assumption(&ds, leaves[0].clone(), 1000, Duration::MAX, 10)
            .await
            .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_qc_chain_new_protocol_cutover() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Upgrade mid-chain and skew view numbers so no HotStuff 2-chain ever forms; the proof
    // for the first (pre-upgrade) leaf must fall through to cert2 finality.
    let leaves = custom_leaf_chain_with_upgrade(
        1..=4,
        2,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
        |proposal| {
            proposal.view_number = ViewNumber::new(proposal.block_header.height() * 2);
        },
    )
    .await;
    assert_eq!(leaves[0].header().version(), DRB_AND_HEADER_UPGRADE_VERSION);
    assert_eq!(leaves[1].header().version(), NEW_PROTOCOL_VERSION);
    let cert2_leaf = &leaves[3];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_qc_chain_new_protocol_cutover_two_chain() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Upgrade at height 3: the HotStuff2 chain begun before the cutover completes without
    // needing a cert2 (none is stored).
    let leaves = leaf_chain_with_upgrade(
        1..=3,
        3,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
    )
    .await;
    assert_eq!(leaves[1].header().version(), DRB_AND_HEADER_UPGRADE_VERSION);
    assert_eq!(leaves[2].header().version(), NEW_PROTOCOL_VERSION);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&VersionCheckQuorum::new(
                leaves.iter().map(|leaf| leaf.leaf().clone())
            )))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_qc_chain_last_legacy_leaf_uses_cert2() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Upgrade at height 2: any 2-chain proving the last legacy leaf spans the cutover, so the
    // prover must fall through to the cert2 at height 3 instead.
    let leaves = leaf_chain_with_upgrade(
        1..=4,
        2,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
    )
    .await;
    assert_eq!(leaves[0].header().version(), DRB_AND_HEADER_UPGRADE_VERSION);
    assert_eq!(leaves[1].header().version(), NEW_PROTOCOL_VERSION);

    let cert2_leaf = &leaves[2];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&VersionCheckQuorum::new(
                leaves.iter().map(|leaf| leaf.leaf().clone())
            )))
            .await
            .unwrap(),
        leaves[0]
    );
}

/// With a gapless cutover, the leaf two before the cutover (`upgrade_height
/// - 2`) is finalized by the old protocol: its HotStuff2 2-chain completes
/// within pre-cutover leaves (deciding QC belongs to `upgrade_height - 1`,
/// still V5), so no cert2 is needed.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_leaf_before_cutover_uses_hotstuff2() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Cutover at height 4, so `upgrade_height - 2` is height 2 (leaves[1]).
    let leaves = leaf_chain_with_upgrade(
        1..=5,
        4,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
    )
    .await;
    assert!(leaves[1].header().version() < NEW_PROTOCOL_VERSION);
    assert!(leaves[3].header().version() >= NEW_PROTOCOL_VERSION);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[1].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&VersionCheckQuorum::new(
                leaves.iter().map(|leaf| leaf.leaf().clone())
            )))
            .await
            .unwrap(),
        leaves[1]
    );
}

/// When a view gap at the cutover prevents a HotStuff2 2-chain from
/// completing within pre-cutover leaves, the leaf two before the cutover
/// (`upgrade_height - 2`) instead falls through to cert2 finality. Skewing
/// the view numbers breaks the consecutive-view requirement, so no 2-chain
/// forms and the proof terminates at the first cert2.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_leaf_before_cutover_uses_cert2() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Cutover at height 4, so `upgrade_height - 2` is height 2 (leaves[1]).
    // Skewed views mean no HotStuff2 2-chain ever forms.
    let leaves = custom_leaf_chain_with_upgrade(
        1..=5,
        4,
        Upgrade::new(DRB_AND_HEADER_UPGRADE_VERSION, NEW_PROTOCOL_VERSION),
        |proposal| {
            proposal.view_number = ViewNumber::new(proposal.block_header.height() * 2);
        },
    )
    .await;
    assert!(leaves[1].header().version() < NEW_PROTOCOL_VERSION);
    assert!(leaves[3].header().version() >= NEW_PROTOCOL_VERSION);
    let cert2_leaf = &leaves[3];
    let cert2 = cert2_for_leaf(cert2_leaf);

    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.insert_cert2(cert2_leaf.height(), cert2).await.unwrap();
        tx.commit().await.unwrap();
    }

    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[1].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. }));
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[1]
    );
}

/// Every leaf of a history whose tip is past the 0.6 cutover is proved from stored leaves and
/// cert2s alone, with no legacy QC chain stored.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_leaf_proofs_across_history() {
    // Every protocol version from mainnet's 0.2 genesis on, with the height it took effect.
    const UPGRADES: [(u64, Version); 4] = [
        (6, EPOCH_VERSION),
        (11, DRB_AND_HEADER_UPGRADE_VERSION),
        (16, EPOCH_REWARD_VERSION),
        (21, NEW_PROTOCOL_VERSION),
    ];
    // A view fails just before each of these heights, under HotStuff and under HotStuff2.
    const FAILED_VIEWS_BEFORE: [u64; 2] = [3, 13];
    const CERT2_HEIGHTS: [u64; 3] = [22, 24, 25];

    let view_of =
        |height: u64| height + FAILED_VIEWS_BEFORE.iter().filter(|&&h| h <= height).count() as u64;
    let leaves = custom_leaf_chain_with_upgrades(1..=25, FEE_VERSION, &UPGRADES, |proposal| {
        proposal.view_number = ViewNumber::new(view_of(proposal.block_header.height()));
        // Upgrades take effect by view, so failed views move where each one starts.
        if let Some(cert) = &mut proposal.upgrade_certificate {
            let (height, _) = UPGRADES
                .into_iter()
                .find(|(_, version)| *version == cert.data.new_version)
                .unwrap();
            let first_view = ViewNumber::new(view_of(height));
            cert.data.old_version_last_view = first_view - 1;
            cert.data.new_version_first_view = first_view;
            cert.data.decide_by = first_view;
        }
    })
    .await;
    let leaf_at = |height: u64| &leaves[height as usize - 1];

    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();
    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        for height in CERT2_HEIGHTS {
            tx.insert_cert2(height, cert2_for_leaf(leaf_at(height)))
                .await
                .unwrap();
        }
        tx.commit().await.unwrap();
    }

    let quorum = VersionCheckQuorum::new(leaves.iter().map(|leaf| leaf.leaf().clone()));
    for leaf in &leaves {
        let height = leaf.height();
        let proof = get_leaf_proof(&ds, leaf.clone(), None, Duration::MAX, CHAIN_LIMIT)
            .await
            .unwrap_or_else(|err| panic!("no proof for height {height}: {err:#}"));
        match height {
            // A 3-chain, which past a failed view closes later on.
            1..=4 => assert!(matches!(proof.proof(), FinalityProof::HotStuff { .. })),
            // From 5, the first 2-chain closes before the 3-chain does.
            5..=19 => assert!(matches!(proof.proof(), FinalityProof::HotStuff2 { .. })),
            // The last legacy leaf's 2-chain would span the cutover.
            _ => assert!(matches!(proof.proof(), FinalityProof::NewProtocol { .. })),
        }
        assert_eq!(
            proof.verify(LeafProofHint::Quorum(&quorum)).await.unwrap(),
            *leaf,
            "height {height}"
        );
    }

    // The client may instead name a leaf it already trusts, here the next one.
    for (leaf, finalized) in leaves.iter().tuple_windows() {
        let height = leaf.height();
        let proof = get_leaf_proof(
            &ds,
            leaf.clone(),
            Some(finalized.height() as usize),
            Duration::MAX,
            CHAIN_LIMIT,
        )
        .await
        .unwrap_or_else(|err| panic!("no proof for height {height}: {err:#}"));
        assert!(
            matches!(proof.proof(), FinalityProof::Assumption),
            "height {height}"
        );
        assert_eq!(
            proof
                .verify(LeafProofHint::<AlwaysTrueQuorum>::Assumption(
                    finalized.leaf()
                ))
                .await
                .unwrap(),
            *leaf,
            "height {height}"
        );
    }
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_bad_finalized() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Insert a single leaf. If we request this leaf but provide a finalized leaf which is
    // earlier, we should fail.
    let leaves = leaf_chain(1..2, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let err = get_leaf_proof_with_finalized_assumption(
        &ds,
        leaves[0].clone(),
        0,
        Duration::MAX,
        CHAIN_LIMIT,
    )
    .await
    .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_no_chain() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Insert multiple leaves that don't chain. We will not be able to prove these are
    // finalized.
    let leaves = leaf_chain(1..=4, EPOCH_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.insert_leaf(&leaves[2]).await.unwrap();
        tx.insert_leaf(&leaves[3]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let err =
        get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::from_secs(1), CHAIN_LIMIT)
            .await
            .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    // Even if we start from a finalized leave that extends one of the leaves we do have (4,
    // extends 3) we fail to generate a proof because we can't generate a chain from the
    // requested leaf (1) to the finalized leaf (4), since leaf 2 is missing.
    let err = get_leaf_proof_with_finalized_assumption(
        &ds,
        leaves[0].clone(),
        4,
        Duration::from_secs(1),
        CHAIN_LIMIT,
    )
    .await
    .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);
}

#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_chain_ends_before_finality() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // A legacy tip, as every replayed pre-0.6 chain has: the next leaf chains onto the
    // requested one, but no deciding QC has arrived and no cert2 ever will.
    let leaves = leaf_chain(1..=3, EPOCH_REWARD_VERSION).await;
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[0]).await.unwrap();
        tx.insert_leaf(&leaves[1]).await.unwrap();
        tx.commit().await.unwrap();
    }

    let err =
        get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::from_secs(1), CHAIN_LIMIT)
            .await
            .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);
    assert!(
        matches!(&err, hotshot_query_service::Error::Custom { message, .. } if message.contains("not enough leaves")),
        "{err}"
    );

    // Once the leaf carrying the deciding QC is stored, the same request succeeds.
    {
        let mut tx = ds.write().await.unwrap();
        tx.insert_leaf(&leaves[2]).await.unwrap();
        tx.commit().await.unwrap();
    }
    let proof = get_leaf_proof_with_leaf_chain(&ds, leaves[0].clone(), Duration::MAX, CHAIN_LIMIT)
        .await
        .unwrap();
    assert_eq!(
        proof
            .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
            .await
            .unwrap(),
        leaves[0]
    );
}

#[tokio::test]
#[test_log::test]
async fn test_header_proof() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Construct a chain of leaves, plus the corresponding block Merkle tree at each leaf.
    let leaves = leaf_chain(0..=2, EPOCH_VERSION).await;
    let mts = leaves
        .iter()
        .scan(
            BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT),
            |mt, leaf| {
                assert_eq!(mt.commitment(), leaf.header().block_merkle_tree_root());
                let item = mt.clone();
                mt.push(leaf.block_hash()).unwrap();
                Some(item)
            },
        )
        .collect::<Vec<_>>();

    // Save all those objects in the DB.
    {
        let mut tx = ds.write().await.unwrap();
        for (leaf, mt) in leaves.iter().zip(&mts) {
            tx.insert_leaf(leaf).await.unwrap();

            if leaf.height() > 0 {
                let merkle_path = mt.lookup(leaf.height() - 1).expect_ok().unwrap().1;
                UpdateStateData::<SeqTypes, BlockMerkleTree, _>::insert_merkle_nodes(
                    &mut tx,
                    merkle_path,
                    ToTraversalPath::<{ BlockMerkleTree::ARITY }>::to_traversal_path(
                        &(leaf.height() - 1),
                        BLOCK_MERKLE_TREE_HEIGHT,
                    ),
                    leaf.height(),
                )
                .await
                .unwrap();
                UpdateStateData::<SeqTypes, BlockMerkleTree, _>::set_last_state_height(
                    &mut tx,
                    leaf.height() as usize,
                )
                .await
                .unwrap();
            }
        }
        tx.commit().await.unwrap();
    }

    // Test happy path.
    for (root, mt) in mts.iter().enumerate().skip(1) {
        for (height, leaf) in leaves.iter().enumerate().take(root) {
            tracing::info!(root, height, "test happy path");
            let proof = get_header_proof(&ds, root as u64, BlockId::Number(height), Duration::MAX)
                .await
                .unwrap();
            assert_eq!(proof.verify_ref(mt.commitment()).unwrap(), leaf.header());
        }
    }

    // Test unknown leaf.
    let err = get_header_proof(&ds, 5, BlockId::Number(4), Duration::from_secs(1))
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    // Test height >= root.
    let err = get_header_proof(&ds, 1, BlockId::Number(1), Duration::MAX)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
}

/// A data source with `num_leaves` headers, state at every height, and state below `pruned`
/// collected. Returns the data source and the block tree at each height. The storage guard
/// keeps the database alive.
async fn pruned_data_source(
    num_leaves: usize,
    pruned: u64,
) -> (impl Sized, DataSource, Vec<BlockMerkleTree>) {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    let leaves = leaf_chain(0..num_leaves as u64, EPOCH_VERSION).await;
    let mts = leaves
        .iter()
        .scan(
            BlockMerkleTree::new(BLOCK_MERKLE_TREE_HEIGHT),
            |mt, leaf| {
                let item = mt.clone();
                mt.push(leaf.block_hash()).unwrap();
                Some(item)
            },
        )
        .collect::<Vec<_>>();

    let mut tx = ds.write().await.unwrap();
    for (leaf, mt) in leaves.iter().zip(&mts) {
        tx.insert_leaf(leaf).await.unwrap();
        if leaf.height() > 0 {
            let merkle_path = mt.lookup(leaf.height() - 1).expect_ok().unwrap().1;
            UpdateStateData::<SeqTypes, BlockMerkleTree, _>::insert_merkle_nodes(
                &mut tx,
                merkle_path,
                ToTraversalPath::<{ BlockMerkleTree::ARITY }>::to_traversal_path(
                    &(leaf.height() - 1),
                    BLOCK_MERKLE_TREE_HEIGHT,
                ),
                leaf.height(),
            )
            .await
            .unwrap();
            UpdateStateData::<SeqTypes, BlockMerkleTree, _>::set_last_state_height(
                &mut tx,
                leaf.height() as usize,
            )
            .await
            .unwrap();
        }
    }
    tx.commit().await.unwrap();

    let cfg = PrunerCfg::default().with_state_tables(vec![BlockMerkleTree::state_type().into()]);
    let sql: &SqlStorage = ds.as_ref();
    sql.prune_state_below(pruned, 0, &cfg).await.unwrap();
    (storage, ds, mts)
}

#[tokio::test]
#[test_log::test]
async fn test_header_proof_pruned_root() {
    let num_leaves = 40;
    let (_storage, ds, mts) = pruned_data_source(num_leaves, 30).await;

    // Roots below the pruned height are rebuilt from the newest snapshot.
    for root in [1, 2, 4, 10, 13, 27, 28] {
        for height in 0..root {
            let proof = get_header_proof(
                &ds,
                root as u64,
                BlockId::Number(height),
                Duration::from_secs(10),
            )
            .await
            .unwrap();
            proof.verify(mts[root].commitment()).unwrap();
        }
    }
}

#[tokio::test]
#[test_log::test]
async fn test_header_proof_root_beyond_head_fails() {
    let (_storage, ds, _) = pruned_data_source(10, 5).await;
    // The state at the head height is stored; beyond it there is no snapshot to rebuild from.
    let err = get_header_proof(&ds, 20, BlockId::Number(3), Duration::from_secs(10))
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[test_log::test]
async fn test_namespace_proof() {
    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = DataSource::create(
        DataSource::persistence_options(&storage),
        Default::default(),
        false,
    )
    .await
    .unwrap();

    // Construct a chain of blocks.
    let client = TestClient::default();
    let leaves = join_all((0..=2).map(|i| client.leaf(i))).await;
    let payloads = join_all((0..=2).map(|i| client.payload(i))).await;
    let vid_commons = join_all((0..=2).map(|i| client.vid_common(i))).await;

    // Save all those objects in the DB.
    {
        let mut tx = ds.write().await.unwrap();
        for (leaf, payload, vid_common) in izip!(&leaves, &payloads, &vid_commons) {
            tx.insert_leaf(leaf).await.unwrap();
            tx.insert_block(&BlockQueryData::<SeqTypes>::new(
                leaf.header().clone(),
                payload.clone(),
            ))
            .await
            .unwrap();
            tx.insert_vid(
                &VidCommonQueryData::<SeqTypes>::new(leaf.header().clone(), vid_common.clone()),
                None,
            )
            .await
            .unwrap();
        }
        tx.commit().await.unwrap();
    }

    // Test happy path: all blocks.
    let ns = payloads[0]
        .transaction(&TransactionIndex {
            ns_index: 0.into(),
            position: 0,
        })
        .unwrap()
        .namespace();
    let proofs = get_namespace_proof_range(&ds, 0, 3, ns.into(), Duration::MAX, 100)
        .await
        .unwrap();
    assert_eq!(proofs.len(), 3);
    for (leaf, proof) in leaves.iter().zip(proofs) {
        proof.verify(leaf.header(), ns).unwrap();
    }

    // Test happy path: subset.
    let tx = payloads[1]
        .transaction(&TransactionIndex {
            ns_index: 0.into(),
            position: 0,
        })
        .unwrap();
    let ns = tx.namespace();
    let proofs = get_namespace_proof_range(&ds, 1, 2, ns.into(), Duration::MAX, 100)
        .await
        .unwrap();
    assert_eq!(proofs.len(), 1);
    assert_eq!(proofs[0].verify(leaves[1].header(), ns).unwrap(), [tx]);

    // Test missing data in range.
    let err = get_namespace_proof_range(&ds, 0, 4, ns.into(), Duration::from_secs(1), 100)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    // Test invalid range.
    let err = get_namespace_proof_range(&ds, 1, 0, ns.into(), Duration::from_secs(1), 100)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    assert!(
        err.to_string().contains("requested empty interval"),
        "{err:#}"
    );

    // Test large range.
    let err = get_namespace_proof_range(&ds, 0, 10_000, ns.into(), Duration::from_secs(1), 100)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::BAD_REQUEST);
    assert!(err.to_string().contains("exceeds maximum size"), "{err:#}");
}

/// A peer with one cert2, 404ing every other height as a node does for a leaf finalized
/// indirectly. Records the heights it is asked for.
async fn serve_cert2(
    cert2: espresso_types::Certificate2<SeqTypes>,
) -> (TrustedQueryServiceProvider, Arc<Mutex<Vec<u64>>>) {
    let requested = Arc::new(Mutex::new(Vec::new()));
    let seen = requested.clone();
    let router = Router::new().route(
        "/availability/cert2/{height}",
        get(move |headers: HeaderMap, Path(height): Path<u64>| {
            seen.lock().unwrap().push(height);
            let served = (height == cert2.data.block_number).then(|| cert2.clone());
            async move {
                respond(
                    &headers,
                    served.ok_or_else(|| ServerError {
                        status: axum::http::StatusCode::NOT_FOUND,
                        message: format!("no cert2 at height {height}"),
                    }),
                )
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let provider = TrustedQueryServiceProvider::new(format!("http://{addr}").parse().unwrap());
    (provider, requested)
}

/// A proof no stored cert2 can complete asks the peers for one. The peer 404s the requested
/// height itself, so recovery has to reach a descendant.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_cert2_recovery_from_peer() {
    let leaves = leaf_chain(1..=4, NEW_PROTOCOL_VERSION).await;
    let (peer, requested) = serve_cert2(cert2_for_leaf(&leaves[1])).await;

    // Once with no cert2 stored, once with one stored too far past the requested leaf.
    for stored_out_of_limit in [false, true] {
        requested.lock().unwrap().clear();
        let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
        let ds = Config::try_from(&*DataSource::persistence_options(&storage))
            .unwrap()
            .builder(Provider::default().with_cert2_provider(peer.clone()))
            .await
            .unwrap()
            .disable_proactive_fetching()
            .disable_aggregator()
            .build()
            .await
            .unwrap();
        {
            let mut tx = ds.write().await.unwrap();
            for leaf in &leaves {
                tx.insert_leaf(leaf).await.unwrap();
            }
            if stored_out_of_limit {
                tx.insert_cert2(leaves[3].height(), cert2_for_leaf(&leaves[3]))
                    .await
                    .unwrap();
            }
            tx.commit().await.unwrap();
        }

        let err = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::from_secs(1), 2)
            .await
            .unwrap_err();
        assert_eq!(err.status(), StatusCode::NOT_FOUND);

        timeout(Duration::from_secs(30), async {
            loop {
                let mut tx = ds.read().await.unwrap();
                let stored: Option<espresso_types::Certificate2<SeqTypes>> =
                    tx.load_cert2(leaves[1].height()).await.unwrap();
                if stored.is_some() {
                    break;
                }
                drop(tx);
                sleep(Duration::from_millis(100)).await;
            }
        })
        .await
        .expect("descendant cert2 was not recovered from the peer");

        let asked = requested.lock().unwrap().clone();
        assert_eq!(
            asked,
            [1, 2],
            "the walk did not stop at the first cert2 it found"
        );

        let proof = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::from_secs(1), 2)
            .await
            .unwrap();
        assert_eq!(
            proof
                .verify(LeafProofHint::Quorum(&AlwaysTrueQuorum))
                .await
                .unwrap(),
            leaves[0]
        );
    }
}

/// Recovery stops at `chain_limit` past the requested leaf, even with a cert2 just beyond.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_cert2_recovery_stops_at_the_window() {
    let leaves = leaf_chain(1..=4, NEW_PROTOCOL_VERSION).await;
    // The only cert2 the peer has is one leaf beyond what a `chain_limit` of 2 may reach.
    let (peer, requested) = serve_cert2(cert2_for_leaf(&leaves[3])).await;

    let storage = <DataSource as TestableSequencerDataSource>::create_storage().await;
    let ds = Config::try_from(&*DataSource::persistence_options(&storage))
        .unwrap()
        .builder(Provider::default().with_cert2_provider(peer))
        .await
        .unwrap()
        .disable_proactive_fetching()
        .disable_aggregator()
        .build()
        .await
        .unwrap();
    {
        let mut tx = ds.write().await.unwrap();
        for leaf in &leaves {
            tx.insert_leaf(leaf).await.unwrap();
        }
        tx.commit().await.unwrap();
    }

    let err = get_leaf_proof_with_cert2(&ds, leaves[0].clone(), Duration::from_secs(1), 2)
        .await
        .unwrap_err();
    assert_eq!(err.status(), StatusCode::NOT_FOUND);

    // Nothing answers, so the walk runs the whole window and then stops.
    timeout(Duration::from_secs(30), async {
        while !(1..=3).all(|h| requested.lock().unwrap().contains(&h)) {
            sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("recovery did not reach every height within the limit");
    sleep(Duration::from_secs(1)).await;
    let asked = requested.lock().unwrap().clone();
    assert!(
        asked.iter().all(|h| (1..=3).contains(h)),
        "recovery reached past the limit: {asked:?}"
    );
}
