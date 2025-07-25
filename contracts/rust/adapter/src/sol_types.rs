//! Solidity types for interacting with contracts
//! Re-export types that are used, sometimes renamed to avoid collision.
//!
//! TODO: (alex) Due to <https://github.com/foundry-rs/foundry/issues/10153>,
//! try to re-export the same type from the "youngest" child contract since that is the contract whose functions are being called,
//! thus from whom the rust bindings are expected.
//! E.g. Both PlonkVerifier and LightClient, and LightClientV2 depends on BN254. The inheritance relationship is:
//!   BN254 <- PlonkVerifier <- LIghtClient <- LightClientV2
//! Most of the time, we interact with PlonkVerifier's function via LightClientV2, thus import BN254.G1Point from `bindings::plonkverifierv2`.
//! When we need to directly interact with PlonkVerifier's method, implement stupid plain `From<lc2::BN254::G1Point> for pv::BN254::G1Point`.
//! If you are lazy, you can even use unsafe memory transmute since they are literally the same representation, duplicated in different modules,
//! thus treated by the rust type systems as distinct types.
//!
//! Another usage is in the differential testing in Solidity tests. In those cases, the actual types don't matter, since they will all `abi_encode()`
//! into the exact same bytes before being communicated over to contract via FFI. Thus using any one of them is fine.

use alloy::sol;

/// # What to re-export, what to hide?
/// - export contract struct itself, but try to avoid export instance type (instead, use ::new() to get a handle)
/// - avoid exporting `xxCall` and `xxReturn` types, they usually can be converted/transmuted from existing struct
/// - Event types should be exported
/// - structs should be exported and renamed with `xxSol` suffix to avoid confusion with other rust types
///   - see module doc for more explanation on types duplication issue in alloy
pub use crate::bindings::{
    erc1967proxy::ERC1967Proxy,
    esptoken::EspToken,
    esptokenv2::EspTokenV2,
    feecontract::FeeContract::{self, Deposit},
    lightclient::{
        IPlonkVerifier::{PlonkProof as PlonkProofSol, VerifyingKey as VerifyingKeySol},
        LightClient::{
            self, LightClientErrors, LightClientInstance, LightClientState as LightClientStateSol,
            StakeTableState as StakeTableStateSol,
        },
        BN254::G1Point as G1PointSol,
    },
    lightclientmock::{self, LightClientMock},
    lightclientv2::{self, LightClientV2},
    lightclientv2mock::{self, LightClientV2Mock},
    lightclientv3::{self, LightClientV3},
    opstimelock::OpsTimelock,
    ownableupgradeable::OwnableUpgradeable,
    plonkverifier::PlonkVerifier,
    plonkverifierv2::PlonkVerifierV2,
    plonkverifierv3::PlonkVerifierV3,
    safeexittimelock::SafeExitTimelock,
    staketable::StakeTable,
    staketablev2::{
        self, EdOnBN254::EdOnBN254Point as EdOnBN254PointSol, StakeTableV2,
        BN254::G2Point as G2PointSol,
    },
};

// For types that we need to interact with some functions but their bindings are not generated
// we manually declare them there. It's possible that they get included in the future commits,
// at which point, the rust type system will complain and we simply remove the manual declaration
// and re-export the type from bindings instead.
sol! {
    /// types in src/legacy/Transcript.sol
    struct TranscriptDataSol {
        bytes32 state;
        bytes transcript;
    }

    /// types in src/libraries/PlonkVerifierV2.sol
    struct ChallengesSol {
        uint256 alpha;
        uint256 alpha2;
        uint256 alpha3;
        uint256 beta;
        uint256 gamma;
        uint256 zeta;
        uint256 v;
        uint256 u;
    }

}

// Due to <https://github.com/foundry-rs/foundry/issues/10153> the rust bindings contain duplicate types for our solidity types.
// In order to avoid writing a lot of boilerplate code we use transmute to convert between these duplicated types.
// Since all the types we transmute between are generated by foundry from the same underlying solidity type
// we expect that the order of fields and types of fields are always the same.
impl From<LightClient::genesisStateReturn> for LightClientStateSol {
    fn from(v: LightClient::genesisStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<lightclientmock::LightClient::LightClientState> for LightClientStateSol {
    fn from(v: lightclientmock::LightClient::LightClientState) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}
impl From<lightclientmock::LightClientMock::finalizedStateReturn> for LightClientStateSol {
    fn from(v: lightclientmock::LightClientMock::finalizedStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientStateSol> for lightclientmock::LightClient::LightClientState {
    fn from(v: LightClientStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<PlonkProofSol> for lightclientmock::IPlonkVerifier::PlonkProof {
    fn from(v: PlonkProofSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<lightclientmock::LightClientMock::genesisStateReturn> for LightClientStateSol {
    fn from(v: lightclientmock::LightClientMock::genesisStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV2::finalizedStateReturn> for LightClientStateSol {
    fn from(v: LightClientV2::finalizedStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV2::votingStakeTableStateReturn> for StakeTableStateSol {
    fn from(v: LightClientV2::votingStakeTableStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<lightclientv2mock::LightClient::LightClientState> for LightClientStateSol {
    fn from(v: lightclientv2mock::LightClient::LightClientState) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}
impl From<LightClientStateSol> for lightclientv2mock::LightClient::LightClientState {
    fn from(v: LightClientStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}
impl From<LightClientStateSol> for lightclientv2::LightClient::LightClientState {
    fn from(v: LightClientStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<StakeTableStateSol> for lightclientv2::LightClient::StakeTableState {
    fn from(v: StakeTableStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}
impl From<StakeTableStateSol> for lightclientv2mock::LightClient::StakeTableState {
    fn from(v: StakeTableStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV2Mock::genesisStateReturn> for LightClientStateSol {
    fn from(v: LightClientV2Mock::genesisStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV2Mock::finalizedStateReturn> for LightClientStateSol {
    fn from(v: LightClientV2Mock::finalizedStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<PlonkProofSol> for lightclientv2::IPlonkVerifier::PlonkProof {
    fn from(v: PlonkProofSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV2Mock::votingStakeTableStateReturn> for StakeTableStateSol {
    fn from(v: LightClientV2Mock::votingStakeTableStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<G1PointSol> for staketablev2::BN254::G1Point {
    fn from(v: G1PointSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<staketablev2::BN254::G1Point> for G1PointSol {
    fn from(v: staketablev2::BN254::G1Point) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

// Transmute conversion functions for LightClientV3
impl From<LightClientV3::finalizedStateReturn> for LightClientStateSol {
    fn from(v: LightClientV3::finalizedStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientV3::votingStakeTableStateReturn> for StakeTableStateSol {
    fn from(v: LightClientV3::votingStakeTableStateReturn) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<LightClientStateSol> for lightclientv3::LightClient::LightClientState {
    fn from(v: LightClientStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<StakeTableStateSol> for lightclientv3::LightClient::StakeTableState {
    fn from(v: StakeTableStateSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

impl From<PlonkProofSol> for lightclientv3::IPlonkVerifier::PlonkProof {
    fn from(v: PlonkProofSol) -> Self {
        unsafe { std::mem::transmute(v) }
    }
}

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use self::{
    staketablev2::{EdOnBN254::EdOnBN254Point, BN254::G2Point},
    StakeTableV2::{
        ConsensusKeysUpdated, ConsensusKeysUpdatedV2, Delegated, Undelegated, ValidatorExit,
        ValidatorRegistered, ValidatorRegisteredV2,
    },
};

impl PartialEq for ValidatorRegistered {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
            && self.blsVk == other.blsVk
            && self.schnorrVk == other.schnorrVk
            && self.commission == other.commission
    }
}

impl PartialEq for ValidatorRegisteredV2 {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
            && self.blsVK == other.blsVK
            && self.schnorrVK == other.schnorrVK
            && self.commission == other.commission
            && self.blsSig == other.blsSig
            && self.schnorrSig == other.schnorrSig
    }
}

impl PartialEq for ConsensusKeysUpdated {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
            && self.blsVK == other.blsVK
            && self.schnorrVK == other.schnorrVK
    }
}

impl PartialEq for ConsensusKeysUpdatedV2 {
    fn eq(&self, other: &Self) -> bool {
        self.account == other.account
            && self.blsVK == other.blsVK
            && self.schnorrVK == other.schnorrVK
            && self.blsSig == other.blsSig
            && self.schnorrSig == other.schnorrSig
    }
}

impl Serialize for ValidatorRegistered {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.account, &self.blsVk, &self.schnorrVk, self.commission).serialize(serializer)
    }
}

#[allow(non_snake_case)]
impl<'de> Deserialize<'de> for ValidatorRegistered {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (account, blsVk, schnorrVk, commission) = <(_, _, _, u16)>::deserialize(deserializer)?;
        Ok(Self {
            account,
            blsVk,
            schnorrVk,
            commission,
        })
    }
}

impl Serialize for ValidatorRegisteredV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (
            &self.account,
            &self.blsVK,
            &self.schnorrVK,
            self.commission,
            &self.blsSig,
            &self.schnorrSig,
        )
            .serialize(serializer)
    }
}

#[allow(non_snake_case)]
impl<'de> Deserialize<'de> for ValidatorRegisteredV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (account, blsVK, schnorrVK, commission, blsSig, schnorrSig) =
            <(_, _, _, u16, _, _)>::deserialize(deserializer)?;
        Ok(ValidatorRegisteredV2 {
            account,
            blsVK,
            schnorrVK,
            commission,
            blsSig,
            schnorrSig,
        })
    }
}

impl Serialize for EdOnBN254Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (self.x, self.y).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EdOnBN254Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = Deserialize::deserialize(deserializer)?;
        Ok(Self { x, y })
    }
}

impl Serialize for G2Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.x0, &self.x1, &self.y0, &self.y1).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for G2Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x0, x1, y0, y1) = Deserialize::deserialize(deserializer)?;

        Ok(Self { x0, x1, y0, y1 })
    }
}

impl Serialize for staketablev2::BN254::G1Point {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.x, &self.y).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for staketablev2::BN254::G1Point {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (x, y) = Deserialize::deserialize(deserializer)?;
        Ok(Self { x, y })
    }
}

impl Serialize for ValidatorExit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.validator,).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ValidatorExit {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (validator,): (alloy::sol_types::private::Address,) =
            Deserialize::deserialize(deserializer)?;
        Ok(ValidatorExit { validator })
    }
}

impl Serialize for Delegated {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.delegator, &self.validator, &self.amount).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Delegated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (delegator, validator, amount) = Deserialize::deserialize(deserializer)?;

        Ok(Delegated {
            delegator,
            validator,
            amount,
        })
    }
}

impl Serialize for Undelegated {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.delegator, &self.validator, &self.amount).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Undelegated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (delegator, validator, amount) = Deserialize::deserialize(deserializer)?;

        Ok(Undelegated {
            delegator,
            validator,
            amount,
        })
    }
}

impl Serialize for ConsensusKeysUpdated {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (&self.account, &self.blsVK, &self.schnorrVK).serialize(serializer)
    }
}

#[allow(non_snake_case)]
impl<'de> Deserialize<'de> for ConsensusKeysUpdated {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (account, blsVK, schnorrVK) = Deserialize::deserialize(deserializer)?;

        Ok(ConsensusKeysUpdated {
            account,
            blsVK,
            schnorrVK,
        })
    }
}

impl Serialize for ConsensusKeysUpdatedV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (
            &self.account,
            &self.blsVK,
            &self.schnorrVK,
            &self.blsSig,
            &self.schnorrSig,
        )
            .serialize(serializer)
    }
}

#[allow(non_snake_case)]
impl<'de> Deserialize<'de> for ConsensusKeysUpdatedV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let (account, blsVK, schnorrVK, blsSig, schnorrSig) =
            Deserialize::deserialize(deserializer)?;

        Ok(ConsensusKeysUpdatedV2 {
            account,
            blsVK,
            schnorrVK,
            blsSig,
            schnorrSig,
        })
    }
}
