use reqwest::StatusCode;
use serde::{Deserialize, Serialize};

/// A response to a healthcheck endpoint.
///
/// The global healthcheck endpoint of an application is considered healthy only if every module
/// healthcheck it aggregates reports [`StatusCode::OK`].
pub trait HealthCheck: Serialize {
    fn status(&self) -> StatusCode;
}

/// Common health statuses of an application.
///
/// Wire-compatible with `tide_disco::healthcheck::HealthStatus` 0.9.6: the `Unavailabale`
/// misspelling and the exact variant order are load-bearing. JSON uses the variant names and
/// vbs/bincode encodes the declaration-order ordinal, so neither may change.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    #[default]
    Available,
    Initializing,
    Unavailabale,
    TemporarilyUnavailable,
    Unhealthy,
    ShuttingDown,
}

impl HealthCheck for HealthStatus {
    fn status(&self) -> StatusCode {
        match self {
            // Healthy in normal states even when not `Available`, so that health monitors don't
            // kill the service while it is starting up or gracefully shutting down.
            Self::Available | Self::Initializing | Self::ShuttingDown => StatusCode::OK,
            _ => StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

#[cfg(test)]
mod test {
    use vbs::{BinarySerializer, Serializer, version::StaticVersion};

    use super::*;

    type Ver01 = StaticVersion<0, 1>;

    #[test]
    fn vbs_ordinals_match_tide_disco_0_9_6_variant_order() {
        // Regression test: bincode encodes the enum tag as the declaration-order ordinal
        // (u32 LE), so reordering variants silently breaks decoding of binary-content-type
        // health responses from tide-disco servers.
        for (status, ordinal) in [
            (HealthStatus::Available, 0u32),
            (HealthStatus::Initializing, 1),
            (HealthStatus::Unavailabale, 2),
            (HealthStatus::TemporarilyUnavailable, 3),
            (HealthStatus::Unhealthy, 4),
            (HealthStatus::ShuttingDown, 5),
        ] {
            let bytes = Serializer::<Ver01>::serialize(&status).unwrap();
            let mut expected = vec![0, 0, 1, 0];
            expected.extend_from_slice(&ordinal.to_le_bytes());
            assert_eq!(bytes, expected, "{status:?}");
            assert_eq!(
                Serializer::<Ver01>::deserialize::<HealthStatus>(&bytes).unwrap(),
                status
            );
        }
    }
}
