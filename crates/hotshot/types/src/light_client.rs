// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs associated with light client state

use std::collections::HashMap;

use alloy::primitives::{FixedBytes, U256};
use ark_ed_on_bn254::EdwardsConfig as Config;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_crhf::CRHF;
use jf_rescue::{crhf::VariableLengthRescueCRHF, RescueError, RescueParameter};
use jf_signature::schnorr;
use jf_utils::to_bytes;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use tagged_base64::tagged;

use crate::signature_key::BLSPubKey;

/// Capacity of the stake table, used for light client
pub const DEFAULT_STAKE_TABLE_CAPACITY: usize = 200;
/// Base field in the prover circuit
pub type CircuitField = ark_ed_on_bn254::Fq;
/// Concrete type for light client state
pub type LightClientState = GenericLightClientState<CircuitField>;
/// Concreate type for light client state message to sign
pub type LightClientStateMsg = GenericLightClientStateMsg<CircuitField>;
/// Concrete type for stake table state
pub type StakeTableState = GenericStakeTableState<CircuitField>;
/// Signature scheme
pub type StateSignatureScheme =
    jf_signature::schnorr::SchnorrSignatureScheme<ark_ed_on_bn254::EdwardsConfig>;
/// Signatures
pub type StateSignature = schnorr::Signature<Config>;
/// Verification key for verifying state signatures
pub type StateVerKey = schnorr::VerKey<Config>;
/// Signing key for signing a light client state
pub type StateSignKey = schnorr::SignKey<ark_ed_on_bn254::Fr>;
/// Key pairs for signing/verifying a light client state
#[derive(Debug, Default, Clone)]
pub struct StateKeyPair(pub schnorr::KeyPair<Config>);

/// The request body for light client V1 to send to the state relay server
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize)]
pub struct LCV1StateSignatureRequestBody {
    /// The public key associated with this request
    pub key: StateVerKey,
    /// The associated light client state
    pub state: LightClientState,
    /// The associated signature of the light client state
    pub signature: StateSignature,
}

impl std::fmt::Display for LCV1StateSignatureRequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LCV1StateSignatureRequestBody {{ key: {}, state: {}, signature: {} }}",
            self.key, self.state, self.signature
        )
    }
}

/// The request body for light client V2 to send to the state relay server
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize)]
pub struct LCV2StateSignatureRequestBody {
    /// The public key associated with this request
    pub key: StateVerKey,
    /// The associated light client state
    pub state: LightClientState,
    /// The stake table used for the next HotShot block
    pub next_stake: StakeTableState,
    /// The associated signature of the light client state
    pub signature: StateSignature,
}

impl std::fmt::Display for LCV2StateSignatureRequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LCV2StateSignatureRequestBody {{ key: {}, state: {}, next_stake: {}, signature: {} }}",
            self.key, self.state, self.next_stake, self.signature
        )
    }
}

/// The request body for light client V3 to send to the state relay server
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LCV3StateSignatureRequestBody {
    /// The public key associated with this request
    pub key: StateVerKey,
    /// The associated light client state
    pub state: LightClientState,
    /// The stake table used for the next HotShot block
    pub next_stake: StakeTableState,
    /// The auth root
    // TODO(Chengyu): FixedBytes doesn't implement Canonical(De)Serialize. Is it a problem?
    pub auth_root: FixedBytes<32>,
    /// The associated signature of the light client state
    pub signature: StateSignature,
    /// The associated signature of the light client state for LCV2
    pub v2_signature: StateSignature,
}

impl std::fmt::Display for LCV3StateSignatureRequestBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "LCV3StateSignatureRequestBody {{ key: {}, state: {}, next_stake: {}, auth_root: {}, signature: {}, v2_signature: {} }}",
            self.key, self.state, self.next_stake, self.auth_root, self.signature, self.v2_signature
        )
    }
}

impl From<LCV1StateSignatureRequestBody> for LCV2StateSignatureRequestBody {
    fn from(value: LCV1StateSignatureRequestBody) -> Self {
        Self {
            key: value.key,
            state: value.state,
            // Filling default values here because the legacy prover/contract doesn't care about this next_stake.
            next_stake: StakeTableState::default(),
            signature: value.signature,
        }
    }
}

impl From<LCV2StateSignatureRequestBody> for LCV1StateSignatureRequestBody {
    fn from(value: LCV2StateSignatureRequestBody) -> Self {
        Self {
            key: value.key,
            state: value.state,
            signature: value.signature,
        }
    }
}

impl From<LCV3StateSignatureRequestBody> for LCV2StateSignatureRequestBody {
    fn from(value: LCV3StateSignatureRequestBody) -> Self {
        Self {
            key: value.key,
            state: value.state,
            next_stake: value.next_stake,
            signature: value.v2_signature,
        }
    }
}

/// The state signatures bundle is a light client V1 state and its signatures collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LCV1StateSignaturesBundle {
    /// The state for this signatures bundle
    pub state: LightClientState,
    /// The collected signatures
    pub signatures: HashMap<StateVerKey, StateSignature>,
    /// Total stakes associated with the signer
    pub accumulated_weight: U256,
}

/// The state signatures bundle is a light client V2 state and its signatures collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LCV2StateSignaturesBundle {
    /// The state for this signatures bundle
    pub state: LightClientState,
    /// The stake table used in the next block (only different from voting_stake_table at the last block of every epoch)
    pub next_stake: StakeTableState,
    /// The collected signatures
    pub signatures: HashMap<StateVerKey, StateSignature>,
    /// Total stakes associated with the signer
    pub accumulated_weight: U256,
}

/// The state signatures bundle is a light client V3 state and its signatures collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LCV3StateSignaturesBundle {
    /// The state for this signatures bundle
    pub state: LightClientState,
    /// The stake table used in the next block (only different from voting_stake_table at the last block of every epoch)
    pub next_stake: StakeTableState,
    /// The auth root
    pub auth_root: FixedBytes<32>,
    /// The collected signatures
    pub signatures: HashMap<StateVerKey, StateSignature>,
    /// Total stakes associated with the signer
    pub accumulated_weight: U256,
}

/// A light client state
#[tagged("LIGHT_CLIENT_STATE")]
#[derive(
    Clone,
    Debug,
    CanonicalSerialize,
    CanonicalDeserialize,
    Default,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Hash,
    Copy,
)]
pub struct GenericLightClientState<F: PrimeField> {
    /// Current view number
    pub view_number: u64,
    /// Current block height
    pub block_height: u64,
    /// Root of the block commitment tree
    pub block_comm_root: F,
}

pub type GenericLightClientStateMsg<F> = [F; 3];

impl<F: PrimeField> From<GenericLightClientState<F>> for GenericLightClientStateMsg<F> {
    fn from(state: GenericLightClientState<F>) -> Self {
        [
            F::from(state.view_number),
            F::from(state.block_height),
            state.block_comm_root,
        ]
    }
}

impl<F: PrimeField> From<&GenericLightClientState<F>> for GenericLightClientStateMsg<F> {
    fn from(state: &GenericLightClientState<F>) -> Self {
        [
            F::from(state.view_number),
            F::from(state.block_height),
            state.block_comm_root,
        ]
    }
}

impl<F: PrimeField + RescueParameter> GenericLightClientState<F> {
    pub fn new(
        view_number: u64,
        block_height: u64,
        block_comm_root: &[u8],
    ) -> anyhow::Result<Self> {
        Ok(Self {
            view_number,
            block_height,
            block_comm_root: hash_bytes_to_field(block_comm_root)?,
        })
    }
}

/// Stake table state
#[tagged("STAKE_TABLE_STATE")]
#[derive(
    Clone,
    Debug,
    CanonicalSerialize,
    CanonicalDeserialize,
    Default,
    Eq,
    PartialEq,
    PartialOrd,
    Ord,
    Hash,
    Copy,
)]
pub struct GenericStakeTableState<F: PrimeField> {
    /// Commitments to the table column for BLS public keys
    pub bls_key_comm: F,
    /// Commitments to the table column for Schnorr public keys
    pub schnorr_key_comm: F,
    /// Commitments to the table column for Stake amounts
    pub amount_comm: F,
    /// threshold
    pub threshold: F,
}

impl<F: PrimeField> From<GenericStakeTableState<F>> for [F; 4] {
    fn from(state: GenericStakeTableState<F>) -> Self {
        [
            state.bls_key_comm,
            state.schnorr_key_comm,
            state.amount_comm,
            state.threshold,
        ]
    }
}

impl std::ops::Deref for StateKeyPair {
    type Target = schnorr::KeyPair<Config>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl StateKeyPair {
    /// Generate key pairs from private signing keys
    #[must_use]
    pub fn from_sign_key(sk: StateSignKey) -> Self {
        Self(schnorr::KeyPair::<Config>::from(sk))
    }

    /// Generate key pairs from `thread_rng()`
    #[must_use]
    pub fn generate() -> StateKeyPair {
        schnorr::KeyPair::generate(&mut rand::thread_rng()).into()
    }

    /// Generate key pairs from seed
    #[must_use]
    pub fn generate_from_seed(seed: [u8; 32]) -> StateKeyPair {
        schnorr::KeyPair::generate(&mut ChaCha20Rng::from_seed(seed)).into()
    }

    /// Generate key pairs from an index and a seed
    #[must_use]
    pub fn generate_from_seed_indexed(seed: [u8; 32], index: u64) -> StateKeyPair {
        let mut hasher = blake3::Hasher::new();
        hasher.update(&seed);
        hasher.update(&index.to_le_bytes());
        let new_seed = *hasher.finalize().as_bytes();
        Self::generate_from_seed(new_seed)
    }
}

impl From<schnorr::KeyPair<Config>> for StateKeyPair {
    fn from(value: schnorr::KeyPair<Config>) -> Self {
        StateKeyPair(value)
    }
}

pub fn hash_bytes_to_field<F: RescueParameter>(bytes: &[u8]) -> Result<F, RescueError> {
    // make sure that `mod_order` won't happen.
    let bytes_len = (<F as PrimeField>::MODULUS_BIT_SIZE.div_ceil(8) - 1) as usize;
    let elem = bytes
        .chunks(bytes_len)
        .map(F::from_le_bytes_mod_order)
        .collect::<Vec<_>>();
    Ok(VariableLengthRescueCRHF::<_, 1>::evaluate(elem)?[0])
}

/// This trait is for light client use. It converts the stake table items into
/// field elements. These items will then be digested into a part of the light client state.
pub trait ToFieldsLightClientCompat {
    const SIZE: usize;
    fn to_fields(&self) -> Vec<CircuitField>;
}

impl ToFieldsLightClientCompat for StateVerKey {
    const SIZE: usize = 2;
    /// This should be compatible with our legacy implementation.
    fn to_fields(&self) -> Vec<CircuitField> {
        let p = self.to_affine();
        vec![p.x, p.y]
    }
}

impl ToFieldsLightClientCompat for BLSPubKey {
    const SIZE: usize = 3;
    /// This should be compatible with our legacy implementation.
    fn to_fields(&self) -> Vec<CircuitField> {
        match to_bytes!(&self.to_affine()) {
            Ok(bytes) => {
                vec![
                    CircuitField::from_le_bytes_mod_order(&bytes[..31]),
                    CircuitField::from_le_bytes_mod_order(&bytes[31..62]),
                    CircuitField::from_le_bytes_mod_order(&bytes[62..]),
                ]
            },
            Err(_) => unreachable!(),
        }
    }
}
