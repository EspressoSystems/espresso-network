#![allow(clippy::needless_lifetimes)]

use core::fmt::Display;
use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::{self, Formatter},
    iter::once,
    path::PathBuf,
    time::Duration,
};

use clap::{Args, FromArgMatches, Parser, error::ErrorKind};
use derivative::Derivative;
use espresso_types::{BackoffParams, L1ClientOptions, parse_duration};
use espresso_utils::logging;
use hotshot_types::addr::NetAddr;
use libp2p::Multiaddr;
use light_client::{state::LightClientOptions, storage::LightClientSqliteOptions};
use serde::Serialize;
use url::Url;

use crate::{api, keyset::KeySetOptions, persistence, proposal_fetcher::ProposalFetcherConfig};

// This options struct is a bit unconventional. The sequencer has multiple optional modules which
// can be added, in any combination, to the service. These include, for example, the API server.
// Each of these modules has its own options, which are all required if the module is added but can
// be omitted otherwise. Clap doesn't have a good way to handle "grouped" arguments like this (they
// have something called an argument group, but it's different). Sub-commands do exactly this, but
// you can't have multiple sub-commands in a single command.
//
// What we do, then, is take the optional modules as if they were sub-commands, but we use a Clap
// `raw` argument to collect all the module commands and their options into a single string. This
// string is then parsed manually (using a secondary Clap `Parser`, the `SequencerModule` type) when
// the user calls `modules()`.
//
// One slightly unfortunate consequence of this is that the auto-generated help documentation for
// `SequencerModule` is not included in the help for this top-level type. Users can still get at the
// help for individual modules by passing `help` as a subcommand, as in
// `sequencer [options] -- help` or `sequencer [options] -- help <module>`. This means that IT IS
// BEST NOT TO ADD REQUIRED ARGUMENTS TO THIS TYPE, since the required arguments will be required
// even if the user is only asking for help on a module. Try to give every argument on this type a
// default value, even if it is a bit arbitrary.
#[derive(Parser, Clone, Derivative)]
#[derivative(Debug(bound = ""))]
#[command(version = build_version())]
pub struct Options {
    /// URL of the HotShot orchestrator.
    #[clap(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_ORCHESTRATOR_URL",
        default_value = "http://localhost:8080"
    )]
    #[derivative(Debug(format_with = "Display::fmt"))]
    pub orchestrator_url: Url,

    /// The socket address of the HotShot CDN's main entry point (the marshal)
    /// in `IP:port` form
    #[clap(
        short,
        long,
        env = "ESPRESSO_SEQUENCER_CDN_ENDPOINT",
        default_value = "127.0.0.1:8081"
    )]
    pub cdn_endpoint: String,

    /// The address to bind to for cliquenet (in `host:port` | `ip:port` form)
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_CLIQUENET_BIND_ADDRESS",
        default_value = "0.0.0.0:9977"
    )]
    pub cliquenet_bind_address: NetAddr,

    /// The address to bind to for Libp2p (in `host:port` form)
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_BIND_ADDRESS",
        default_value = "0.0.0.0:1769"
    )]
    pub libp2p_bind_address: String,

    /// Time between each Libp2p heartbeat
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_HEARTBEAT_INTERVAL", default_value = "1s", value_parser = parse_duration)]
    pub libp2p_heartbeat_interval: Duration,

    /// Number of past heartbeats to gossip about on Libp2p
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_HISTORY_GOSSIP",
        default_value = "3"
    )]
    pub libp2p_history_gossip: usize,

    /// Number of heartbeats to keep in the Libp2p `memcache`
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_HISTORY_LENGTH",
        default_value = "5"
    )]
    pub libp2p_history_length: usize,

    /// Target number of peers for the Libp2p mesh network
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_MESH_N", default_value = "8")]
    pub libp2p_mesh_n: usize,

    /// Maximum number of peers in the Libp2p mesh network before removing some
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MESH_N_HIGH",
        default_value = "12"
    )]
    pub libp2p_mesh_n_high: usize,

    /// Minimum number of peers in the Libp2p mesh network before adding more
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MESH_N_LOW",
        default_value = "6"
    )]
    pub libp2p_mesh_n_low: usize,

    /// Minimum number of outbound Libp2p peers in the mesh network before adding more
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MESH_OUTBOUND_MIN",
        default_value = "2"
    )]
    pub libp2p_mesh_outbound_min: usize,

    /// The maximum number of messages to include in a Libp2p IHAVE message
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_IHAVE_LENGTH",
        default_value = "5000"
    )]
    pub libp2p_max_ihave_length: usize,

    /// The maximum number of IHAVE messages to accept from a Libp2p peer within a heartbeat
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_IHAVE_MESSAGES",
        default_value = "10"
    )]
    pub libp2p_max_ihave_messages: usize,

    /// Libp2p published message ids time cache duration
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_PUBLISHED_MESSAGE_IDS_CACHE_TIME", default_value = "10s", value_parser = parse_duration)]
    pub libp2p_published_message_ids_cache_time: Duration,

    /// Time to wait for a Libp2p message requested through IWANT following an IHAVE advertisement
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_IWANT_FOLLOWUP_TIME",
        default_value = "3s", value_parser = parse_duration
    )]
    pub libp2p_iwant_followup_time: Duration,

    /// The maximum number of Libp2p messages we will process in a given RPC
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_MESSAGES_PER_RPC")]
    pub libp2p_max_messages_per_rpc: Option<usize>,

    /// How many times we will allow a Libp2p peer to request the same message id through IWANT gossip before we start ignoring them
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_RETRANSMISSION",
        default_value = "3"
    )]
    pub libp2p_gossip_retransmission: u32,

    /// If enabled newly created messages will always be sent to all peers that are subscribed to the topic and have a good enough score
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_FLOOD_PUBLISH",
        default_value = "true"
    )]
    pub libp2p_flood_publish: bool,

    /// The time period that Libp2p message hashes are stored in the cache
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_DUPLICATE_CACHE_TIME", default_value = "20m", value_parser = parse_duration)]
    pub libp2p_duplicate_cache_time: Duration,

    /// Time to live for Libp2p fanout peers
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_FANOUT_TTL", default_value = "60s", value_parser = parse_duration)]
    pub libp2p_fanout_ttl: Duration,

    /// Initial delay in each Libp2p heartbeat
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_HEARTBEAT_INITIAL_DELAY", default_value = "5s", value_parser = parse_duration)]
    pub libp2p_heartbeat_initial_delay: Duration,

    /// How many Libp2p peers we will emit gossip to at each heartbeat
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_FACTOR",
        default_value = "0.25"
    )]
    pub libp2p_gossip_factor: f64,

    /// Minimum number of Libp2p peers to emit gossip to during a heartbeat
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_GOSSIP_LAZY",
        default_value = "6"
    )]
    pub libp2p_gossip_lazy: usize,

    /// The maximum number of bytes we will send in a single Libp2p gossip message
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_GOSSIP_TRANSMIT_SIZE",
        default_value = "2000000"
    )]
    pub libp2p_max_gossip_transmit_size: usize,

    /// The maximum number of bytes we will send in a single Libp2p direct message
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_MAX_DIRECT_TRANSMIT_SIZE",
        default_value = "20000000"
    )]
    pub libp2p_max_direct_transmit_size: u64,

    /// The URL we advertise to other nodes as being for our public API.
    /// Should be supplied in `http://host:port` form.
    #[clap(long, env = "ESPRESSO_SEQUENCER_PUBLIC_API_URL")]
    pub public_api_url: Option<Url>,

    /// The address we advertise to other nodes as being a Libp2p endpoint.
    /// Should be supplied in `host:port` form.
    ///
    /// Operators should set this to a publicly routable address whenever the bind address
    /// is not directly reachable from peers (NAT, K8s NodePort, Docker bridge). It is added
    /// to libp2p as an `external_address` so that Identify and Kademlia announce it to the
    /// network. Non-globally-routable IP literals (loopback, RFC 1918 private, link-local,
    /// etc.) only work for local testing (`demo-native`, `docker-compose`) and are dropped
    /// from the libp2p announcement; hostnames are passed through unchanged.
    ///
    /// Also required when bootstrapping a fresh network from the orchestrator, where it is
    /// registered into the stake table so peers can dial us.
    #[clap(long, env = "ESPRESSO_SEQUENCER_LIBP2P_ADVERTISE_ADDRESS")]
    pub libp2p_advertise_address: Option<String>,

    /// A comma-separated list of Libp2p multiaddresses to use as bootstrap
    /// nodes.
    ///
    /// Overrides those loaded from the `HotShot` config.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_BOOTSTRAP_NODES",
        value_delimiter = ',',
        num_args = 1..
    )]
    pub libp2p_bootstrap_nodes: Option<Vec<Multiaddr>>,

    /// The URL of the builders to use for submitting transactions
    #[clap(long, env = "ESPRESSO_SEQUENCER_BUILDER_URLS", value_delimiter = ',')]
    pub builder_urls: Vec<Url>,

    /// URL of the Light Client State Relay Server
    #[clap(
        long,
        env = "ESPRESSO_STATE_RELAY_SERVER_URL",
        default_value = "http://localhost:8083"
    )]
    #[derivative(Debug(format_with = "Display::fmt"))]
    pub state_relay_server_url: Url,

    /// Path to TOML file containing genesis state.
    #[clap(
        long,
        name = "GENESIS_FILE",
        env = "ESPRESSO_SEQUENCER_GENESIS_FILE",
        default_value = "/genesis/demo.toml"
    )]
    pub genesis_file: PathBuf,

    #[clap(flatten)]
    pub key_set: KeySetOptions,

    /// Add optional modules to the service.
    ///
    /// Modules are added by specifying the name of the module followed by it's arguments, as in
    ///
    /// sequencer [options] -- api --port 3000
    ///
    /// to run the API module with port 3000.
    ///
    /// To see a list of available modules and their arguments, use
    ///
    /// sequencer -- help
    ///
    /// Multiple modules can be specified, provided they are separated by --
    #[clap(raw = true)]
    modules: Vec<String>,

    /// Url we will use for RPC communication with L1.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_L1_PROVIDER",
        default_value = "http://localhost:8545",
        value_delimiter = ',',
        num_args = 1..,
    )]
    #[derivative(Debug = "ignore")]
    pub l1_provider_url: Vec<Url>,

    /// Configuration for the L1 client.
    #[clap(flatten)]
    pub l1_options: L1ClientOptions,

    /// Whether or not we are a DA node.
    #[clap(long, env = "ESPRESSO_SEQUENCER_IS_DA", action)]
    pub is_da: bool,

    /// Peer nodes use to fetch missing state
    #[clap(long, env = "ESPRESSO_SEQUENCER_STATE_PEERS", value_delimiter = ',')]
    #[derivative(Debug(format_with = "fmt_urls"))]
    pub state_peers: Vec<Url>,

    /// Peer nodes use to fetch missing config
    ///
    /// Typically, the network-wide config is fetched from the orchestrator on startup and then
    /// persisted and loaded from local storage each time the node restarts. However, if the
    /// persisted config is missing when the node restarts (for example, the node is being migrated
    /// to new persistent storage), it can instead be fetched directly from a peer.
    #[clap(long, env = "ESPRESSO_SEQUENCER_CONFIG_PEERS", value_delimiter = ',')]
    #[derivative(Debug(format_with = "fmt_opt_urls"))]
    pub config_peers: Option<Vec<Url>>,

    /// Exponential backoff for fetching missing state from peers.
    #[clap(flatten)]
    pub catchup_backoff: BackoffParams,

    /// Base timeout for catchup requests to peers.
    ///
    /// This is the initial per peer timeout for HTTP requests during state catchup
    #[clap(long, env = "ESPRESSO_SEQUENCER_CATCHUP_BASE_TIMEOUT", default_value = "2s", value_parser = parse_duration)]
    pub catchup_base_timeout: Duration,

    /// Timeout for local catchup provider requests.
    ///
    /// If a local provider (e.g. database) takes longer than this, the node falls back to
    /// remote providers so it can still vote within the current view.
    #[clap(long, env = "ESPRESSO_SEQUENCER_LOCAL_CATCHUP_TIMEOUT", default_value = "5s", value_parser = parse_duration)]
    pub local_catchup_timeout: Duration,

    #[clap(flatten)]
    pub logging: logging::Config,

    #[clap(flatten)]
    pub identity: Identity,

    #[clap(flatten)]
    pub proposal_fetcher_config: ProposalFetcherConfig,
}

impl Options {
    pub fn modules(&self) -> Modules {
        ModuleArgs(self.modules.clone()).parse()
    }
}

/// Identity represents identifying information concerning the sequencer node.
/// This information is used to populate relevant information in the metrics
/// endpoint.  This information will also potentially be scraped and displayed
/// in a public facing dashboard.
#[derive(Parser, Clone, Derivative, Serialize)]
#[derivative(Debug(bound = ""))]
pub struct Identity {
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_COUNTRY_CODE")]
    pub country_code: Option<String>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_LATITUDE")]
    pub latitude: Option<f64>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_LONGITUDE")]
    pub longitude: Option<f64>,

    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_NODE_NAME")]
    pub node_name: Option<String>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_NODE_DESCRIPTION")]
    pub node_description: Option<String>,

    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_COMPANY_NAME")]
    pub company_name: Option<String>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_COMPANY_WEBSITE")]
    pub company_website: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_OPERATING_SYSTEM", default_value = std::env::consts::OS)]
    pub operating_system: Option<String>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_NODE_TYPE", default_value = get_default_node_type())]
    pub node_type: Option<String>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_NETWORK_TYPE")]
    pub network_type: Option<String>,

    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_1x")]
    pub icon_14x14_1x: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_2x")]
    pub icon_14x14_2x: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_14x14_3x")]
    pub icon_14x14_3x: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_1x")]
    pub icon_24x24_1x: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_2x")]
    pub icon_24x24_2x: Option<Url>,
    #[clap(long, env = "ESPRESSO_SEQUENCER_IDENTITY_ICON_24x24_3x")]
    pub icon_24x24_3x: Option<Url>,
}

/// get_default_node_type returns the current public facing binary name and
/// version of this program.
fn get_default_node_type() -> String {
    format!("espresso-sequencer {}", env!("CARGO_PKG_VERSION"))
}

fn build_version() -> String {
    let info = espresso_utils::build_info!();
    format!(
        "{}\nfeatures: {}",
        info.clap_version(),
        env!("VERGEN_CARGO_FEATURES"),
    )
}

// The Debug implementation for Url is noisy, we just want to see the URL
fn fmt_urls(v: &[Url], fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
    write!(
        fmt,
        "{:?}",
        v.iter().map(|i| i.to_string()).collect::<Vec<_>>()
    )
}

fn fmt_opt_urls(
    v: &Option<Vec<Url>>,
    fmt: &mut std::fmt::Formatter,
) -> Result<(), std::fmt::Error> {
    match v {
        Some(urls) => {
            write!(fmt, "Some(")?;
            fmt_urls(urls, fmt)?;
            write!(fmt, ")")?;
        },
        None => {
            write!(fmt, "None")?;
        },
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    pub numerator: u64,
    pub denominator: u64,
}

impl From<Ratio> for (u64, u64) {
    fn from(r: Ratio) -> Self {
        (r.numerator, r.denominator)
    }
}

impl Display for Ratio {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.numerator, self.denominator)
    }
}

impl PartialOrd for Ratio {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Ratio {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.numerator * other.denominator).cmp(&(other.numerator * self.denominator))
    }
}

#[derive(Clone, Debug)]
struct ModuleArgs(Vec<String>);

impl ModuleArgs {
    fn parse(&self) -> Modules {
        match self.try_parse() {
            Ok(modules) => modules,
            Err(err) => err.exit(),
        }
    }

    fn try_parse(&self) -> Result<Modules, clap::Error> {
        let mut modules = Modules::default();
        let mut curr = self.0.clone();
        let mut provided = Default::default();

        while !curr.is_empty() {
            // The first argument (the program name) is used only for help generation. We include a
            // `--` so that the generated usage will look like `sequencer -- <command>` which is the
            // way these commands must be invoked due to the use of `raw` arguments.
            let module = SequencerModule::try_parse_from(
                once("sequencer --").chain(curr.iter().map(|s| s.as_str())),
            )?;
            match module {
                SequencerModule::Storage(m) => {
                    curr = m.add(&mut modules.storage_fs, &mut provided)?
                },
                SequencerModule::StorageFs(m) => {
                    curr = m.add(&mut modules.storage_fs, &mut provided)?
                },
                SequencerModule::StorageSql(m) => {
                    curr = m.add(&mut modules.storage_sql, &mut provided)?
                },
                SequencerModule::Http(m) => curr = m.add(&mut modules.http, &mut provided)?,
                SequencerModule::Query(m) => curr = m.add(&mut modules.query, &mut provided)?,
                SequencerModule::Submit(m) => curr = m.add(&mut modules.submit, &mut provided)?,
                SequencerModule::Status(m) => curr = m.add(&mut modules.status, &mut provided)?,
                SequencerModule::Catchup(m) => curr = m.add(&mut modules.catchup, &mut provided)?,
                SequencerModule::Config(m) => curr = m.add(&mut modules.config, &mut provided)?,
                SequencerModule::HotshotEvents(m) => {
                    curr = m.add(&mut modules.hotshot_events, &mut provided)?
                },
                SequencerModule::Explorer(m) => {
                    curr = m.add(&mut modules.explorer, &mut provided)?
                },
                SequencerModule::LightClient(m) => {
                    curr = m.add(&mut modules.light_client, &mut provided)?
                },
            }
        }

        Ok(modules)
    }
}

trait ModuleInfo: Args + FromArgMatches {
    const NAME: &'static str;
    fn requires() -> Vec<&'static str>;
}

macro_rules! module {
    ($name:expr, $opt:ty $(,requires: $($req:expr),*)?) => {
        impl ModuleInfo for $opt {
            const NAME: &'static str = $name;

            fn requires() -> Vec<&'static str> {
                vec![$($($req),*)?]
            }
        }
    };
}

module!("storage-fs", persistence::fs::Options);
module!("storage-sql", persistence::sql::Options);
module!("http", api::options::Http);
module!("query", api::options::Query, requires: "http");
module!("submit", api::options::Submit, requires: "http");
module!("status", api::options::Status, requires: "http");
module!("catchup", api::options::Catchup, requires: "http");
module!("config", api::options::Config, requires: "http");
module!("hotshot-events", api::options::HotshotEvents, requires: "http");
module!("explorer", api::options::Explorer, requires: "http", "storage-sql");
module!("light-client", api::options::LightClient, requires: "http", "storage-sql");

#[derive(Clone, Debug, Args)]
struct Module<Options: ModuleInfo> {
    #[clap(flatten)]
    options: Box<Options>,

    /// Add more optional modules.
    #[clap(raw = true)]
    modules: Vec<String>,
}

impl<Options: ModuleInfo> Module<Options> {
    /// Add this as an optional module. Return the next optional module args.
    fn add(
        self,
        options: &mut Option<Options>,
        provided: &mut HashSet<&'static str>,
    ) -> Result<Vec<String>, clap::Error> {
        if options.is_some() {
            return Err(clap::Error::raw(
                ErrorKind::TooManyValues,
                format!("optional module {} can only be started once", Options::NAME),
            ));
        }
        for req in Options::requires() {
            if !provided.contains(&req) {
                return Err(clap::Error::raw(
                    ErrorKind::MissingRequiredArgument,
                    format!("module {} is missing required module {req}", Options::NAME),
                ));
            }
        }
        *options = Some(*self.options);
        provided.insert(Options::NAME);
        Ok(self.modules)
    }
}

#[derive(Clone, Debug, Parser)]
enum SequencerModule {
    /// Run an HTTP server.
    ///
    /// The basic HTTP server comes with healthcheck and version endpoints. Add additional endpoints
    /// by enabling additional modules:
    /// * query: add query service endpoints
    /// * submit: add transaction submission endpoints
    Http(Module<api::options::Http>),
    /// Alias for storage-fs.
    Storage(Module<persistence::fs::Options>),
    /// Use the file system for persistent storage.
    StorageFs(Module<persistence::fs::Options>),
    /// Use a Postgres database for persistent storage.
    StorageSql(Module<persistence::sql::Options>),
    /// Run the query API module.
    ///
    /// This module requires the http module to be started.
    Query(Module<api::options::Query>),
    /// Run the transaction submission API module.
    ///
    /// This module requires the http module to be started.
    Submit(Module<api::options::Submit>),
    /// Run the status API module.
    ///
    /// This module requires the http module to be started.
    Status(Module<api::options::Status>),
    /// Run the state catchup API module.
    ///
    /// This module requires the http module to be started.
    Catchup(Module<api::options::Catchup>),
    /// Run the config API module.
    Config(Module<api::options::Config>),

    /// Run the hotshot events API module.
    ///
    /// This module requires the http module to be started.
    HotshotEvents(Module<api::options::HotshotEvents>),
    /// Run the explorer API module.
    ///
    /// This module requires the http and storage-sql modules to be started.
    Explorer(Module<api::options::Explorer>),
    /// Run the light client API module.
    ///
    /// This module provides data and proofs necessary for an untrusting light client to retrieve
    /// and verify Espresso data from this server.
    ///
    /// This module requires the http and storage-sql modules to be started.
    LightClient(Module<api::options::LightClient>),
}

#[derive(Clone, Debug, Default)]
pub struct Modules {
    pub storage_fs: Option<persistence::fs::Options>,
    pub storage_sql: Option<persistence::sql::Options>,
    pub http: Option<api::options::Http>,
    pub query: Option<api::options::Query>,
    pub submit: Option<api::options::Submit>,
    pub status: Option<api::options::Status>,
    pub catchup: Option<api::options::Catchup>,
    pub config: Option<api::options::Config>,
    pub hotshot_events: Option<api::options::HotshotEvents>,
    pub explorer: Option<api::options::Explorer>,
    pub light_client: Option<api::options::LightClient>,
}

#[derive(Clone, Debug, Serialize)]
pub struct PublicNodeConfig {
    pub orchestrator_url: Url,
    pub cdn_endpoint: String,
    pub cliquenet_bind_address: NetAddr,
    pub cliquenet_advertise_address: Option<NetAddr>,
    pub libp2p_bind_address: String,
    pub libp2p_advertise_address: Option<String>,
    pub libp2p_bootstrap_nodes: Option<Vec<Multiaddr>>,
    pub public_api_url: Option<Url>,
    pub builder_urls: Vec<Url>,
    pub state_relay_server_url: Url,
    pub state_peers: Vec<Url>,
    pub config_peers: Option<Vec<Url>>,
    pub is_da: bool,
    pub genesis_file: PathBuf,
    pub identity: Identity,
    pub catchup_base_timeout: Duration,
    pub local_catchup_timeout: Duration,
    pub bootstrap_epoch_catchup_timeout: Duration,
    pub catchup_backoff: BackoffParams,
    pub proposal_fetcher: ProposalFetcherConfig,
    pub libp2p: Libp2pTuning,
    pub l1: L1Tuning,
    pub l1_provider_count: usize,
    pub l1_ws_provider_count: usize,
    pub storage: StorageConfig,
    pub modules: ApiModulesConfig,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum StorageBackend {
    Sql,
    Fs,
    FsDefault,
}

#[derive(Clone, Debug, Serialize)]
pub struct StorageConfig {
    /// Active backend.
    pub backend: StorageBackend,
    pub fs: Option<FsStorageConfig>,
    pub sql: Option<SqlStorageConfig>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FsStorageConfig {
    pub path: PathBuf,
    pub consensus_view_retention: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct SqlStorageConfig {
    pub prune: bool,
    pub archive: bool,
    pub lightweight: bool,
    pub disable_proactive_fetching: bool,
    pub fetch_rate_limit: Option<usize>,
    pub active_fetch_delay: Option<Duration>,
    pub chunk_fetch_delay: Option<Duration>,
    pub sync_status_chunk_size: Option<usize>,
    pub sync_status_ttl: Option<Duration>,
    pub proactive_scan_chunk_size: Option<usize>,
    pub proactive_scan_interval: Option<Duration>,
    pub idle_connection_timeout: Duration,
    pub connection_timeout: Duration,
    pub slow_statement_threshold: Duration,
    pub statement_timeout: Duration,
    pub min_connections: u32,
    pub max_connections: u32,
    pub query_min_connections: Option<u32>,
    pub query_max_connections: Option<u32>,
    pub pruning: PruningView,
    pub consensus_pruning: ConsensusPruningView,
}

#[derive(Clone, Debug, Serialize)]
pub struct PruningView {
    pub pruning_threshold: Option<u64>,
    pub minimum_retention: Option<Duration>,
    pub target_retention: Option<Duration>,
    pub batch_size: Option<u64>,
    pub max_usage: Option<u16>,
    pub interval: Option<Duration>,
    pub pages: Option<u64>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ConsensusPruningView {
    pub target_retention: u64,
    pub minimum_retention: u64,
    pub target_usage: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ApiModulesConfig {
    pub http: Option<HttpConfig>,
    pub query: Option<QueryConfig>,
    pub submit: bool,
    pub status: bool,
    pub catchup: bool,
    pub config: bool,
    pub hotshot_events: bool,
    pub explorer: bool,
    pub light_client: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct HttpConfig {
    pub port: u16,
    pub max_connections: Option<usize>,
    pub axum_port: Option<u16>,
    pub tonic_port: Option<u16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryConfig {
    pub peers: Vec<Url>,
    pub light_client: LightClientOptions,
    pub light_client_db: LightClientSqliteOptions,
}

impl From<&persistence::sql::PruningOptions> for PruningView {
    fn from(o: &persistence::sql::PruningOptions) -> Self {
        Self {
            pruning_threshold: o.pruning_threshold,
            minimum_retention: o.minimum_retention,
            target_retention: o.target_retention,
            batch_size: o.batch_size,
            max_usage: o.max_usage,
            interval: o.interval,
            pages: o.pages,
        }
    }
}

impl From<&persistence::sql::ConsensusPruningOptions> for ConsensusPruningView {
    fn from(o: &persistence::sql::ConsensusPruningOptions) -> Self {
        Self {
            target_retention: o.target_retention,
            minimum_retention: o.minimum_retention,
            target_usage: o.target_usage,
        }
    }
}

impl From<&persistence::sql::Options> for SqlStorageConfig {
    fn from(o: &persistence::sql::Options) -> Self {
        Self {
            prune: o.prune,
            archive: o.archive,
            lightweight: o.lightweight,
            disable_proactive_fetching: o.disable_proactive_fetching,
            fetch_rate_limit: o.fetch_rate_limit,
            active_fetch_delay: o.active_fetch_delay,
            chunk_fetch_delay: o.chunk_fetch_delay,
            sync_status_chunk_size: o.sync_status_chunk_size,
            sync_status_ttl: o.sync_status_ttl,
            proactive_scan_chunk_size: o.proactive_scan_chunk_size,
            proactive_scan_interval: o.proactive_scan_interval,
            idle_connection_timeout: o.idle_connection_timeout,
            connection_timeout: o.connection_timeout,
            slow_statement_threshold: o.slow_statement_threshold,
            statement_timeout: o.statement_timeout,
            min_connections: o.min_connections,
            max_connections: o.max_connections,
            #[cfg(not(feature = "embedded-db"))]
            query_min_connections: o.query_min_connections,
            #[cfg(feature = "embedded-db")]
            query_min_connections: None,
            #[cfg(not(feature = "embedded-db"))]
            query_max_connections: o.query_max_connections,
            #[cfg(feature = "embedded-db")]
            query_max_connections: None,
            pruning: PruningView::from(&o.pruning),
            consensus_pruning: ConsensusPruningView::from(&o.consensus_pruning),
        }
    }
}

impl From<&persistence::fs::Options> for FsStorageConfig {
    fn from(o: &persistence::fs::Options) -> Self {
        Self {
            path: o.path.clone(),
            consensus_view_retention: o.consensus_view_retention,
        }
    }
}

impl From<&api::options::Http> for HttpConfig {
    fn from(o: &api::options::Http) -> Self {
        Self {
            port: o.port,
            max_connections: o.max_connections,
            axum_port: o.axum_port,
            tonic_port: o.tonic_port,
        }
    }
}

impl From<&api::options::Query> for QueryConfig {
    fn from(o: &api::options::Query) -> Self {
        Self {
            peers: o.peers.clone(),
            light_client: o.light_client.clone(),
            light_client_db: o.light_client_db.clone(),
        }
    }
}

impl From<&Modules> for ApiModulesConfig {
    fn from(m: &Modules) -> Self {
        Self {
            http: m.http.as_ref().map(HttpConfig::from),
            query: m.query.as_ref().map(QueryConfig::from),
            submit: m.submit.is_some(),
            status: m.status.is_some(),
            catchup: m.catchup.is_some(),
            config: m.config.is_some(),
            hotshot_events: m.hotshot_events.is_some(),
            explorer: m.explorer.is_some(),
            light_client: m.light_client.is_some(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Libp2pTuning {
    pub heartbeat_interval: Duration,
    pub heartbeat_initial_delay: Duration,
    pub history_gossip: usize,
    pub history_length: usize,
    pub mesh_n: usize,
    pub mesh_n_high: usize,
    pub mesh_n_low: usize,
    pub mesh_outbound_min: usize,
    pub max_ihave_length: usize,
    pub max_ihave_messages: usize,
    pub published_message_ids_cache_time: Duration,
    pub iwant_followup_time: Duration,
    pub max_messages_per_rpc: Option<usize>,
    pub gossip_retransmission: u32,
    pub gossip_factor: f64,
    pub gossip_lazy: usize,
    pub max_gossip_transmit_size: usize,
    pub max_direct_transmit_size: u64,
    pub fanout_ttl: Duration,
    pub duplicate_cache_time: Duration,
    pub flood_publish: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct L1Tuning {
    pub retry_delay: Duration,
    pub polling_interval: Duration,
    pub blocks_cache_size: usize,
    pub events_channel_capacity: usize,
    pub events_max_block_range: u64,
    pub subscription_timeout: Duration,
    pub frequent_failure_tolerance: Duration,
    pub consecutive_failure_tolerance: usize,
    pub failover_revert: Duration,
    pub rate_limit_delay: Option<Duration>,
    pub stake_table_update_interval: Duration,
    pub events_max_retry_duration: Duration,
    pub finalized_safety_margin: Option<u64>,
}

impl From<&Options> for Libp2pTuning {
    fn from(o: &Options) -> Self {
        Self {
            heartbeat_interval: o.libp2p_heartbeat_interval,
            heartbeat_initial_delay: o.libp2p_heartbeat_initial_delay,
            history_gossip: o.libp2p_history_gossip,
            history_length: o.libp2p_history_length,
            mesh_n: o.libp2p_mesh_n,
            mesh_n_high: o.libp2p_mesh_n_high,
            mesh_n_low: o.libp2p_mesh_n_low,
            mesh_outbound_min: o.libp2p_mesh_outbound_min,
            max_ihave_length: o.libp2p_max_ihave_length,
            max_ihave_messages: o.libp2p_max_ihave_messages,
            published_message_ids_cache_time: o.libp2p_published_message_ids_cache_time,
            iwant_followup_time: o.libp2p_iwant_followup_time,
            max_messages_per_rpc: o.libp2p_max_messages_per_rpc,
            gossip_retransmission: o.libp2p_gossip_retransmission,
            gossip_factor: o.libp2p_gossip_factor,
            gossip_lazy: o.libp2p_gossip_lazy,
            max_gossip_transmit_size: o.libp2p_max_gossip_transmit_size,
            max_direct_transmit_size: o.libp2p_max_direct_transmit_size,
            fanout_ttl: o.libp2p_fanout_ttl,
            duplicate_cache_time: o.libp2p_duplicate_cache_time,
            flood_publish: o.libp2p_flood_publish,
        }
    }
}

impl From<&L1ClientOptions> for L1Tuning {
    fn from(o: &L1ClientOptions) -> Self {
        Self {
            retry_delay: o.l1_retry_delay,
            polling_interval: o.l1_polling_interval,
            blocks_cache_size: o.l1_blocks_cache_size.get(),
            events_channel_capacity: o.l1_events_channel_capacity,
            events_max_block_range: o.l1_events_max_block_range,
            subscription_timeout: o.subscription_timeout,
            frequent_failure_tolerance: o.l1_frequent_failure_tolerance,
            consecutive_failure_tolerance: o.l1_consecutive_failure_tolerance,
            failover_revert: o.l1_failover_revert,
            rate_limit_delay: o.l1_rate_limit_delay,
            stake_table_update_interval: o.stake_table_update_interval,
            events_max_retry_duration: o.l1_events_max_retry_duration,
            finalized_safety_margin: o.l1_finalized_safety_margin,
        }
    }
}

impl PublicNodeConfig {
    pub fn new(opt: &Options, modules: &Modules) -> Self {
        let storage = if let Some(sql) = modules.storage_sql.as_ref() {
            StorageConfig {
                backend: StorageBackend::Sql,
                fs: None,
                sql: Some(SqlStorageConfig::from(sql)),
            }
        } else if let Some(fs) = modules.storage_fs.as_ref() {
            StorageConfig {
                backend: StorageBackend::Fs,
                fs: Some(FsStorageConfig::from(fs)),
                sql: None,
            }
        } else {
            let fs = persistence::fs::Options::try_parse_from(std::iter::empty::<String>()).ok();
            StorageConfig {
                backend: StorageBackend::FsDefault,
                fs: fs.as_ref().map(FsStorageConfig::from),
                sql: None,
            }
        };

        Self {
            orchestrator_url: opt.orchestrator_url.clone(),
            cdn_endpoint: opt.cdn_endpoint.clone(),
            cliquenet_bind_address: opt.cliquenet_bind_address.clone(),
            cliquenet_advertise_address: opt.cliquenet_advertise_address.clone(),
            libp2p_bind_address: opt.libp2p_bind_address.clone(),
            libp2p_advertise_address: opt.libp2p_advertise_address.clone(),
            libp2p_bootstrap_nodes: opt.libp2p_bootstrap_nodes.clone(),
            public_api_url: opt.public_api_url.clone(),
            builder_urls: opt.builder_urls.clone(),
            state_relay_server_url: opt.state_relay_server_url.clone(),
            state_peers: opt.state_peers.clone(),
            config_peers: opt.config_peers.clone(),
            is_da: opt.is_da,
            genesis_file: opt.genesis_file.clone(),
            identity: opt.identity.clone(),
            catchup_base_timeout: opt.catchup_base_timeout,
            local_catchup_timeout: opt.local_catchup_timeout,
            bootstrap_epoch_catchup_timeout: opt.bootstrap_epoch_catchup_timeout,
            catchup_backoff: opt.catchup_backoff,
            proposal_fetcher: opt.proposal_fetcher_config,
            libp2p: Libp2pTuning::from(opt),
            l1: L1Tuning::from(&opt.l1_options),
            l1_provider_count: opt.l1_provider_url.len(),
            l1_ws_provider_count: opt
                .l1_options
                .l1_ws_provider
                .as_ref()
                .map(|v| v.len())
                .unwrap_or(0),
            storage,
            modules: ApiModulesConfig::from(modules),
        }
    }
}

#[cfg(test)]
mod tests {
    use espresso_types::PubKey;
    use hotshot_types::{light_client::StateKeyPair, traits::signature_key::SignatureKey, x25519};
    use tagged_base64::TaggedBase64;

    use super::*;

    #[test]
    fn test_build_version() {
        let version = build_version();
        for field in [
            "describe:",
            "rev:",
            "modified:",
            "branch:",
            "commit-timestamp:",
            "debug:",
            "os:",
            "arch:",
            "features:",
        ] {
            assert!(version.contains(field), "missing {field}: {version}");
        }
        assert!(
            version.contains("debug: true"),
            "expected debug build in test: {version}"
        );
        assert!(
            version.contains("testing"),
            "expected testing in features: {version}"
        );
    }

    /// Build a minimal `Options` for tests, using freshly generated keys and the supplied extra args.
    pub(super) fn parse_options_with(extra: &[&str]) -> Options {
        let (_, priv_key) = PubKey::generated_from_seed_indexed([0; 32], 0);
        let state_key = StateKeyPair::generate_from_seed_indexed([0; 32], 0);
        let x25519_kp = x25519::Keypair::generate().unwrap();

        let priv_staking = priv_key.to_tagged_base64().expect("valid key").to_string();
        let priv_state = state_key
            .sign_key_ref()
            .to_tagged_base64()
            .expect("valid key")
            .to_string();
        let priv_x25519 = TaggedBase64::try_from(x25519_kp.secret_key())
            .expect("valid key")
            .to_string();

        let mut args: Vec<String> = vec![
            "sequencer".into(),
            "--private-staking-key".into(),
            priv_staking,
            "--private-state-key".into(),
            priv_state,
            "--private-x25519-key".into(),
            priv_x25519,
        ];
        args.extend(extra.iter().map(|s| s.to_string()));

        Options::parse_from(args)
    }

    #[test]
    fn public_node_config_no_secrets() {
        let opt = parse_options_with(&[
            "--l1-provider-url",
            "https://user:pass@example.invalid/v2/SECRET_API_KEY,https://example2.invalid/key2",
            "--cliquenet-bind-address",
            "127.0.0.1:9999",
            "--state-peers",
            "https://peer1.test,https://peer2.test",
        ]);
        let modules = opt.modules();

        let cfg = PublicNodeConfig::new(&opt, &modules);
        let json = serde_json::to_string(&cfg).unwrap();
        let json_lc = json.to_lowercase();

        assert!(
            json.contains("127.0.0.1:9999"),
            "CLI override missing from JSON: {json}"
        );
        assert!(
            !json.contains("SECRET_API_KEY"),
            "L1 API key leaked into JSON: {json}"
        );
        assert!(
            !json.contains("user:pass"),
            "L1 URL userinfo leaked into JSON: {json}"
        );
        assert!(
            !json.contains("example.invalid"),
            "L1 host leaked into JSON: {json}"
        );
        assert!(
            !json.contains("example2.invalid"),
            "second L1 host leaked into JSON: {json}"
        );
        assert!(
            json.contains("\"l1_provider_count\":2"),
            "missing l1_provider_count: {json}"
        );
        assert!(
            !json.contains("\"uri\""),
            "DB URI key leaked into JSON: {json}"
        );
        assert!(
            !json.contains("postgres://"),
            "DB connection string leaked into JSON: {json}"
        );

        const FORBIDDEN: &[&str] = &[
            "private", "mnemonic", "secret", "x25519", "key_file", "seed", "password",
        ];
        for token in FORBIDDEN {
            assert!(
                !json_lc.contains(token),
                "forbidden token '{token}' leaked into JSON: {json}"
            );
        }

        assert!(
            json.contains("peer1.test") && json.contains("peer2.test"),
            "state_peers missing from JSON: {json}"
        );
    }

    #[test]
    fn public_node_config_optionals() {
        let opt = parse_options_with(&[]);
        let modules = opt.modules();

        let cfg = PublicNodeConfig::new(&opt, &modules);

        assert!(
            cfg.cliquenet_advertise_address.is_none(),
            "expected no advertise address: {:?}",
            cfg.cliquenet_advertise_address
        );
        assert!(
            cfg.libp2p_bootstrap_nodes.is_none(),
            "expected no bootstrap nodes: {:?}",
            cfg.libp2p_bootstrap_nodes
        );
        assert!(
            cfg.config_peers.is_none(),
            "expected no config peers: {:?}",
            cfg.config_peers
        );
        assert_eq!(cfg.l1_ws_provider_count, 0);
        assert_eq!(cfg.storage.backend, StorageBackend::FsDefault);
        assert!(cfg.storage.fs.is_none());
        assert!(cfg.storage.sql.is_none());
        assert!(!cfg.modules.submit);
        assert!(cfg.modules.http.is_none());
        assert!(cfg.modules.query.is_none());

        let value: serde_json::Value = serde_json::to_value(&cfg).unwrap();
        assert_eq!(
            value["cliquenet_advertise_address"],
            serde_json::Value::Null
        );
        assert_eq!(value["libp2p_bootstrap_nodes"], serde_json::Value::Null);
        assert_eq!(value["config_peers"], serde_json::Value::Null);
        assert_eq!(value["public_api_url"], serde_json::Value::Null);
    }

    // Document the JSON shape of GET /config/runtime. Runs under Postgres builds only;
    // the embedded-db variant produces a near-identical shape and the duplication isn't
    // worth the test complexity.
    #[cfg(not(feature = "embedded-db"))]
    #[test]
    fn config_node_response_snapshot() {
        let opt = parse_options_with(&[
            "--orchestrator-url",
            "http://orchestrator.example:8080",
            "--cdn-endpoint",
            "cdn.example:8081",
            "--cliquenet-bind-address",
            "0.0.0.0:9977",
            "--cliquenet-advertise-address",
            "node1.example:9977",
            "--libp2p-bind-address",
            "0.0.0.0:1769",
            "--libp2p-advertise-address",
            "node1.example:1769",
            "--libp2p-bootstrap-nodes",
            "/ip4/10.0.0.1/tcp/1769",
            "--public-api-url",
            "http://node1.example:24000",
            "--builder-urls",
            "http://builder.example:31004",
            "--state-relay-server-url",
            "http://relay.example:8083",
            "--state-peers",
            "https://peer1.example,https://peer2.example",
            "--config-peers",
            "https://peer1.example",
            "--is-da",
            "--genesis-file",
            "/path/to/genesis.toml",
            "--l1-provider-url",
            "https://eth.example",
            "--country-code",
            "US",
            "--node-name",
            "Snapshot Node",
            // Pin host-dependent identity defaults so the snapshot is portable.
            "--operating-system",
            "linux",
            "--node-type",
            "espresso-sequencer 0.0.0",
            "--",
            "storage-sql",
            "--prune",
            "--pruning-threshold",
            "1000000000000",
            "--",
            "http",
            "--port",
            "24000",
            "--",
            "query",
            "--",
            "config",
        ]);
        let modules = opt.modules();

        let cfg = PublicNodeConfig::new(&opt, &modules);

        insta::assert_yaml_snapshot!("config_node_response_postgres", cfg);
    }

    // Postgres only: storage-sql under embedded-db requires a --path arg that's
    // irrelevant to what this test asserts.
    #[cfg(not(feature = "embedded-db"))]
    #[test]
    fn public_node_config_includes_pruning() {
        let opt = parse_options_with(&[
            "--cliquenet-bind-address",
            "127.0.0.1:1",
            "--",
            "storage-sql",
            "--prune",
            "--pruning-threshold",
            "1000000000000",
        ]);
        let modules = opt.modules();

        let cfg = PublicNodeConfig::new(&opt, &modules);
        let json = serde_json::to_string(&cfg).unwrap();

        assert_eq!(cfg.storage.backend, StorageBackend::Sql);
        assert!(
            json.contains("\"prune\":true"),
            "expected prune:true in JSON: {json}"
        );
        assert!(
            json.contains("pruning_threshold"),
            "expected pruning_threshold in JSON: {json}"
        );
        assert!(
            json.contains("1000000000000"),
            "expected pruning threshold value in JSON: {json}"
        );
        assert!(
            json.contains("\"consensus_pruning\""),
            "expected consensus_pruning object in JSON: {json}"
        );
        assert!(
            json.contains("\"pruning\""),
            "expected pruning object in JSON: {json}"
        );
        assert!(
            json.contains("\"target_retention\":302000"),
            "expected consensus_pruning target_retention default in JSON: {json}"
        );
        assert!(
            json.contains("\"minimum_retention\":130000"),
            "expected consensus_pruning minimum_retention default in JSON: {json}"
        );
        assert!(
            json.contains("\"target_usage\":1000000000"),
            "expected consensus_pruning target_usage default in JSON: {json}"
        );
        assert!(
            !json.contains("\"uri\""),
            "DB URI key leaked into JSON: {json}"
        );
        assert!(
            !json.contains("postgres://"),
            "DB connection string leaked into JSON: {json}"
        );
    }
}
