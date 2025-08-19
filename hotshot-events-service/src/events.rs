use std::path::PathBuf;

use clap::Args;
use derive_more::From;
use futures::{FutureExt, StreamExt, TryFutureExt};
use hotshot_types::traits::node_implementation::NodeType;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tide_disco::{method::ReadState, Api, RequestError, StatusCode};
use vbs::version::StaticVersionType;

use crate::{api::load_api, events_source::EventsSource};

#[derive(Args, Default, Debug)]
pub struct Options {
    #[arg(
        long = "hotshot-events-service-api-path",
        env = "HOTSHOT_EVENTS_SERVICE_API_PATH"
    )]
    pub api_path: Option<PathBuf>,

    /// Additional API specification files to merge with `hotshot-events-service-api-path`.
    ///
    /// These optional files may contain route definitions for application-specific routes that have
    /// been added as extensions to the basic hotshot-events-service API.
    #[arg(
        long = "hotshot-events-extension",
        env = "HOTSHOT_EVENTS_SERVICE_EXTENSIONS",
        value_delimiter = ','
    )]
    pub extensions: Vec<toml::Value>,
}

#[derive(Clone, Debug, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum EventError {
    /// The requested resource does not exist or is not known to this hotshot node.
    NotFound,
    /// The requested resource exists but is not currently available.
    Missing,
    /// There was an error while trying to fetch the requested resource.
    #[snafu(display("Failed to fetch requested resource: {message}"))]
    Error { message: String },
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
#[snafu(visibility(pub))]
pub enum Error {
    Request {
        source: RequestError,
    },
    #[snafu(display("error receiving events {resource}: {source}"))]
    #[from(ignore)]
    EventAvailable {
        source: EventError,
        resource: String,
    },
    Custom {
        message: String,
        status: StatusCode,
    },
}

impl tide_disco::error::Error for Error {
    fn catch_all(status: StatusCode, msg: String) -> Self {
        Error::Custom {
            message: msg,
            status,
        }
    }
    fn status(&self) -> StatusCode {
        match self {
            Error::Request { .. } => StatusCode::BAD_REQUEST,
            Error::EventAvailable { source, .. } => match source {
                EventError::NotFound => StatusCode::NOT_FOUND,
                EventError::Missing => StatusCode::NOT_FOUND,
                EventError::Error { .. } => StatusCode::INTERNAL_SERVER_ERROR,
            },
            Error::Custom { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub fn define_api<State, Types, Ver>(
    options: &Options,
    api_ver: semver::Version,
) -> anyhow::Result<Api<State, Error, Ver>>
where
    State: 'static + Send + Sync + ReadState,
    <State as ReadState>::State: Send + Sync + EventsSource<Types>,
    Types: NodeType,
    Ver: StaticVersionType + 'static,
{
    let mut api = load_api::<State, Error, Ver>(
        options.api_path.as_ref(),
        include_str!("../api/hotshot_events.toml"),
        options.extensions.clone(),
    )?;

    api.with_version(api_ver.clone());

    if api_ver.major == 0 {
        api.stream("events", move |_, state| {
            async move {
                tracing::info!("client subscribed to legacy events");
                state
                    .read(|state| {
                        async move {
                            match state.get_legacy_event_stream(None).await {
                                Ok(stream) => Ok(stream.map(Ok)),
                                Err(e) => Err(Error::Custom {
                                    message: e.to_string(),
                                    status: StatusCode::INTERNAL_SERVER_ERROR,
                                }),
                            }
                        }
                        .boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    } else {
        api.stream("events", move |_, state| {
            async move {
                tracing::info!("client subscribed to events");
                state
                    .read(|state| {
                        async move {
                            match state.get_event_stream(None).await {
                                Ok(stream) => Ok(stream.map(Ok)),
                                Err(e) => Err(Error::Custom {
                                    message: e.to_string(),
                                    status: StatusCode::INTERNAL_SERVER_ERROR,
                                }),
                            }
                        }
                        .boxed()
                    })
                    .await
            }
            .try_flatten_stream()
            .boxed()
        })?;
    }

    api.get("startup_info", |_, state| {
        async move {
            state.get_startup_info().await.map_err(|e| Error::Custom {
                message: e.to_string(),
                status: StatusCode::INTERNAL_SERVER_ERROR,
            })
        }
        .boxed()
    })?;

    Ok(api)
}
