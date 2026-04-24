use std::time::Duration;

use minicbor::{Decode, Encode};

#[derive(Debug, Encode, Decode)]
pub struct Hello {
    #[cbor(n(0))]
    pub(crate) status: Option<Status>,
}

#[derive(Debug, Encode, Decode)]
pub enum Status {
    #[cbor(n(0))]
    Ok,
    #[cbor(n(1))]
    BackOff {
        #[cbor(n(0))]
        seconds: u64,
    },
}

impl Hello {
    pub fn ok() -> Self {
        Self {
            status: Some(Status::Ok),
        }
    }

    pub fn backoff(d: Duration) -> Self {
        Self {
            status: Some(Status::BackOff {
                seconds: d.as_secs(),
            }),
        }
    }

    pub fn is_ok(&self) -> bool {
        matches!(self.status, Some(Status::Ok))
    }

    pub fn backoff_duration(&self) -> Option<Duration> {
        if let Some(Status::BackOff { seconds }) = &self.status {
            Some(Duration::from_secs(*seconds))
        } else {
            None
        }
    }
}
