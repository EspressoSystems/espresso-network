//! Metadata types for validator node information.
//!
//! These types are copied from staking-ui-service and should be replaced with
//! imports from that crate once version compatibility is resolved.
//!
//! Source: https://github.com/EspressoSystems/staking-ui-service/blob/main/src/types/common.rs

use hotshot_types::signature_key::BLSPubKey;
use serde::{Deserialize, Serialize};
use url::Url;

/// Optional descriptive information about a node.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeMetadataContent {
    /// The public key of the node this metadata belongs to.
    ///
    /// This is the only required field of the [`NodeMetadataContent`]. It is included in the
    /// metadata content for authentication purposes. If this does not match the public key of the
    /// node whose metadata is being fetched, then the metadata is treated as invalid. This feature
    /// applies in two scenarios:
    ///
    /// 1. The operator of the node has innocently but erroneously pointed the node's metadata URI
    ///    to the metadata page for a different node (this is an easy mistake to make when running
    ///    multiple nodes). In this case we will detect the error and display no metadata for the
    ///    misconfigured node, which is better for users than displaying incorrect metadata, and is
    ///    a clear sign to the operator that something is wrong.
    ///
    /// 2. A malicious operator attempts to impersonate a trusted party by setting the metadata URI
    ///    for the malicious node to the metadata URI of some existing trusted node (e.g.
    ///    `https://trusted-operator.com/metadata`). Users of the UI see that the malicious node is
    ///    associated with a `trusted-operator.com` domain name and thus believe it to be more
    ///    trustworthy than it perhaps is. We would detect this, since the malicious operator and
    ///    the trusted operator must have nodes with different public keys, and we would display
    ///    no metadata for the malicious operator.
    ///
    /// Note that the mere presence of a matching public key in a metadata dump does not in itself
    /// guarantee that this metadata was intended for this node. The metadata must also have been
    /// sourced from the URI that was registered for that node in the contract. Specifically:
    /// * A metadata dump having the expected public key ensures that the operator of the web site
    ///   which served the metadata intended it for that particular node.
    /// * A node having a certain metadata URI in the contract ensures that the operator of the
    ///   _node_ intended its metadata to be sourced from that particular web site.
    pub pub_key: BLSPubKey,

    /// Human-readable name for the node.
    pub name: Option<String>,

    /// Longer description of the node.
    pub description: Option<String>,

    /// Company or individual operating the node.
    pub company_name: Option<String>,

    /// Website for `company_name`.
    pub company_website: Option<Url>,

    /// Consensus client the node is running.
    pub client_version: Option<String>,

    /// Icon for the node (at different resolutions and pixel aspect ratios).
    pub icon: Option<ImageSet>,
}

/// Different versions of the same image, at different resolutions and pixel aspect ratios.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct ImageSet {
    /// 14x14 icons at different pixel ratios.
    #[serde(rename = "14x14")]
    pub small: RatioSet,

    /// 24x24 icons at different pixel ratios.
    #[serde(rename = "24x24")]
    pub large: RatioSet,
}

/// Different versions of the same image, at different pixel aspect ratios.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct RatioSet {
    /// Image source for 1:1 pixel aspect ratio
    #[serde(rename = "@1x")]
    pub ratio1: Option<Url>,

    /// Image source for 2:1 pixel aspect ratio
    #[serde(rename = "@2x")]
    pub ratio2: Option<Url>,

    /// Image source for 3:1 pixel aspect ratio
    #[serde(rename = "@3x")]
    pub ratio3: Option<Url>,
}
