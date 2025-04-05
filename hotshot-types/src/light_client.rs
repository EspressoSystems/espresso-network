// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs associated with light client state

use std::collections::HashMap;

use alloy::primitives::U256;
use ark_ed_on_bn254::EdwardsConfig as Config;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_crhf::CRHF;
use jf_rescue::{crhf::VariableLengthRescueCRHF, RescueError, RescueParameter};
use jf_signature::schnorr;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;
use serde::{Deserialize, Serialize};
use tagged_base64::tagged;

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
/// Concrete for circuit's public input
pub type PublicInput = GenericPublicInput<CircuitField>;
/// Key pairs for signing/verifying a light client state
#[derive(Debug, Default, Clone)]
pub struct StateKeyPair(pub schnorr::KeyPair<Config>);

/// Request body to send to the state relay server
#[derive(Clone, Debug, CanonicalSerialize, CanonicalDeserialize, Serialize, Deserialize)]
pub struct StateSignatureRequestBody {
    /// The public key associated with this request
    pub key: StateVerKey,
    /// The associated light client state
    pub state: LightClientState,
    /// The stake table used for the next HotShot block
    pub next_stake: StakeTableState,
    /// The associated signature of the light client state
    pub signature: StateSignature,
}

/// The state signatures bundle is a light client state and its signatures collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSignaturesBundle {
    /// The state for this signatures bundle
    pub state: LightClientState,
    /// The stake table used in the next block (only different from voting_stake_table at the last block of every epoch)
    pub next_stake: StakeTableState,
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

/// Public input to the light client state prover service
#[derive(Clone, Debug)]
pub struct GenericPublicInput<F: PrimeField> {
    // new light client state
    pub lc_state: GenericLightClientState<F>,
    // voting stake table state
    pub voting_st_state: GenericStakeTableState<F>,
    // next-block stake table state
    pub next_st_state: GenericStakeTableState<F>,
}

impl<F: PrimeField> GenericPublicInput<F> {
    /// Construct a public input from light client state and static stake table state
    pub fn new(
        lc_state: GenericLightClientState<F>,
        voting_st_state: GenericStakeTableState<F>,
        next_st_state: GenericStakeTableState<F>,
    ) -> Self {
        Self {
            lc_state,
            voting_st_state,
            next_st_state,
        }
    }

    /// Convert to a vector of field elements
    pub fn to_vec(&self) -> Vec<F> {
        vec![
            F::from(self.lc_state.view_number),
            F::from(self.lc_state.block_height),
            self.lc_state.block_comm_root,
            self.voting_st_state.bls_key_comm,
            self.voting_st_state.schnorr_key_comm,
            self.voting_st_state.amount_comm,
            self.voting_st_state.threshold,
            self.next_st_state.bls_key_comm,
            self.next_st_state.schnorr_key_comm,
            self.next_st_state.amount_comm,
            self.next_st_state.threshold,
        ]
    }
}

impl<F: PrimeField> From<GenericPublicInput<F>> for Vec<F> {
    fn from(v: GenericPublicInput<F>) -> Self {
        vec![
            F::from(v.lc_state.view_number),
            F::from(v.lc_state.block_height),
            v.lc_state.block_comm_root,
            v.voting_st_state.bls_key_comm,
            v.voting_st_state.schnorr_key_comm,
            v.voting_st_state.amount_comm,
            v.voting_st_state.threshold,
            v.next_st_state.bls_key_comm,
            v.next_st_state.schnorr_key_comm,
            v.next_st_state.amount_comm,
            v.next_st_state.threshold,
        ]
    }
}

impl<F: PrimeField> From<Vec<F>> for GenericPublicInput<F> {
    fn from(v: Vec<F>) -> Self {
        let lc_state = GenericLightClientState {
            view_number: v[0].into_bigint().as_ref()[0],
            block_height: v[1].into_bigint().as_ref()[0],
            block_comm_root: v[2],
        };
        let voting_st_state = GenericStakeTableState {
            bls_key_comm: v[3],
            schnorr_key_comm: v[4],
            amount_comm: v[5],
            threshold: v[6],
        };
        let next_st_state = GenericStakeTableState {
            bls_key_comm: v[7],
            schnorr_key_comm: v[8],
            amount_comm: v[9],
            threshold: v[10],
        };
        Self {
            lc_state,
            voting_st_state,
            next_st_state,
        }
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
