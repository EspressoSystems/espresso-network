//! Smoke test: spawn a small committee (5 nodes) in-process
//! and verify that consensus advances and produces CSV metrics.
//!
//! Nodes self-bootstrap via `seed_genesis` + `start()` — no external
//! orchestrator is needed to inject the first proposal.

use std::time::Duration;

use hotshot_new_protocol_bench::config::NodeConfig;
use tempfile::TempDir;
use tokio::time::timeout;

const NUM_NODES: usize = 5;
const TARGET_VIEWS: u64 = 10;
const SEED: u8 = 0;
const TIMEOUT_MS: u64 = 5000;
/// Per-test timeout to prevent hanging.
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

/// Base port for CliqueNet. Each node gets `BASE_PORT + node_id`.
const BASE_PORT: u16 = 19000;

fn peer_list() -> Vec<String> {
    (0..NUM_NODES)
        .map(|i| format!("127.0.0.1:{}", BASE_PORT + i as u16))
        .collect()
}

fn node_config(node_id: u64, output_dir: &std::path::Path) -> NodeConfig {
    NodeConfig {
        node_id,
        total_nodes: NUM_NODES,
        seed: SEED,
        timeout_ms: TIMEOUT_MS,
        target_views: TARGET_VIEWS,
        bind_addr: format!("127.0.0.1:{}", BASE_PORT + node_id as u16),
        peers: peer_list(),
        output_file: output_dir
            .join(format!("node_{node_id}.csv"))
            .to_string_lossy()
            .into_owned(),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke_5_nodes() {
    tracing_subscriber::fmt()
        .with_env_filter("info")
        .try_init()
        .ok();

    let tmp = TempDir::new().expect("failed to create temp dir");

    let result = timeout(TEST_TIMEOUT, run_benchmark(tmp.path())).await;

    match result {
        Ok(Ok(())) => {},
        Ok(Err(e)) => panic!("benchmark failed: {e:#}"),
        Err(_) => panic!("benchmark timed out after {TEST_TIMEOUT:?}"),
    }
}

async fn run_benchmark(output_dir: &std::path::Path) -> anyhow::Result<()> {
    let mut node_handles = Vec::new();

    // Spawn all nodes as async tasks.  Each node self-bootstraps via
    // seed_genesis + start() — no orchestrator needed.
    for i in 0..NUM_NODES as u64 {
        let cfg = node_config(i, output_dir);
        node_handles.push(tokio::spawn(async move {
            hotshot_new_protocol_bench::node::run(cfg).await
        }));
    }

    // Wait for all nodes to reach target_views and exit.
    for (i, handle) in node_handles.into_iter().enumerate() {
        handle
            .await
            .unwrap_or_else(|e| panic!("node {i} task panicked: {e}"))
            .unwrap_or_else(|e| panic!("node {i} failed: {e:#}"));
    }

    // Verify each node produced a CSV with decided views.
    for i in 0..NUM_NODES as u64 {
        let csv_path = output_dir.join(format!("node_{i}.csv"));
        assert!(
            csv_path.exists(),
            "node {i} did not produce CSV at {csv_path:?}"
        );

        let content = std::fs::read_to_string(&csv_path)
            .unwrap_or_else(|e| panic!("failed to read CSV for node {i}: {e}"));

        let lines: Vec<&str> = content.lines().collect();
        assert!(
            lines.len() >= 2,
            "node {i} CSV has only {} lines (expected header + data)",
            lines.len()
        );

        let header = lines[0];
        let decided_col = header
            .split(',')
            .position(|h| h == "leaf_decided_ns")
            .expect("CSV missing leaf_decided_ns column");

        let decided_count = lines[1..]
            .iter()
            .filter(|line| {
                line.split(',')
                    .nth(decided_col)
                    .is_some_and(|v| !v.is_empty())
            })
            .count();

        assert!(
            decided_count > 0,
            "node {i} has no decided views in CSV ({} data rows)",
            lines.len() - 1
        );

        eprintln!(
            "node {i}: {decided_count} decided views in {} total rows",
            lines.len() - 1
        );
    }

    Ok(())
}
