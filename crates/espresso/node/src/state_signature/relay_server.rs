use std::{fmt, sync::Arc};

use async_lock::RwLock;
use axum::{
    Json, Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use hotshot_types::{
    light_client::{
        LCV1StateSignatureRequestBody, LCV1StateSignaturesBundle, LCV2StateSignatureRequestBody,
        LCV2StateSignaturesBundle, LCV3StateSignatureRequestBody, LCV3StateSignaturesBundle,
    },
    traits::signature_key::LCV1StateSignatureKey,
};
use lcv1_relay::{LCV1StateRelayServerDataSource, LCV1StateRelayServerState};
use lcv2_relay::{LCV2StateRelayServerDataSource, LCV2StateRelayServerState};
use lcv3_relay::{LCV3StateRelayServerDataSource, LCV3StateRelayServerState};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use tokio::{net::TcpListener, sync::oneshot};
use url::Url;
use vbs::{
    BinarySerializer, Serializer,
    version::{StaticVersion, StaticVersionType},
};

pub mod lcv1_relay;
pub mod lcv2_relay;
pub mod lcv3_relay;
pub mod stake_table_tracker;

/// Binary framing version used by `state_signature.rs` and `hotshot-state-prover`, whose
/// surf-disco clients default to `Accept`/`Content-Type: application/octet-stream`.
type WireVersion = StaticVersion<0, 1>;

/// Wire-compatible error envelope: mirrors `tide_disco::error::ServerError`'s `{status, message}`
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

fn wants_binary(headers: &HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("application/octet-stream"))
}

/// Encode a successful response body, negotiating VBS binary vs JSON from the `Accept` header,
/// matching tide-disco's content negotiation for the real (default-binary) surf-disco clients.
fn encode_ok<T: Serialize>(headers: &HeaderMap, value: T) -> Response {
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&value) {
            Ok(bytes) => {
                ([(header::CONTENT_TYPE, "application/octet-stream")], bytes).into_response()
            },
            Err(err) => encode_err(
                headers,
                RelayError::catch_all(StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
            ),
        }
    } else {
        Json(value).into_response()
    }
}

/// Encode an error response using the same content negotiation as [`encode_ok`].
fn encode_err(headers: &HeaderMap, err: RelayError) -> Response {
    let status = StatusCode::from_u16(err.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    if wants_binary(headers) {
        match Serializer::<WireVersion>::serialize(&err) {
            Ok(bytes) => (
                status,
                [(header::CONTENT_TYPE, "application/octet-stream")],
                bytes,
            )
                .into_response(),
            Err(_) => (status, Json(err)).into_response(),
        }
    } else {
        (status, Json(err)).into_response()
    }
}

/// Decode a request body, matching tide-disco's `body_auto` (VBS for
/// `application/octet-stream`, JSON for `application/json`).
fn decode_body<T: DeserializeOwned>(headers: &HeaderMap, body: &[u8]) -> Result<T, RelayError> {
    let content_type = headers
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if content_type.starts_with("application/octet-stream") {
        Serializer::<WireVersion>::deserialize(body).map_err(|e| {
            RelayError::catch_all(StatusCode::BAD_REQUEST, format!("invalid binary body: {e}"))
        })
    } else if content_type.starts_with("application/json") {
        serde_json::from_slice(body).map_err(|e| {
            RelayError::catch_all(StatusCode::BAD_REQUEST, format!("invalid json body: {e}"))
        })
    } else {
        Err(RelayError::catch_all(
            StatusCode::BAD_REQUEST,
            "missing or unsupported Content-Type",
        ))
    }
}

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
    fn get_latest_signature_bundle(&self) -> Result<LCV1StateSignaturesBundle, RelayError> {
        self.lcv1_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV1StateSignatureRequestBody,
    ) -> Result<(), RelayError> {
        self.lcv1_state.post_signature(req).await
    }
}

#[async_trait::async_trait]
impl LCV2StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV2StateSignaturesBundle, RelayError> {
        self.lcv2_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV2StateSignatureRequestBody,
    ) -> Result<(), RelayError> {
        self.lcv2_state.post_signature(req).await
    }
}

#[async_trait::async_trait]
impl LCV3StateRelayServerDataSource for StateRelayServerState {
    fn get_latest_signature_bundle(&self) -> Result<LCV3StateSignaturesBundle, RelayError> {
        self.lcv3_state.get_latest_signature_bundle()
    }

    async fn post_signature(
        &mut self,
        req: LCV3StateSignatureRequestBody,
    ) -> Result<(), RelayError> {
        self.lcv3_state.post_signature(req).await
    }
}

/// Shared, lock-guarded server state, cloned into every axum handler via `State`.
type SharedState = Arc<RwLock<StateRelayServerState>>;

/// Handle a `POST` to `state`/`api/state`: tries LCV3, then LCV2 (auto-downgrading to LCV1 if the
/// signature verifies against the legacy scheme), then LCV1. Mirrors tide's `poststatesignature`.
async fn post_state_signature(
    state: &SharedState,
    headers: &HeaderMap,
    body: &[u8],
) -> Result<(), RelayError> {
    if let Ok(req) = decode_body::<LCV3StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV3 state signature: {req}");
        let mut state = state.write().await;
        if let Err(e) =
            LCV2StateRelayServerDataSource::post_signature(&mut *state, req.clone().into()).await
        {
            tracing::error!("Failed to post downgraded LCV2 state signature: {}", e);
        }
        LCV3StateRelayServerDataSource::post_signature(&mut *state, req).await
    } else if let Ok(req) = decode_body::<LCV2StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV2 state signature: {req}");
        let mut state = state.write().await;
        if LCV1StateSignatureKey::verify_state_sig(&req.key, &req.signature, &req.state) {
            LCV1StateRelayServerDataSource::post_signature(&mut *state, req.into()).await
        } else {
            LCV2StateRelayServerDataSource::post_signature(&mut *state, req).await
        }
    } else if let Ok(req) = decode_body::<LCV1StateSignatureRequestBody>(headers, body) {
        tracing::debug!("Received LCV1 state signature: {req}");
        let mut state = state.write().await;
        LCV1StateRelayServerDataSource::post_signature(&mut *state, req).await
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
    let mut state = state.write().await;
    LCV1StateRelayServerDataSource::post_signature(&mut *state, req).await
}

/// `GET state` (deprecated): mirrors tide's `getlateststate`, always the LCV2 bundle regardless
/// of version, since all three registered API versions shared this handler.
async fn get_latest_state(state: &SharedState) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let state = state.read().await;
    LCV2StateRelayServerDataSource::get_latest_signature_bundle(&*state)
}

/// `GET legacy-state`: mirrors tide's `getlatestlegacystate`.
async fn get_latest_legacy_state(
    state: &SharedState,
) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let state = state.read().await;
    LCV1StateRelayServerDataSource::get_latest_signature_bundle(&*state)
        .map(LCV2StateSignaturesBundle::from_v1)
}

async fn get_latest_state_v1(state: &SharedState) -> Result<LCV1StateSignaturesBundle, RelayError> {
    let state = state.read().await;
    LCV1StateRelayServerDataSource::get_latest_signature_bundle(&*state)
}

async fn get_latest_state_v2(state: &SharedState) -> Result<LCV2StateSignaturesBundle, RelayError> {
    let state = state.read().await;
    LCV2StateRelayServerDataSource::get_latest_signature_bundle(&*state)
}

async fn get_latest_state_v3(state: &SharedState) -> Result<LCV3StateSignaturesBundle, RelayError> {
    let state = state.read().await;
    LCV3StateRelayServerDataSource::get_latest_signature_bundle(&*state)
}

async fn healthcheck() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "Available" }))
}

async fn post_state(State(state): State<SharedState>, headers: HeaderMap, body: Bytes) -> Response {
    match post_state_signature(&state, &headers, &body).await {
        Ok(()) => encode_ok(&headers, ()),
        Err(e) => encode_err(&headers, e),
    }
}

async fn get_state(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    match get_latest_state(&state).await {
        Ok(bundle) => encode_ok(&headers, bundle),
        Err(e) => encode_err(&headers, e),
    }
}

async fn post_legacy_state(
    State(state): State<SharedState>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    match post_legacy_state_signature(&state, &headers, &body).await {
        Ok(()) => encode_ok(&headers, ()),
        Err(e) => encode_err(&headers, e),
    }
}

async fn get_legacy_state(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    match get_latest_legacy_state(&state).await {
        Ok(bundle) => encode_ok(&headers, bundle),
        Err(e) => encode_err(&headers, e),
    }
}

async fn get_lateststate_v1(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    match get_latest_state_v1(&state).await {
        Ok(bundle) => encode_ok(&headers, bundle),
        Err(e) => encode_err(&headers, e),
    }
}

async fn get_lateststate_v2(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    match get_latest_state_v2(&state).await {
        Ok(bundle) => encode_ok(&headers, bundle),
        Err(e) => encode_err(&headers, e),
    }
}

async fn get_lateststate_v3(State(state): State<SharedState>, headers: HeaderMap) -> Response {
    match get_latest_state_v3(&state).await {
        Ok(bundle) => encode_ok(&headers, bundle),
        Err(e) => encode_err(&headers, e),
    }
}

const STATE_PATH: &str = "/api/state";
const LEGACY_STATE_PATH: &str = "/api/legacy-state";
const LATEST_STATE_PATH: &str = "/api/lateststate";

/// Build the relay server router. Tide-disco registered `api` under three stacked major API
/// versions (v1, v2, v3) with identical handlers for every route except `lateststate`; requests
/// with no version prefix were redirected to the latest (v3). We reproduce that by serving the
/// same handlers at the unversioned and all three `/v{1,2,3}` paths, and only special-casing
/// `lateststate` per version.
fn router(state: SharedState) -> Router {
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
    router.with_state(state)
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
    axum::serve(listener, router(Arc::new(RwLock::new(state)))).await?;
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
    use alloy::primitives::{FixedBytes, U256};
    use espresso_types::SeqTypes;
    use hotshot::types::SchnorrPubKey;
    use hotshot_contract_adapter::light_client::derive_signed_state_digest;
    use hotshot_types::{
        PeerConfig, ValidatorConfig,
        light_client::{LightClientState, StakeTableState},
        traits::signature_key::{LCV2StateSignatureKey, LCV3StateSignatureKey},
    };
    use surf_disco::Client;
    use tide_disco::error::ServerError;
    use vbs::version::StaticVersion;

    use super::*;

    type TestApiVer = StaticVersion<0, 1>;

    /// Fake sequencer serving just enough of `config/hotshot` for the relay's
    /// [`stake_table_tracker::StakeTableTracker`] to bootstrap a genesis stake table with a
    /// single validator, with `epoch_height: 0` so every lookup takes the genesis path.
    async fn spawn_fake_sequencer(peer: PeerConfig<SeqTypes>) -> Url {
        let config = serde_json::json!({
            "config": {
                "known_nodes_with_stake": [peer],
                "epoch_height": 0,
                "epoch_start_block": 0,
            }
        });
        let app = Router::new().route(
            "/config/hotshot",
            get(move || {
                let config = config.clone();
                async move { Json(config) }
            }),
        );
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });
        format!("http://{addr}").parse().unwrap()
    }

    /// Spins the axum relay on an ephemeral port and returns its URL.
    async fn spawn_relay(state: StateRelayServerState) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, router(Arc::new(RwLock::new(state))))
                .await
                .unwrap();
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
        let sequencer_url = spawn_fake_sequencer(validator.public_config()).await;
        let relay_url = spawn_relay(StateRelayServerState::new(sequencer_url)).await;

        let light_client_state = LightClientState {
            view_number: 1,
            block_height: 1,
            block_comm_root: Default::default(),
        };
        let next_stake = StakeTableState::default();
        let auth_root = FixedBytes::<32>::default();
        let digest = derive_signed_state_digest(&light_client_state, &next_stake, &auth_root);
        let signature = <SchnorrPubKey as LCV3StateSignatureKey>::sign_state(
            &validator.state_private_key,
            digest,
        )
        .unwrap();
        let v2_signature = <SchnorrPubKey as LCV2StateSignatureKey>::sign_state(
            &validator.state_private_key,
            &light_client_state,
            &next_stake,
        )
        .unwrap();
        let request_body = LCV3StateSignatureRequestBody {
            key: validator.state_public_key.clone(),
            state: light_client_state,
            next_stake,
            auth_root,
            signature,
            v2_signature,
        };

        let client = Client::<ServerError, TestApiVer>::new(relay_url);
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
        assert_eq!(bundle.state, light_client_state);
        assert_eq!(bundle.signatures.len(), 1);
        assert!(bundle.signatures.contains_key(&validator.state_public_key));
    }
}
