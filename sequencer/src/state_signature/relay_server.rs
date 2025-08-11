use std::{path::PathBuf, sync::Arc};

use async_lock::RwLock;
use clap::Args;
use futures::FutureExt;
use hotshot_types::{
    light_client::{
        LCV1StateSignatureRequestBody, LCV1StateSignaturesBundle, LCV2StateSignaturesBundle,
        LCV3StateSignatureRequestBody, LCV3StateSignaturesBundle,
    },
    traits::signature_key::LCV1StateSignatureKey,
};
use lcv1_relay::{LCV1StateRelayServerDataSource, LCV1StateRelayServerState};
use lcv2_relay::{LCV2StateRelayServerDataSource, LCV2StateRelayServerState};
use lcv3_relay::{LCV3StateRelayServerDataSource, LCV3StateRelayServerState};
use tide_disco::{
    api::ApiError,
    error::ServerError,
    method::{ReadState, WriteState},
    Api, App, Error as _, StatusCode,
};
use tokio::sync::oneshot;
use url::Url;
use vbs::version::StaticVersionType;

use super::LCV2StateSignatureRequestBody;

pub mod lcv1_relay;
pub mod lcv2_relay;
pub mod lcv3_relay;
pub mod stake_table_tracker;

/// State that checks the light client state update and the signature collection
pub struct StateRelayServerState {
    /// Handling LCV1 state signatures
    lcv1_state: LCV1StateRelayServerState,
    /// Handling LCV2 state signatures
    lcv2_state: LCV2StateRelayServerState,
    /// Handling LCV3 state signatures
    lcv3_state: LCV3StateRelayServerState,
    /// shutdown signal
    shutdown: Option<oneshot::Receiver<()>>,
}

impl StateRelayServerState {
    /// Init the server state
    pub fn new(sequencer_url: Url) -> Self {
        let stake_table_tracker =
            Arc::new(stake_table_tracker::StakeTableTracker::new(sequencer_url));
        Self {
            lcv1_state: LCV1StateRelayServerState::new(stake_table_tracker.clone()),
            lcv2_state: LCV2StateRelayServerState::new(stake_table_tracker.clone()),
            lcv3_state: LCV3StateRelayServerState::new(stake_table_tracker),
            shutdown: None,
        }
    }

    pub fn with_shutdown_signal(
        mut self,
        shutdown_listener: Option<oneshot::Receiver<()>>,
    ) -> Self {
        if self.shutdown.is_some() {
            panic!("A shutdown signal is already registered and can not be registered twice");
        }
        self.shutdown = shutdown_listener;
        self
    }
}

#[async_trait::async_trait]
impl LCV1StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV1StateSignaturesBundle, ServerError> {
        self.lcv1_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV1StateSignatureRequestBody,
    ) -> Result<(), ServerError> {
        self.lcv1_state.post_signature(req).await
    }
}

#[async_trait::async_trait]
impl LCV2StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV2StateSignaturesBundle, ServerError> {
        self.lcv2_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV2StateSignatureRequestBody,
    ) -> Result<(), ServerError> {
        self.lcv2_state.post_signature(req).await
    }
}

#[async_trait::async_trait]
impl LCV3StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV3StateSignaturesBundle, ServerError> {
        self.lcv3_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV3StateSignatureRequestBody,
    ) -> Result<(), ServerError> {
        self.lcv3_state.post_signature(req).await
    }
}

/// configurability options for the web server
#[derive(Args, Default)]
pub struct Options {
    #[arg(
        long = "state-relay-server-api-path",
        env = "STATE_RELAY_SERVER_API_PATH"
    )]
    /// path to API
    pub api_path: Option<PathBuf>,
}

/// Set up APIs for relay server
fn define_api<State, BindVer: StaticVersionType + 'static>(
    options: &Options,
    bind_version: BindVer,
    api_ver: semver::Version,
) -> Result<Api<State, ServerError, BindVer>, ApiError>
where
    State: 'static + Send + Sync + ReadState + WriteState,
    <State as ReadState>::State: Send
        + Sync
        + LCV1StateRelayServerDataSource
        + LCV2StateRelayServerDataSource
        + LCV3StateRelayServerDataSource,
{
    let mut api = match &options.api_path {
        Some(path) => Api::<State, ServerError, BindVer>::from_file(path)?,
        None => {
            let toml: toml::Value = toml::from_str(include_str!(
                "../../api/state_relay_server.toml"
            ))
            .map_err(|err| ApiError::CannotReadToml {
                reason: err.to_string(),
            })?;
            Api::<State, ServerError, BindVer>::new(toml)?
        },
    };

    api.with_version(api_ver.clone());

    api.post("postlegacystatesignature", move |req, state| {
        async move {
            let req = match req.body_auto::<LCV1StateSignatureRequestBody, BindVer>(bind_version) {
                Ok(req) => req,
                Err(_) => {
                    match req.body_auto::<LCV2StateSignatureRequestBody, BindVer>(bind_version) {
                        Ok(req) => req.into(),
                        Err(_) => {
                            return Err(ServerError::catch_all(
                                StatusCode::BAD_REQUEST,
                                "Invalid request body".to_string(),
                            ))
                        },
                    }
                },
            };
            LCV1StateRelayServerDataSource::post_signature(state, req).await?;
            Ok(())
        }
        .boxed()
    })?
    .post("poststatesignature", move |req, state| {
        async move {
            if let Ok(req) = req.body_auto::<LCV3StateSignatureRequestBody, BindVer>(bind_version) {
                tracing::debug!("Received LCV3 state signature: {req}");
                if let Err(e) =
                    LCV2StateRelayServerDataSource::post_signature(state, req.clone().into()).await
                {
                    tracing::error!("Failed to post downgraded LCV2 state signature: {}", e);
                }
                LCV3StateRelayServerDataSource::post_signature(state, req).await
            } else if let Ok(req) =
                req.body_auto::<LCV2StateSignatureRequestBody, BindVer>(bind_version)
            {
                tracing::debug!("Received LCV2 state signature: {req}");
                if LCV1StateSignatureKey::verify_state_sig(&req.key, &req.signature, &req.state) {
                    LCV1StateRelayServerDataSource::post_signature(state, req.into()).await
                } else {
                    LCV2StateRelayServerDataSource::post_signature(state, req).await
                }
            } else if let Ok(req) =
                req.body_auto::<LCV1StateSignatureRequestBody, BindVer>(bind_version)
            {
                tracing::debug!("Received LCV1 state signature: {req}");
                LCV1StateRelayServerDataSource::post_signature(state, req).await
            } else {
                Err(ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "Invalid request body".to_string(),
                ))
            }
        }
        .boxed()
    })?
    .get("getlatestlegacystate", |_req, state| {
        async move { LCV1StateRelayServerDataSource::get_latest_signature_bundle(state) }.boxed()
    })?
    .get("getlateststate", |_req, state| {
        async move { LCV2StateRelayServerDataSource::get_latest_signature_bundle(state) }.boxed()
    })?;

    if api_ver.major == 1 {
        api.get("lateststate", |_req, state| {
            async move { LCV1StateRelayServerDataSource::get_latest_signature_bundle(state) }
                .boxed()
        })?;
    } else if api_ver.major == 2 {
        api.get("lateststate", |_req, state| {
            async move { LCV2StateRelayServerDataSource::get_latest_signature_bundle(state) }
                .boxed()
        })?;
    } else {
        api.get("lateststate", |_req, state| {
            async move { LCV3StateRelayServerDataSource::get_latest_signature_bundle(state) }
                .boxed()
        })?;
    }
    Ok(api)
}

pub async fn run_relay_server<BindVer: StaticVersionType + 'static>(
    shutdown_listener: Option<oneshot::Receiver<()>>,
    sequencer_url: Url,
    url: Url,
    bind_version: BindVer,
) -> anyhow::Result<()> {
    let options = Options::default();

    let state = RwLock::new(
        StateRelayServerState::new(sequencer_url).with_shutdown_signal(shutdown_listener),
    );
    let mut app = App::<RwLock<StateRelayServerState>, ServerError>::with_state(state);

    let v1_api = define_api(&options, bind_version, "1.0.0".parse().unwrap()).unwrap();
    let v2_api = define_api(&options, bind_version, "2.0.0".parse().unwrap()).unwrap();
    let v3_api = define_api(&options, bind_version, "3.0.0".parse().unwrap()).unwrap();
    app.register_module("api", v1_api)?
        .register_module("api", v2_api)?
        .register_module("api", v3_api)?;

    let app_future = app.serve(url.clone(), bind_version);
    app_future.await?;

    tracing::info!(%url, "Relay server starts serving at ");

    Ok(())
}

pub async fn run_relay_server_with_state<BindVer: StaticVersionType + 'static>(
    server_url: Url,
    bind_version: BindVer,
    state: StateRelayServerState,
) -> anyhow::Result<()> {
    let options = Options::default();

    let mut app = App::<RwLock<StateRelayServerState>, ServerError>::with_state(RwLock::new(state));

    app.register_module(
        "api",
        define_api(&options, bind_version, "1.0.0".parse().unwrap()).unwrap(),
    )
    .unwrap();
    app.register_module(
        "api",
        define_api(&options, bind_version, "2.0.0".parse().unwrap()).unwrap(),
    )
    .unwrap();
    app.register_module(
        "api",
        define_api(&options, bind_version, "3.0.0".parse().unwrap()).unwrap(),
    )
    .unwrap();

    let app_future = app.serve(server_url.clone(), bind_version);
    app_future.await?;

    tracing::info!(%server_url, "Relay server starts serving at ");

    Ok(())
}
