use std::fmt;

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use anyhow::{Result, anyhow, bail};
use clap::Args;
use espresso_keyset::KeySet;
use hotshot_types::{light_client::StateKeyPair, x25519};

use crate::{BLSKeyPair, BLSPrivKey, StateSignKey};

/// The Espresso node key mnemonic, an alternative to passing the validator's keys individually.
///
/// Distinct from the Ethereum wallet mnemonic `--mnemonic`, which derives the L1 account that
/// signs and pays for transactions. This mnemonic derives the validator's BLS, Schnorr state and
/// x25519 keys, with the same derivation `espresso-node` performs at startup, so a single mnemonic
/// configures both the node and its on-chain registration.
///
/// Only accepted as a flag or environment variable. Unlike the Ethereum wallet, it is never read
/// from or written to the config file.
#[derive(Args, Clone, Default)]
pub struct EspressoKeyArgs {
    /// BIP-39 mnemonic the validator's Espresso keys are derived from.
    ///
    /// Keys passed individually take precedence over the ones this derives.
    #[clap(long, env = "ESPRESSO_NODE_KEY_MNEMONIC")]
    pub espresso_mnemonic: Option<String>,

    /// Keyset index to derive from `--espresso-mnemonic`. Defaults to 0.
    #[clap(long, env = "ESPRESSO_NODE_KEY_INDEX")]
    pub espresso_key_index: Option<u64>,
}

impl EspressoKeyArgs {
    /// The BLS and Schnorr key pairs, from the individual keys if both are given, otherwise
    /// derived from the mnemonic.
    pub fn key_pairs(
        &self,
        consensus_private_key: Option<BLSPrivKey>,
        state_private_key: Option<StateSignKey>,
    ) -> Result<(BLSKeyPair, StateKeyPair)> {
        match (consensus_private_key, state_private_key) {
            (Some(consensus), Some(state)) => {
                Ok((consensus.into(), StateKeyPair::from_sign_key(state)))
            },
            (consensus, state) => match self.key_set()? {
                Some(keys) => Ok((keys.staking.into(), StateKeyPair::from_sign_key(keys.state))),
                None if consensus.is_none() => {
                    bail!("--consensus-private-key or --espresso-mnemonic is required")
                },
                None if state.is_none() => {
                    bail!("--state-private-key or --espresso-mnemonic is required")
                },
                None => unreachable!("one of the keys is missing"),
            },
        }
    }

    /// The x25519 public key, from the individual key if given, otherwise derived from the
    /// mnemonic. `None` when neither is available.
    pub fn resolve_x25519_key(
        &self,
        x25519_key: Option<x25519::PublicKey>,
    ) -> Result<Option<x25519::PublicKey>> {
        match x25519_key {
            Some(key) => Ok(Some(key)),
            None => Ok(self.key_set()?.map(|keys| keys.x25519.into())),
        }
    }

    /// The keys the mnemonic derives, or `None` if no mnemonic was given.
    fn key_set(&self) -> Result<Option<KeySet>> {
        let Some(phrase) = &self.espresso_mnemonic else {
            return Ok(None);
        };
        // The phrase must not reach the error, which is printed to stderr and ends up in shell
        // history, CI logs and journals.
        let mnemonic = Mnemonic::<English>::new_from_phrase(phrase)
            .map_err(|_| anyhow!("--espresso-mnemonic is not a valid BIP-39 mnemonic"))?;
        Ok(Some(KeySet::from_mnemonic(
            mnemonic,
            self.espresso_key_index,
        )?))
    }
}

impl fmt::Debug for EspressoKeyArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EspressoKeyArgs")
            .field(
                "espresso_mnemonic",
                &self.espresso_mnemonic.as_ref().map(|_| "***"),
            )
            .field("espresso_key_index", &self.espresso_key_index)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use hotshot_types::{signature_key::BLSPubKey, traits::signature_key::SignatureKey as _};

    use super::*;
    use crate::DEV_MNEMONIC;

    fn args(index: Option<u64>) -> EspressoKeyArgs {
        EspressoKeyArgs {
            espresso_mnemonic: Some(DEV_MNEMONIC.into()),
            espresso_key_index: index,
        }
    }

    fn other_keys() -> (BLSPrivKey, StateSignKey) {
        let keys = args(Some(1)).key_set().unwrap().unwrap();
        (keys.staking, keys.state)
    }

    #[test]
    fn no_mnemonic_derives_nothing() {
        assert!(EspressoKeyArgs::default().key_set().unwrap().is_none());
        assert!(
            EspressoKeyArgs::default()
                .resolve_x25519_key(None)
                .unwrap()
                .is_none()
        );
    }

    /// The keys must match what `espresso-node` derives from the same mnemonic and index,
    /// otherwise the registered validator record would not match the running node.
    #[test]
    fn matches_node_derivation() {
        let (consensus, state) = args(Some(20)).key_pairs(None, None).unwrap();
        let keyset = KeySet::from_mnemonic(DEV_MNEMONIC.parse().unwrap(), Some(20)).unwrap();

        assert_eq!(
            BLSPubKey::from(consensus.ver_key()),
            BLSPubKey::from_private(&keyset.staking)
        );
        assert_eq!(
            state.ver_key(),
            StateKeyPair::from_sign_key(keyset.state).ver_key()
        );
        assert_eq!(
            args(Some(20)).resolve_x25519_key(None).unwrap(),
            Some(keyset.x25519.into())
        );
    }

    #[test]
    fn index_changes_keys() {
        let zero = args(None).resolve_x25519_key(None).unwrap();
        let explicit_zero = args(Some(0)).resolve_x25519_key(None).unwrap();
        let one = args(Some(1)).resolve_x25519_key(None).unwrap();

        assert_eq!(zero, explicit_zero);
        assert_ne!(zero, one);
    }

    /// Individually passed keys win over the mnemonic, matching `espresso-keyset`.
    #[test]
    fn individual_keys_take_precedence() {
        let (consensus, state) = other_keys();
        let (from_args, _) = args(None)
            .key_pairs(Some(consensus.clone()), Some(state))
            .unwrap();

        assert_eq!(
            BLSPubKey::from(from_args.ver_key()),
            BLSPubKey::from_private(&consensus)
        );

        let derived = args(Some(1)).resolve_x25519_key(None).unwrap().unwrap();
        assert_eq!(
            args(None).resolve_x25519_key(Some(derived)).unwrap(),
            Some(derived)
        );
    }

    /// One key alone is not enough, and without a mnemonic the missing one is named.
    #[test]
    fn partial_keys_without_mnemonic_fail() {
        let (consensus, state) = other_keys();
        let empty = EspressoKeyArgs::default();

        assert!(
            empty
                .key_pairs(Some(consensus), None)
                .unwrap_err()
                .to_string()
                .contains("--state-private-key")
        );
        assert!(
            empty
                .key_pairs(None, Some(state))
                .unwrap_err()
                .to_string()
                .contains("--consensus-private-key")
        );
    }

    /// A partial key set falls back to the mnemonic for both keys rather than mixing sources.
    #[test]
    fn partial_keys_fall_back_to_mnemonic() {
        let (consensus, _) = other_keys();
        let (from_args, _) = args(Some(20)).key_pairs(Some(consensus), None).unwrap();
        let keyset = KeySet::from_mnemonic(DEV_MNEMONIC.parse().unwrap(), Some(20)).unwrap();

        assert_eq!(
            BLSPubKey::from(from_args.ver_key()),
            BLSPubKey::from_private(&keyset.staking)
        );
    }

    #[test]
    fn invalid_mnemonic_is_not_echoed() {
        let bad = "test test test test test test test test test test test wrongword";
        let err = EspressoKeyArgs {
            espresso_mnemonic: Some(bad.into()),
            espresso_key_index: None,
        }
        .key_set()
        .unwrap_err()
        .to_string();

        assert!(!err.contains("wrongword"), "{err}");
        assert!(err.contains("--espresso-mnemonic"), "{err}");
    }

    #[test]
    fn debug_redacts_mnemonic() {
        let debug = format!("{:?}", args(None));
        assert!(!debug.contains("junk"), "{debug}");
        assert!(debug.contains("***"), "{debug}");
    }
}
