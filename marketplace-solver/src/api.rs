use std::{
    fs,
    path::{Path, PathBuf},
};

use espresso_types::{
    v0_99::{BidTx, RollupRegistration, RollupUpdate},
    NamespaceId,
};
use futures::FutureExt;
use hotshot_types::{data::ViewNumber, traits::node_implementation::ConsensusTime};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tide_disco::{
    api::ApiError,
    method::{ReadState, WriteState},
    Api, RequestError, StatusCode,
};
use toml::{map::Entry, Value};
use vbs::version::StaticVersionType;

use crate::state::UpdateSolverState;

#[derive(Debug, Error, Serialize, Deserialize)]
pub enum SolverError {
    #[error("rollup already exists: {0}")]
    RollupAlreadyExists(NamespaceId),
    #[error("Invalid signature: {0}")]
    InvalidSignature(String),
    #[error("Signature key is not from the keys provided: {0}")]
    SignatureKeysMismatch(String),
    #[error("Signature key {0} does not match signatures in the database")]
    SignatureDatabaseKeysMismatch(String),
    #[error("bincode err: {0}")]
    BincodeError(String),
    #[error("database err: {0}")]
    Database(String),
    #[error("serde json err: {0}")]
    SerdeJsonError(String),
    #[error("request error: {0}")]
    Request(#[from] RequestError),
    #[error("err {status:?} : {message:?}")]
    Custom { status: StatusCode, message: String },
}

pub(crate) fn overflow_err(err: std::num::TryFromIntError) -> SolverError {
    SolverError::Custom {
        status: StatusCode::BAD_REQUEST,
        message: format!("overflow {err}"),
    }
}

impl From<Box<bincode::ErrorKind>> for SolverError {
    fn from(err: Box<bincode::ErrorKind>) -> Self {
        Self::BincodeError(err.to_string())
    }
}

impl tide_disco::Error for SolverError {
    fn catch_all(status: StatusCode, message: String) -> Self {
        Self::Custom { status, message }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Custom { status, .. } => *status,
            _ => StatusCode::BAD_REQUEST,
        }
    }
}

#[derive(Default)]
pub struct ApiOptions {
    pub api_path: Option<PathBuf>,

    /// Additional API specification files to merge with `solver-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic solver API.
    pub extensions: Vec<toml::Value>,
}

pub fn define_api<State, VERSION>(
    options: ApiOptions,
) -> Result<Api<State, SolverError, VERSION>, ApiError>
where
    VERSION: StaticVersionType + 'static,
    State: 'static + Send + Sync + ReadState + WriteState,
    <State as ReadState>::State: Send + Sync + UpdateSolverState,
{
    let mut api = load_api::<State, SolverError, VERSION>(
        options.api_path.as_ref(),
        include_str!("../api/solver.toml"),
        options.extensions.clone(),
    )?;

    api.post("submit_bid", |req, state| {
        async move {
            let bid = req.body_json::<BidTx>()?;
            state.submit_bid_tx(bid).await
        }
        .boxed()
    })?
    .get("auction_results", |req, state| {
        async move {
            let view_num: u64 = req.integer_param("view_number")?;
            state
                .calculate_auction_results_permissionless(ViewNumber::new(view_num))
                .await
        }
        .boxed()
    })?
    .get("auction_results_permissioned", |req, state| {
        async move {
            let view_num: u64 = req.integer_param("view_number")?;
            let signature = req.blob_param("signature")?;
            state
                .calculate_auction_results_permissioned(ViewNumber::new(view_num), signature)
                .await
        }
        .boxed()
    })?
    .post("register_rollup", |req, state| {
        async move {
            let body = req.body_json::<RollupRegistration>()?;
            state.register_rollup(body).await
        }
        .boxed()
    })?
    .post("update_rollup", |req, state| {
        async move {
            let body = req.body_json::<RollupUpdate>()?;
            state.update_rollup_registration(body).await
        }
        .boxed()
    })?
    .get("rollup_registrations", |_req, state| {
        async move { state.get_all_rollup_registrations().await }.boxed()
    })?;
    Ok(api)
}

pub(crate) fn load_api<State: 'static, Error: 'static, ApiVer: StaticVersionType + 'static>(
    path: Option<impl AsRef<Path>>,
    default: &str,
    extensions: impl IntoIterator<Item = Value>,
) -> Result<Api<State, Error, ApiVer>, ApiError> {
    let mut toml = match path {
        Some(path) => load_toml(path.as_ref())?,
        None => toml::from_str(default).map_err(|err| ApiError::CannotReadToml {
            reason: err.to_string(),
        })?,
    };
    for extension in extensions {
        merge_toml(&mut toml, extension);
    }
    Api::new(toml)
}

fn merge_toml(into: &mut Value, from: Value) {
    if let (Value::Table(into), Value::Table(from)) = (into, from) {
        for (key, value) in from {
            match into.entry(key) {
                Entry::Occupied(mut entry) => merge_toml(entry.get_mut(), value),
                Entry::Vacant(entry) => {
                    entry.insert(value);
                },
            }
        }
    }
}

fn load_toml(path: &Path) -> Result<Value, ApiError> {
    let bytes = fs::read(path).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })?;
    let string = std::str::from_utf8(&bytes).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })?;
    toml::from_str(string).map_err(|err| ApiError::CannotReadToml {
        reason: err.to_string(),
    })
}
