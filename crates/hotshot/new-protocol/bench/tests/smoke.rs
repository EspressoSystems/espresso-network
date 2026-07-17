//! Smoke test: spawn a small committee (5 nodes) in-process
//! and verify that consensus advances and produces CSV metrics.
use std::time::Duration;

use hotshot_new_protocol_bench::config::NodeConfig;
use tempfile::TempDir;
use tokio::time::timeout;

const NUM_NODES: usize = 5;
const TARGET_VIEWS: u64 = 10;
const TIMEOUT_MS: u64 = 5000;
const TEST_TIMEOUT: Duration = Duration::from_secs(60);

fn allocate_ports(n: usize) -> Vec<u16> {
    (0..n)
        .map(|_| test_utils::reserve_tcp_port().expect("ephemeral port"))
        .collect()
}

fn peer_list(ports: &[u16]) -> Vec<String> {
    ports.iter().map(|p| format!("127.0.0.1:{p}")).collect()
}

fn node_config(
    node_id: u64,
    output_dir: &std::path::Path,
    block_size: usize,
    ports: &[u16],
) -> NodeConfig {
    NodeConfig {
        node_id,
        total_nodes: NUM_NODES,
        timeout_ms: TIMEOUT_MS,
        target_views: TARGET_VIEWS,
        bind_addr: format!("127.0.0.1:{}", ports[node_id as usize]),
        peers: peer_list(ports),
        output_file: output_dir
            .join(format!("node_{node_id}.csv"))
            .to_string_lossy()
            .into_owned(),
        block_size,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke_5_nodes_empty_blocks() {
    hotshot_new_protocol::logging::init_logging();

    let tmp = TempDir::new().expect("failed to create temp dir");
    let ports = allocate_ports(NUM_NODES);

    let result = timeout(TEST_TIMEOUT, run_benchmark(tmp.path(), 0, &ports)).await;

    match result {
        Ok(Ok(())) => {},
        Ok(Err(e)) => panic!("benchmark failed: {e:#}"),
        Err(_) => panic!("benchmark timed out after {TEST_TIMEOUT:?}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn smoke_5_nodes_1kb_blocks() {
    hotshot_new_protocol::logging::init_logging();

    let tmp = TempDir::new().expect("failed to create temp dir");
    let ports = allocate_ports(NUM_NODES);

    let result = timeout(TEST_TIMEOUT, run_benchmark(tmp.path(), 1024, &ports)).await;

    match result {
        Ok(Ok(())) => {},
        Ok(Err(e)) => panic!("benchmark failed: {e:#}"),
        Err(_) => panic!("benchmark timed out after {TEST_TIMEOUT:?}"),
    }
}

async fn run_benchmark(
    output_dir: &std::path::Path,
    block_size: usize,
    ports: &[u16],
) -> anyhow::Result<()> {
    let mut node_handles = Vec::new();

    for i in 0..NUM_NODES as u64 {
        let cfg = node_config(i, output_dir, block_size, ports);
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
