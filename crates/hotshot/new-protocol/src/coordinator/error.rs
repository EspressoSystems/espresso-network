use std::fmt;

use hotshot::traits::NetworkError;

use crate::network::is_critical;

#[derive(Debug, thiserror::Error)]
#[error("{severity} coordinator error: {context}")]
pub struct CoordinatorError {
    pub severity: Severity,
    pub source: ErrorKind,
    pub context: &'static str,
}

impl CoordinatorError {
    pub fn regular<E: Into<ErrorKind>>(e: E) -> Self {
        Self {
            context: "",
            severity: Severity::Regular,
            source: e.into(),
        }
    }

    pub fn critical<E: Into<ErrorKind>>(e: E) -> Self {
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
            source: ErrorKind::Unspecified,
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
pub enum ErrorKind {
    #[error("network error: {0}")]
    Network(#[from] NetworkError),

    #[error("unspecified error")]
    Unspecified,

    #[error("coordinator has no inputs")]
    NoInput,

    #[error("{0}")]
    StaticMessage(&'static str),

    #[error("{0}")]
    Message(String),
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

impl From<&'static str> for ErrorKind {
    fn from(msg: &'static str) -> Self {
        Self::StaticMessage(msg)
    }
}

impl From<String> for ErrorKind {
    fn from(msg: String) -> Self {
        Self::Message(msg)
    }
}
