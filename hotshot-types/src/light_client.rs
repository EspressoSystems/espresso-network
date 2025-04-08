// Copyright (c) 2021-2024 Espresso Systems (espressosys.com)
// This file is part of the HotShot repository.

// You should have received a copy of the MIT License
// along with the HotShot repository. If not, see <https://mit-license.org/>.

//! Types and structs associated with light client state

use std::collections::HashMap;

use ark_ed_on_bn254::EdwardsConfig as Config;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use jf_crhf::CRHF;
use jf_rescue::{crhf::VariableLengthRescueCRHF, RescueError, RescueParameter};
use jf_signature::schnorr;
use primitive_types::U256;
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
    /// The associated signature of the light client state
    pub signature: StateSignature,
}

/// The state signatures bundle is a light client state and its signatures collected
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StateSignaturesBundle {
    /// The state for this signatures bundle
    pub state: LightClientState,
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
pub struct GenericPublicInput<F: PrimeField>(Vec<F>);

impl<F: PrimeField> GenericPublicInput<F> {
    /// Construct a public input from light client state and static stake table state
    pub fn new(lc_state: GenericLightClientState<F>, st_state: GenericStakeTableState<F>) -> Self {
        let lc_state_f: [F; 3] = lc_state.into();
        Self(vec![
            lc_state_f[0],
            lc_state_f[1],
            lc_state_f[2],
            st_state.bls_key_comm,
            st_state.schnorr_key_comm,
            st_state.amount_comm,
            st_state.threshold,
        ])
    }
}

impl<F: PrimeField> AsRef<[F]> for GenericPublicInput<F> {
    fn as_ref(&self) -> &[F] {
        &self.0
    }
}

impl<F: PrimeField> From<Vec<F>> for GenericPublicInput<F> {
    fn from(v: Vec<F>) -> Self {
        Self(v)
    }
}

impl<F: PrimeField> GenericPublicInput<F> {
    /// Return the view number of the light client state
    #[must_use]
    pub fn view_number(&self) -> F {
        self.0[0]
    }

    /// Return the block height of the light client state
    #[must_use]
    pub fn block_height(&self) -> F {
        self.0[1]
    }

    /// Return the block commitment root of the light client state
    #[must_use]
    pub fn block_comm_root(&self) -> F {
        self.0[2]
    }

    /// Return the stake table commitment of the light client state
    #[must_use]
    pub fn stake_table_comm(&self) -> (F, F, F) {
        (self.0[3], self.0[4], self.0[5])
    }

    /// Return the qc key commitment of the light client state
    #[must_use]
    pub fn qc_key_comm(&self) -> F {
        self.0[3]
    }

    /// Return the state key commitment of the light client state
    #[must_use]
    pub fn state_key_comm(&self) -> F {
        self.0[4]
    }

    /// Return the stake amount commitment of the light client state
    #[must_use]
    pub fn stake_amount_comm(&self) -> F {
        self.0[5]
    }

    /// Return the threshold
    #[must_use]
    pub fn threshold(&self) -> F {
        self.0[6]
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
