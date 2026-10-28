//! Sequencer-specific API options and initialization.

use std::{collections::BTreeSet, env, path::PathBuf, sync::Arc};

use ::light_client::{state::LightClientOptions, storage::LightClientSqliteOptions};
use anyhow::{Context, bail};
use clap::Parser;
use espresso_telemetry as telemetry;
use espresso_types::v0::traits::NullEventConsumer;
use futures::{channel::oneshot, future::BoxFuture};
use hotshot_query_service::{
    data_source::{ExtensibleDataSource, MetricsDataSource},
    status::{HasMetrics, UpdateStatusData},
};
use hotshot_types::traits::metrics::{Metrics, NoMetrics};
use process_metrics::ProcessMetrics;
use url::Url;

use super::{
    ApiState, StorageState,
    context::ApiContext,
    data_source::{DataSourceOptions, PruningDataSource, SequencerDataSource, provider},
    fs, sql,
    state::NodeApiStateImpl,
    update::{ApiEventConsumer, ApiSink},
};
use crate::{
    catchup::CatchupStorage, context::TaskList, options::PublicNodeConfig, persistence,
    request_response::data_source::Storage as RequestResponseStorage,
};

#[derive(Clone, Debug)]
pub struct Options {
    pub http: Http,
    pub query: Option<Query>,
    pub submit: Option<Submit>,
    pub status: Option<Status>,
    pub catchup: Option<Catchup>,
    pub config: Option<Config>,
    pub explorer: Option<Explorer>,
    pub storage_fs: Option<persistence::fs::Options>,
    pub storage_sql: Option<persistence::sql::Options>,
    pub public_node_config: Option<Box<PublicNodeConfig>>,
}

impl From<Http> for Options {
    fn from(http: Http) -> Self {
        Self {
            http,
            query: None,
            submit: None,
            status: None,
            catchup: None,
            config: None,
            explorer: None,
            storage_fs: None,
            storage_sql: None,
            public_node_config: None,
        }
    }
}

impl Options {
    /// Default options for running a web server on the given port.
    pub fn with_port(port: u16) -> Self {
        Http::with_port(port).into()
    }

    /// Add a query API module backed by a Postgres database.
    pub fn query_sql(mut self, query: Query, storage: persistence::sql::Options) -> Self {
        self.query = Some(query);
        self.storage_sql = Some(storage);
        self
    }

    /// Add a query API module backed by the file system.
    pub fn query_fs(mut self, query: Query, storage: persistence::fs::Options) -> Self {
        self.query = Some(query);
        self.storage_fs = Some(storage);
        self
    }

    /// Add a submit API module.
    pub fn submit(mut self, opt: Submit) -> Self {
        self.submit = Some(opt);
        self
    }

    /// Add a status API module.
    pub fn status(mut self, opt: Status) -> Self {
        self.status = Some(opt);
        self
    }

    /// Add a catchup API module.
    pub fn catchup(mut self, opt: Catchup) -> Self {
        self.catchup = Some(opt);
        self
    }

    /// Add a config API module.
    pub fn config(mut self, opt: Config) -> Self {
        self.config = Some(opt);
        self
    }

    /// Set the merged runtime configuration exposed via `GET /config/runtime`.
    ///
    /// If unset, the `/config/runtime` route returns 404.
    pub fn public_node_config(mut self, c: PublicNodeConfig) -> Self {
        self.public_node_config = Some(Box::new(c));
        self
    }

    /// Add an explorer API module.
    pub fn explorer(mut self, opt: Explorer) -> Self {
        self.explorer = Some(opt);
        self
    }

    /// Whether these options will run the query API.
    pub fn has_query_module(&self) -> bool {
        self.query.is_some() && (self.storage_fs.is_some() || self.storage_sql.is_some())
    }

    /// Start the server.
    ///
    /// The function `init_context` is used to create a sequencer context from a metrics object and
    /// optional saved consensus state. The metrics object is created from the API data source, so
    /// that consensus will populuate metrics that can then be read and served by the API.
    pub async fn serve<C, F>(mut self, init_context: F) -> anyhow::Result<C>
    where
        C: ApiContext + Clone,
        F: FnOnce(
            Box<dyn Metrics>,
            Box<dyn ApiSink>,
            Option<RequestResponseStorage>,
        ) -> BoxFuture<'static, anyhow::Result<C>>,
    {
        // Create a channel to send the context to the web server after it is initialized. This
        // allows the web server to start before initialization can complete, since initialization
        // can take a long time (and is dependent on other nodes).
        let (send_ctx, recv_ctx) = oneshot::channel();
        let state = ApiState::new(async move {
            recv_ctx
                .await
                .expect("context initialized and sent over channel")
        });
        let mut tasks = TaskList::default();

        // The server state type depends on whether we are running a query or status API or not, so
        // we handle the two cases differently.
        #[allow(clippy::type_complexity)]
        let (metrics, consumer, storage): (
            Box<dyn Metrics>,
            Box<dyn ApiSink>,
            Option<RequestResponseStorage>,
        ) = if let Some(query_opt) = self.query.take() {
            if let Some(opt) = self.storage_sql.take() {
                match opt.into_backend() {
                    persistence::sql::SqlBackendOptions::Sqlite(opt) => {
                        sql::sqlite::init_query_module(&self, query_opt, opt, state, &mut tasks)
                            .await?
                    },
                    persistence::sql::SqlBackendOptions::Postgres(opt) => {
                        sql::postgres::init_query_module(&self, query_opt, opt, state, &mut tasks)
                            .await?
                    },
                }
            } else if let Some(opt) = self.storage_fs.take() {
                self.init_with_query_module_fs(query_opt, opt, state, &mut tasks)
                    .await?
            } else {
                bail!("query module requested but not storage provided");
            }
        } else if self.status.is_some() {
            // If a status API is requested but no availability API, we use the
            // `MetricsDataSource`, which allows us to run the status API with no persistent
            // storage.
            let ds = MetricsDataSource::default();
            let metrics = ds.populate_metrics();
            telemetry::set_registry(Arc::new(ds.metrics().registry().clone()));
            tasks.spawn("process_metrics", ProcessMetrics::new(ds.metrics()).run());
            let axum_ds = Arc::new(ExtensibleDataSource::new(ds, state.clone()));

            let port = self.http.port;
            let env_vars = get_public_env_vars().unwrap_or_default();
            let node_cfg = self.public_node_config.as_deref().cloned();
            let modules = espresso_api::OptionalModules {
                submit: self.submit.is_some(),
                catchup: self.catchup.is_some(),
                config: self.config.is_some(),
                ..Default::default()
            };
            let max_connections = self.http.max_connections;
            tasks.spawn("API server", async move {
                let state = NodeApiStateImpl::new(axum_ds)
                    .with_env_vars(env_vars)
                    .with_public_node_config(node_cfg);
                if let Err(e) =
                    espresso_api::serve_axum_status(port, state, modules, max_connections).await
                {
                    tracing::error!("Axum server error: {}", e);
                }
                anyhow::Ok(())
            });

            if self.http.tonic_port.is_some() {
                tracing::warn!("gRPC API not available in status-only mode");
            }

            (metrics, Box::new(NullEventConsumer), None)
        } else {
            // If no status or availability API is requested, we don't need metrics or a query
            // service data source. The only app state is the HotShot handle, which we use to
            // submit transactions.
            //
            // If we have no availability API, we cannot load a saved leaf from local storage,
            // so we better have been provided the leaf ahead of time if we want it at all.
            let port = self.http.port;
            let env_vars = get_public_env_vars().unwrap_or_default();
            let node_cfg = self.public_node_config.as_deref().cloned();
            let modules = espresso_api::OptionalModules {
                submit: self.submit.is_some(),
                catchup: self.catchup.is_some(),
                config: self.config.is_some(),
                ..Default::default()
            };
            let axum_ds = Arc::new(state.clone());
            let max_connections = self.http.max_connections;
            tasks.spawn("API server", async move {
                let state = NodeApiStateImpl::new(axum_ds)
                    .with_env_vars(env_vars)
                    .with_public_node_config(node_cfg);
                if let Err(e) =
                    espresso_api::serve_axum_bare(port, state, modules, max_connections).await
                {
                    tracing::error!("Axum server error: {}", e);
                }
                anyhow::Ok(())
            });

            (Box::new(NoMetrics), Box::new(NullEventConsumer), None)
        };

        let ctx = init_context(metrics, consumer, storage.clone()).await?;
        send_ctx
            .send(Box::new(ctx.clone()))
            .ok()
            .context("API server exited without receiving context")?;
        Ok(ctx.with_task_list(tasks))
    }

    async fn init_with_query_module_fs(
        &self,
        query_opt: Query,
        mod_opt: persistence::fs::Options,
        state: ApiState,
        tasks: &mut TaskList,
    ) -> anyhow::Result<(
        Box<dyn Metrics>,
        Box<dyn ApiSink>,
        Option<RequestResponseStorage>,
    )> {
        let light_client_db = mod_opt.light_client_db(&query_opt)?;
        let ds = <fs::DataSource as SequencerDataSource>::create(
            mod_opt,
            provider(
                query_opt.peers,
                &state,
                query_opt.light_client,
                light_client_db,
            )
            .await?,
            false,
        )
        .await?;

        // Get the inner storage from the data source
        let inner_storage = ds.inner();

        tasks.spawn("process_metrics", ProcessMetrics::new(ds.metrics()).run());

        let (metrics, ds) = init_query_data_source(ds, state.clone());

        let port = self.http.port;
        let ds_for_axum = ds.clone();
        let env_vars = get_public_env_vars().unwrap_or_default();
        let node_cfg = self.public_node_config.as_deref().cloned();
        let modules = espresso_api::OptionalModules {
            submit: self.submit.is_some(),
            config: self.config.is_some(),
            ..Default::default()
        };
        let max_connections = self.http.max_connections;
        tasks.spawn("API server", async move {
            let state = NodeApiStateImpl::new(ds_for_axum)
                .with_env_vars(env_vars)
                .with_public_node_config(node_cfg);
            if let Err(e) = espresso_api::serve_axum_fs(port, state, modules, max_connections).await
            {
                tracing::error!("Axum server error: {}", e);
            }
            anyhow::Ok(())
        });

        if self.http.tonic_port.is_some() {
            tracing::warn!("gRPC API not available with filesystem storage");
        }

        Ok((
            metrics,
            Box::new(ApiEventConsumer::from(ds)),
            Some(RequestResponseStorage::Fs(inner_storage)),
        ))
    }
}

/// The minimal HTTP API.
///
/// The API automatically includes health and version endpoints. Additional API modules can be
/// added by including the query-api or submit-api modules.
#[derive(Parser, Clone, Copy, Debug)]
pub struct Http {
    /// Port that the HTTP API will use.
    #[clap(long, env = "ESPRESSO_NODE_API_PORT", default_value = "8080")]
    pub port: u16,

    /// Maximum number of concurrent HTTP connections the server will allow.
    ///
    /// Connections exceeding this will receive and immediate 429 response and be closed.
    ///
    /// Leave unset for no connection limit.
    #[clap(long, env = "ESPRESSO_NODE_API_MAX_CONNECTIONS")]
    pub max_connections: Option<usize>,

    /// Optional port for Tonic gRPC API server.
    #[clap(long, env = "ESPRESSO_NODE_TONIC_PORT")]
    pub tonic_port: Option<u16>,
}

impl Http {
    /// Default options for running a web server on the given port.
    pub fn with_port(port: u16) -> Self {
        Self {
            port,
            max_connections: None,
            tonic_port: None,
        }
    }
}

/// Options for the submission API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct Submit;

/// Options for the status API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct Status;

/// Options for the catchup API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct Catchup;

/// Options for the config API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct Config;

/// Options for the query API module.
#[derive(Parser, Clone, Debug, Default)]
pub struct Query {
    /// Peers for fetching missing data for the query service.
    #[clap(long, env = "ESPRESSO_NODE_API_PEERS", value_delimiter = ',')]
    pub peers: Vec<Url>,

    /// Light client configuration, for fetching data from peers.
    #[clap(flatten)]
    pub light_client: LightClientOptions,

    /// Persistence for the light client, enabling faster startup.
    #[clap(flatten)]
    pub light_client_db: LightClientSqliteOptions,

    /// Node storage directory. A SQLite light client database without `--light-client-db-path`
    /// is kept under it, so the stake tables it verified survive a restart.
    #[clap(long, env = "ESPRESSO_NODE_STORAGE_PATH")]
    pub storage_path: Option<PathBuf>,
}

impl Query {
    pub fn sqlite_light_client_db(&self) -> LightClientSqliteOptions {
        let lc_path = self.light_client_db.lc_path.clone().or_else(|| {
            self.storage_path
                .as_ref()
                .map(|dir| dir.join("light-client").join("lc.db"))
        });
        LightClientSqliteOptions {
            lc_path,
            ..self.light_client_db.clone()
        }
    }
}

#[cfg(test)]
impl Query {
    pub fn test() -> Self {
        Self::default()
    }
}

/// Options for the state API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct State;

/// Options for the retired Hotshot events streaming API module, which is still accepted on the
/// command line but serves nothing.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct HotshotEvents;

/// Options for the explorer API module.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct Explorer;

/// Options for the light client API module, which is still accepted on the command line but has
/// no effect: every SQL query node serves the light client API.
#[derive(Parser, Clone, Copy, Debug, Default)]
pub struct LightClient;

/// Populate consensus metrics on `ds`, deposit its prometheus registry for the in-process
/// telemetry push task (idempotent), and wrap it with the API state.
///
/// Returns the metrics handle plus the wrapped query data source shared by the axum server and
/// update loops.
#[allow(clippy::type_complexity)]
pub(super) fn init_query_data_source<D>(
    ds: D,
    state: ApiState,
) -> (Box<dyn Metrics>, Arc<StorageState<D>>)
where
    D: SequencerDataSource + CatchupStorage + PruningDataSource + Send + Sync + 'static,
{
    let metrics = ds.populate_metrics();
    telemetry::set_registry(Arc::new(ds.metrics().registry().clone()));
    let ds = Arc::new(ExtensibleDataSource::new(ds, state));
    (metrics, ds)
}

/// The environment variables listed in `api/public-env-vars.toml`, as `KEY=value` strings.
pub(super) fn get_public_env_vars() -> anyhow::Result<Vec<String>> {
    #[derive(serde::Deserialize)]
    struct PublicEnvVars {
        variables: BTreeSet<String>,
    }

    let PublicEnvVars { variables } =
        toml::from_str(include_str!("../../api/public-env-vars.toml"))?;
    Ok(variables
        .into_iter()
        .map(|key| {
            let value = env::var(&key).unwrap_or_default();
            format!("{key}={value}")
        })
        .collect())
}
