// The bindings types are small and pure data, there is no reason they
// shouldn't be Copy. However some of them do have a bytes field which cannot be Copy.
impl Copy for crate::sol_types::G1PointSol {}
impl Copy for crate::sol_types::G2PointSol {}
impl Copy for crate::sol_types::EdOnBN254PointSol {}
impl Copy for crate::sol_types::StakeTableV3::ValidatorRegistered {}
impl Copy for crate::sol_types::StakeTableV3::ValidatorExit {}
impl Copy for crate::sol_types::StakeTableV3::ConsensusKeysUpdated {}
impl Copy for crate::sol_types::StakeTableV3::Delegated {}
impl Copy for crate::sol_types::StakeTableV3::Undelegated {}
impl Copy for crate::sol_types::stake_table_v3::BN254::G1Point {}
