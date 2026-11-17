use std::collections::BTreeSet;

use hotshot_types::{
    data::EpochNumber, signature_key::BLSPubKey, traits::signature_key::SignatureKey,
};

use crate::tests::common::runner::TestRunner;

/// Every node signs a record for each epoch it enters and sends it, and every node keeps, per
/// epoch, the records of all nodes, its own included.
#[tokio::test(flavor = "multi_thread")]
async fn nodes_keep_each_others_service_records_per_epoch() {
    let num_nodes = 5;
    let mut runner = TestRunner::builder()
        .num_nodes(num_nodes)
        .epoch_height(10)
        .target_decisions(30)
        .build();
    runner.run().await.unwrap();

    let keys: Vec<BLSPubKey> = (0..num_nodes as u64)
        .map(|i| BLSPubKey::generated_from_seed_indexed([0; 32], i).0)
        .collect();
    for (idx, store) in runner.node_stores().iter().enumerate() {
        let records = store.load().unwrap().service_records;
        for record in &records {
            assert!(record.is_valid());
            let other = keys.iter().position(|k| *k == record.signer).unwrap();
            assert_eq!(
                record.record.api_url,
                format!("http://node{other}.test:24000")
            );
        }
        // Every node has left epoch 2 by the 30th decision.
        let epoch = EpochNumber::new(2);
        let signers: BTreeSet<_> = records
            .iter()
            .filter(|r| r.record.epoch == epoch)
            .map(|r| r.signer)
            .collect();
        assert_eq!(
            signers,
            keys.iter().copied().collect(),
            "node {idx} lacks records of epoch {epoch}"
        );

        let (served_epoch, served) = store.service_records(Some(epoch)).unwrap().unwrap();
        assert_eq!(served_epoch, epoch);
        assert_eq!(served.len(), num_nodes);
    }
}
