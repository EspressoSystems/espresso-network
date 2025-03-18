use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
};

use async_lock::RwLock;
use clap::Args;
use ethers::types::U256;
use futures::FutureExt;
use hotshot_stake_table::vec_based::config::FieldType;
use hotshot_types::light_client::{
    StateSignature, StateSignatureScheme, StateSignaturesBundle, StateVerKey,
};
use jf_signature::SignatureScheme;
use tide_disco::{
    api::ApiError,
    error::ServerError,
    method::{ReadState, WriteState},
    Api, App, Error as _, StatusCode,
};
use tokio::sync::oneshot;
use url::Url;
use vbs::version::StaticVersionType;

use super::{LightClientState, StateSignatureRequestBody};

/// State that checks the light client state update and the signature collection
#[derive(Default)]
struct StateRelayServerState {
    /// Minimum weight to form an available state signature bundle
    threshold: U256,
    /// Stake table
    known_nodes: HashMap<StateVerKey, U256>,
    /// Signatures bundles for each block height
    bundles: HashMap<u64, HashMap<LightClientState, StateSignaturesBundle>>,

    /// The latest state signatures bundle whose total weight exceeds the threshold
    latest_available_bundle: Option<StateSignaturesBundle>,
    /// The block height of the latest available state signature bundle
    latest_block_height: Option<u64>,

    /// A ordered queue of block heights, used for garbage collection.
    queue: BTreeSet<u64>,

    /// shutdown signal
    shutdown: Option<oneshot::Receiver<()>>,
}

impl StateRelayServerState {
    pub fn new(threshold: U256) -> Self {
        Self {
            threshold,
            ..Default::default()
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

// TODO(Chengyu): move this `RwLock` inside `StateRelayServerState` so that when nodes are submitting
//                signatures, it won't block the prover from fetching the available signatures.
type State = RwLock<StateRelayServerState>;
type Error = ServerError;

pub trait StateRelayServerDataSource {
    /// Get the latest available signatures bundle.
    /// # Errors
    /// Errors if there's no available signatures bundle.
    fn get_latest_signature_bundle(&self) -> Result<StateSignaturesBundle, Error>;

    /// Post a signature to the relay server
    /// # Errors
    /// Errors if the signature is invalid, already posted, or no longer needed.
    fn post_signature(
        &mut self,
        key: StateVerKey,
        state: LightClientState,
        signature: StateSignature,
    ) -> Result<(), Error>;
}

impl StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<StateSignaturesBundle, Error> {
        match &self.latest_available_bundle {
            Some(bundle) => Ok(bundle.clone()),
            None => Err(tide_disco::error::ServerError::catch_all(
                StatusCode::NOT_FOUND,
                "The light client state signatures are not ready.".to_owned(),
            )),
        }
    }

    fn post_signature(
        &mut self,
        key: StateVerKey,
        state: LightClientState,
        signature: StateSignature,
    ) -> Result<(), Error> {
        if (state.block_height as u64) <= self.latest_block_height.unwrap_or(0) {
            // This signature is no longer needed
            return Ok(());
        }
        let one = U256::one();
        let weight = self.known_nodes.get(&key).unwrap_or(&one);
        // TODO(Chengyu): We don't know where to fetch the stake table yet.
        // Related issue: [https://github.com/EspressoSystems/espresso-sequencer/issues/1022]
        // .ok_or(tide_disco::error::ServerError::catch_all(
        //     StatusCode::Unauthorized,
        //     "The posted key is not found in the stake table.".to_owned(),
        // ))?;
        let state_msg: [FieldType; 3] = (&state).into();
        if StateSignatureScheme::verify(&(), &key, state_msg, &signature).is_err() {
            return Err(tide_disco::error::ServerError::catch_all(
                StatusCode::BAD_REQUEST,
                "The posted signature is not valid.".to_owned(),
            ));
        }
        let block_height = state.block_height as u64;
        // TODO(Chengyu): this serialization should be removed once `LightClientState` implements `Eq`.
        let bundles_at_height = self.bundles.entry(block_height).or_insert_with(|| {
            self.queue.insert(block_height);
            Default::default()
        });
        let bundle = bundles_at_height
            .entry(state.clone())
            .or_insert(StateSignaturesBundle {
                state,
                signatures: Default::default(),
                accumulated_weight: U256::from(0),
            });
        tracing::debug!(
            "Accepting new signature for block height {} from {}.",
            block_height,
            key
        );
        match bundle.signatures.entry(key) {
            std::collections::hash_map::Entry::Occupied(_) => {
                // A signature is already posted for this key with this state
                return Err(tide_disco::error::ServerError::catch_all(
                    StatusCode::BAD_REQUEST,
                    "A signature of this light client state is already posted at this block height for this key.".to_owned(),
                ));
            },
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(signature);
                bundle.accumulated_weight += *weight;
            },
        }

        if bundle.accumulated_weight >= self.threshold {
            tracing::info!(
                "State signature bundle at block height {} is ready to serve.",
                block_height
            );
            self.latest_block_height = Some(block_height);
            self.latest_available_bundle = Some(bundle.clone());
            while let Some(height) = self.queue.pop_first() {
                self.bundles.remove(&height);
                if height == block_height {
                    break;
                }
            }
        }
        Ok(())
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
fn define_api<State, ApiVer: StaticVersionType + 'static>(
    options: &Options,
    _: ApiVer,
) -> Result<Api<State, Error, ApiVer>, ApiError>
where
    State: 'static + Send + Sync + ReadState + WriteState,
    <State as ReadState>::State: Send + Sync + StateRelayServerDataSource,
{
    let mut api = match &options.api_path {
        Some(path) => Api::<State, Error, ApiVer>::from_file(path)?,
        None => {
            let toml: toml::Value = toml::from_str(include_str!(
                "../../api/state_relay_server.toml"
            ))
            .map_err(|err| ApiError::CannotReadToml {
                reason: err.to_string(),
            })?;
            Api::<State, Error, ApiVer>::new(toml)?
        },
    };

    api.get("getlateststate", |_req, state| {
        async move { state.get_latest_signature_bundle() }.boxed()
    })?
    .post("poststatesignature", |req, state| {
        async move {
            let StateSignatureRequestBody {
                key,
                state: lcstate,
                signature,
            } = req
                .body_auto::<StateSignatureRequestBody, ApiVer>(ApiVer::instance())
                .map_err(Error::from_request_error)?;
            state.post_signature(key, lcstate, signature)
        }
        .boxed()
    })?;

    Ok(api)
}

pub async fn run_relay_server<ApiVer: StaticVersionType + 'static>(
    shutdown_listener: Option<oneshot::Receiver<()>>,
    threshold: U256,
    url: Url,
    bind_version: ApiVer,
) -> std::io::Result<()> {
    let options = Options::default();

    let api = define_api(&options, bind_version).unwrap();

    // We don't have a stake table yet, putting some temporary value here.
    // Related issue: [https://github.com/EspressoSystems/espresso-sequencer/issues/1022]
    let state =
        State::new(StateRelayServerState::new(threshold).with_shutdown_signal(shutdown_listener));
    let mut app = App::<State, Error>::with_state(state);

    app.register_module("api", api).unwrap();

    let app_future = app.serve(url, bind_version);

    app_future.await
}
