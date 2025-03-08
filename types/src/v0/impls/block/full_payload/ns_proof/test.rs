use futures::future;
use hotshot::{helpers::initialize_logging, traits::BlockPayload};
use hotshot_types::{
    data::VidCommitment,
    traits::EncodeBytes,
    vid::advz::{advz_scheme, ADVZScheme},
};
use jf_vid::{VidDisperse, VidScheme};

use crate::{v0::impls::block::test::ValidTest, NsProof, Payload};

#[tokio::test(flavor = "multi_thread")]
async fn ns_proof() {
    let test_cases = vec![
        vec![
            vec![5, 8, 8],
            vec![7, 9, 11],
            vec![10, 5, 8],
            vec![7, 8, 9],
            vec![],
        ],
        vec![vec![1, 2, 3], vec![4, 5, 6]],
        vec![],
    ];

    initialize_logging();

    let mut rng = jf_utils::test_rng();
    let mut tests = ValidTest::many_from_tx_lengths(test_cases, &mut rng);

    struct BlockInfo {
        block: Payload,
        vid: VidDisperse<ADVZScheme>,
        ns_proofs: Vec<NsProof>,
    }

    let blocks: Vec<BlockInfo> = {
        // compute blocks separately to avoid async error `captured variable
        // cannot escape `FnMut` closure body` caused by mutable variable `vid`
        // below.
        let blocks_only = future::join_all(tests.iter().map(|t| async {
            Payload::from_transactions(t.all_txs(), &Default::default(), &Default::default())
                .await
                .unwrap()
                .0
        }))
        .await;

        let mut vid = advz_scheme(10);
        blocks_only
            .into_iter()
            .map(|block| {
                let vid = vid.disperse(block.encode()).unwrap();
                let ns_proofs: Vec<NsProof> = block
                    .ns_table()
                    .iter()
                    .map(|ns_index| NsProof::new(&block, &ns_index, &vid.common).unwrap())
                    .collect();
                BlockInfo {
                    block,
                    vid,
                    ns_proofs,
                }
            })
            .collect()
    };

    // sanity: verify all valid namespace proofs
    for (
        BlockInfo {
            block,
            vid,
            ns_proofs,
        },
        test,
    ) in blocks.iter().zip(tests.iter_mut())
    {
        for ns_proof in ns_proofs.iter() {
            let ns_id = block.ns_table().read_ns_id(&ns_proof.ns_index).unwrap();
            let txs = test
                .nss
                .remove(&ns_id)
                .unwrap_or_else(|| panic!("namespace {} missing from test", ns_id));

            // verify ns_proof
            let (ns_proof_txs, ns_proof_ns_id) = ns_proof
                .verify(
                    block.ns_table(),
                    &VidCommitment::V0(vid.commit),
                    &vid.common,
                )
                .unwrap_or_else(|| panic!("namespace {} proof verification failure", ns_id));

            assert_eq!(ns_proof_ns_id, ns_id);
            assert_eq!(ns_proof_txs, txs);
        }
    }

    assert!(blocks.len() >= 2, "need at least 2 test_cases");

    let ns_proof_0_0 = &blocks[0].ns_proofs[0];
    let ns_table_0 = blocks[0].block.ns_table();
    let ns_table_1 = blocks[1].block.ns_table();
    let vid_commit_0 = &VidCommitment::V0(blocks[0].vid.commit);
    let vid_commit_1 = &VidCommitment::V0(blocks[1].vid.commit);
    let vid_common_0 = &blocks[0].vid.common;
    let vid_common_1 = &blocks[1].vid.common;

    // mix and match ns_table, vid_commit, vid_common
    {
        // wrong ns_table
        assert!(ns_proof_0_0
            .verify(ns_table_1, vid_commit_0, vid_common_0)
            .is_none());

        // wrong vid commitment
        assert!(ns_proof_0_0
            .verify(ns_table_0, vid_commit_1, vid_common_0)
            .is_none());

        // wrong vid common
        assert!(ns_proof_0_0
            .verify(ns_table_0, vid_commit_0, vid_common_1)
            .is_none());

        // wrong ns_proof
        assert!(ns_proof_0_0
            .verify(ns_table_1, vid_commit_1, vid_common_1)
            .is_none());
    }

    // hack the proof
    {
        ns_proof_0_0
            .verify(ns_table_0, vid_commit_0, vid_common_0)
            .expect("sanity: correct proof should succeed");

        let wrong_ns_index_ns_proof_0_0 = NsProof {
            ns_index: blocks[0].ns_proofs[1].ns_index.clone(),
            ..ns_proof_0_0.clone()
        };
        assert!(wrong_ns_index_ns_proof_0_0
            .verify(ns_table_0, vid_commit_0, vid_common_0)
            .is_none());

        let wrong_ns_payload_ns_proof_0_0 = NsProof {
            ns_payload: blocks[0].ns_proofs[1].ns_payload.clone(),
            ..ns_proof_0_0.clone()
        };
        assert!(wrong_ns_payload_ns_proof_0_0
            .verify(ns_table_0, vid_commit_0, vid_common_0)
            .is_none());

        let wrong_proof_ns_proof_0_0 = NsProof {
            ns_proof: blocks[0].ns_proofs[1].ns_proof.clone(),
            ..ns_proof_0_0.clone()
        };
        assert!(wrong_proof_ns_proof_0_0
            .verify(ns_table_0, vid_commit_0, vid_common_0)
            .is_none());
    }
}
