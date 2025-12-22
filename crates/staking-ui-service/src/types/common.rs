//! Primitive types.

use derivative::Derivative;
use espresso_types::PubKey;
use hotshot_types::traits::signature_key::SignatureKey;
use serde::{Deserialize, Serialize};
use surf_disco::Url;

pub use alloy::primitives::{Address, BlockHash, U256};
pub use tagged_base64::TaggedBase64;

/// An amount of Espresso tokens in WEI.
pub type ESPTokenAmount = U256;

/// A Unix timestamps in seconds since epoch.
pub type Timestamp = u64;

/// A ratio between 0 and 1.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Deserialize, Serialize)]
pub struct Ratio(f32);

impl Ratio {
    pub fn new(num: usize, den: usize) -> Self {
        Self((num as f32) / (den as f32))
    }
}

impl From<f32> for Ratio {
    fn from(value: f32) -> Self {
        Self(value)
    }
}

impl From<Ratio> for f32 {
    fn from(val: Ratio) -> Self {
        val.0
    }
}

/// An entry in the full node set.
#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct NodeSetEntry {
    /// Node's Ethereum address.
    pub address: Address,

    /// The key used for the node for signing consensus messages.
    pub staking_key: TaggedBase64,

    /// state verifying key
    pub state_key: TaggedBase64,

    /// Total stake currently attributed to the node.
    pub stake: ESPTokenAmount,

    /// How much commission the node charges.
    pub commission: Ratio,

    /// Optional metadata like a human-readable name and icon.
    ///
    /// May be [`None`] if no metadata URI is registered for this node.
    pub metadata: Option<NodeMetadata>,
}

/// Information about an L1 block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct L1BlockInfo {
    /// The block number
    pub number: u64,

    /// The hash of this block (useful for detecting reorgs)
    pub hash: BlockHash,

    /// The timestamp of this block.
    pub timestamp: Timestamp,
}

/// Minimal information needed to identify an L1 block and check for reorgs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct L1BlockId {
    /// The block number.
    pub number: u64,

    /// The hash of this block.
    pub hash: BlockHash,

    /// The parent of this block, used for reorg detection.
    pub parent: BlockHash,
}

/// Information about the exiting of a node from the node set.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeExit {
    /// The exiting node.
    pub address: Address,

    /// The timestamp for the exit escrow delay time.
    pub exit_time: Timestamp,
}

/// Information about the current "time" on the Espresso chain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize, Serialize)]
pub struct EpochAndBlock {
    /// The current epoch of the Espresso chain
    pub epoch: u64,

    /// The current block of the Espresso chain
    pub block: u64,

    /// The timestamp of the last block
    pub timestamp: Timestamp,
}

/// An entry in the active node set.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize, Serialize)]
pub struct ActiveNodeSetEntry {
    /// The node's address.
    pub address: Address,

    /// The number of times this node has voted in the current epoch.
    pub votes: u64,

    /// The number of times this node has been eligible to vote in the current epoch.
    pub eligible_votes: u64,

    /// The number of times this node has successfully proposed as leader in the current epoch.
    pub proposals: u64,

    /// The number of times this node has been eligible to propose as leader in the current epoch.
    pub slots: u64,
}

/// A single delegation from a particular user to a particular node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Delegation {
    /// The user delegating.
    pub delegator: Address,

    /// The node being delegated to.
    pub node: Address,

    /// Amount of stake delegated by this user to this node.
    pub amount: ESPTokenAmount,
}

/// A withdrawal of stake that is waiting to be claimed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct PendingWithdrawal {
    /// The owner of the pending stake.
    pub delegator: Address,

    /// The node which was previously delegated to, which stake is now being withdrawn.
    pub node: Address,

    /// The amount of stake pending withdrawal.
    pub amount: ESPTokenAmount,

    /// The timestamp recorded for the exit escrow time.
    ///
    /// Any attempts to withdrawal before this will fail.
    pub available_time: Timestamp,
}

/// A completed withdrawal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Withdrawal {
    /// The owner of the withdrawn stake.
    pub delegator: Address,

    /// The node which was previously delegated to, which stake is now withdrawn.
    pub node: Address,

    /// The amount of stake.
    pub amount: ESPTokenAmount,
}

/// Optional descriptive information about a node, fetched from a third-party URI.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct NodeMetadata {
    /// The URI this metadata is fetched from.
    ///
    /// This URI is registered alongside the node in the staking contraact.
    pub uri: Url,

    /// The content of the metadata.
    ///
    /// This content is fetched from a third-party URI, and thus should not be considered trusted,
    /// reliable, or deterministic. It is informational only.
    ///
    /// May be [`None`] if no (valid) content is available at the published `uri`.
    pub content: Option<NodeMetadataContent>,
}

/// Optional descriptive information about a node.
#[derive(Clone, Debug, Derivative, PartialEq, Eq, Deserialize, Serialize)]
#[derivative(Default)]
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
    #[derivative(Default(value = "PubKey::generated_from_seed_indexed(Default::default(), 0).0"))]
    pub pub_key: PubKey,

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
