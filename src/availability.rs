// Copyright (c) 2022 Espresso Systems (espressosys.com)
// This file is part of the HotShot Query Service library.
//
// This program is free software: you can redistribute it and/or modify it under the terms of the GNU
// General Public License as published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
// This program is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without
// even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
// You should have received a copy of the GNU General Public License along with this program. If not,
// see <https://www.gnu.org/licenses/>.

use clap::Args;
use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::path::PathBuf;
use tide_disco::{api::ApiError, method::ReadState, Api, RequestError, StatusCode};

#[derive(Args, Default)]
pub struct Options {
    #[arg(long = "availability-api-path", env = "HOTSHOT_AVAILABILITY_API_PATH")]
    pub api_path: Option<PathBuf>,
}

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
pub enum Error {
    Request { source: RequestError },
}

impl Error {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Request { .. } => StatusCode::BadRequest,
        }
    }
}

pub fn define_api<State>(options: &Options) -> Result<Api<State, Error>, ApiError>
where
    State: 'static + Send + Sync + ReadState,
{
    let mut api = match &options.api_path {
        Some(path) => Api::<State, Error>::from_file(path)?,
        None => {
            let toml = toml::from_str(include_str!("../api/availability.toml")).map_err(|err| {
                ApiError::CannotReadToml {
                    reason: err.to_string(),
                }
            })?;
            Api::<State, Error>::new(toml)?
        }
    };
    api.with_version(env!("CARGO_PKG_VERSION").parse().unwrap());
    Ok(api)
}
