use std::{fmt, sync::Arc};

use async_lock::RwLock;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
};
use hotshot_types::{
    light_client::{
        LCV1StateSignatureRequestBody, LCV1StateSignaturesBundle, LCV2StateSignatureRequestBody,
        LCV2StateSignaturesBundle, LCV3StateSignatureRequestBody, LCV3StateSignaturesBundle,
    },
    traits::signature_key::LCV1StateSignatureKey,
};
use http_wire::{self as wire, DecodeFailure, WireError, cors_layer, healthcheck_response};
use lcv1_relay::{LCV1StateRelayServerDataSource, LCV1StateRelayServerState};
use lcv2_relay::{LCV2StateRelayServerDataSource, LCV2StateRelayServerState};
use lcv3_relay::{LCV3StateRelayServerDataSource, LCV3StateRelayServerState};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use stake_table_tracker::StakeTableTracker;
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;
use vbs::version::StaticVersionType;

pub mod lcv1_relay;
pub mod lcv2_relay;
pub mod lcv3_relay;
pub mod stake_table_tracker;

/// Wire-compatible error envelope: mirrors `http_client::error::ClientErr`'s `{status, message}`
/// JSON/VBS shape, since production clients (`state_signature.rs`, `hotshot-state-prover`)
/// deserialize error responses into that type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayError {
    pub status: u16,
    pub message: String,
}

impl RelayError {
    pub fn catch_all(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status: status.as_u16(),
            message: message.into(),
        }
    }
}

impl fmt::Display for RelayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {}: {}", self.status, self.message)
    }
}

impl std::error::Error for RelayError {}

impl WireError for RelayError {
    fn status(&self) -> StatusCode {
        StatusCode::from_u16(self.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)
    }

    fn catch_all(status: StatusCode, message: String) -> Self {
        RelayError::catch_all(status, message)
    }
}

fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RelayError> {
    wire::decode_body(headers, body).map_err(|failure| match failure {
        DecodeFailure::Binary(e) => {
            RelayError::catch_all(StatusCode::BAD_REQUEST, format!("invalid binary body: {e}"))
        },
        DecodeFailure::Json(e) => {
            RelayError::catch_all(StatusCode::BAD_REQUEST, format!("invalid json body: {e}"))
        },
        DecodeFailure::UnsupportedContentType => RelayError::catch_all(
            StatusCode::BAD_REQUEST,
            "missing or unsupported Content-Type",
        ),
    })
}

/// State that checks the light client state update and the signature collection
pub struct StateRelayServerState {
    /// Handling LCV1 state signatures
    lcv1_state: LCV1StateRelayServerState,
    /// Handling LCV2 state signatures
    lcv2_state: LCV2StateRelayServerState,
    /// Handling LCV3 state signatures
    lcv3_state: LCV3StateRelayServerState,
    stake_table_tracker: Arc<StakeTableTracker>,
    /// shutdown signal
    shutdown: Option<oneshot::Receiver<()>>,
}

impl StateRelayServerState {
    /// Init the server state
    pub fn new(sequencer_url: Url) -> Self {
        Self {
            lcv1_state: LCV1StateRelayServerState::default(),
            lcv2_state: LCV2StateRelayServerState::default(),
            lcv3_state: LCV3StateRelayServerState::default(),
            stake_table_tracker: Arc::new(StakeTableTracker::new(sequencer_url)),
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

/// Router state, cloned into every axum handler via `State`. The stake table tracker sits outside
/// the lock because a lookup can go to the sequencer for a height the unauthenticated caller
/// picked.
#[derive(Clone)]
struct SharedState {
    stake_tables: Arc<StakeTableTracker>,
    signatures: Arc<RwLock<StateRelayServerState>>,
}

fn stake_table_unavailable(err: anyhow::Error) -> RelayError {
    RelayError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#}"))
}

/// Handle a `POST` to `state`/`api/state`: tries LCV3, then LCV2 (auto-downgrading to LCV1 if the
/// signature verifies against the legacy scheme), then LCV1. Mirrors tide's `poststatesignature`.
async fn post_state_signature(
    state: &SharedState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), RelayError> {
    if let Ok(req) = decode_body::<LCV3StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV3 state signature: {req}");
        let stake_table = state
            .stake_tables
            .stake_table_info_for_block(req.state.block_height)
            .await
            .map_err(stake_table_unavailable)?;
        let mut signatures = state.signatures.write().await;
        if let Err(e) = signatures
            .lcv2_state
            .post_signature(req.clone().into(), &stake_table)
        {
            tracing::error!("Failed to post downgraded LCV2 state signature: {}", e);
        }
        signatures.lcv3_state.post_signature(req, &stake_table)
    } else if let Ok(req) = decode_body::<LCV2StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV2 state signature: {req}");
        if LCV1StateSignatureKey::verify_state_sig(&req.key, &req.signature, &req.state) {
            post_lcv1_signature(state, req.into()).await
        } else {
            let stake_table = state
                .stake_tables
                .stake_table_info_for_block(req.state.block_height)
                .await
                .map_err(stake_table_unavailable)?;
            let mut signatures = state.signatures.write().await;
            signatures.lcv2_state.post_signature(req, &stake_table)
        }
    } else if let Ok(req) = decode_body::<LCV1StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV1 state signature: {req}");
        post_lcv1_signature(state, req).await
    } else {
        Err(RelayError::catch_all(
            StatusCode::BAD_REQUEST,
            "Invalid request body",
        ))
    }
}

/// Handle a `POST` to `legacy-state`: tries LCV1, then LCV2 (downgraded to LCV1). Mirrors tide's
/// `postlegacystatesignature`.
async fn post_legacy_state_signature(
    state: &SharedState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), RelayError> {
    let req = if let Ok(req) = decode_body::<LCV1StateSignatureRequestBody>(headers, body) {
        req
    } else if let Ok(req) = decode_body::<LCV2StateSignatureRequestBody>(headers, body) {
        req.into()
    } else {
        return Err(RelayError::catch_all(
            StatusCode::BAD_REQUEST,
            "Invalid request body",
        ));
    };
    post_lcv1_signature(state, req).await
}

async fn post_lcv1_signature(
    state: &SharedState,
    req: LCV1StateSignatureRequestBody,
) -> Result<(), RelayError> {
    let stake_table = state
        .stake_tables
        .genesis_stake_table_info()
        .await
        .map_err(stake_table_unavailable)?;
    let mut signatures = state.signatures.write().await;
    signatures.lcv1_state.post_signature(req, &stake_table)
}

/// `GET state` (deprecated): mirrors tide's `getlateststate`, always the LCV2 bundle regardless
/// of version, since all three registered API versions shared this handler.
async fn get_latest_state(state: &SharedState) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let signatures = state.signatures.read().await;
    signatures.lcv2_state.get_latest_signature_bundle()
}

/// `GET legacy-state`: mirrors tide's `getlatestlegacystate`.
async fn get_latest_legacy_state(
    state: &SharedState,
) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let signatures = state.signatures.read().await;
    signatures
        .lcv1_state
        .get_latest_signature_bundle()
        .map(LCV2StateSignaturesBundle::from_v1)
}

async fn get_latest_state_v1(state: &SharedState) -> Result<LCV1StateSignaturesBundle, RelayError> {
    let signatures = state.signatures.read().await;
    signatures.lcv1_state.get_latest_signature_bundle()
}

async fn get_latest_state_v2(state: &SharedState) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let signatures = state.signatures.read().await;
    signatures.lcv2_state.get_latest_signature_bundle()
}

async fn get_latest_state_v3(state: &SharedState) -> Result<LCV3StateSignaturesBundle, RelayError> {
    let signatures = state.signatures.read().await;
    signatures.lcv3_state.get_latest_signature_bundle()
}

async fn healthcheck(headers: HeaderMap) -> Response {
    healthcheck_response(&headers)
}

async fn post_state(State(state): State<SharedState>, headers: HeaderMap, body: Bytes) -> Response {
    wire::respond(
        &headers,
        post_state_signature(&state, &headers, &body).await,
    )
}

async fn get_state(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    wire::respond(&headers, get_latest_state(&state).await)
}

async fn post_legacy_state(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    wire::respond(
        &headers,
        post_legacy_state_signature(&state, &headers, &body).await,
    )
}

async fn get_legacy_state(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    wire::respond(&headers, get_latest_legacy_state(&state).await)
}

async fn get_lateststate_v1(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    wire::respond(&headers, get_latest_state_v1(&state).await)
}

async fn get_lateststate_v2(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    wire::respond(&headers, get_latest_state_v2(&state).await)
}

async fn get_lateststate_v3(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    wire::respond(&headers, get_latest_state_v3(&state).await)
}

const STATE_PATH: &str = "/api/state";
const LEGACY_STATE_PATH: &str = "/api/legacy-state";
const LATEST_STATE_PATH: &str = "/api/lateststate";

/// Build the relay server router. Tide-disco registered `api` under three stacked major API
/// versions (v1, v2, v3) with identical handlers for every route except `lateststate`; requests
/// with no version prefix were redirected to the latest (v3). We reproduce that by serving the
/// same handlers at the unversioned and all three `/v{1,2,3}` paths, and only special-casing
/// `lateststate` per version.
fn router(state: StateRelayServerState) -> Router {
    let state = SharedState {
        stake_tables: Arc::clone(&state.stake_table_tracker),
        signatures: Arc::new(RwLock::new(state)),
    };
    let mut router = Router::<SharedState>::new()
        .route("/healthcheck", get(healthcheck))
        .route(STATE_PATH, post(post_state).get(get_state))
        .route(
            LEGACY_STATE_PATH,
            post(post_legacy_state).get(get_legacy_state),
        )
        .route(LATEST_STATE_PATH, get(get_lateststate_v3))
        .route(&format!("/v1{LATEST_STATE_PATH}"), get(get_lateststate_v1))
        .route(&format!("/v2{LATEST_STATE_PATH}"), get(get_lateststate_v2))
        .route(&format!("/v3{LATEST_STATE_PATH}"), get(get_lateststate_v3));
    for v in 1..=3 {
        router = router
            .route(
                &format!("/v{v}{STATE_PATH}"),
                post(post_state).get(get_state),
            )
            .route(
                &format!("/v{v}{LEGACY_STATE_PATH}"),
                post(post_legacy_state).get(get_legacy_state),
            );
    }
    router.with_state(state).layer(cors_layer())
}

async fn serve(server_url: Url, state: StateRelayServerState) -> anyhow::Result<()> {
    let host = server_url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("relay server url missing host: {server_url}"))?;
    let port = server_url
        .port_or_known_default()
        .ok_or_else(|| anyhow::anyhow!("relay server url missing port: {server_url}"))?;
    let listener = TcpListener::bind((host, port)).await?;

    tracing::info!(%server_url, "Relay server starts serving at ");
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub async fn run_relay_server<BindVer: StaticVersionType + 'static>(
    shutdown_listener: Option<oneshot::Receiver<()>>,
    sequencer_url: Url,
    url: Url,
    // Kept only for compatibility with the binary's call site; the axum server no longer needs a
    // binary framing version for its own top-level endpoints.
    _bind_version: BindVer,
) -> anyhow::Result<()> {
    let state = StateRelayServerState::new(sequencer_url).with_shutdown_signal(shutdown_listener);
    serve(url, state).await
}

pub async fn run_relay_server_with_state<BindVer: StaticVersionType + 'static>(
    server_url: Url,
    _bind_version: BindVer,
    state: StateRelayServerState,
) -> anyhow::Result<()> {
    serve(server_url, state).await
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use alloy::primitives::{FixedBytes, U256};
    use axum::Json;
    use espresso_types::SeqTypes;
    use hotshot_contract_adapter::light_client::derive_signed_state_digest;
    use hotshot_types::{
        PeerConfig, ValidatorConfig,
        light_client::{LightClientState, StakeTableState},
        signature_key::SchnorrPubKey,
        traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey},
    };
    use http_client::{Client, error::ClientErr};
    use tokio::sync::Notify;
    use vbs::version::StaticVersion;

    use super::*;

    type TestApiVer = StaticVersion<0, 1>;

    /// Fake sequencer serving just enough of `config/hotshot` for the relay's
    /// [`stake_table_tracker::StakeTableTracker`] to bootstrap a genesis stake table with a
    /// single validator. Heights up to 100 take the genesis path. Requests for a per-epoch stake
    /// table, which any higher height needs, never complete and notify the returned [`Notify`].
    async fn spawn_fake_sequencer(peer: PeerConfig<SeqTypes>) -> (Url, Arc<Notify>) {
        let config = serde_json::json!({
            "config": {
                "known_nodes_with_stake": [peer],
                "epoch_height": 10,
                "epoch_start_block": 100,
            }
        });
        let stake_table_requested = Arc::new(Notify::new());
        let notify = Arc::clone(&stake_table_requested);
        let app = Router::new()
            .route(
                "/config/hotshot",
                get(move || {
                    let config = config.clone();
                    async move { Json(config) }
                }),
            )
            .route(
                "/node/stake-table/{epoch}",
                get(move || {
                    notify.notify_one();
                    std::future::pending::<String>()
                }),
            );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        (
            format!("http://{addr}").parse().unwrap(),
            stake_table_requested,
        )
    }

    /// Spins the axum relay on an ephemeral port and returns its URL.
    async fn spawn_relay(state: StateRelayServerState) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router(state)).await.unwrap();
        });
        format!("http://{addr}").parse().unwrap()
    }

    /// Posts a signature the way `state_signature.rs` does (unversioned path, VBS-binary body),
    /// then fetches it back the way `hotshot-state-prover`'s v3 service does (unversioned path,
    /// default VBS-binary `Accept`).
    #[tokio::test]
    async fn post_and_fetch_lcv3_state_signature() {
        let validator = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
            [7u8; 32],
            0,
            U256::from(1),
            true,
        );
        let (sequencer_url, _) = spawn_fake_sequencer(validator.public_config()).await;
        let relay_url = spawn_relay(StateRelayServerState::new(sequencer_url)).await;
        let request_body = signed_lcv3_request(&validator, 1);

        let client = Client::<ClientErr, TestApiVer>::new(relay_url);
        client
            .post::<()>("api/state")
            .body_binary(&request_body)
            .unwrap()
            .send()
            .await
            .unwrap();

        let bundle = client
            .get::<LCV3StateSignaturesBundle>("api/lateststate")
            .send()
            .await
            .unwrap();
        assert_eq!(bundle.state, request_body.state);
        assert_eq!(bundle.signatures.len(), 1);
        assert!(bundle.signatures.contains_key(&validator.state_public_key));
    }

    #[tokio::test]
    async fn signature_lands_while_a_stake_table_fetch_hangs() {
        let validator = ValidatorConfig::<SeqTypes>::generated_from_seed_indexed(
            [7u8; 32],
            0,
            U256::from(1),
            true,
        );
        let (sequencer_url, stake_table_requested) =
            spawn_fake_sequencer(validator.public_config()).await;
        let relay_url = spawn_relay(StateRelayServerState::new(sequencer_url)).await;
        let client = Client::<ClientErr, TestApiVer>::new(relay_url);

        let far_future = signed_lcv3_request(&validator, 1_000_000);
        tokio::spawn({
            let client = client.clone();
            async move {
                _ = client
                    .post::<()>("api/state")
                    .body_binary(&far_future)
                    .unwrap()
                    .send()
                    .await;
            }
        });
        stake_table_requested.notified().await;

        let request_body = signed_lcv3_request(&validator, 1);
        let bundle = tokio::time::timeout(Duration::from_secs(5), async {
            client
                .post::<()>("api/state")
                .body_binary(&request_body)
                .unwrap()
                .send()
                .await
                .unwrap();
            client
                .get::<LCV3StateSignaturesBundle>("api/lateststate")
                .send()
                .await
                .unwrap()
        })
        .await
        .expect("relay blocked behind the hanging stake table fetch");
        assert_eq!(bundle.state, request_body.state);
    }

    fn signed_lcv3_request(
        validator: &ValidatorConfig<SeqTypes>,
        block_height: u64,
    ) -> LCV3StateSignatureRequestBody {
        let state = LightClientState {
            view_number: block_height,
            block_height,
            block_comm_root: Default::default(),
        };
        let next_stake = StakeTableState::default();
        let auth_root = FixedBytes::<32>::default();
        let digest = derive_signed_state_digest(&state, &next_stake, &auth_root);
        let signature = <SchnorrPubKey as LCV3StateSignatureKey>::sign_state(
            &validator.state_private_key,
            digest,
        )
        .unwrap();
        let v2_signature = <SchnorrPubKey as LCV2StateSignatureKey>::sign_state(
            &validator.state_private_key,
            &state,
            &next_stake,
        )
        .unwrap();
        LCV3StateSignatureRequestBody {
            key: validator.state_public_key.clone(),
            state,
            next_stake,
            auth_root,
            signature,
            v2_signature,
        }
    }

    /// Like tide-disco, every response carries permissive CORS headers. The light client prover
    /// reads `lateststate` over surf-disco, but the staking UI reads it from a browser.
    #[tokio::test]
    async fn responses_carry_cors_headers() {
        let relay_url = spawn_relay(StateRelayServerState::new(
            "http://127.0.0.1:1".parse().unwrap(),
        ))
        .await;
        for path in ["healthcheck", "api/lateststate", "no/such/route"] {
            let resp = reqwest::Client::new()
                .get(relay_url.join(path).unwrap())
                .header("Origin", "https://example.com")
                .send()
                .await
                .unwrap();
            assert_eq!(
                resp.headers()
                    .get("access-control-allow-origin")
                    .unwrap_or_else(|| panic!("no CORS header on {path}")),
                "*",
                "{path}"
            );
        }
    }
}
