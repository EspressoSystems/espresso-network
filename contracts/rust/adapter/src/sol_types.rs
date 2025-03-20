//! Solidity types for interacting with contracts
//! Re-export types that are used, sometimes renamed to avoid collision.

use alloy::sol;

pub use crate::bindings::{
    erc1967proxy::ERC1967Proxy,
    feecontract::FeeContract::{self, Deposit},
    iplonkverifier::{
        IPlonkVerifier::{PlonkProof as PlonkProofSol, VerifyingKey as VerifyingKeySol},
        BN254::G1Point as G1PointSol,
    },
    lightclient::LightClient::{
        self, LightClientState as LightClientStateSol, StakeTableState as StakeTableStateSol,
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
