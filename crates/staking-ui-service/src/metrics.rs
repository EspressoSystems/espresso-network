//! Prometheus metrics for the staking UI service.

use prometheus::{Encoder, Gauge, IntGaugeVec, Opts, Registry, TextEncoder};

/// Prometheus metrics for the staking UI service.
#[derive(Clone, Debug)]
pub struct PrometheusMetrics {
    registry: Registry,

    // L1 metrics
    /// Number of unique wallets in the latest L1 snapshot.
    pub unique_wallets: Gauge,
    /// The latest L1 block number
    pub latest_l1_block: Gauge,
    /// The finalized L1 block number.
    pub finalized_l1_block: Gauge,
    /// Number of nodes in the latest L1 snapshot.
    pub node_count: Gauge,

    // Espresso metrics
    /// Latest Espresso block number.
    pub latest_espresso_block: Gauge,
    /// Current epoch number.
    pub current_epoch: Gauge,
    /// Number of active validators in current epoch.
    pub active_validators: Gauge,
}

impl Default for PrometheusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

impl PrometheusMetrics {
    /// Create a new metrics instance with all gauges registered.
    pub fn new() -> Self {
        let registry = Registry::new();

        // L1 metrics
        let unique_wallets = Gauge::with_opts(Opts::new(
            "unique_wallets",
            "Number of unique wallets in the latest L1 snapshot",
        ))
        .expect("failed to create unique_wallets gauge");
        registry
            .register(Box::new(unique_wallets.clone()))
            .expect("failed to register unique_wallets gauge");

        let latest_l1_block = Gauge::with_opts(Opts::new(
            "latest_l1_block",
            "The latest L1 block number that has been processed",
        ))
        .expect("failed to create latest_l1_block gauge");
        registry
            .register(Box::new(latest_l1_block.clone()))
            .expect("failed to register latest_l1_block gauge");

        let finalized_l1_block = Gauge::with_opts(Opts::new(
            "finalized_l1_block",
            "The finalized L1 block number",
        ))
        .expect("failed to create finalized_l1_block gauge");
        registry
            .register(Box::new(finalized_l1_block.clone()))
            .expect("failed to register finalized_l1_block gauge");

        let node_count = Gauge::with_opts(Opts::new(
            "node_count",
            "Number of validators/nodes in the latest L1 snapshot",
        ))
        .expect("failed to create node_count gauge");
        registry
            .register(Box::new(node_count.clone()))
            .expect("failed to register node_count gauge");

        // Espresso metrics
        let latest_espresso_block = Gauge::with_opts(Opts::new(
            "latest_espresso_block",
            "Latest Espresso block number processed",
        ))
        .expect("failed to create latest_espresso_block gauge");
        registry
            .register(Box::new(latest_espresso_block.clone()))
            .expect("failed to register latest_espresso_block gauge");

        let current_epoch = Gauge::with_opts(Opts::new("current_epoch", "Current epoch number"))
            .expect("failed to create current_epoch gauge");
        registry
            .register(Box::new(current_epoch.clone()))
            .expect("failed to register current_epoch gauge");

        let active_validators = Gauge::with_opts(Opts::new(
            "active_validators",
            "Number of active validators in current epoch",
        ))
        .expect("failed to create active_validators gauge");
        registry
            .register(Box::new(active_validators.clone()))
            .expect("failed to register active_validators gauge");

        Self {
            registry,
            unique_wallets,
            latest_l1_block,
            finalized_l1_block,
            node_count,
            latest_espresso_block,
            current_epoch,
            active_validators,
        }
    }

    /// Expose version information via metrics.
    pub fn register_version_info(&self) -> prometheus::Result<()> {
        let version_info = IntGaugeVec::new(
            Opts::new("version", "The version of this binary"),
            &["rev", "desc", "timestamp"],
        )?;
        self.registry.register(Box::new(version_info.clone()))?;
        version_info
            .get_metric_with_label_values(&[
                env!("VERGEN_GIT_SHA"),
                env!("VERGEN_GIT_DESCRIBE"),
                env!("VERGEN_GIT_COMMIT_TIMESTAMP"),
            ])?
            .set(1);
        Ok(())
    }
}

impl tide_disco::metrics::Metrics for PrometheusMetrics {
    type Error = prometheus::Error;

    fn export(&self) -> Result<String, Self::Error> {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = vec![];
        encoder.encode(&metric_families, &mut buffer)?;
        String::from_utf8(buffer).map_err(|err| {
            prometheus::Error::Msg(format!("metrics output is not valid UTF-8: {err}"))
        })
    }
}
