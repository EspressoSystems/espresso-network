use super::*;

/// The v2 node, config, database and availability endpoints adapt the v1 handlers, so on one
/// node both versions must report the same values, with v2's query parameters selecting what
/// v1's path parameters do.
#[test_log::test(tokio::test(flavor = "multi_thread"))]
async fn test_v2_api_agrees_with_v1() {
    let port = reserve_tcp_port().expect("OS should have ephemeral ports available");

    let url = format!("http://localhost:{port}").parse().unwrap();
    let client: Client<ClientErr, StaticVersion<0, 1>> = Client::new(url);

    let storage = SqlDataSource::create_storage().await;
    let network_config = TestConfigBuilder::default().build();
    let mut ds_opts = tmp_options(&storage);
    ds_opts.disable_proactive_fetching = true;
    let config = TestNetworkConfigBuilder::default()
        .api_config(
            Options::with_port(port)
                .query_sql(Default::default(), ds_opts)
                .submit(Default::default())
                .config(Default::default()),
        )
        .network_config(network_config)
        .build();
    let network = TestNetwork::new(config, MOCK_SEQUENCER_VERSIONS).await;
    let mut events = network.server.event_stream();

    client.connect(None).await;

    let namespace_counts = [(101u8, 1u8), (102, 2)];
    let mut blocks = Vec::new();
    for (ns, count) in namespace_counts {
        for i in 0..count {
            let txn = Transaction::new(NamespaceId::from(u64::from(ns)), vec![ns, i]);
            client
                .post::<()>("submit/submit")
                .body_json(&txn)
                .unwrap()
                .send()
                .await
                .unwrap();
            let (block, _) = wait_for_decide_on_handle(&mut events, &txn).await;
            blocks.push(block);
        }
    }
    let first_block = blocks[0];
    let last_block = *blocks.last().unwrap();

    // The counts come from aggregates a background task fills in after each block is
    // stored, so wait for them to reach the last submitted transaction first; nothing else
    // submits, so every number below is stable from then on.
    let expected_total: u64 = namespace_counts
        .iter()
        .map(|(_, count)| u64::from(*count))
        .sum();
    let total = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let count: u64 = client.get("node/transactions/count").send().await.unwrap();
            if count >= expected_total {
                return count;
            }
            sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("transaction count never caught up");
    assert_eq!(total, expected_total);

    let v2_total: serde_json::Value = client
        .get("v2/node/transaction-count")
        .send()
        .await
        .unwrap();
    assert_eq!(
        v2_total,
        serde_json::json!({"count": expected_total.to_string()})
    );

    // Each range below leaves a transaction out, so the strict inequalities are what give the
    // comparisons teeth: a handler that dropped `from` or `to` would answer with the
    // chain-wide total instead, and only those assertions notice.
    let v1_through_first: u64 = client
        .get(&format!("node/transactions/count/{first_block}"))
        .send()
        .await
        .unwrap();
    assert!(
        v1_through_first < expected_total,
        "every transaction landed in one block {blocks:?}, so no range excludes one"
    );
    let v2_through_first: serde_json::Value = client
        .get(&format!("v2/node/transaction-count?to={first_block}"))
        .send()
        .await
        .unwrap();
    assert_eq!(
        v2_through_first,
        serde_json::json!({"count": v1_through_first.to_string()})
    );

    let v1_last_only: u64 = client
        .get(&format!(
            "node/transactions/count/{last_block}/{last_block}"
        ))
        .send()
        .await
        .unwrap();
    assert!(v1_last_only < expected_total, "{blocks:?}");
    let v2_last_only: serde_json::Value = client
        .get(&format!(
            "v2/node/transaction-count?from={last_block}&to={last_block}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        v2_last_only,
        serde_json::json!({"count": v1_last_only.to_string()})
    );

    let v1_total_size: u64 = client.get("node/payloads/size").send().await.unwrap();
    let v2_total_size: serde_json::Value = client.get("v2/node/payload-size").send().await.unwrap();
    assert_eq!(
        v2_total_size,
        serde_json::json!({"size": v1_total_size.to_string()})
    );

    let v1_block_size: u64 = client
        .get(&format!("node/payloads/size/{last_block}/{last_block}"))
        .send()
        .await
        .unwrap();
    assert!(v1_block_size < v1_total_size, "{blocks:?}");
    let v2_block_size: serde_json::Value = client
        .get(&format!(
            "v2/node/payload-size?from={last_block}&to={last_block}"
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(
        v2_block_size,
        serde_json::json!({"size": v1_block_size.to_string()})
    );

    for (ns, count) in namespace_counts {
        let v1_count: u64 = client
            .get(&format!("node/transactions/count/namespace/{ns}"))
            .send()
            .await
            .unwrap();
        assert_eq!(v1_count, count as u64);
        let v2_count: serde_json::Value = client
            .get(&format!("v2/node/transaction-count?namespace={ns}"))
            .send()
            .await
            .unwrap();
        assert_eq!(v2_count, serde_json::json!({"count": v1_count.to_string()}));

        let v1_size: u64 = client
            .get(&format!("node/payloads/size/namespace/{ns}"))
            .send()
            .await
            .unwrap();
        let v2_size: serde_json::Value = client
            .get(&format!("v2/node/payload-size?namespace={ns}"))
            .send()
            .await
            .unwrap();
        assert_eq!(v2_size, serde_json::json!({"size": v1_size.to_string()}));
    }

    // The query service caches sync status for minutes, so a fresh node still reports its
    // startup snapshot here; what is checked is that v2 relays exactly what v1 reports.
    let v1_sync: hotshot_query_service::node::SyncStatusQueryData =
        client.get("node/sync-status").send().await.unwrap();
    let v2_sync: espresso_api::proto::SyncStatusResponse =
        client.get("v2/node/sync-status").send().await.unwrap();
    assert!(!v1_sync.blocks.ranges.is_empty(), "{v1_sync:?}");
    for (what, v1, v2) in [
        ("blocks", &v1_sync.blocks, v2_sync.blocks.unwrap()),
        ("leaves", &v1_sync.leaves, v2_sync.leaves.unwrap()),
        (
            "vid_common",
            &v1_sync.vid_common,
            v2_sync.vid_common.unwrap(),
        ),
    ] {
        assert!(!v1.ranges.is_empty(), "{what}: {v1:?}");
        assert_eq!(v2.missing, v1.missing as u64, "{what}");
        assert_eq!(v2.ranges.len(), v1.ranges.len(), "{what}");
        for (v1, v2) in v1.ranges.iter().zip(&v2.ranges) {
            assert_eq!((v2.start, v2.end), (v1.start as u64, v1.end as u64));
            let expected = match v1.status {
                hotshot_query_service::node::SyncStatus::Present => {
                    espresso_api::proto::SyncStatus::Present
                },
                hotshot_query_service::node::SyncStatus::Missing => {
                    espresso_api::proto::SyncStatus::Missing
                },
                hotshot_query_service::node::SyncStatus::Pruned => {
                    espresso_api::proto::SyncStatus::Pruned
                },
            };
            assert_eq!(v2.status(), expected);
        }
    }
    assert_eq!(
        v2_sync.pruned_height,
        v1_sync.pruned_height.map(|height| height as u64)
    );

    let v1_limits: serde_json::Value = client.get("node/limits").send().await.unwrap();
    let v2_limits: espresso_api::proto::NodeLimitsResponse =
        client.get("v2/node/limits").send().await.unwrap();
    assert_eq!(
        v2_limits.window_limit,
        v1_limits["window_limit"].as_u64().unwrap()
    );

    for (v1_route, v2_route) in [("node/stake-table/current", "v2/node/stake-table")] {
        let v1_table: serde_json::Value = client.get(v1_route).send().await.unwrap();
        let v2_table: espresso_api::proto::StakeTableResponse =
            client.get(v2_route).send().await.unwrap();
        assert_eq!(v2_table.epoch, v1_table["epoch"].as_u64());
        let v1_peers = v1_table["stake_table"].as_array().unwrap();
        assert!(!v1_peers.is_empty(), "{v1_table}");
        assert_eq!(v2_table.stake_table.len(), v1_peers.len());
        for (v1_peer, v2_peer) in v1_peers.iter().zip(&v2_table.stake_table) {
            let v1_entry = &v1_peer["stake_table_entry"];
            let v2_entry = v2_peer.stake_table_entry.as_ref().unwrap();
            assert_eq!(
                v2_entry.stake_key.as_ref().unwrap().key,
                v1_entry["stake_key"].as_str().unwrap()
            );
            assert_eq!(
                v2_entry.stake_amount,
                v1_entry["stake_amount"].as_str().unwrap()
            );
            assert_eq!(
                v2_peer.state_ver_key.as_ref().unwrap().key,
                v1_peer["state_ver_key"].as_str().unwrap()
            );
            match (&v2_peer.connect_info, v1_peer["connect_info"].as_object()) {
                (Some(v2_info), Some(v1_info)) => {
                    assert_eq!(v2_info.p2p_addr, v1_info["p2p_addr"].as_str().unwrap());
                    let v1_key = bs58::decode(v1_info["x25519_key"].as_str().unwrap())
                        .into_vec()
                        .unwrap();
                    assert_eq!(
                        v2_info.x25519_key.parse::<x25519::PublicKey>().unwrap(),
                        x25519::PublicKey::try_from(&v1_key[..]).unwrap()
                    );
                },
                (None, None) => {},
                (v2_info, v1_info) => panic!("{v2_info:?} against {v1_info:?}"),
            }
        }
    }

    // An epoch this network never reaches, so only the status is comparable.
    let v1_err = client
        .get::<serde_json::Value>("node/stake-table/1")
        .send()
        .await
        .unwrap_err();
    let v2_err = client
        .get::<serde_json::Value>("v2/node/stake-table?epoch=1")
        .send()
        .await
        .unwrap_err();
    assert_eq!(v2_err.status, v1_err.status);

    // Every decided view moves these, so retry until a pair straddles no view.
    let (v1_votes, v2_votes) = {
        let mut attempts = 0;
        loop {
            let v1: serde_json::Value = client
                .get("node/participation/vote/current")
                .send()
                .await
                .unwrap();
            let v2: espresso_api::proto::ParticipationResponse = client
                .get("v2/node/participation/vote")
                .send()
                .await
                .unwrap();
            let v1: std::collections::BTreeMap<String, f64> = v1
                .as_object()
                .unwrap()
                .iter()
                .map(|(key, value)| (key.clone(), value.as_f64().unwrap()))
                .collect();
            let v2_map: std::collections::BTreeMap<String, f64> = v2
                .participation
                .iter()
                .map(|entry| (entry.key.as_ref().unwrap().key.clone(), entry.participation))
                .collect();
            if v1 == v2_map {
                break (v1, v2);
            }
            attempts += 1;
            assert!(attempts < 5, "v1 and v2 never agreed: {v1:?} vs {v2_map:?}");
        }
    };
    assert!(!v1_votes.is_empty());
    let keys: Vec<_> = v2_votes
        .participation
        .iter()
        .map(|entry| &entry.key.as_ref().unwrap().key)
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] <= pair[1]), "{keys:?}");

    let height_before: u64 = client.get("node/block-height").send().await.unwrap();
    let v2_height: espresso_api::proto::NodeBlockHeightResponse =
        client.get("v2/node/block-height").send().await.unwrap();
    let height_after: u64 = client.get("node/block-height").send().await.unwrap();
    assert!(
        (height_before..=height_after).contains(&v2_height.height),
        "{} outside {height_before}..={height_after}",
        v2_height.height
    );

    // End at a timestamp already passed, so blocks decided meanwhile fall outside the window.
    let tip: serde_json::Value = client
        .get("node/header/window/0/999999999999")
        .send()
        .await
        .unwrap();
    let end = tip["window"].as_array().unwrap().last().unwrap()["timestamp"]
        .as_u64()
        .unwrap();
    assert!(end > 0, "{tip}");
    let v1_window: serde_json::Value = client
        .get(&format!("node/header/window/0/{end}"))
        .send()
        .await
        .unwrap();
    let v2_window: espresso_api::proto::HeaderWindowResponse = client
        .get(&format!("v2/node/header-window?start_time=0&end={end}"))
        .send()
        .await
        .unwrap();
    let v1_headers = v1_window["window"].as_array().unwrap();
    assert!(!v1_headers.is_empty(), "{v1_window}");
    assert_eq!(v2_window.window.len(), v1_headers.len());
    for (v1_header, v2_header) in v1_headers.iter().zip(&v2_window.window) {
        let v2_header = match v2_header.header.as_ref().unwrap() {
            espresso_api::proto::header_response::Header::V1(header) => header,
            other => panic!("this network runs 0.1, not {other:?}"),
        };
        assert_eq!(v2_header.height, v1_header["height"].as_u64().unwrap());
        assert_eq!(
            v2_header.payload_commitment,
            v1_header["payload_commitment"].as_str().unwrap()
        );
        assert_eq!(
            v2_header.builder_commitment,
            v1_header["builder_commitment"].as_str().unwrap()
        );
        assert_eq!(
            v2_header.block_merkle_tree_root,
            v1_header["block_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            v2_header.fee_merkle_tree_root,
            v1_header["fee_merkle_tree_root"].as_str().unwrap()
        );
        assert_eq!(
            v2_header.timestamp,
            v1_header["timestamp"].as_u64().unwrap()
        );
        assert_eq!(v2_header.l1_head, v1_header["l1_head"].as_u64().unwrap());
        let v2_fee = v2_header.fee_info.as_ref().unwrap();
        assert_eq!(
            v2_fee.account,
            v1_header["fee_info"]["account"].as_str().unwrap()
        );
        assert_eq!(
            v2_fee.amount,
            v1_header["fee_info"]["amount"].as_str().unwrap()
        );
        let v1_chain_config = &v1_header["chain_config"]["chain_config"]["Left"];
        let v2_chain_config = match v2_header
            .chain_config
            .as_ref()
            .unwrap()
            .chain_config
            .as_ref()
            .unwrap()
        {
            espresso_api::proto::resolvable_chain_config::ChainConfig::Full(config) => config,
            other => panic!("a test network header carries its config: {other:?}"),
        };
        assert_eq!(
            v2_chain_config.chain_id,
            v1_chain_config["chain_id"].as_str().unwrap()
        );
        assert_eq!(
            v2_chain_config.max_block_size.to_string(),
            v1_chain_config["max_block_size"].as_str().unwrap()
        );
        assert_eq!(
            v2_chain_config.base_fee,
            v1_chain_config["base_fee"].as_str().unwrap()
        );
        assert_eq!(
            v2_chain_config.fee_recipient,
            v1_chain_config["fee_recipient"].as_str().unwrap()
        );
        assert_eq!(
            v2_header.ns_table.as_ref().unwrap().bytes,
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                v1_header["ns_table"]["bytes"].as_str().unwrap()
            )
            .unwrap(),
        );
        assert_eq!(
            v2_header.l1_finalized.is_some(),
            !v1_header["l1_finalized"].is_null()
        );
        assert_eq!(
            v2_header.builder_signature.is_some(),
            !v1_header["builder_signature"].is_null()
        );
    }
    let v2_next = match v2_window.next.as_ref().unwrap().header.as_ref().unwrap() {
        espresso_api::proto::header_response::Header::V1(header) => header,
        other => panic!("this network runs 0.1, not {other:?}"),
    };
    assert_eq!(
        v2_next.height,
        v1_window["next"]["height"].as_u64().unwrap()
    );
    // start_time=0 precedes every block, so like v1 the window has nothing before it.
    assert!(v1_window["prev"].is_null(), "{v1_window}");
    assert!(v2_window.prev.is_none());

    // The other two selectors name the window by its first block, and each must agree with
    // the v1 route it mirrors. Starting at block 1 also gives `prev` something to hold.
    let first: espresso_types::Header = serde_json::from_value(v1_headers[1].clone()).unwrap();
    assert_eq!(first.height(), 1);
    let first_hash = committable::Committable::commit(&first);
    let height = |header: &espresso_api::proto::HeaderResponse| match header.header.as_ref() {
        Some(espresso_api::proto::header_response::Header::V1(header)) => header.height,
        other => panic!("this network runs 0.1, not {other:?}"),
    };
    for (v1_route, v2_query) in [
        (
            format!("node/header/window/from/1/{end}"),
            format!("start_height=1&end={end}"),
        ),
        (
            format!("node/header/window/from/hash/{first_hash}/{end}"),
            format!("start_hash={first_hash}&end={end}"),
        ),
    ] {
        let v1_window: serde_json::Value = client.get(&v1_route).send().await.unwrap();
        let v2_window: espresso_api::proto::HeaderWindowResponse = client
            .get(&format!("v2/node/header-window?{v2_query}"))
            .send()
            .await
            .unwrap();
        assert_eq!(v1_window["prev"]["height"].as_u64(), Some(0), "{v1_route}");
        assert_eq!(
            v2_window.window.iter().map(height).collect::<Vec<_>>(),
            v1_window["window"]
                .as_array()
                .unwrap()
                .iter()
                .map(|header| header["height"].as_u64().unwrap())
                .collect::<Vec<_>>(),
            "{v2_query}"
        );
        assert_eq!(
            v2_window.prev.as_ref().map(height),
            v1_window["prev"]["height"].as_u64(),
            "{v2_query}"
        );
        assert_eq!(
            v2_window.next.as_ref().map(height),
            v1_window["next"]["height"].as_u64(),
            "{v2_query}"
        );
    }

    let v1_share: serde_json::Value = client.get("node/vid/share/1").send().await.unwrap();
    let v2_share: espresso_api::proto::VidShareResponse = client
        .get("v2/node/vid-share?height=1")
        .send()
        .await
        .unwrap();
    let v1_advz = &v1_share["V0"];
    let v2_advz = match v2_share.share.as_ref().unwrap() {
        espresso_api::proto::vid_share_response::Share::V0(share) => share,
        other => panic!("this network disperses with ADVZ, not {other:?}"),
    };
    assert_eq!(
        v2_advz.aggregate_proofs,
        v1_advz["aggregate_proofs"].as_str().unwrap()
    );
    assert_eq!(v2_advz.evals, v1_advz["evals"].as_str().unwrap());
    let v2_proof = v2_advz.evals_proof.as_ref().unwrap();
    assert_eq!(
        v2_proof.pos,
        v1_advz["evals_proof"]["pos"].as_str().unwrap()
    );
    let v1_nodes = v1_advz["evals_proof"]["proof"].as_array().unwrap();
    assert!(v1_nodes.len() > 1, "{v1_advz}");
    assert_eq!(v2_proof.proof.len(), v1_nodes.len());
    fn assert_node(v1: &serde_json::Value, v2: &espresso_api::proto::AdvzMerkleNode) {
        use espresso_api::proto::advz_merkle_node::Node;
        match (v2.node.as_ref().unwrap(), v1) {
            (Node::Leaf(leaf), v1) if v1.get("Leaf").is_some() => {
                let v1 = &v1["Leaf"];
                assert_eq!(leaf.elem, v1["elem"].as_str().unwrap());
                assert_eq!(leaf.pos, v1["pos"].as_str().unwrap());
                assert_eq!(leaf.value, v1["value"].as_str().unwrap());
            },
            (Node::Branch(branch), v1) if v1.get("Branch").is_some() => {
                let v1 = &v1["Branch"];
                assert_eq!(branch.value, v1["value"].as_str().unwrap());
                let v1_children = v1["children"].as_array().unwrap();
                assert_eq!(branch.children.len(), v1_children.len());
                for (v1_child, v2_child) in v1_children.iter().zip(&branch.children) {
                    assert_node(v1_child, v2_child);
                }
            },
            (Node::ForgottenSubtree(subtree), v1) if v1.get("ForgettenSubtree").is_some() => {
                assert_eq!(
                    subtree.value,
                    v1["ForgettenSubtree"]["value"].as_str().unwrap()
                );
            },
            (Node::Empty(_), v1) if v1.as_str() == Some("Empty") => {},
            (v2, v1) => panic!("{v2:?} against {v1}"),
        }
    }
    for (v1_node, v2_node) in v1_nodes.iter().zip(&v2_proof.proof) {
        assert_node(v1_node, v2_node);
    }

    // hash and payload_hash select the share by its block's hashes, as v1's own routes do, so
    // block 1's share comes back either way.
    let payload_hash = first.payload_commitment();
    for (v1_route, v2_query) in [
        (
            format!("node/vid/share/hash/{first_hash}"),
            format!("hash={first_hash}"),
        ),
        (
            format!("node/vid/share/payload-hash/{payload_hash}"),
            format!("payload_hash={payload_hash}"),
        ),
    ] {
        let v1: serde_json::Value = client.get(&v1_route).send().await.unwrap();
        assert_eq!(v1, v1_share, "{v1_route}");
        let v2: espresso_api::proto::VidShareResponse = client
            .get(&format!("v2/node/vid-share?{v2_query}"))
            .send()
            .await
            .unwrap();
        assert_eq!(v2, v2_share, "{v2_query}");
    }

    // Naming the block by none or two of the selectors is refused, as is a hash that does
    // not parse; v1 has no route for the first two and answers the third with a 400.
    let v1_err = client
        .get::<serde_json::Value>("node/vid/share/hash/not-a-hash")
        .send()
        .await
        .unwrap_err();
    assert_eq!(v1_err.status, StatusCode::BAD_REQUEST, "{v1_err}");
    for query in [
        "v2/node/vid-share".to_string(),
        format!("v2/node/vid-share?height=1&hash={first_hash}"),
        "v2/node/vid-share?hash=not-a-hash".to_string(),
        format!("v2/node/header-window?end={end}"),
        format!("v2/node/header-window?start_time=0&start_height=1&end={end}"),
        format!("v2/node/header-window?start_hash=not-a-hash&end={end}"),
    ] {
        let err = client
            .get::<serde_json::Value>(&query)
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST, "{query}: {err}");
    }

    let v1_config = client
        .get::<espresso_types::config::PublicNetworkConfig>("config/hotshot")
        .send()
        .await
        .unwrap();
    let v1_hotshot = v1_config.hotshot_config().into_hotshot_config();
    let v2_hotshot: espresso_api::proto::HotshotConfigResponse =
        client.get("v2/config/hotshot").send().await.unwrap();
    // The handler destructures HotShotConfig exhaustively, so a field it forgets to serve is
    // a compile error. What that cannot catch is a field wired to the wrong source or scaled
    // wrongly, so every field is compared here, especially the millisecond conversions.
    assert_eq!(
        v2_hotshot,
        espresso_api::proto::HotshotConfigResponse {
            start_threshold_numerator: v1_hotshot.start_threshold.0,
            start_threshold_denominator: v1_hotshot.start_threshold.1,
            num_nodes_with_stake: v1_hotshot.num_nodes_with_stake.get() as u64,
            da_staked_committee_size: v1_hotshot.da_staked_committee_size as u64,
            next_view_timeout_ms: v1_hotshot.next_view_timeout,
            view_sync_timeout_ms: v1_hotshot.view_sync_timeout.as_millis() as u64,
            builder_timeout_ms: v1_hotshot.builder_timeout.as_millis() as u64,
            data_request_delay_ms: v1_hotshot.data_request_delay.as_millis() as u64,
            builder_urls: v1_hotshot
                .builder_urls
                .iter()
                .map(ToString::to_string)
                .collect(),
            start_proposing_view: v1_hotshot.start_proposing_view,
            stop_proposing_view: v1_hotshot.stop_proposing_view,
            start_voting_view: v1_hotshot.start_voting_view,
            stop_voting_view: v1_hotshot.stop_voting_view,
            start_proposing_time: v1_hotshot.start_proposing_time,
            stop_proposing_time: v1_hotshot.stop_proposing_time,
            start_voting_time: v1_hotshot.start_voting_time,
            stop_voting_time: v1_hotshot.stop_voting_time,
            epoch_height: v1_hotshot.epoch_height,
            epoch_start_block: v1_hotshot.epoch_start_block,
            stake_table_capacity: v1_hotshot.stake_table_capacity as u64,
            drb_difficulty: v1_hotshot.drb_difficulty,
            drb_upgrade_difficulty: v1_hotshot.drb_upgrade_difficulty,
            known_nodes_with_stake: v1_hotshot
                .known_nodes_with_stake
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            known_da_nodes: v1_hotshot
                .known_da_nodes
                .iter()
                .cloned()
                .map(Into::into)
                .collect(),
            da_committees: v1_hotshot
                .da_committees
                .iter()
                .map(|da_committee| espresso_api::proto::VersionedDaCommittee {
                    start_version: da_committee.start_version.to_string(),
                    start_epoch: da_committee.start_epoch,
                    committee: da_committee
                        .committee
                        .iter()
                        .cloned()
                        .map(Into::into)
                        .collect(),
                })
                .collect(),
            fixed_leader_for_gpuvid: v1_hotshot.fixed_leader_for_gpuvid as u64,
            num_bootstrap: v1_hotshot.num_bootstrap as u64,
            commit_sha: v1_config.commit_sha().to_string(),
            indexed_da: v1_config.indexed_da(),
            cdn_marshal_address: v1_config.cdn_marshal_address().map(ToString::to_string),
            libp2p_config: v1_config.libp2p_config().map(|libp2p| {
                espresso_api::proto::Libp2pNetworkConfig {
                    bootstrap_nodes: libp2p
                        .bootstrap_nodes
                        .iter()
                        .map(
                            |(peer_id, multiaddr)| espresso_api::proto::Libp2pBootstrapNode {
                                peer_id: peer_id.to_string(),
                                multiaddr: multiaddr.to_string(),
                            },
                        )
                        .collect(),
                }
            }),
            combined_network_config: v1_config.combined_network_config().map(|combined| {
                espresso_api::proto::CombinedNetworkConfig {
                    delay_duration_ms: combined.delay_duration.as_millis() as u64,
                }
            }),
            builder: match v1_config.builder() {
                hotshot_types::network::BuilderType::External => {
                    espresso_api::proto::BuilderType::External
                },
                hotshot_types::network::BuilderType::Simple => {
                    espresso_api::proto::BuilderType::Simple
                },
                hotshot_types::network::BuilderType::Random => {
                    espresso_api::proto::BuilderType::Random
                },
            }
            .into(),
        }
    );
    // The comparison above builds its expected value with the same `From<PeerConfig>` the
    // handler uses, so it would agree with itself. Check one peer against the v1 types.
    assert!(!v2_hotshot.known_nodes_with_stake.is_empty());
    assert!(!v2_hotshot.known_da_nodes.is_empty());
    let v1_peer = &v1_hotshot.known_nodes_with_stake[0];
    let v2_peer = &v2_hotshot.known_nodes_with_stake[0];
    let v2_entry = v2_peer.stake_table_entry.as_ref().unwrap();
    assert_eq!(
        v2_entry.stake_key.as_ref().unwrap().key,
        v1_peer.stake_table_entry.stake_key.to_string()
    );
    assert_eq!(
        v2_entry.stake_amount,
        format!("{:#x}", v1_peer.stake_table_entry.stake_amount)
    );
    assert_eq!(
        v2_peer.state_ver_key.as_ref().unwrap().key,
        v1_peer.state_ver_key.to_string()
    );
    assert_eq!(
        v2_peer.connect_info.as_ref().unwrap().p2p_addr,
        v1_peer
            .connect_info
            .as_ref()
            .unwrap()
            .p2p_addr
            .unbracketed_string()
    );

    let v1_env: Vec<String> = client.get("config/env").send().await.unwrap();
    let v2_env: espresso_api::proto::EnvResponse =
        client.get("v2/config/env").send().await.unwrap();
    assert_eq!(
        v2_env
            .variables
            .iter()
            .map(|var| format!("{}={}", var.name, var.value))
            .collect::<Vec<_>>(),
        v1_env
    );

    // A TestNetwork registers no runtime config, which v1 reports as 404; v2 must not turn
    // that into a 500.
    let err = client
        .get::<serde_json::Value>("v2/config/runtime")
        .send()
        .await
        .unwrap_err();
    assert_eq!(err.status, StatusCode::NOT_FOUND);

    let v1_tables: Vec<crate::api::data_source::TableSize> =
        client.get("database/table-sizes").send().await.unwrap();
    let v2_tables: espresso_api::proto::TableSizesResponse =
        client.get("v2/database/table-sizes").send().await.unwrap();
    // Names are stable; row counts and byte sizes are not, since the network keeps deciding
    // blocks between the two requests and Postgres reports both approximately.
    let sorted = |mut names: Vec<String>| {
        names.sort();
        names
    };
    assert_eq!(
        sorted(
            v2_tables
                .tables
                .iter()
                .map(|t| t.table_name.clone())
                .collect()
        ),
        sorted(v1_tables.iter().map(|t| t.table_name.clone()).collect())
    );
    // Postgres reports names schema-qualified (`hotshot.header`), SQLite bare.
    assert!(
        v2_tables
            .tables
            .iter()
            .any(|table| table.table_name.ends_with("header")),
        "{v2_tables:?}"
    );
    // Values drift like the counts above, so compare only whether each table reports a size.
    // That presence is the one thing the mapping could quietly change.
    let v1_sizes: HashMap<&str, bool> = v1_tables
        .iter()
        .map(|table| (table.table_name.as_str(), table.total_size_bytes.is_some()))
        .collect();
    for table in &v2_tables.tables {
        assert_eq!(
            v1_sizes.get(table.table_name.as_str()),
            Some(&table.total_size_bytes.is_some()),
            "{table:?}"
        );
    }

    // Nothing in the repository runs a deferred migration yet, so without these rows both
    // versions report an empty list and agree vacuously.
    {
        let cfg = Config::try_from(&tmp_options(&storage)).unwrap();
        let db = SqlStorage::connect(cfg, StorageConnectionType::Query)
            .await
            .unwrap();
        let mut tx = db.write().await.unwrap();
        for (name, started_at, completed_at, last_offset) in [
            (
                "backfill_done",
                "2026-01-02T03:04:05.123456Z",
                Some("2026-01-02T03:14:15Z"),
                Some(4242i64),
            ),
            ("backfill_running", "2026-01-02T03:24:25.5Z", None, Some(0)),
        ] {
            let parse = |time: &str| {
                chrono::DateTime::parse_from_rfc3339(time)
                    .unwrap()
                    .with_timezone(&chrono::Utc)
            };
            sqlx::query(
                "INSERT INTO deferred_migrations (name, started_at, completed_at, last_offset)
                 VALUES ($1, $2, $3, $4)",
            )
            .bind(name)
            .bind(parse(started_at))
            .bind(completed_at.map(parse))
            .bind(last_offset)
            .execute(tx.as_mut())
            .await
            .unwrap();
        }
        hotshot_query_service::data_source::Transaction::commit(tx)
            .await
            .unwrap();
    }

    // As raw JSON, so the timestamp strings are compared as v1 actually serves them.
    let v1_migrations: serde_json::Value = client
        .get("database/migration-status")
        .send()
        .await
        .unwrap();
    let v2_migrations: espresso_api::proto::MigrationStatusResponse = client
        .get("v2/database/migration-status")
        .send()
        .await
        .unwrap();
    let v1_migrations = v1_migrations.as_array().unwrap();
    assert_eq!(v1_migrations.len(), 2, "{v1_migrations:?}");
    assert_eq!(v2_migrations.migrations.len(), v1_migrations.len());
    for (v1, v2) in v1_migrations.iter().zip(&v2_migrations.migrations) {
        assert_eq!(v2.name, v1["name"].as_str().unwrap());
        assert_eq!(v2.started_at, v1["started_at"].as_str().unwrap());
        assert_eq!(
            v2.completed_at.as_deref(),
            v1["completed_at"].as_str(),
            "{v1:?}"
        );
        assert_eq!(v2.last_offset, v1["last_offset"].as_i64());
    }

    check_availability_v2_parity(&client, port, first_block, last_block).await;
    check_merklized_state_v2_parity(&client, last_block).await;
}

/// Every merklized-state response on v2 must be the conversion of what v1 serves for the same
/// snapshot. The state is persisted behind decide, so this first waits for it to cover
/// `last_block`, after which every snapshot below the state height is stable.
async fn check_merklized_state_v2_parity(client: &HttpClient, last_block: u64) {
    use espresso_api::proto;

    let state_height = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let height: u64 = fetch(client, "block-state/block-height").await;
            if height > last_block {
                return height;
            }
            sleep(Duration::from_millis(200)).await;
        }
    })
    .await
    .expect("merklized state never caught up");
    let v2: proto::StateHeightResponse = fetch(client, "v2/merklized-state/height").await;
    assert_eq!(v2.height, state_height);

    // A snapshot at `state_height` commits to the headers below it, so the newest one is a
    // member and gets a leaf-first path.
    let key = state_height - 1;
    let v1_block: MerkleProof<Commitment<Header>, u64, Sha3Node, 3> =
        fetch(client, &format!("block-state/{state_height}/{key}")).await;
    let expected = proto::MerklePathResponse::from(&v1_block);
    let v2: proto::MerklePathResponse = fetch(
        client,
        &format!("v2/merklized-state/block/path?key={key}&height={state_height}"),
    )
    .await;
    assert_eq!(v2, expected);
    let root: Header = fetch(client, &format!("availability/header/{state_height}")).await;
    let commit = root.block_merkle_tree_root();
    let v1_block: MerkleProof<Commitment<Header>, u64, Sha3Node, 3> =
        fetch(client, &format!("block-state/commit/{commit}/{key}")).await;
    let v2: proto::MerklePathResponse = fetch(
        client,
        &format!("v2/merklized-state/block/path?key={key}&commit={commit}"),
    )
    .await;
    assert_eq!(v2, proto::MerklePathResponse::from(&v1_block));

    // The builder paid for `last_block`, so its account is in the fee tree.
    let header: Header = fetch(client, &format!("availability/header/{last_block}")).await;
    let account = header.fee_info().first().expect("a fee was paid").account();
    let v1_fee: MerkleProof<FeeAmount, FeeAccount, Sha3Node, 256> =
        fetch(client, &format!("fee-state/{state_height}/{account}")).await;
    let v2: proto::MerklePathResponse = fetch(
        client,
        &format!("v2/merklized-state/fee/path?address={account}&height={state_height}"),
    )
    .await;
    assert_eq!(v2, proto::MerklePathResponse::from(&v1_fee));

    let v1_balance: Option<FeeAmount> =
        fetch(client, &format!("fee-state/fee-balance/latest/{account}")).await;
    let v2: proto::FeeBalanceResponse = fetch(
        client,
        &format!("v2/merklized-state/fee/balance?address={account}"),
    )
    .await;
    assert_eq!(
        v2.balance,
        v1_balance.expect("the builder has a balance").0.to_string()
    );
    // An account the tree has never seen is a zero balance, not an error.
    let unknown = FeeAccount::from(alloy::primitives::Address::repeat_byte(0xee));
    let v1_balance: Option<FeeAmount> =
        fetch(client, &format!("fee-state/fee-balance/latest/{unknown}")).await;
    assert!(v1_balance.is_none());
    let v2: proto::FeeBalanceResponse = fetch(
        client,
        &format!("v2/merklized-state/fee/balance?address={unknown}"),
    )
    .await;
    assert_eq!(v2.balance, "0");

    let beyond = state_height + 1_000;
    for (v1, v2) in [
        (
            format!("block-state/{beyond}/{key}"),
            format!("v2/merklized-state/block/path?key={key}&height={beyond}"),
        ),
        (
            format!("block-state/commit/not-a-commitment/{key}"),
            format!("v2/merklized-state/block/path?key={key}&commit=not-a-commitment"),
        ),
        (
            format!("fee-state/{state_height}/not-an-address"),
            format!("v2/merklized-state/fee/path?address=not-an-address&height={state_height}"),
        ),
    ] {
        assert_eq!(
            error_status(client, &v2).await,
            error_status(client, &v1).await,
            "{v2}"
        );
    }
    for missing_selector in [
        format!("block/path?key={key}"),
        format!("block/path?key={key}&height={state_height}&commit={commit}"),
        format!("block/path?height={state_height}"),
        "fee/balance".to_owned(),
    ] {
        let status = error_status(client, &format!("v2/merklized-state/{missing_selector}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{missing_selector}");
    }
}

/// Every availability response on v2 must be the conversion of what v1 serves for the same
/// request, with v2's query parameters selecting what v1's path segments do.
async fn check_availability_v2_parity(
    client: &HttpClient,
    port: u16,
    first_block: u64,
    last_block: u64,
) {
    use espresso_api::proto;
    use espresso_types::{Header, NamespaceProofQueryData};
    use hotshot_query_service::availability::{
        BlockQueryData, BlockSummaryQueryData, LeafQueryData, Limits, PayloadQueryData,
        TransactionQueryData, TransactionWithProofQueryData, VidCommonQueryData,
    };

    let v1_limits: Limits = fetch(client, "availability/limits").await;
    let limits: proto::LimitsResponse = fetch(client, "v2/availability/limits").await;
    assert_eq!(
        limits.small_object_range_limit,
        v1_limits.small_object_range_limit as u64
    );
    assert_eq!(
        limits.large_object_range_limit,
        v1_limits.large_object_range_limit as u64
    );
    assert!(limits.namespace_proof_range_limit > 0);

    // A payload hash can match several blocks, so v1's answer for it is the reference rather
    // than block 1.
    let v1_header: Header = fetch(client, "availability/header/1").await;
    let header = proto::HeaderResponse::from(&v1_header);
    let payload_hash = v1_header.payload_commitment();
    let by_payload_hash: Header = fetch(
        client,
        &format!("availability/header/payload-hash/{payload_hash}"),
    )
    .await;
    for (query, expected) in [
        ("height=1".to_string(), header.clone()),
        (format!("hash={}", v1_header.commit()), header.clone()),
        (
            format!("payloadHash={payload_hash}"),
            proto::HeaderResponse::from(&by_payload_hash),
        ),
    ] {
        let v2: proto::HeaderResponse =
            fetch(client, &format!("v2/availability/header?{query}")).await;
        assert_eq!(v2, expected, "{query}");
    }

    let v1_leaf: LeafQueryData<SeqTypes> = fetch(client, "availability/leaf/1").await;
    let leaf = proto::LeafResponse::from(&v1_leaf);
    for query in ["height=1".to_string(), format!("hash={}", v1_leaf.hash())] {
        let v2: proto::LeafResponse = fetch(client, &format!("v2/availability/leaf?{query}")).await;
        assert_eq!(v2, leaf, "{query}");
    }

    let v1_block: BlockQueryData<SeqTypes> = fetch(client, "availability/block/1").await;
    let block = proto::BlockResponse::from(&v1_block);
    let block_hash = v1_block.hash();
    let by_payload_hash: BlockQueryData<SeqTypes> = fetch(
        client,
        &format!("availability/block/payload-hash/{payload_hash}"),
    )
    .await;
    for (query, expected) in [
        ("height=1".to_string(), block.clone()),
        (format!("hash={block_hash}"), block.clone()),
        (
            format!("payloadHash={payload_hash}"),
            proto::BlockResponse::from(&by_payload_hash),
        ),
    ] {
        let v2: proto::BlockResponse =
            fetch(client, &format!("v2/availability/block?{query}")).await;
        assert_eq!(v2, expected, "{query}");
    }

    let v1_payload: PayloadQueryData<SeqTypes> = fetch(client, "availability/payload/1").await;
    let payload = proto::PayloadResponse::from(&v1_payload);
    let by_hash: PayloadQueryData<SeqTypes> =
        fetch(client, &format!("availability/payload/hash/{payload_hash}")).await;
    for (query, expected) in [
        ("height=1".to_string(), payload.clone()),
        (format!("blockHash={block_hash}"), payload.clone()),
        (
            format!("hash={payload_hash}"),
            proto::PayloadResponse::from(&by_hash),
        ),
    ] {
        let v2: proto::PayloadResponse =
            fetch(client, &format!("v2/availability/payload?{query}")).await;
        assert_eq!(v2, expected, "{query}");
    }

    let v1_vid: VidCommonQueryData<SeqTypes> = fetch(client, "availability/vid/common/1").await;
    let vid = proto::VidCommonResponse::try_from(&v1_vid).unwrap();
    let by_payload_hash: VidCommonQueryData<SeqTypes> = fetch(
        client,
        &format!("availability/vid/common/payload-hash/{payload_hash}"),
    )
    .await;
    for (query, expected) in [
        ("height=1".to_string(), vid.clone()),
        (format!("hash={block_hash}"), vid.clone()),
        (
            format!("payloadHash={payload_hash}"),
            proto::VidCommonResponse::try_from(&by_payload_hash).unwrap(),
        ),
    ] {
        let v2: proto::VidCommonResponse =
            fetch(client, &format!("v2/availability/vid-common?{query}")).await;
        assert_eq!(v2, expected, "{query}");
    }

    // Transactions are submitted only after connecting, so block 1 is empty and `last_block` is
    // the one height known to carry a transaction.
    for height in [1, last_block] {
        let v1: BlockSummaryQueryData<SeqTypes> =
            fetch(client, &format!("availability/block/summary/{height}")).await;
        let v2: proto::BlockSummaryResponse = fetch(
            client,
            &format!("v2/availability/block-summary?height={height}"),
        )
        .await;
        assert_eq!(v2, proto::BlockSummaryResponse::from(&v1), "{height}");
    }

    let v1_tx: TransactionQueryData<SeqTypes> = fetch(
        client,
        &format!("availability/transaction/{last_block}/0/noproof"),
    )
    .await;
    let tx = proto::TransactionResponse::from(&v1_tx);
    let v1_proven: TransactionWithProofQueryData<SeqTypes> = fetch(
        client,
        &format!("availability/transaction/{last_block}/0/proof"),
    )
    .await;
    let proven = proto::TransactionWithProofResponse::try_from(&v1_proven).unwrap();
    for query in [
        format!("height={last_block}&index=0"),
        format!("hash={}", v1_tx.hash()),
    ] {
        let v2: proto::TransactionResponse =
            fetch(client, &format!("v2/availability/transaction?{query}")).await;
        assert_eq!(v2, tx, "{query}");
        let v2: proto::TransactionWithProofResponse = fetch(
            client,
            &format!("v2/availability/transaction-proof?{query}"),
        )
        .await;
        assert_eq!(v2, proven, "{query}");
    }
    // A 0.1 block is disseminated with ADVZ, so the proof must land on that arm and carry
    // the range proof of a non-empty transaction.
    let Some(proto::tx_proof::Proof::V0(v0)) = proven.proof.unwrap().proof else {
        panic!("a 0.1 block's inclusion proof is ADVZ");
    };
    assert!(v0.payload_proof_tx.is_some());

    // 102 is the namespace of the last submitted transaction, so `last_block` carries it.
    let v1_proof: NamespaceProofQueryData = fetch(
        client,
        &format!("availability/block/{last_block}/namespace/102"),
    )
    .await;
    assert!(v1_proof.proof.is_some() && !v1_proof.transactions.is_empty());
    let proof = proto::NamespaceProofResponse::try_from(&v1_proof).unwrap();
    let last: BlockQueryData<SeqTypes> =
        fetch(client, &format!("availability/block/{last_block}")).await;
    for selector in [
        format!("height={last_block}"),
        format!("hash={}", last.hash()),
        format!("payloadHash={}", last.payload_hash()),
    ] {
        let v2: proto::NamespaceProofResponse = fetch(
            client,
            &format!("v2/availability/namespace-proof?{selector}&namespace=102"),
        )
        .await;
        assert_eq!(v2, proof, "{selector}");
    }
    // A namespace no block carries is an absent proof, not an error.
    let v1_absent: NamespaceProofQueryData =
        fetch(client, "availability/block/1/namespace/4294967295").await;
    assert!(v1_absent.proof.is_none() && v1_absent.transactions.is_empty());
    let v2_absent: proto::NamespaceProofResponse = fetch(
        client,
        "v2/availability/namespace-proof?height=1&namespace=4294967295",
    )
    .await;
    assert_eq!(
        v2_absent,
        proto::NamespaceProofResponse::try_from(&v1_absent).unwrap()
    );

    for missing_selector in [
        "header",
        "header?height=1&hash=x",
        "leaf",
        "block",
        "vid-common",
        "transaction?height=1",
        "namespace-proof?namespace=1",
        "block-range?from=0",
        // Without `from`, a stream would replay the chain from genesis.
        "stream/headers",
    ] {
        let status = error_status(client, &format!("v2/availability/{missing_selector}")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{missing_selector}");
    }

    // Without epochs there is no state certificate, a 0.1 block has no AvidM encoding to prove
    // wrong, and no cert2 exists before the new protocol takes over. These only show both
    // versions refusing alike. The conversions are covered by the reference-vector tests.
    for (v1, v2) in [
        (
            "availability/state-cert/1".to_string(),
            "v2/availability/state-cert?epoch=1".to_string(),
        ),
        (
            "availability/state-cert-v2/1".to_string(),
            "v2/availability/state-cert-v2?epoch=1".to_string(),
        ),
        (
            format!("availability/incorrect-encoding-proof/{last_block}/102"),
            format!("v2/availability/incorrect-encoding-proof?height={last_block}&namespace=102"),
        ),
        (
            "availability/cert2/1".to_string(),
            "v2/availability/cert2?height=1".to_string(),
        ),
    ] {
        assert_eq!(
            error_status(client, &v2).await,
            error_status(client, &v1).await,
            "{v2}"
        );
    }

    check_ranges_v2_parity(client, &limits, first_block, last_block).await;

    // Each stream's first frame is the unary answer for the height it starts from.
    assert_eq!(
        first_sse_frame::<proto::LeafResponse>(port, "leaves?from=1").await,
        leaf
    );
    assert_eq!(
        first_sse_frame::<proto::HeaderResponse>(port, "headers?from=1").await,
        header
    );
    assert_eq!(
        first_sse_frame::<proto::BlockResponse>(port, "blocks?from=1").await,
        block
    );
    assert_eq!(
        first_sse_frame::<proto::PayloadResponse>(port, "payloads?from=1").await,
        payload
    );
    assert_eq!(
        first_sse_frame::<proto::VidCommonResponse>(port, "vid-common?from=1").await,
        vid
    );
    let streamed: proto::TransactionResponse =
        first_sse_frame(port, &format!("transactions?from={last_block}")).await;
    assert_eq!(streamed, tx);
    let streamed: proto::NamespaceProofResponse = first_sse_frame(
        port,
        &format!("namespace-proofs?from={last_block}&namespace=102"),
    )
    .await;
    assert_eq!(streamed, proof);
    // From `first_block`, whose transaction is in namespace 101, the filter must skip ahead.
    let streamed: proto::TransactionResponse = first_sse_frame(
        port,
        &format!("transactions?from={first_block}&namespace=102"),
    )
    .await;
    let v1: TransactionQueryData<SeqTypes> = fetch(
        client,
        &format!("availability/transaction/hash/{}/noproof", streamed.hash),
    )
    .await;
    assert_eq!(v1.namespace().0, 102);
    assert_eq!(streamed, proto::TransactionResponse::from(&v1));
}

/// The range and batch endpoints reuse the single lookups' item conversions, so what these
/// pin is that the bounds select what v1's do, and that each is held to its own limit class.
async fn check_ranges_v2_parity(
    client: &HttpClient,
    limits: &espresso_api::proto::LimitsResponse,
    first_block: u64,
    last_block: u64,
) {
    use espresso_api::proto;
    use hotshot_query_service::availability::{
        BlockQueryData, BlockSummaryQueryData, LeafQueryData, PayloadQueryData, VidCommonQueryData,
    };

    let (from, until) = (first_block, last_block + 1);
    let bounds = format!("{from}/{until}");
    let query = format!("from={from}&until={until}");

    let headers: Vec<espresso_types::Header> =
        fetch(client, &format!("availability/header/{bounds}")).await;
    let v2_headers: proto::HeaderRangeResponse =
        fetch(client, &format!("v2/availability/header-range?{query}")).await;
    assert_eq!(v2_headers, proto::HeaderRangeResponse::from(&*headers));
    let leaves: Vec<LeafQueryData<SeqTypes>> =
        fetch(client, &format!("availability/leaf/{bounds}")).await;
    let v2_leaves: proto::LeafRangeResponse =
        fetch(client, &format!("v2/availability/leaf-range?{query}")).await;
    assert_eq!(v2_leaves, proto::LeafRangeResponse::from(&*leaves));
    let blocks: Vec<BlockQueryData<SeqTypes>> =
        fetch(client, &format!("availability/block/{bounds}")).await;
    let v2_blocks: proto::BlockRangeResponse =
        fetch(client, &format!("v2/availability/block-range?{query}")).await;
    assert_eq!(v2_blocks, proto::BlockRangeResponse::from(&*blocks));
    let payloads: Vec<PayloadQueryData<SeqTypes>> =
        fetch(client, &format!("availability/payload/{bounds}")).await;
    let v2_payloads: proto::PayloadRangeResponse =
        fetch(client, &format!("v2/availability/payload-range?{query}")).await;
    assert_eq!(v2_payloads, proto::PayloadRangeResponse::from(&*payloads));
    let vid: Vec<VidCommonQueryData<SeqTypes>> =
        fetch(client, &format!("availability/vid/common/{bounds}")).await;
    let v2_vid: proto::VidCommonRangeResponse =
        fetch(client, &format!("v2/availability/vid-common-range?{query}")).await;
    assert_eq!(
        v2_vid,
        proto::VidCommonRangeResponse::try_from(&*vid).unwrap()
    );
    let summaries: Vec<BlockSummaryQueryData<SeqTypes>> =
        fetch(client, &format!("availability/block/summaries/{bounds}")).await;
    let v2_summaries: proto::BlockSummaryRangeResponse = fetch(
        client,
        &format!("v2/availability/block-summary-range?{query}"),
    )
    .await;
    assert_eq!(
        v2_summaries,
        proto::BlockSummaryRangeResponse::from(&*summaries)
    );
    let proofs: Vec<espresso_types::NamespaceProofQueryData> = fetch(
        client,
        &format!("availability/block/{bounds}/namespace/102"),
    )
    .await;
    let v2_proofs: proto::NamespaceProofRangeResponse = fetch(
        client,
        &format!("v2/availability/namespace-proof-range?{query}&namespace=102"),
    )
    .await;
    assert_eq!(
        v2_proofs,
        proto::NamespaceProofRangeResponse::try_from(&*proofs).unwrap()
    );

    // One height past each class's limit, so an endpoint held to the wrong class would pass
    // where v1 refuses.
    let small = limits.small_object_range_limit + 1;
    let large = limits.large_object_range_limit + 1;
    let namespace = limits.namespace_proof_range_limit + 1;
    for (v1, v2) in [
        (
            format!("availability/leaf/0/{small}"),
            format!("v2/availability/leaf-range?from=0&until={small}"),
        ),
        (
            format!("availability/vid/common/0/{small}"),
            format!("v2/availability/vid-common-range?from=0&until={small}"),
        ),
        (
            format!("availability/header/0/{large}"),
            format!("v2/availability/header-range?from=0&until={large}"),
        ),
        (
            format!("availability/block/0/{large}"),
            format!("v2/availability/block-range?from=0&until={large}"),
        ),
        (
            format!("availability/payload/0/{large}"),
            format!("v2/availability/payload-range?from=0&until={large}"),
        ),
        (
            format!("availability/block/summaries/0/{large}"),
            format!("v2/availability/block-summary-range?from=0&until={large}"),
        ),
        (
            format!("availability/block/0/{namespace}/namespace/102"),
            format!("v2/availability/namespace-proof-range?from=0&until={namespace}&namespace=102"),
        ),
    ] {
        let v1_status = error_status(client, &v1).await;
        let v2_status = error_status(client, &v2).await;
        assert_eq!(v2_status, StatusCode::BAD_REQUEST, "{v2}");
        assert_eq!(v2_status, v1_status, "{v2}");
    }

    // Two ranges with a gap between them, which is the case the batch endpoints exist for.
    let ranges = [first_block..first_block + 1, last_block..last_block + 1];
    let body = serde_json::json!({
        "ranges": ranges
            .iter()
            .map(|range| serde_json::json!({"from": range.start, "until": range.end}))
            .collect::<Vec<_>>(),
    });
    let leaves: Vec<LeafQueryData<SeqTypes>> =
        post(client, "availability/leaf/ranges", &ranges).await;
    assert_eq!(leaves.len(), 2);
    let v2_leaves: proto::LeafRangeResponse =
        post(client, "v2/availability/leaf-ranges", &body).await;
    assert_eq!(v2_leaves, proto::LeafRangeResponse::from(&*leaves));
    let blocks: Vec<BlockQueryData<SeqTypes>> =
        post(client, "availability/block/ranges", &ranges).await;
    let v2_blocks: proto::BlockRangeResponse =
        post(client, "v2/availability/block-ranges", &body).await;
    assert_eq!(v2_blocks, proto::BlockRangeResponse::from(&*blocks));
    let vid: Vec<VidCommonQueryData<SeqTypes>> =
        post(client, "availability/vid/common/ranges", &ranges).await;
    let v2_vid: proto::VidCommonRangeResponse =
        post(client, "v2/availability/vid-common-ranges", &body).await;
    assert_eq!(
        v2_vid,
        proto::VidCommonRangeResponse::try_from(&*vid).unwrap()
    );
    // Out of order is refused by the shared validation, and an absent bound by v2's own.
    for body in [
        serde_json::json!({"ranges": [
            {"from": last_block, "until": last_block + 1},
            {"from": first_block, "until": first_block + 1},
        ]}),
        serde_json::json!({"ranges": [{"from": first_block}]}),
    ] {
        let err = client
            .post::<proto::BlockRangeResponse>("v2/availability/block-ranges")
            .body_json(&body)
            .unwrap()
            .send()
            .await
            .unwrap_err();
        assert_eq!(err.status, StatusCode::BAD_REQUEST, "{body}");
    }
}

/// The first data frame of a v2 availability stream, read under a deadline since a stream
/// never ends on its own.
async fn first_sse_frame<T>(port: u16, stream: &str) -> T
where
    T: serde::de::DeserializeOwned,
{
    let mut response = reqwest::Client::new()
        .get(format!(
            "http://localhost:{port}/v2/availability/stream/{stream}"
        ))
        .header("Accept", "text/event-stream")
        .send()
        .await
        .unwrap();
    assert!(
        response.headers()["content-type"]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream"),
        "{stream}: {:?}",
        response.headers()
    );
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(60);
    let mut body = Vec::new();
    loop {
        let chunk = tokio::time::timeout_at(deadline, response.chunk())
            .await
            .unwrap_or_else(|_| panic!("{stream}: no event before the deadline"))
            .unwrap()
            .expect("stream still open");
        body.extend_from_slice(&chunk);
        // An event ends at a blank line and a chunk boundary can split one, even mid
        // character, so only the events before the last blank line are decoded. A keep-alive
        // comment is a complete event with no data line, which is skipped.
        let Some(end) = body.windows(2).rposition(|pair| pair == b"\n\n") else {
            continue;
        };
        let complete = std::str::from_utf8(&body[..end]).unwrap();
        if let Some(data) = complete
            .split("\n\n")
            .find_map(|event| event.lines().find_map(|line| line.strip_prefix("data:")))
        {
            return serde_json::from_str(data.trim())
                .unwrap_or_else(|err| panic!("{stream}: {err}: {data}"));
        }
    }
}
