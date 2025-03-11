use crate::SeqTypes;
use derive_more::derive::{From, Into};
use hotshot_contract_adapter::stake_table::NodeInfoJf;
use hotshot_types::{network::PeerConfigKeys, PeerConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct PermissionedStakeTableEntry(NodeInfoJf);

/// Stake table holding all staking information (DA and non-DA stakers)
#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub struct CombinedStakeTable(Vec<PeerConfigKeys<SeqTypes>>);

#[derive(Clone, Debug, From, Into)]
/// NewType to disambiguate DA Membership
pub struct DAMembers(pub Vec<PeerConfig<SeqTypes>>);

#[derive(Clone, Debug, From, Into)]
/// NewType to disambiguate StakeTable
pub struct StakeTable(pub Vec<PeerConfig<SeqTypes>>);

#[derive(Clone, Debug)]
pub struct StakeTables {
    pub stake_table: StakeTable,
    pub da_members: DAMembers,
}
