///`Node(address,uint8,uint64,uint64,uint64,(uint256,uint256))`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct Node {
    pub account: ::ethers::core::types::Address,
    pub stake_type: u8,
    pub balance: u64,
    pub register_epoch: u64,
    pub exit_epoch: u64,
    pub schnorr_vk: EdOnBN254Point,
}
///`G1Point(uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct G1Point {
    pub x: ::ethers::core::types::U256,
    pub y: ::ethers::core::types::U256,
}
///`G2Point(uint256,uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct G2Point {
    pub x_0: ::ethers::core::types::U256,
    pub x_1: ::ethers::core::types::U256,
    pub y_0: ::ethers::core::types::U256,
    pub y_1: ::ethers::core::types::U256,
}
///`EdOnBN254Point(uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct EdOnBN254Point {
    pub x: ::ethers::core::types::U256,
    pub y: ::ethers::core::types::U256,
}
///`PlonkProof((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct PlonkProof {
    pub wire_0: G1Point,
    pub wire_1: G1Point,
    pub wire_2: G1Point,
    pub wire_3: G1Point,
    pub wire_4: G1Point,
    pub prod_perm: G1Point,
    pub split_0: G1Point,
    pub split_1: G1Point,
    pub split_2: G1Point,
    pub split_3: G1Point,
    pub split_4: G1Point,
    pub zeta: G1Point,
    pub zeta_omega: G1Point,
    pub wire_eval_0: ::ethers::core::types::U256,
    pub wire_eval_1: ::ethers::core::types::U256,
    pub wire_eval_2: ::ethers::core::types::U256,
    pub wire_eval_3: ::ethers::core::types::U256,
    pub wire_eval_4: ::ethers::core::types::U256,
    pub sigma_eval_0: ::ethers::core::types::U256,
    pub sigma_eval_1: ::ethers::core::types::U256,
    pub sigma_eval_2: ::ethers::core::types::U256,
    pub sigma_eval_3: ::ethers::core::types::U256,
    pub prod_perm_zeta_omega_eval: ::ethers::core::types::U256,
}
///`VerifyingKey(uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256))`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct VerifyingKey {
    pub domain_size: ::ethers::core::types::U256,
    pub num_inputs: ::ethers::core::types::U256,
    pub sigma_0: G1Point,
    pub sigma_1: G1Point,
    pub sigma_2: G1Point,
    pub sigma_3: G1Point,
    pub sigma_4: G1Point,
    pub q_1: G1Point,
    pub q_2: G1Point,
    pub q_3: G1Point,
    pub q_4: G1Point,
    pub q_m12: G1Point,
    pub q_m34: G1Point,
    pub q_o: G1Point,
    pub q_c: G1Point,
    pub q_h1: G1Point,
    pub q_h2: G1Point,
    pub q_h3: G1Point,
    pub q_h4: G1Point,
    pub q_ecc: G1Point,
}
///`LightClientState(uint64,uint64,uint256,uint256,uint256,uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct LightClientState {
    pub view_num: u64,
    pub block_height: u64,
    pub block_comm_root: ::ethers::core::types::U256,
    pub fee_ledger_comm: ::ethers::core::types::U256,
    pub stake_table_bls_key_comm: ::ethers::core::types::U256,
    pub stake_table_schnorr_key_comm: ::ethers::core::types::U256,
    pub stake_table_amount_comm: ::ethers::core::types::U256,
    pub threshold: ::ethers::core::types::U256,
}
///`Challenges(uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct Challenges {
    pub alpha: ::ethers::core::types::U256,
    pub alpha_2: ::ethers::core::types::U256,
    pub alpha_3: ::ethers::core::types::U256,
    pub beta: ::ethers::core::types::U256,
    pub gamma: ::ethers::core::types::U256,
    pub zeta: ::ethers::core::types::U256,
    pub v: ::ethers::core::types::U256,
    pub u: ::ethers::core::types::U256,
}
///`EvalData(uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct EvalData {
    pub vanish_eval: ::ethers::core::types::U256,
    pub lagrange_one: ::ethers::core::types::U256,
    pub pi_eval: ::ethers::core::types::U256,
}
///`FuzzSelector(address,bytes4[])`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct FuzzSelector {
    pub addr: ::ethers::core::types::Address,
    pub selectors: ::std::vec::Vec<[u8; 4]>,
}
///`TranscriptData(bytes,bytes32[2])`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct TranscriptData {
    pub transcript: ::ethers::core::types::Bytes,
    pub state: [[u8; 32]; 2],
}
///`FsMetadata(bool,bool,uint256,bool,uint256,uint256,uint256)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct FsMetadata {
    pub is_dir: bool,
    pub is_symlink: bool,
    pub length: ::ethers::core::types::U256,
    pub read_only: bool,
    pub modified: ::ethers::core::types::U256,
    pub accessed: ::ethers::core::types::U256,
    pub created: ::ethers::core::types::U256,
}
///`Log(bytes32[],bytes,address)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct Log {
    pub topics: ::std::vec::Vec<[u8; 32]>,
    pub data: ::ethers::core::types::Bytes,
    pub emitter: ::ethers::core::types::Address,
}
///`Rpc(string,string)`
#[derive(
    Clone,
    ::ethers::contract::EthAbiType,
    ::ethers::contract::EthAbiCodec,
    serde::Serialize,
    serde::Deserialize,
    Default,
    Debug,
    PartialEq,
    Eq,
    Hash,
)]
pub struct Rpc {
    pub key: ::std::string::String,
    pub url: ::std::string::String,
}
