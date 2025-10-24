//! State certificate query data type

use derive_more::From;
use hotshot_types::{
    simple_certificate::LightClientStateUpdateCertificateV1,
    traits::node_implementation::NodeType,
};
use serde::{Deserialize, Serialize};

/// A wrapper around `LightClientStateUpdateCertificateV1`.
///
/// This struct is returned by the `state-cert` API endpoint for backward compatibility.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, From)]
#[serde(bound = "")]
pub struct StateCertQueryDataV1<Types: NodeType>(pub LightClientStateUpdateCertificateV1<Types>);
