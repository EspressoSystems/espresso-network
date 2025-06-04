// The bindings types are small and pure data, there is no reason they
// shouldn't be Copy. However some of them do have a bytes field which cannot be Copy.
impl Copy for crate::sol_types::G1PointSol {}
impl Copy for crate::sol_types::G2PointSol {}
impl Copy for crate::sol_types::EdOnBN254PointSol {}
impl Copy for crate::sol_types::StakeTableV2::ValidatorRegistered {}
// schnorr sig in ValidatorRegisteredV2 uses Bytes, cannot implement copy
impl Copy for crate::sol_types::StakeTableV2::ValidatorExit {}
impl Copy for crate::sol_types::StakeTableV2::ConsensusKeysUpdated {}
// schnorr sig in ConsensusKeysUpdatedV2 Bytes, cannot implement copy
impl Copy for crate::sol_types::StakeTableV2::Delegated {}
impl Copy for crate::sol_types::StakeTableV2::Undelegated {}
impl Copy for crate::sol_types::staketablev2::BN254::G1Point {}
