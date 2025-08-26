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

use std::fmt::Display;

use derive_more::From;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tide_disco::{Error as TideError, StatusCode};

use crate::{availability, explorer, merklized_state, node, status};

#[derive(Clone, Debug, From, Snafu, Deserialize, Serialize)]
pub enum Error {
    #[snafu(display("{source}"))]
    Availability { source: availability::Error },
    #[snafu(display("{source}"))]
    Node { source: node::Error },
    #[snafu(display("{source}"))]
    Status { source: status::Error },
    #[snafu(display("{source}"))]
    MerklizedState { source: merklized_state::Error },
    #[snafu(display("{source}"))]
    Explorer {
        #[serde(rename = "error")]
        source: explorer::Error,
    },
    #[snafu(display("error {status}: {message}"))]
    Custom { message: String, status: StatusCode },
}

impl Error {
    pub fn internal<M: Display>(message: M) -> Self {
        Self::Custom {
            message: message.to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl tide_disco::Error for Error {
    fn catch_all(status: StatusCode, message: String) -> Self {
        Self::Custom { status, message }
    }

    fn status(&self) -> StatusCode {
        match self {
            Self::Availability { source } => source.status(),
            Self::Node { source } => source.status(),
            Self::Status { source } => source.status(),
            Self::MerklizedState { source } => source.status(),
            Self::Explorer { source } => source.status(),
            Self::Custom { status, .. } => *status,
        }
    }
}

/// Here we converge the events service error type into the `tide-disco` error type
impl From<hotshot_events_service::events::Error> for Error {
    fn from(err: hotshot_events_service::events::Error) -> Self {
        Self::Custom {
            message: err.to_string(),
            status: err.status(),
        }
    }
}
