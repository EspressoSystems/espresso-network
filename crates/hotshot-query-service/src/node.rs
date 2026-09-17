// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

//! A node's view of a HotShot chain
//!
//! The node API provides a subjective view of the HotShot blockchain, from the perspective of
//! one particular node. It provides access to information that the
//! [availability](crate::availability) API does not, because this information depends on the
//! perspective of the node observing it, and may be subject to eventual consistency. For example,
//! `/node/block-height` may return smaller counts than expected, if the node being queried is not
//! fully synced with the entire history of the chain. However, the node will _eventually_ sync and
//! return the expected counts.

pub(crate) mod data_source;
pub(crate) mod query_data;
pub use data_source::*;
pub use hotshot_query_service_types::node::*;

#[derive(Debug)]
pub struct Options {
    /// The maximum number of headers which can be loaded in a single `header/window` query.
    pub window_limit: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self { window_limit: 500 }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use committable::Committable;
    use futures::StreamExt;
    use hotshot_types::{
        data::{VidDisperseShare, VidShare},
        event::{EventType, LeafInfo},
        traits::{
            EncodeBytes,
            block_contents::{BlockHeader, BlockPayload},
        },
    };
    use tokio::time::sleep;

    use super::*;
    use crate::{
        Header, QueryError,
        availability::BlockId,
        testing::{
            consensus::{MockDataSource, MockNetwork, MockSqlDataSource},
            mocks::{MockTypes, mock_transaction},
        },
    };

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_api() {
        let window_limit = 78;

        // Create the consensus network.
        let mut network = MockNetwork::<MockDataSource>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        let ds = network.data_source();

        // Wait until a few blocks have been sequenced.
        let block_height = loop {
            let block_height = ds.block_height().await.unwrap();
            if block_height > network.num_nodes() {
                break block_height;
            }
            sleep(Duration::from_secs(1)).await;
        };

        // We test these counters with non-trivial values in `data_source.rs`, here we just want to
        // make sure the queries are working, so a response of 0 is fine.
        assert_eq!(ds.count_transactions().await.unwrap(), 0);
        assert_eq!(ds.payload_size().await.unwrap(), 0);

        let mut headers = vec![];

        // Get VID share for each block.
        tracing::info!(block_height, "checking VID shares");
        'outer: while let Some(event) = events.next().await {
            let EventType::Decide { leaf_chain, .. } = event.event else {
                continue;
            };
            for LeafInfo {
                leaf, vid_share, ..
            } in leaf_chain.iter().rev()
            {
                headers.push(leaf.block_header().clone());
                if leaf.block_header().block_number >= block_height as u64 {
                    break 'outer;
                }
                tracing::info!(height = leaf.block_header().block_number, "checking share");

                let share = ds
                    .vid_share(BlockId::<MockTypes>::Number(
                        leaf.block_header().block_number as usize,
                    ))
                    .await
                    .unwrap();
                if let Some(vid_share) = vid_share.as_ref() {
                    let VidDisperseShare::V0(new_share) = vid_share else {
                        panic!("VID share is not V0");
                    };
                    assert_eq!(share, VidShare::V0(new_share.share.clone()));
                }

                // Query various other ways.
                assert_eq!(
                    share,
                    ds.vid_share(BlockId::<MockTypes>::Hash(leaf.block_header().commit()))
                        .await
                        .unwrap()
                );
                assert_eq!(
                    share,
                    ds.vid_share(BlockId::<MockTypes>::PayloadHash(
                        leaf.block_header().payload_commitment
                    ))
                    .await
                    .unwrap()
                );
            }
        }

        // Check time window queries. The various edge cases are thoroughly tested for each
        // individual data source. In this test, we just smoketest parameter handling. Sleep 2
        // seconds to ensure a new header is produced with a timestamp after the latest one in
        // `headers`
        sleep(Duration::from_secs(2)).await;
        let first_header = &headers[0];
        let last_header = &headers.last().unwrap();
        let window: TimeWindowQueryData<Header<MockTypes>> = ds
            .get_header_window(
                WindowStart::Time(first_header.timestamp),
                last_header.timestamp + 1,
                window_limit,
            )
            .await
            .unwrap();
        assert!(window.window.contains(first_header));
        assert!(window.window.contains(last_header));
        assert!(window.next.is_some());

        // Query for the same window other ways.
        assert_eq!(
            window,
            ds.get_header_window(
                WindowStart::<MockTypes>::Height(0),
                last_header.timestamp + 1,
                window_limit
            )
            .await
            .unwrap()
        );
        assert_eq!(
            window,
            ds.get_header_window(
                WindowStart::<MockTypes>::Hash(first_header.commit()),
                last_header.timestamp + 1,
                window_limit
            )
            .await
            .unwrap()
        );

        // In this simple test, the node should be fully synchronized.
        let sync_status = ds.sync_status().await.unwrap();
        assert!(sync_status.is_fully_synced(), "{sync_status:#?}");

        network.shut_down().await;
    }

    #[test_log::test(tokio::test(flavor = "multi_thread"))]
    async fn test_aggregate_ranges() {
        // Create the consensus network.
        let mut network = MockNetwork::<MockSqlDataSource>::init().await;
        let mut events = network.handle().event_stream();
        network.start().await;

        let ds = network.data_source();

        // Wait until a few transactions have been sequenced.
        let mut tx_heights = vec![];
        let mut tx_sizes = vec![];
        for i in [1, 2] {
            let txn = mock_transaction(vec![0; i]);
            let hash = txn.commit();

            network.submit_transaction(txn).await;

            let leaf = 'outer: loop {
                let EventType::Decide { leaf_chain, .. } = events.next().await.unwrap().event
                else {
                    continue;
                };
                for info in leaf_chain.iter().rev() {
                    let leaf = &info.leaf;
                    if BlockPayload::<MockTypes>::transaction_commitments(
                        &leaf.block_payload().unwrap(),
                        BlockHeader::<MockTypes>::metadata(leaf.block_header()),
                    )
                    .contains(&hash)
                    {
                        break 'outer leaf.clone();
                    }
                }

                tracing::info!("waiting for tx {i}");
                sleep(Duration::from_secs(1)).await;
            };
            tx_heights.push(leaf.height() as usize);
            tx_sizes.push(leaf.block_payload().unwrap().encode().len());
        }
        tracing::info!(?tx_heights, ?tx_sizes, "transactions sequenced");

        // Wait for the aggregator to process the inserted blocks.
        while let Err(err) = ds
            .count_transactions_in_range(0..=tx_heights[1], None)
            .await
        {
            match err {
                QueryError::NotFound | QueryError::Missing => {
                    tracing::info!(?tx_heights, "waiting for aggregator");
                    sleep(Duration::from_secs(1)).await;
                },
                err => panic!("unexpected error: {err:#}"),
            }
        }

        // Range including empty blocks (genesis block) only
        assert_eq!(
            0,
            ds.count_transactions_in_range(0..=0, None).await.unwrap()
        );
        assert_eq!(0, ds.payload_size_in_range(0..=0, None).await.unwrap());

        // First transaction only
        assert_eq!(
            1,
            ds.count_transactions_in_range(0..=tx_heights[0], None)
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[0],
            ds.payload_size_in_range(0..=tx_heights[0], None)
                .await
                .unwrap()
        );

        // Last transaction only
        assert_eq!(
            1,
            ds.count_transactions_in_range(tx_heights[0] + 1..=tx_heights[1], None)
                .await
                .unwrap()
        );
        assert_eq!(
            tx_sizes[1],
            ds.payload_size_in_range(tx_heights[0] + 1..=tx_heights[1], None)
                .await
                .unwrap()
        );

        // All transactions
        assert_eq!(2, ds.count_transactions().await.unwrap());
        assert_eq!(tx_sizes[0] + tx_sizes[1], ds.payload_size().await.unwrap());

        network.shut_down().await;
    }
}
