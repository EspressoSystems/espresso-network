use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use tide_disco::{Error as _, RequestError, StatusCode};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Error {
    message: String,
    status: StatusCode,
}

impl Error {
    /// Extend an error message with additional context, keeping the same status.
    pub fn context(self, context: impl Display) -> Self {
        Self {
            message: format!("{context}: {}", self.message),
            status: self.status,
        }
    }

    /// An error indicating the service or some submodule is not yet ready to be used.
    pub fn not_initialized() -> Self {
        Self {
            message: "initializing, try again later".into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }

    /// Stock error message for when a requested object is not found and not known to have ever
    /// existed.
    ///
    /// This corresponds to a generic error message with status code 404. It is generally best
    /// practice to extend the error message with more specific information using
    /// [`context`](Self::context).
    ///
    /// This should be used for requests that cannot possibly be satisfied (e.g. asking for an
    /// unknown block hash, or a block number greater than the latest head). For requests that may
    /// have been satisfiable but for which the data has since been deleted from the server by
    /// garbage collection, prefer [`gone`](Self::gone).
    pub fn not_found() -> Self {
        Self {
            message: "not found".to_string(),
            status: StatusCode::NOT_FOUND,
        }
    }

    /// Stock error message for when a requested object may have existed but has been deleted.
    ///
    /// This corresponds to a generic error message with status code 410. It is generally best
    /// practice to extend the error message with more specific information using
    /// [`context`](Self::context).
    ///
    /// This should be used for requests for objects that once existed but are no longer stored on
    /// the server (e.g. requests for very old blocks). For requests for objects that may never have
    /// existed, prefer [`gone`](Self::not_found).
    pub fn gone() -> Self {
        Self {
            message: "permanently deleted".to_string(),
            status: StatusCode::GONE,
        }
    }

    /// An error internal to the service.
    ///
    /// This can either indicate an internal consistency error, or a transient failure of some inner
    /// component that is out of the client's control.
    pub fn internal() -> Self {
        Self {
            message: "internal error".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// An error arising from the client's request.
    pub fn bad_request() -> Self {
        Self {
            message: "bad request".to_string(),
            status: StatusCode::BAD_REQUEST,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.status, self.message)
    }
}

impl std::error::Error for Error {}

impl tide_disco::Error for Error {
    fn catch_all(status: StatusCode, message: String) -> Self {
        Self { message, status }
    }

    fn status(&self) -> StatusCode {
        self.status
    }
}

impl From<RequestError> for Error {
    fn from(err: RequestError) -> Self {
        Self::bad_request().context(err)
    }
}

impl From<anyhow::Error> for Error {
    fn from(err: anyhow::Error) -> Self {
        Self {
            message: format!("{err:#}"),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(err: sqlx::Error) -> Self {
        Self {
            message: format!("{err:#}"),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<hotshot_query_service::Error> for Error {
    fn from(err: hotshot_query_service::Error) -> Self {
        Self {
            status: err.status(),
            message: err.to_string(),
        }
    }
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Extension functions for converting other result types into [`Result`].
pub trait ResultExt {
    type Ok;

    /// Wrap an error with an HTTP status code, preserving the original error context.
    fn context(self, f: impl FnOnce() -> Error) -> Result<Self::Ok>;
}

impl<T, E> ResultExt for Result<T, E>
where
    E: std::error::Error,
{
    type Ok = T;

    fn context(self, f: impl FnOnce() -> Error) -> Result<<Self as ResultExt>::Ok> {
        self.map_err(|inner| {
            let mut err = f();
            err.message = format!("{}: {inner:#}", err.message);
            err
        })
    }
}

macro_rules! ensure {
    ($cond:expr, $err:expr) => {
        if !$cond {
            return Err($err);
        }
    };
}
pub(crate) use ensure;
