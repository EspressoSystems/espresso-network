#![allow(clippy::needless_lifetimes)]

use core::fmt::Display;
use std::{
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{self, Formatter},
    iter::once,
    path::PathBuf,
    time::Duration,
};

use anyhow::{bail, Context};
use clap::{error::ErrorKind, Args, FromArgMatches, Parser};
use derivative::Derivative;
use espresso_types::{parse_duration, BackoffParams, L1ClientOptions};
use hotshot_types::{light_client::StateSignKey, signature_key::BLSPrivKey};
use jf_signature::{bls_over_bn254, schnorr};
use libp2p::Multiaddr;
use sequencer_utils::logging;
use tagged_base64::TaggedBase64;
use url::Url;

use crate::{api, persistence, proposal_fetcher::ProposalFetcherConfig};

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
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_LIBP2P_ADVERTISE_ADDRESS",
        default_value = "localhost:1769"
    )]
    pub libp2p_advertise_address: String,

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

    /// Path to file containing private keys.
    ///
    /// The file should follow the .env format, with two keys:
    /// * ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY
    /// * ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY
    ///
    /// Appropriate key files can be generated with the `keygen` utility program.
    #[clap(long, name = "KEY_FILE", env = "ESPRESSO_SEQUENCER_KEY_FILE")]
    pub key_file: Option<PathBuf>,

    /// Private staking key.
    ///
    /// This can be used as an alternative to KEY_FILE.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub private_staking_key: Option<TaggedBase64>,

    /// Private state signing key.
    ///
    /// This can be used as an alternative to KEY_FILE.
    #[clap(
        long,
        env = "ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub private_state_key: Option<TaggedBase64>,

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

    pub fn private_keys(&self) -> anyhow::Result<(BLSPrivKey, StateSignKey)> {
        if let Some(path) = &self.key_file {
            let vars = dotenvy::from_path_iter(path)?.collect::<Result<HashMap<_, _>, _>>()?;
            let staking = TaggedBase64::parse(
                vars.get("ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY")
                    .context("key file missing ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY")?,
            )?
            .try_into()?;

            let state = TaggedBase64::parse(
                vars.get("ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY")
                    .context("key file missing ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY")?,
            )?
            .try_into()?;

            Ok((staking, state))
        } else if let (Some(staking), Some(state)) = (
            self.private_staking_key.clone(),
            self.private_state_key.clone(),
        ) {
            let staking = bls_over_bn254::SignKey::try_from(staking)?;
            let state = schnorr::SignKey::try_from(state)?;

            Ok((staking, state))
        } else {
            bail!("neither key file nor full set of private keys was provided")
        }
    }
}

/// Identity represents identifying information concerning the sequencer node.
/// This information is used to populate relevant information in the metrics
/// endpoint.  This information will also potentially be scraped and displayed
/// in a public facing dashboard.
#[derive(Parser, Clone, Derivative)]
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
}

/// get_default_node_type returns the current public facing binary name and
/// version of this program.
fn get_default_node_type() -> String {
    format!("espresso-sequencer {}", env!("CARGO_PKG_VERSION"))
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
}
