use std::time::Duration;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "new-protocol-bench")]
#[command(about = "Benchmark binary for the new consensus protocol")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Run a single consensus node.
    Node(NodeConfig),
    /// Run the orchestrator that bootstraps consensus.
    Orchestrator(OrchestratorConfig),
}

#[derive(Parser, Clone)]
pub struct NodeConfig {
    /// This node's index (0-based).
    #[arg(long)]
    pub node_id: u64,

    /// Total number of consensus nodes.
    #[arg(long)]
    pub total_nodes: usize,

    /// Seed for deterministic key generation.
    #[arg(long, default_value_t = 0)]
    pub seed: u8,

    /// View timeout in milliseconds.
    #[arg(long, default_value_t = 5000)]
    pub timeout_ms: u64,

    /// Stop after this many views have been decided.
    #[arg(long, default_value_t = 100)]
    pub target_views: u64,

    /// Address to bind CliqueNet on (e.g. "0.0.0.0:9000").
    #[arg(long)]
    pub bind_addr: String,

    /// Comma-separated list of peer addresses in order of node index.
    /// Format: "host1:port1,host2:port2,..."
    /// Must have exactly `total_nodes` entries.
    #[arg(long, value_delimiter = ',')]
    pub peers: Vec<String>,

    /// Output CSV file path.
    #[arg(long, default_value = "results.csv")]
    pub output_file: String,
}

impl NodeConfig {
    pub fn timeout_duration(&self) -> Duration {
        Duration::from_millis(self.timeout_ms)
    }
}

#[derive(Parser, Clone)]
pub struct OrchestratorConfig {
    /// Total number of consensus nodes.
    #[arg(long)]
    pub total_nodes: usize,

    /// Seed for deterministic key generation (must match nodes).
    #[arg(long, default_value_t = 0)]
    pub seed: u8,

    /// Number of views to run.
    #[arg(long, default_value_t = 100)]
    pub target_views: u64,

    /// Address to bind CliqueNet on (e.g. "0.0.0.0:9100").
    #[arg(long)]
    pub bind_addr: String,

    /// Comma-separated list of all node addresses in order of node index.
    #[arg(long, value_delimiter = ',')]
    pub peers: Vec<String>,
}
