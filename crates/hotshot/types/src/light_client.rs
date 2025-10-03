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
            "LCV3StateSignatureRequestBody {{ key: {}, state: {}, next_stake: {}, auth_root: {}, \
             signature: {}, v2_signature: {} }}",
            self.key,
            self.state,
            self.next_stake,
            self.auth_root,
            self.signature,
            self.v2_signature
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

impl LCV2StateSignaturesBundle {
    /// This is for backward compatibility reason
    pub fn from_v1(value: LCV1StateSignaturesBundle) -> Self {
        Self {
            state: value.state,
            next_stake: StakeTableState::default(), // filling arbitrary value here
            signatures: value.signatures,
            accumulated_weight: value.accumulated_weight,
        }
    }
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

#[cfg(test)]
/// These tests asserts that our code is compatible with what's already deployed on Mainnet/testnets.
mod compatibility_tests {
    use crate::{
        light_client::{LCV1StateSignaturesBundle, LCV2StateSignaturesBundle},
        traits::signature_key::{LCV1StateSignatureKey, LCV2StateSignatureKey},
    };

    #[test]
    fn test_lcv1_mainnet_compatibility() {
        // This bundle is fetched from the Mainnet on 2025-10-01.
        let bundle_str = r#"
        {
            "state": "LIGHT_CLIENT_STATE~XyRlAAAAAAD522AAAAAAAFlGs906TNgv_P4idwB6AXzSXB8p2VzJJ1s6qP0wws4ANA",
            "signatures": {
                "SCHNORR_VER_KEY~gHJJAXJX-lKYArudDnVid-24gdBkt2Ufczhvi4ZK5h-1FcQFLfnKuHChpMNVXahUdAoKw-HG8CoQG9u71_HcKDE": "SCHNORR_SIG~XQ9FjRiUbK-pzWUX0w1hR9DjHNxQp--l9_mTklqQtAHiQRj8xtdhWJZ9_WQJJ7cuNYWjBa9F5y85loDW44qiIo5iVqZd7nbl2cCwDySOahk_GNk2pFN8pvI6RvEtDMAXgQ",
                "SCHNORR_VER_KEY~kmfoB545vVyDBaxptgNhcFmImraKVumH2o0iydLw7SiRd0GQ-Sj--tclKpNIb2KxFMj3TxGPAZA6zeYPhCleBBM": "SCHNORR_SIG~6FcbwLDzta_Ewd8Tw5UX2wNHembkAjLQ0GRa6fHQggTyWi0DzDk263Ljd8Ha9XFyMVH-z6PqS6Ixcfl3h8SyHwzROFXYuz3oCe8VTLet-MDpW5uaPerCCjv7vmAJVBIXTg",
                "SCHNORR_VER_KEY~MTFJYRhckCA8x02yg9RUSOIpeQhLaKjJ7PfOJ0qqVQrX9JG_HEHKn6sw4uTVFCi2HESJgrbedcb5OCraEvocCqk": "SCHNORR_SIG~BFFH5AjT3Pq2nyfwk7vJHkxR0aRCprCOwoZrEaL07wSbmtaC9jhq0r63lExfPpWO3e0t_rImEBPzWyhhb8rSKlwbdY_UyoLcBWDRk9vf-8LGOZAxvestRJxxrDeFuCEZFg",
                "SCHNORR_VER_KEY~gUyMlXmZZ-cqVhVd55BbCc4rmxI0uoAL6vSvfwqR0yfYMZPS1Ivxi5StH7pzHSY1N9YswDqVOEkyNNrVWn3gB90": "SCHNORR_SIG~DmbD39moI8ICzL6N_oWpJZPe3-nfGmGqhh1ZqOwS1ADXv98_BkmCrJrftQ31ltyiPgtMY6Qi3r4t4-pl4sWrJ9v3HZiD1P7bRtDl8n8x7Q83N0FhIRCmh6wi1CmGhKMAEQ",
                "SCHNORR_VER_KEY~De0xW_K70tnzmaNBP8MG8_pRb9EjiC109mvI502SwSLvdOU1-AMpcAcVafzZ47Boe3uxzFCjCYSkyzPWkymNDGU": "SCHNORR_SIG~Xpyf5j-Abp-j0YCpI3uOkI3m5XKRHuxLOHFfr5CdbgFnR8mtecV4nqo4LO-2pNoynIAaq9tv51KL6s3nDDTGGqcwp-cv23eNDSTQ5_rXXqXK88MUXl80OVVlLxCAz-sQgQ",
                "SCHNORR_VER_KEY~N96223m4scH2_YY7CP93JPZqXDIdcu5aPvWIXlsgAAW-ulDWRJ3TOSGaGHRaDVMW9Jhg0I47VktTihSg9x0gE_g": "SCHNORR_SIG~Z7IDoFaaTLoofcAjeOiQRRgBeQXsRaj4IDRHfwVlRACfgbGvATcEOEBu6xbEfazDRP_PeFfTh2lDz7GxPQiPChblwzUoWnByPBSmoKqnUOw3T66N3zYjlMXR-VeA3PsC3w",
                "SCHNORR_VER_KEY~hcb567Aj1Ro7W1uLyecMhV7rWuIXtht2wSM7os6g4SIgideHvkpZEEncoKhVuMhqwXzLgjwyk-j2oVTg3UPODjI": "SCHNORR_SIG~RjV5bDS1Wb2EWmPEIeUTUqIp4uPokobOzBunZm5EHgDg8cbThg-li-z1zDb0dOBoLQF3jOOj0aV6Z4OmSi7ICaZJLBxkbLVxHR804oGqTZ-onW2ZeL_ZFbxusYEzMw4wVA",
                "SCHNORR_VER_KEY~SO9wqmkZx3SynQKSeoftRNfYQD5Ic3eE4YjYC8sDqi2ORleYR0eifKvwlFsTur1M3oJKfa4XK2_z5aiw9voXCZ4": "SCHNORR_SIG~k7bZUbMUdvggn7k1iiUBYYc-cjRDkFZohzSZNJr4tQOosQIIvJXs5YYA6W9_5kcW-XUm2QYMcAWfr1yp8akNAFqBDp4YC759tvRgfmZHS1lFD-62S10OL2WZ8lfhu-AqaQ",
                "SCHNORR_VER_KEY~m7WbBkhEI2D4xuqD8wgyuHYV6M__Lg5fi8z74zcwogijanmJB01iszuh9qGf_0vP9VsT-yWSByWe_hV7xM6kB0Y": "SCHNORR_SIG~Qq-afLDDTnvFg4DyC3tT-lRufuLFDyUkRZMkK0JdGAUZDwxbHjvytNJtaLqBAD_Y1yHSrWA6YfCYevaoqXHZJ5TOt_QY6WIo9NyKwEEeWnIp0FsuUM26UBdukwn5k6QsNw",
                "SCHNORR_VER_KEY~WqOr0vivGMlYQp5gdw21euY-lffGljRM_nlU7QjKDBnnCg6jdLB8g6lHubQIlNW7aoTmAFO_GOUi48jUxBBxDrA": "SCHNORR_SIG~1E8kJmV-1bkrRx4tB5XiujoKhytWfQiGMXbY1fvhIASQTE2wBGv5BkVuGNx1OyEEpI433DoPDcQU7uxZaPp1IBkY8CeA2fKesSnROmDnNOv5QjAdxKa1nQaPmEW1VeUiVw",
                "SCHNORR_VER_KEY~ZxdqOX5WOlABX2l_ZgY_TLOxsNcfWN2p0NM0YudK9AdHl2l1vEVE4OGM533b78EJGJO2sQwSDmcs-P64KoHTKVk": "SCHNORR_SIG~cq6ZTme1xI4mpTr8-3_M_tLEasp5GYwooV6Rgt6TRgIxpFCAFvLo9CS3bAFljx8U3BIqaWYkfWDKQLmS9z2qLG1hOVOqoY3MjVO97bWX1BIzBSqU0Yrqx-hWBk-ymSUuHw",
                "SCHNORR_VER_KEY~GFSdirS1OZOmA7gJvcx0f0VLIwpzx6QA6mMliinw_RPh9B4_r8vaZmHVTtCs-V1p21t0OWm2Nko17NC3n0_5KPE": "SCHNORR_SIG~gOz-WlPy6gNliwcEoRA9yMKXYsme1PKU7kO7TbsTigGyxk6n7nVHLS9tY5vH7bjrlE-RNb_ua-YAsezi68y5L_rhwR8JjHgj3EKbPlBlydIb4PAkN1zw74BXPZdYDIkguw",
                "SCHNORR_VER_KEY~Jm5oGAiBdSHVLz9S6QHJMaEVTJaAYA8YpI0TPbcB_S9y2xEcaRRpplxLHoXFh_rrsRta9kPTjatWpuGttepRKJ8": "SCHNORR_SIG~p8MId9t62ALpuxMTMlWDzZ4zE15NO8SltFT5iQlkuwO3hfgskF51rZjDXm3l6nop_NcyFJlv0MuSu0SedTBXH43l9VdJE8FykErh6LB5-3DhXC_xWEsvf5eJSL_PXHUrcA",
                "SCHNORR_VER_KEY~LFlkwVLtY7yEmShTPq2KlaQ3PlrnICLNO8QhWeyKVCvWoew_Pek3izjkLorqL56PxVLozP5eDdSnn0TzyU_sDak": "SCHNORR_SIG~hUINuZ9Nn4Fz-7JeYE1H9snvs7IZ5Kam_o4Rcr8sewWy1cX0aUyqgdQwnmU88j3pLPNEuav06fFP4uOfosL0KXt6KQD07JEUJ7JEGbD_dDsqtJ8VG8fdKq6ICD_DlQQwIw",
                "SCHNORR_VER_KEY~lyvKQ8QWBHhRyKA4tVh8srJQ3-putmFj6oMjtS03AxOflCBul5tanZ7i5t9Xzrm33KwddTw7i1-GGzWkpgC6K5M": "SCHNORR_SIG~jJkLbymNnYlT5xAo67EzrvbnwareoqNSaYW_Dp23awUwZu8f3CAPNAY0BheDopM24m_Ij3naPsCDulcS9urIFkEMElqJr1vJItvWJidFE4wcaJy4-GSiMLWW3aSYxoEIDQ",
                "SCHNORR_VER_KEY~rgb7ls4GgUwAePoz-EM8hnXe1E2nrjgTZzVZzYmdKiaCl_9vsiQZF6ufPOkeiAEDAlO037AaASUYcC8-4-hEE3g": "SCHNORR_SIG~MSZk8yhEgtP12B1_IIwXLGAoqDdskdVu5wXchgW26QDH8D7EhS8HmAf6XkMrFa5p-hrlBnDNY9ydRkArct5AINTIG9w-X-4LJhGby4k2MuswjAj_eKFC4w_una6J15QnnQ",
                "SCHNORR_VER_KEY~shFP72-c7v6UBbcYg26JwowFOLEgpw3ms70UK_y06xFZwodsdMvOCYQTZNq1lmbonmqoTtzqxyfuIUOB6oY_F90": "SCHNORR_SIG~SxyYfYZZvkVCLtVzxDHzpstGd-IX_E4vJz4y0WFg_gPWSQ7PEynOkponhmd2cZQD56l2pgfkGaHvTW9S5OBpCAmKCv2ii3O1D3Lfzmchs5qUkMyB9l6K3kKtujqd9YECzg",
                "SCHNORR_VER_KEY~SWUNvbC1cL1nOPLJV1Ac_OrGNwGIU_BNhzkh9zzJYgr4yfsPR2C4lodKlQuwSPIJkYje0LqZRdC8DSNUtv45Lpo": "SCHNORR_SIG~zoc06l9_Lq8oJM_0aQ1bjHyYXHpoicI5U994IBpo4gS4-W1qq6opliyhjSeyQpQMrYThZrECCPJtbmau-ex8E8I9QCu7DJv8K4u76t-QVp3Y5zLrj5qSL7vv8LJeZj8Emg",
                "SCHNORR_VER_KEY~YnGWr-sbkEVb3Cn6rQ4wAjEIRhvxb-dTdiifFyksiAUQTkUzhc-cIy36LEYaqLqEac2Ua3RUGXcrWm6k23WFKaA": "SCHNORR_SIG~3RZT0WzEn92NkrdB_XgcF_8wlRInsukzGBTHJKvmrgNf11ytqLTQtEN4rPn8uSmX4kIUHdxqH7czzRHP8OQQHc62Q3hFcNIoVJIKg5aki3fRyZParAWAWeI2nrXm_44Mww",
                "SCHNORR_VER_KEY~uurkd7VpzTaUAOP5JdqcCHmAXQRkod7tRRHq5p5-2w0meWLKQwdsl5t1YPdNh9zgplAUPTpS4Y_ETbp0qsw3Feg": "SCHNORR_SIG~FkMImW_9ND-btIL3t1Bbl1ZVET3wisYg1SHM2NBZ7wJSfyAM9PfA90V1jGXeWMRj6R-fQV14Q_vjDxMDb5fRD3hHg9aoMIcqz4JPVGnQ3FYGoxErUeOxhlyGI7kH66sH5g",
                "SCHNORR_VER_KEY~ka35EM8oap-0K6MOk1UXk1kLo4dZbfOSX5bRU0pebQY3KyUrWDWbOC5Ci5MkgnyZiHlbyG0ou7-qUlvBlpUAGA8": "SCHNORR_SIG~SMe6zQmNqKSM6RFCNciVWqbWiteok6xToX8LUR0MMQJAogNsbcP4-m5MW_dckueY4GFm14FtYMp8G4gsbR2xEvxqK4Bh53hti1yGJMOyBNZL6W8FB2IM-mo2RNkA6lgG0Q",
                "SCHNORR_VER_KEY~9jdrweDWTZ6pJ5rTpDP46QO7sW5M6uDDrrL7SYinGhjWT1j2J9pbCwO5siXLJEzoFDlPpwu_9u9lExRi8PeHHMM": "SCHNORR_SIG~HrvZuxnfn2W0w36bmbADMcylPu2PcspgZOtazQtSewXoNYXJs5EBoOdIFzz1iNjwliM8-KEvrOQMhpwZ5iiaChtDDVtd91t5peFBUHSTF0pn_KKgOntl1Wx-so1zTLIMbg",
                "SCHNORR_VER_KEY~IUCHaCAd26VgcO3cvO8hqKgo07QhowFv163GcznMXg1mrCEgyM2OX_bTv_O1jWmPGz_IMEZ8QTrmiKxDxrbsDJc": "SCHNORR_SIG~YpClmLHZs654eFN5Vu1v1JUFCsN11d7HcRr41moG6wHkoPP_P5mI55EN1f3aGxwqE36WiRGBlmaJSx2-iv_RBdumM8TRrbfM78xc5Sd71ghtd6ahH3tKmqal6_1lArMKAw",
                "SCHNORR_VER_KEY~wdxx3PG61CQxLlm4tvbCbv_Vn7FPwDYDGSBW024UHBjbQi2W3OS76dGZI2xw3QmqI12oiN31d78iMXtzc232ExM": "SCHNORR_SIG~ReQ6_gYh7f0i4cui3_ZFT3zOuIuU9KGhwNyq5aq-pwVXGhaGv2OA26nUM393zSCDasPOCupd6FgQwgEPR3jpJfY6WCtppfUWxZisXSuwf-RcERXrzgZ3e6awLDdMye0rhg",
                "SCHNORR_VER_KEY~pxstRlYXI8x6KTV9s7mC1SYER3dCTqp7q74UwZ0YYxEFdDuhTWf4UYaNTQvoUJz3GG10nCtVNTs5oyjDiz85D9k": "SCHNORR_SIG~WEiiDxM8dqBtMZMCI35CbPchHaa1X950uo6-E_S8vQElB7BQRcBOm8QM3Br_8dgufBg-1xXidyBFCPFk0GHxLbgGoVzzQmZiVfgW8YkMNEHDys8C8Cm31dUaRKRlOL4s5g",
                "SCHNORR_VER_KEY~WW7AJ757PR18mUHXZ0GPtg1Juh2NHzj-s_JgN8eKmBCzhc-i_TZZSwLEgO4ddlo-q2B-jQE-xke_PKY45J3vAT8": "SCHNORR_SIG~rDzvVdf1t_I87Z_l_aTCz1QuypRPEBlwn4XFwP0rRgAXcMrXTAUdQKk__5SNGFCqCV3w1q8L8TsY9TjXbN6uExDEr8iAMkrMcxjF8p-I6VsUvSLKM05L3GArBXoNI30OJg",
                "SCHNORR_VER_KEY~8HBM6QZGOIDqVH2J-wtNPLv3nkNZe1VUN2_VBRvh6SM2-B4R5GaYW5jG-zMOm-ZA3p5mDkAZEBuY-c13Vwh7CyE": "SCHNORR_SIG~ulf700BBSYYERmMA27Bai7SZW889KZkZHxiGCA82TwUDt4U70riYo17Nx08v_GP3-YMlk0uALgAxmDdhhkjVLON56X0UAcF7T0qdpHQgEdqyop9pet19UPI5AgIid6cQlQ",
                "SCHNORR_VER_KEY~Dh42XhAHbEdYMWBmREBU4C3OdCnGHnyYw1rbJ9oWwS6sVdNNCSmlU9WfM-CsKPqeG7RtbUsyXbT5CcS6m9FgBD4": "SCHNORR_SIG~B5Pxk21rQHPEHepLEUnjWRMxyg2Xa7GBfxbndq2svQVe3VfWM7H2Fj0SMo02kUF0nTnUHjjH85P3vGOO65eLAdpuI8evbVG2z8g74lFEa9SXmCHEsUEY4ZvcKkU59WIcmg",
                "SCHNORR_VER_KEY~GRnCkfXSG2ffWcgbQFi9OwPTuy9IQHnzwmRSHWWZ-B9v7avzS5AeRXM6iVSyKd0eV_npHeusVqXSl2dkJSoyH6U": "SCHNORR_SIG~WCLKGYWcpWJlO03nJDTyLivOnTPE69LGb_2dVygbVgSceoFofs47r0mR4MpO71lV7t_NZ_xe9LM0Y-vRfJcECNd4kVhwhXKKXT5i_v_to_gFjudOQHQ5gVaNatbi1C4LGg",
                "SCHNORR_VER_KEY~uxqpGJKvtDPwJWqSKorrHZslAAhmkwrvlZVAUWKfCBBQI7-PEMWBtXjh11w_2ERNpgueSrfwf9ja8p9reJBRIDc": "SCHNORR_SIG~t39Uw1e3etyyoOy-I8A3eWGrKduq_aBpuWXKHmKcpAJVweHDsEollV8Y4BG3ye3jP8zFjUyf3cRD-7RVnjRGJaOJB6bYwcKjCyqkwO5PyoRuiOiF49LmvxBFXrMP3E0LvA",
                "SCHNORR_VER_KEY~EtHIlJ4VuOtzSxvvvvuMHUEcTQjgKbBVSA3Y0gFALQqf-zKeaXu1VxCZWVfKIcmNt6zGARf9mDZCQHWiP0_GDpg": "SCHNORR_SIG~quU3wrDdofNx3AMRU4l3XVsuYKwmxRgl1Rdve6YgWQTUkQeWzZlWAtIFeTLwj_wa3e0WiNhbbd7KHLRqFoNAAYaN2mCk6boKr453dZRvHbrPsqnk_LpqHFkR_Crn6jQcGg",
                "SCHNORR_VER_KEY~-Dq_4ixA6ilTVOMMEXzrFya36UsjwGmDKPWecT89Mw4YZ97l_9xA3F1wIkTd8qH6o2b_fq-pruymq_F51hxDJQA": "SCHNORR_SIG~ylA1ZKJr5wKmUpivVVkvM9hpdmUXkX7J5fBQPQEq_QGoSYu5iG2wXOIFo8ti3tFHRDTrELC3gXGbmhbDfRqPIuMCPSNsvzg1YhgZacPSz3ps4MEFH-mywSkYYxwIaqwNVQ",
                "SCHNORR_VER_KEY~N_5J4yU8lpvyZQVCuE6FW3JjlhSFFdbzXGp8sAiTViQygbdtlfTMAOL6WHBlsjAWNePSzkYzZf9D43oEw33sDVY": "SCHNORR_SIG~xRWSqItCn-cmKrO2abO-tCDjCMr_CzJhhKHpTcVtVALBCVTtFtVidmX4eaqWC94uMarO5TRPPNDpxjVwLGiuBsVW5_JYuRjABxD9mF85JELgrFCOf1yIuahuIaycCJUSKQ",
                "SCHNORR_VER_KEY~F78Xy8-5pcYAHGgqlULvCnlUi62tp89o64KDO1hCrRkEuFj3jWrVWEUUeXRLFC8cTmWlMHQlej3JPULbWf-yLys": "SCHNORR_SIG~5POCCNOIXlZ7tXRJiYY7laHm4wO_sd1CVT1PkmN_1wIR_5lxNvuCvrOQmG5C345VaEhTmQ73bSVE9U6Xk8fTB_Iw660gGHxbTakKq_vsWdm0BPnCVMOB6HLGGMpU90oC7Q"
            },
            "accumulated_weight": "0x22"
        }
        "#;
        let bundle: LCV1StateSignaturesBundle =
            serde_json::from_str(bundle_str).expect("Deserialization error.");
        for (vk, sig) in bundle.signatures.iter() {
            assert!(LCV1StateSignatureKey::verify_state_sig(
                vk,
                sig,
                &bundle.state
            ));
        }
    }

    #[test]
    fn test_lcv2_decaf_compatibility() {
        // This bundle is fetched from the Decaf on 2025-10-01.
        let bundle_str = r#"
        {
            "state": "LIGHT_CLIENT_STATE~ydNbAAAAAACiBVMAAAAAADJlVWXKIR6Qz9CviL2MhaqD26s0_1V4b7KSKeQcvjEp4Q",
            "next_stake": "STAKE_TABLE_STATE~yF8fZmItFOmLBhjX0-Ok4jvcvi38iSnwEcrKGadyIwwZkRD73Xhsf7j9KN72wRWLZKH5vlB4WLJHodOS-GVIIDkTeXP_1IyATn5FEh1R1SRXIPeqZu2VLgIIxjBTlL0rVlV1eTIzy72XBgAAAAAAAAAAAAAAAAAAAAAAAAAAAABc",
            "signatures": {
                "SCHNORR_VER_KEY~xgoBgBJgjGYxguWFkgAZ-y5gJ9D0uxWRfjw7hbl-EAn2FZQ1eCtQM1zPWb-NEzQNAcPYlQ9gey6FnX264RGSDy4": "SCHNORR_SIG~JL_jho563R0K9Bw7xRfRj1_1u3JttEktx1pI__6EjgVGZMAqzrqfGNlg6VOAGc8nwCJ3JCwBKZm8RfjB62kVCCY8zz9VTdNByGZgaS4u8vE_EpUenfa40XWao0zB-YAVTg",
                "SCHNORR_VER_KEY~fq7InLeaQTv2sHdhlQG8MEyCQv9qHBDgM5QdzTeNNCkZl6E1oG9hVVGnX65fLzDuRKGClMB6Qyug3dvkNSOmAZk": "SCHNORR_SIG~oKwfr4T_pNf4NuELFG6l_4F3DoGBe0apY-xtmpSKXgWxRdrr5XJgnubTZbicyhnqv_ehlVBEeba6fhRLdkw7CQh5Z8qSN250nADk0Rjjk1yExiJBMXjx8agi6aA8d00HgA",
                "SCHNORR_VER_KEY~KQMik17TU_UlpkGjkTe06-mUUfWCZb-IRe50zIH0FiH4AAUszBpPd6yDMy6BsWpU-DROG_wfZ5MkKNFg0KuUGoU": "SCHNORR_SIG~SJFPdEzSB-ir867kZWTtQ_4qLwO_qe5b3JstNy-GawDQs8SuT1_dmsSVmilCXCiN_xRg0EhFc_9p1eSH-OuSCRDqvYdfEGvADXcil6SQG9pOHR_ap_rME51_sIY8JIAXOQ",
                "SCHNORR_VER_KEY~lyNawzJ3meaEesJn3A4sX23H-ZqEFlxrjBOQK_cZhgLmYzlTsOs7sNhO7wxGZEeYIE4ndkZkH1mWx807k4eMGAc": "SCHNORR_SIG~B7k3RVeFvHCuo7riS_9s0JJZ05NdoiWXmdG07383JgDUIeQ-n87S25iWvRVVDyYBvCklZ2iB32wL376YjsbYK2YY05g4MeSLKRLbInCnF_gvdFFHMVKNY1E-KfrrMfgDCQ",
                "SCHNORR_VER_KEY~tVa4AyJiqc8HV-kibLCKAfTJMtwl_BNYP_vyoTUZgwXS4IHcYKa8YzXPdd-uO1JepllzEaM9k0L7qPBIFDutCOo": "SCHNORR_SIG~XvEie__MLke-dKrYAY00bwyCrw0KOJyKO4FSaXdUPgTKgDiRiilqC9Y0vOmFD7pTp202_5uWLMbwH7xi8I3WGmANwpbd3f7COSbKmtdvI9cJntaM-XJZiiJut7ZVDrUoPA",
                "SCHNORR_VER_KEY~LN9War8PX85SCBr0g-UEEV4AGD5XNcoTuyZspiBN3QlaCDKfwj2TV9FUaWM1vE-2RKrTBDQsL9SkC1jta2XdGec": "SCHNORR_SIG~1wwn73N63GW6oxPE_s5MsP6BHRqhvHtcCFaF4tY6Cgayh0fNfxKHJbcPLdg3d4Hx3gkW2-YhgUDT_Zq5iPgvIuz16EMJCT7wRvN5qU5txLEHdD4hep0_jQj1w-Nb6TQPAw",
                "SCHNORR_VER_KEY~s9XAIs0ru9lmPoPCAlmIBct1rpi3kuQep-RfGXj-JA0sPx0RLBGQklBqCVJ5TlFSdaa5gihSyjZkajNwWIRiBm0": "SCHNORR_SIG~DnwCUe63PVud8hHyYgKmndmx7fUGZK7mjMFAJqCidQRH7XKaQVF7aYjJ148hGyrtl5sRLOyghfoT01An0RBNMFVcK1Yu2e5mnvisiamnhB8xHpVQ-FSxB1-9RpOaObEcIA",
                "SCHNORR_VER_KEY~zCd6TrfWOBNdI4C47RNw-_GaGqCSmKbVC9NpVZLiTBZlABqf_Yr2vweGiuIwhewk2nbBZzkpx53TFHJnvSFdAHo": "SCHNORR_SIG~MS24gBRLE4n-T6IAGOql7TCsMPwMiE1wOV25kWvMYQDB2LD7FGhNoth9bgd6HFyrYSpeYg3ALqXyDvLjnVLFEqMlFOKVanNY1PFfp9z5Zn6peUbHG1gHzVDpN2Ld8gcjSA",
                "SCHNORR_VER_KEY~xPgp9Iu8fhK6LIamh6AYFohIXcphv-ncrUy-jb5ryiu2x3DJGpctKqUbUpz8iabHtqKGQXEjeDKBZ1sYC36oCy0": "SCHNORR_SIG~CMIFb_gecp_u_IO6qV-OCSeI2gkYdZeBapVk7XUo9wHbcIsynKYQWoNnWzBJtlogMzw9I6Tk_R3CbVqwXOCpJN87NJEq8tgC0FnKgCHM1h0Ncz-VUJNvsHaFJkVlko8fwA",
                "SCHNORR_VER_KEY~UmUJaKQOkjsFYm7uowVJgyxffU29gf1bjNymFV3SIgqX496XneDqKMwS2SxD2pYJAlipqLkkY29JB2r3XGsCEJs": "SCHNORR_SIG~KQtz3rDWFU6urmGLTo9Fnd-SFZnWPGN0uV82jMdsHQDFpqrgMFejjXDHPpx5ZTd3G6P15llrMHVQQ4j20hO_EFfV-o0OrlEl37BzvL8bFuLzUCDNFuKxi_8Xk6AiMNAE0A",
                "SCHNORR_VER_KEY~8_rNIOmPEvsPBvQS3P_CBE-yqt4UAMySp1rwP7HIUwRIXD1NzFa8kCAeGiW5BGyZocNz0Uw4GTSY3LvCV7TwG2s": "SCHNORR_SIG~Gdgkl4pmTtdnw8k62kYFNXG1NxwYq-uYHRLtofzqGwL8jNOqgVp4yHdp3EAcvVsE10Eo9x9uhgf-Q1yEixPjJlb6Y-0KI52e4Ak8YV4inF0airD2wxEHPNQCnwX8WjINgA",
                "SCHNORR_VER_KEY~njpgFPYOCSs8TbyL3ZNU08Kiv_uRIEtgXOf3FVNp-B_f6WzCAZvEyuJ4lt5COrKH7o5k3aAjNRpUaysItG8UIuA": "SCHNORR_SIG~QLaaA7rOFYhsrrmNNkesY7UJgIlKl6MSULpv2brRlgOcOkURa_XAJAPPnyQM1OncME_E2c0zgIvINycK_9ciIHbdUJqdMrzE-qMVBcKNxBvGpPsomYGBCbAwMufqS3YNUQ",
                "SCHNORR_VER_KEY~lpgX__UHWmtI0m6pj-ekaKz4qPxTjfTXhUB2kfqklySlVA5jh-z1DZGH3hheqkrM04HvoW89F2Jo9ch72AkhBPs": "SCHNORR_SIG~TbMgfX764013LA79ZLxbjr8uyTlFyJ6RDL7YJybtjwK__Mzlcbquy078_b8KcYiSI8j_nEZJV7ta9EzzVHVyE6c8j5RaTU1dGMcGLENkKt4DPoB94qX-6BQ6GfjBVHQWsA",
                "SCHNORR_VER_KEY~tBVAc5OGb1QHoliKo0J2fySvAerxtt5nEcTPKl64uQ3_u2rs2ZOOqO9QNkUCcU6fl1D91g2dBud4QXiFyAyiFd4": "SCHNORR_SIG~FpXl2jgmHMKYzs6lv8O8Z_BGQOIaZVVGgpbyXdmypwQt5B7NYksXJuyasQDgZ0UZgXO4sncSAV24fOXslXycEeafYLed8vvZCCpfYjre_-92YlFbJDsH7mcK9H68K40qlw",
                "SCHNORR_VER_KEY~0cmLKy5PUNin8EXLI0mfMT858KZ4-wV_u-fnJge0pRKQCfrxpXzJ_JzZA1l9oEIXaCrS0ZTrtk0GEK62CS_pI1o": "SCHNORR_SIG~7J0OyvvrSEWNkFnmJhdEo_Tl2fTkhdEC-RzaQjwStASVZ4g9oGuksTv-vroIJwjr54pDFMnoYtlIxWePkUBvAQHveBE6VmFYNgCJs7aM_KGOviRaD-PINKU2t7hs7OgX1A",
                "SCHNORR_VER_KEY~66TRKQkrP_L5rs1Z8At8Ny6Pv4GC2MpYoe4N7WE4WQ27wZ-CuhUXRP0jrYXuXO35cUB2tpv-3jYSYCJFX5KtKgM": "SCHNORR_SIG~F1HQHZrcFJr09AYkzpGd3OUq9MA6ShPw81JXhlr7NQHkP1VK-F0ew95932A1nydapJZUXho5eVTXlet1215CL1qaVsGkBqUGtTlPtZxTbGFDd31BbA42vXCXYzGIj9Uudw",
                "SCHNORR_VER_KEY~cDYJ0it01uLDI1PczMcbB8qSpt8APtPpgIP1WXYYKB5rV5KCHPhMmwwWDruxDfuKIWT81vxF1lEcmnOCQ4fFBwc": "SCHNORR_SIG~lyfJHUTDQnQVlaaJqeDJe2X59gf1hVZuxVqoN46PwwHd4EAcDJC_IOQEg41By6dMyW8D4ijocrLEYzI_AnEqCWKi2AAAMWLFOjosVOJ-9dUt2d_EK6e4uiDcRx6VopYnig",
                "SCHNORR_VER_KEY~5Ib0sFp7T9FMiYYkvkpkYd13hB-rvrAJxuPbWeVLvy29yBrJr1_g1xrDq4BEaSD_bX79USL-JnEfY6lVaYCBCUg": "SCHNORR_SIG~dV-2pwkCOWoseSfFEDhsZvqJCfg_GZUleUDdCg9EaAT3HNUfh35UVw2_6M5Go0DPkKixvzuNGfVtkhaf-4GOIOnzg6aArhxxruTjbbQTRzC73wJdVPyJzkQeBzsUdpoUXQ",
                "SCHNORR_VER_KEY~C216v8z9imh4otKDDr96IiSZ--wAlS8VMr46pHIfwy2aol7i-ca8HUArmw_ogyCPoqPKiA94Pq4hP-Lvj5NWApQ": "SCHNORR_SIG~QZxzwfHvxAXcnMRcn0af0Sfv4Tf3vzmxmFwW3kUQnQQgBkHhqPtrZw948gkHGP6eKQsMZhKczk2DVIB84UrHIhgPOjX2RAkac9xlJNsMPn9rNaWYkBG1oHvIFQts9T0lPw",
                "SCHNORR_VER_KEY~qmEqPaINrX-8t6aSCK7E97-UIJ6MlIQ0zB-KbILkLyxfdxfqfIWXJP_DqL_5nBudxhZB8J4nA0SIxc9g5bf9F9w": "SCHNORR_SIG~LsS1p_NFmeksQniWjOqZxlmAZtAPfWVNmTWRjay-NAJ2t1kFYZwD9s99kS7mlERXdnxo7kEKb8L3R4BKTJJZGnbSNEGoQA7GVYza1lMDpihTc3EzjXrYJy01bxBWy8AqmA",
                "SCHNORR_VER_KEY~EhZmVnzLNkiJUOBBYSJk79ltptddDjOUfCi3PwgOyBWT3IONE4ntSyxXd-u0f6zezqmDvDyEfqmTXYz_ybDLKag": "SCHNORR_SIG~xTipAqO0YjkA-I7saltNnRwbbYqfhAiLiQ9xazd8EQOHp7y90ZhOqjQw83F7w1XsXtGW6Mm38X1Mr59UmmEMHAHtVN7tKlu_X79tlUFRyH9kPfju2PDr5eKu0pbIQjcc3A",
                "SCHNORR_VER_KEY~Mbb4cfFTwEuzhmAvAp9-33Vc-itx5mqIw8x_wJmq6Q8lDiAgoyVW6-38gc8Oc4gYGFataSwtiomk0Y-EG6eQD2c": "SCHNORR_SIG~6Nw80gahQoyOxHfXbzoO3r_8CwsVEa1NiqbgzNGpMwN-AcH6mb-gc_KQ9a7nev4wNtg2zHCCFc_-lqd_um4aGOdhc_hsTCVniZUeSF83Yvc0bTtI5X-ePBI9I3FtKPUepQ",
                "SCHNORR_VER_KEY~8-2GgswvPu_bMe-bt4H8XxZlpRkv9VWQsmoFpc9IFiLHnXNKH1XGZN77lwlvQtcfsBk6dV0aKonFzwKm1fJ4JAo": "SCHNORR_SIG~p5W-oJhFNwH3SFHZkYn5rmNbt4NN-h8F4a7pTC0cxQCEPngKIrEuhHNe1e7NpsxBjv1lBHxIGOBN2f1sMZ_mLkYan2CcLArfiL2hVkGYoh7aoxucOhsb1s0acG25SyUg0g",
                "SCHNORR_VER_KEY~7anAcXpO122kjU_RXiTyd9cyLOzChxFv5I60J2He1Q3y1BUVxI-dZzTJjmmsqlKh7dQplOpnyj4PjRyWAxGnKMo": "SCHNORR_SIG~nivVoTHyZnU0Zlrp6TSHImLXpzuWYRzv_BtbvvreQQLV5jpKBB1C6BkchOctLI1Ih6opEtqgRV22RZ-8B-fWDRr8q_4DrbpQp2JdJT6z5QgQJVuXLGbXrRYumhqGUd4b2g",
                "SCHNORR_VER_KEY~wE9kEoMWep-5gcSR8At4ATl0aGScyAajJsITkmY1ihGNUOeArjVLwRnNQIQXu21akHYHDHZgZ4JojaHwDayPH_o": "SCHNORR_SIG~1gSGlEyh1SlhGEtGY6OBwAb3ZWJjFsrcQjnaK-CjZwNjmGsYPdZgJ9amRe2J17-ZlIgi9lDuaAzhNVwgQLb8Ki7HJkUClsBi7_LZGKlLdYZAZbyBFkJM6GpMAZaIJsUWIg",
                "SCHNORR_VER_KEY~1_UFa9Q_Km7mQJ2hN6gIcBtcOsJhqoaYPnfzZMvEPSiTsP0MFm-4F5SfEmLLOUrb57ZlpWe48qkAT8HZwRMBIdQ": "SCHNORR_SIG~WzFKsZA7Ak6B8ch3QNrTFnTqvTenSbXmzjuWs50cpgNE3mIOvKj8fzkuEvRldUDtNisgDK8EoOJYOgBCUTohIrvhg8tRE9TDhVFyC_VzXu3HeHOyXOUdTltdAs4EE7Md9g",
                "SCHNORR_VER_KEY~M3saSwxD2BEYTyDKGcJix-F7w9qJwN5ylbvTXYRzKip-EGrZL_e8sZElPYq_tdAKGaK0GLTQlz0ygLNsWYM8KFw": "SCHNORR_SIG~fPxgl5lXZR1UYG8y8FklPbJcQcwzErBJ87itoqsX6gFRAxflxhC-28TFgmkPEagSgt--PeIgjvH7G2vM4azKLxG5KwIJint9lCN5-nllFsdrTubNMh2CGB9T5Yhvu1YsfA",
                "SCHNORR_VER_KEY~9mW7p3nVhYnosVwMfI6j_fAb71A8dICctX5n5nmgOSm729BIX8nA57syKK-t-c9nOYjFxJNILJQia9xBVxXSLnA": "SCHNORR_SIG~CB_9AULM5Y5yz6-pzYZeZUBaPSbo0Cp_v_jNUAcyYwMNnRbCEvfRhzG0kcdLEHWDz7MHiHUNI4rJy6SFUBHpEF-hbfcuKaQWEBgWrfDJbJMmBuiUGJOPOy2Y0BcoulMJ0Q",
                "SCHNORR_VER_KEY~Ec-zbsu8xGuDZhSNLDMaiosDa_iSHpEo0poEDbJqsQ3Bm47MCMaRFFHHFB5D-L-uOuGupqea8IP7XvEs1Pf7D8M": "SCHNORR_SIG~Jbomh7RujM0JlY2mez6a_jVnwXRu6xHCNkvDBFMAkAFPHRU3W3PUt4-US_sGArZQV9-2flULLD3yT6V-qYLfAX2vdyOag2n23PEB4j6x-2pNvivbV-fvnocpbpT0_38LQA"
            },
            "accumulated_weight": "0x6c6b935b8bbd4000000"
        }
        "#;
        let bundle: LCV2StateSignaturesBundle =
            serde_json::from_str(bundle_str).expect("Deserialization error.");
        for (vk, sig) in bundle.signatures.iter() {
            assert!(LCV2StateSignatureKey::verify_state_sig(
                vk,
                sig,
                &bundle.state,
                &bundle.next_stake,
            ));
        }
    }
}
