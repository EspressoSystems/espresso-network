//! The cases the generator writes, one JSON object per line.
//!
//! The Lean checker (`lean/stake-table-diff`) reads them, runs its implementation on
//! each case's input and compares with the node's result recorded here. Byte strings
//! (keys, addresses, a DRB result) are `0x`-prefixed hex, amounts and hashes decimal
//! strings, and events carry their keys as the node decoded and checked them.

use serde::Serialize;

/// A generated case: an input and what the node made of it.
#[derive(Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Case {
    /// Events replayed from the empty state: the outcome of each, the state if the
    /// replay finished, and the active validators selected from it.
    Replay {
        name: String,
        events: Vec<Event>,
        outcomes: Vec<String>,
        state: Option<State>,
        selections: Vec<Selection>,
    },
    /// The block reward for a supply, a stake and an average block time, if any.
    Reward {
        supply: String,
        stake: String,
        avg_ms: u64,
        reward: Option<String>,
    },
    /// Paying a validator a reward: what each account is credited, or none on an error.
    Payout {
        account: String,
        commission: u16,
        stake: String,
        delegators: Vec<(String, String)>,
        reward: String,
        credits: Option<Vec<(String, String)>>,
    },
    /// Log keys (block, index), and the positions of the logs in L1 order.
    Order {
        keys: Vec<(u64, u64)>,
        order: Vec<usize>,
    },
    /// A block range split into chunks of at most `size` blocks.
    Chunks {
        first: u64,
        last: u64,
        size: u64,
        chunks: Vec<(u64, u64)>,
    },
    /// A committee's schedule: its entries (key, stake), their positions in schedule
    /// order, and for each view the hash that draws its breakpoint and the position of
    /// its leader.
    Schedule {
        drb: String,
        entries: Vec<(String, String)>,
        order: Vec<usize>,
        hashes: Vec<String>,
        leaders: Vec<usize>,
    },
}

/// The active validators, by account, or none if there is no candidate.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    /// The network addresses the node takes as usable.
    pub usable: Vec<String>,
    pub active: Option<Vec<String>>,
}

/// The keys of a registration emitted by `StakeTableV2` or later.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "verdict", rename_all = "camelCase")]
pub enum RegistrationKeys {
    /// Both keys decoded and both signatures verified.
    Verified { bls: String, schnorr: String },
    /// Not verified: the keys that decoded.
    Unverified {
        bls: Option<String>,
        schnorr: Option<String>,
    },
}

/// The verdict on the keys of a key update emitted by `StakeTableV2` or later.
#[derive(Clone, Debug, Serialize)]
#[serde(tag = "verdict", rename_all = "camelCase")]
pub enum KeyUpdateKeys {
    /// Both keys decoded and both signatures verified.
    Verified { bls: String, schnorr: String },
    /// The BLS key did not decode.
    InvalidBls,
    /// The Schnorr key did not decode.
    InvalidSchnorr,
    /// The keys decoded but a signature did not verify.
    Failed,
}

/// A stake table event, with its keys decoded and checked by the node.
#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Event {
    Register {
        account: String,
        bls: Option<String>,
        schnorr: Option<String>,
        commission: u16,
    },
    RegisterV2 {
        account: String,
        keys: RegistrationKeys,
        commission: u16,
    },
    RegisterV3 {
        account: String,
        keys: RegistrationKeys,
        commission: u16,
        x25519: Option<String>,
        p2p_addr: Option<String>,
    },
    Deregister {
        validator: String,
    },
    Delegate {
        delegator: String,
        validator: String,
        amount: String,
    },
    Undelegate {
        delegator: String,
        validator: String,
        amount: String,
    },
    KeyUpdate {
        validator: String,
        bls: Option<String>,
        schnorr: Option<String>,
    },
    KeyUpdateV2 {
        validator: String,
        keys: KeyUpdateKeys,
    },
    CommissionUpdate {
        validator: String,
        commission: u16,
    },
    X25519Update {
        validator: String,
        key: Option<String>,
    },
    P2pAddrUpdate {
        validator: String,
        addr: Option<String>,
    },
}

/// A registered validator as the node holds it.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Validator {
    pub account: String,
    pub bls: Option<String>,
    pub schnorr: Option<String>,
    pub x25519: Option<String>,
    pub p2p_addr: Option<String>,
    pub stake: String,
    pub commission: u16,
    pub authenticated: bool,
    /// Each delegator with its amount.
    pub delegators: Vec<(String, String)>,
}

/// The node's stake table state.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    /// In registration order.
    pub validators: Vec<Validator>,
    pub used_bls: Vec<String>,
    pub used_schnorr: Vec<String>,
    pub used_x25519: Vec<String>,
    pub exits: Vec<String>,
}
