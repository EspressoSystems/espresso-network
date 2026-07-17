//! State certificate query data type

use derive_more::From;
use hotshot_types::{
    simple_certificate::LightClientStateUpdateCertificateV2, traits::node_implementation::NodeType,
};
use serde::{Deserialize, Serialize};

/// A wrapper around `LightClientStateUpdateCertificateV2`.
///
/// The V2 certificate includes additional fields compared to earlier versions:
/// - Light client v3 signatures
/// - `auth_root` — used by the reward claim contract to verify that its
///   calculated `auth_root` matches the one in the Light Client contract.
///
/// This struct is returned by the `state-cert-v2` API endpoint.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, From)]
#[serde(bound = "")]
pub struct StateCertQueryDataV2<Types: NodeType>(pub LightClientStateUpdateCertificateV2<Types>);

impl<Types> From<StateCertQueryDataV2<Types>> for crate::v0_3::StateCertQueryDataV1<Types>
where
    Types: NodeType,
{
    fn from(cert: StateCertQueryDataV2<Types>) -> Self {
        Self(cert.0.into())
    }
}
