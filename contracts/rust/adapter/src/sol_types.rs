//! Solidity types for interacting with contracts
//! Re-export types that are used, sometimes renamed to avoid collision.
//!
//! NOTE: Due to <https://github.com/foundry-rs/foundry/issues/10153>,
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

pub use crate::bindings::{
    erc1967proxy::ERC1967Proxy,
    feecontract::FeeContract::{self, Deposit},
    lightclient::{
        IPlonkVerifier::{PlonkProof as PlonkProofSol, VerifyingKey as VerifyingKeySol},
        LightClient::{
            self, LightClientInstance, LightClientState as LightClientStateSol,
            StakeTableState as StakeTableStateSol,
        },
        BN254::G1Point as G1PointSol,
    },
    lightclientmock::LightClientMock,
    permissionedstaketable::{
        EdOnBN254::EdOnBN254Point as EdOnBN254PointSol,
        PermissionedStakeTable::{self, NodeInfo as NodeInfoSol, StakersUpdated},
        BN254::G2Point as G2PointSol,
    },
    plonkverifier::PlonkVerifier::{self},
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

    /// types in src/libraries/PlonkVerifier.sol
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
