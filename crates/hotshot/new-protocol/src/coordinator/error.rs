use std::fmt;

use hotshot::traits::NetworkError;

use crate::{block::BlockError, network::is_critical, proposal::ValidationError};

#[derive(Debug, thiserror::Error)]
#[error("{severity} coordinator error ({context}): source: {source}")]
pub struct CoordinatorError {
    pub severity: Severity,
    pub source: ErrorSource,
    pub context: &'static str,
}

impl CoordinatorError {
    pub fn regular<E: Into<ErrorSource>>(e: E) -> Self {
        Self {
            context: "",
            severity: Severity::Regular,
            source: e.into(),
        }
    }

    pub fn critical<E: Into<ErrorSource>>(e: E) -> Self {
        Self {
            context: "",
            severity: Severity::Critical,
            source: e.into(),
        }
    }

    pub fn unspecified() -> Self {
        Self {
            context: "",
            severity: Severity::Unspecified,
            source: ErrorSource::Unspecified,
        }
    }

    pub fn context(mut self, m: &'static str) -> Self {
        self.context = m;
        self
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Severity {
    Unspecified,
    Regular,
    Critical,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unspecified => f.write_str("unspecified"),
            Self::Regular => f.write_str("regular"),
            Self::Critical => f.write_str("critical"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ErrorSource {
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    #[error("proposal validation error: {0}")]
    Proposal(#[from] ValidationError),

    #[error("unspecified error")]
    Unspecified,

    #[error("coordinator has no inputs")]
    NoInput,

    #[error("{0}")]
    StaticMessage(&'static str),

    #[error("{0}")]
    Message(String),

    #[error("block builder error: {0}")]
    Block(#[from] BlockError),
}

impl From<NetworkError> for CoordinatorError {
    fn from(e: NetworkError) -> Self {
        if is_critical(&e) {
            Self::critical(e)
        } else {
            Self::regular(e)
        }
    }
}

impl From<&'static str> for ErrorSource {
    fn from(msg: &'static str) -> Self {
        Self::StaticMessage(msg)
    }
}

impl From<String> for ErrorSource {
    fn from(msg: String) -> Self {
        Self::Message(msg)
    }
}
