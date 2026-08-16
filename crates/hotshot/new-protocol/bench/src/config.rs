use std::time::Duration;

use clap::Parser;

#[derive(Parser, Clone)]
#[command(name = "new-protocol-bench-node")]
#[command(about = "Benchmark node for the new consensus protocol")]
pub struct NodeConfig {
    /// This node's index.
    #[arg(long)]
    pub node_id: u64,

    /// Total number of consensus nodes.
    #[arg(long)]
    pub total_nodes: usize,

    /// View timeout in milliseconds.
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,

    /// Stop after this many views have been decided.
    #[arg(long, default_value_t = 100)]
    pub target_views: u64,

    /// Network bind address.
    #[arg(long)]
    pub bind_addr: String,

    /// Comma-separated list of peer addresses in order of node index.
    #[arg(long, value_delimiter = ',')]
    pub peers: Vec<String>,

    /// Output CSV file path.
    #[arg(long, default_value = "results.csv")]
    pub output_file: String,

    /// Block payload size in bytes.
    #[arg(long, default_value_t = 0)]
    pub block_size: usize,

    /// Number of namespaces to split each block payload into. AvidmGf2Scheme's
    /// `ns_disperse` and `recover` parallelize per-namespace via rayon, so
    /// increasing this should reduce dispersal/recovery latency up to the
    /// machine's core count. Default 1 = legacy single-namespace behaviour.
    #[arg(long, default_value_t = 1)]
    pub namespaces: u32,

    /// Period between CPU + network sampler ticks (milliseconds). 50ms is the
    /// default; lower values give finer resolution at the cost of more
    /// /proc reads per second.
    #[arg(long, default_value_t = 50)]
    pub sampler_tick_ms: u64,
}

impl NodeConfig {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}
