use std::{collections::HashMap, path::PathBuf};

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use anyhow::Context;
use clap::Parser;
use derivative::Derivative;
use hotshot_types::{
    light_client::{StateKeyPair, StateSignKey},
    signature_key::{BLSPrivKey, BLSPubKey},
    traits::signature_key::SignatureKey,
    x25519,
};
use tagged_base64::TaggedBase64;

/// Keys can be specified in one of three ways:
/// * A mnemonic phrase
/// * A path to a key file
/// * Individual private keys set via environment variable
///
/// Note that the third option always takes precedence, but if not all keys are specified explicitly
/// in this way, one of the first two options may be used to generate the remaining keys.
#[derive(Clone, Derivative, Parser)]
#[derivative(Debug)]
pub struct KeySetOptions {
    /// Mnemonic phrase used to generate keys.
    #[clap(
        long,
        name = "MNEMONIC",
        env = "ESPRESSO_NODE_KEY_MNEMONIC",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub mnemonic: Option<Mnemonic<English>>,

    /// Optional index to enable generating multiple keysets from the same mnemonic.
    #[clap(long, env = "ESPRESSO_NODE_KEY_INDEX", requires = "MNEMONIC")]
    pub index: Option<u64>,

    /// Path to file containing private keys.
    ///
    /// The file should follow the .env format, with keys:
    /// * ESPRESSO_NODE_PRIVATE_STAKING_KEY
    /// * ESPRESSO_NODE_PRIVATE_STATE_KEY
    /// * ESPRESSO_NODE_PRIVATE_X25519_KEY
    ///
    /// Appropriate key files can be generated with the `keygen` utility program.
    #[clap(
        long,
        name = "KEY_FILE",
        env = "ESPRESSO_NODE_KEY_FILE",
        conflicts_with = "MNEMONIC"
    )]
    pub key_file: Option<PathBuf>,

    /// Private staking key.
    ///
    /// This can be used as an alternative to MNEMONIC or KEY_FILE.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRIVATE_STAKING_KEY",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub private_staking_key: Option<TaggedBase64>,

    /// Private state signing key.
    ///
    /// This can be used as an alternative to MNEMONIC or KEY_FILE.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRIVATE_STATE_KEY",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub private_state_key: Option<TaggedBase64>,

    /// Private x25519 key.
    ///
    /// This can be used as an alternative to MNEMONIC or KEY_FILE.
    #[clap(
        long,
        env = "ESPRESSO_NODE_PRIVATE_X25519_KEY",
        conflicts_with = "KEY_FILE"
    )]
    #[derivative(Debug = "ignore")]
    pub private_x25519_key: Option<TaggedBase64>,
}

impl KeySetOptions {
    /// Resolve the staking key without requiring state or network keys.
    pub fn staking_key(self) -> anyhow::Result<BLSPrivKey> {
        self.resolve()?
            .staking
            .context("missing private staking key")
    }

    /// Resolve the state signing key without requiring staking or network keys.
    pub fn state_key(self) -> anyhow::Result<StateSignKey> {
        self.resolve()?.state.context("missing private state key")
    }

    /// Resolve the persistent network key without requiring signing keys.
    pub fn x25519_key(self) -> anyhow::Result<x25519::SecretKey> {
        self.resolve()?.x25519.context(MISSING_X25519_KEY)
    }
}

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct KeySet {
    #[derivative(Debug = "ignore")]
    pub staking: BLSPrivKey,
    #[derivative(Debug = "ignore")]
    pub state: StateSignKey,
    #[derivative(Debug = "ignore")]
    pub x25519: x25519::SecretKey,
}

impl KeySet {
    /// Derive a keyset from a mnemonic, at `index` or 0.
    pub fn from_mnemonic(mnemonic: Mnemonic<English>, index: Option<u64>) -> anyhow::Result<Self> {
        Self::try_from(KeySetOptions {
            mnemonic: Some(mnemonic),
            index,
            key_file: None,
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        })
    }
}

impl TryFrom<KeySetOptions> for KeySet {
    type Error = anyhow::Error;

    fn try_from(opt: KeySetOptions) -> Result<Self, Self::Error> {
        let keys = opt.resolve()?;
        Ok(Self {
            staking: keys.staking.context("missing private staking key")?,
            state: keys.state.context("missing private state key")?,
            x25519: keys.x25519.context(MISSING_X25519_KEY)?,
        })
    }
}

const MISSING_X25519_KEY: &str =
    "missing persistent x25519 key; configure ESPRESSO_NODE_PRIVATE_X25519_KEY, include it in \
     ESPRESSO_NODE_KEY_FILE, or derive it with ESPRESSO_NODE_KEY_MNEMONIC";

struct ResolvedKeys {
    staking: Option<BLSPrivKey>,
    state: Option<StateSignKey>,
    x25519: Option<x25519::SecretKey>,
}

impl KeySetOptions {
    fn resolve(self) -> anyhow::Result<ResolvedKeys> {
        let opt = self;
        // If any of the keys are set explicitly, those take precedence.
        let mut staking = opt
            .private_staking_key
            .map(BLSPrivKey::try_from)
            .transpose()
            .context("parsing private staking key")?;
        let mut state = opt
            .private_state_key
            .map(StateSignKey::try_from)
            .transpose()
            .context("parsing private state key")?;
        let mut x25519 = opt
            .private_x25519_key
            .map(x25519::SecretKey::try_from)
            .transpose()
            .context("parsing private x25519 key")?;

        // If provided, a mnemonic or key file can be used to fill in missing keys.
        if let Some(mnemonic) = opt.mnemonic {
            let entropy = mnemonic.to_seed(None).context("invalid mnemonic")?;
            let index = opt.index.unwrap_or_default();
            if staking.is_none() {
                let seed = blake3::derive_key("espresso staking key", &entropy);
                staking = Some(BLSPubKey::generated_from_seed_indexed(seed, index).1);
            }
            if state.is_none() {
                let seed = blake3::derive_key("espresso state key", &entropy);
                state = Some(
                    StateKeyPair::generate_from_seed_indexed(seed, index)
                        .0
                        .sign_key(),
                );
            }
            if x25519.is_none() {
                let seed = blake3::derive_key("espresso x25519 key", &entropy);
                x25519 = Some(
                    x25519::Keypair::generated_from_seed_indexed(seed, index)
                        .context("generating x25519 key from mnemonic")?
                        .secret_key(),
                );
            }
        } else if let Some(path) = &opt.key_file {
            let vars = dotenvy::from_path_iter(path)
                .context("reading key file")?
                .collect::<Result<HashMap<_, _>, _>>()
                .context("reading key file")?;
            if staking.is_none() {
                staking = read_from_key_file(&vars, "ESPRESSO_NODE_PRIVATE_STAKING_KEY")?;
            }
            if state.is_none() {
                state = read_from_key_file(&vars, "ESPRESSO_NODE_PRIVATE_STATE_KEY")?;
            }
            if x25519.is_none() {
                x25519 = read_from_key_file(&vars, "ESPRESSO_NODE_PRIVATE_X25519_KEY")?;
            }
        }

        Ok(ResolvedKeys {
            staking,
            state,
            x25519,
        })
    }
}

fn read_from_key_file<
    T: TryFrom<TaggedBase64, Error: Send + Sync + std::error::Error + 'static>,
>(
    vars: &HashMap<String, String>,
    key: &str,
) -> anyhow::Result<Option<T>> {
    lookup_key_file_var(vars, key)
        .map(|val| {
            TaggedBase64::parse(val)
                .context(format!("key file has malformed {key}"))?
                .try_into()
                .context(format!("key file has malformed {key}"))
        })
        .transpose()
}

/// Look up a key file variable by new name, falling back to the deprecated
/// `ESPRESSO_SEQUENCER_` prefix if the new name is not found.
fn lookup_key_file_var<'a>(vars: &'a HashMap<String, String>, key: &str) -> Option<&'a String> {
    vars.get(key).or_else(|| {
        let legacy = key.replacen("ESPRESSO_NODE_", "ESPRESSO_SEQUENCER_", 1);
        vars.get(&legacy).inspect(|_| {
            tracing::warn!("Key file uses deprecated {legacy}, please rename to {key}");
        })
    })
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn generate_keys() -> KeySet {
        KeySet::from_mnemonic(Mnemonic::new(&mut rand::rngs::OsRng), None).unwrap()
    }

    fn staking_tb64(keys: &KeySet) -> TaggedBase64 {
        keys.staking.to_tagged_base64().unwrap()
    }

    fn state_tb64(keys: &KeySet) -> TaggedBase64 {
        StateKeyPair::from_sign_key(keys.state.clone())
            .sign_key_ref()
            .to_tagged_base64()
            .unwrap()
    }

    fn x25519_tb64(keys: &KeySet) -> TaggedBase64 {
        TaggedBase64::try_from(keys.x25519.clone()).unwrap()
    }

    fn write_key_file(lines: &[String]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{line}").unwrap();
        }
        f
    }

    #[test]
    fn mnemonic_network_key_is_stable_and_explicit_key_takes_precedence() {
        let mnemonic = Mnemonic::<English>::new_from_phrase(
            "test test test test test test test test test test test junk",
        )
        .unwrap();
        let derived = KeySet::from_mnemonic(mnemonic.clone(), Some(7)).unwrap();
        let opts = KeySetOptions {
            mnemonic: Some(mnemonic),
            index: Some(7),
            key_file: None,
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        assert_eq!(opts.clone().x25519_key().unwrap(), derived.x25519);
        assert_eq!(
            KeySet::try_from(opts.clone()).unwrap().x25519,
            derived.x25519
        );
        let explicit = generate_keys();
        let opts = KeySetOptions {
            private_x25519_key: Some(x25519_tb64(&explicit)),
            ..opts
        };
        assert_eq!(KeySet::try_from(opts).unwrap().x25519, explicit.x25519);
    }

    #[test]
    fn staking_only_configuration_supports_telemetry() {
        let keys = generate_keys();
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: None,
            private_staking_key: Some(staking_tb64(&keys)),
            private_state_key: None,
            private_x25519_key: None,
        };
        assert_eq!(opts.clone().staking_key().unwrap(), keys.staking);
        assert!(KeySet::try_from(opts).is_err());
    }

    #[test]
    fn env_vars_without_x25519_fails() {
        let keys = generate_keys();
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: None,
            private_staking_key: Some(staking_tb64(&keys)),
            private_state_key: Some(state_tb64(&keys)),
            private_x25519_key: None,
        };
        assert_eq!(opts.clone().staking_key().unwrap(), keys.staking);
        assert_eq!(opts.clone().state_key().unwrap(), keys.state);
        assert!(
            KeySet::try_from(opts)
                .unwrap_err()
                .to_string()
                .contains("missing persistent x25519 key")
        );
    }

    #[test]
    fn env_vars_with_x25519_uses_provided() {
        let keys = generate_keys();
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: None,
            private_staking_key: Some(staking_tb64(&keys)),
            private_state_key: Some(state_tb64(&keys)),
            private_x25519_key: Some(x25519_tb64(&keys)),
        };
        let result = KeySet::try_from(opts).unwrap();
        assert_eq!(result.x25519, keys.x25519);
    }

    #[test]
    fn key_file_without_x25519_fails() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!("ESPRESSO_NODE_PRIVATE_STAKING_KEY={}", staking_tb64(&keys)),
            format!("ESPRESSO_NODE_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
        ]);
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        assert_eq!(opts.clone().staking_key().unwrap(), keys.staking);
        assert_eq!(opts.clone().state_key().unwrap(), keys.state);
        assert!(
            KeySet::try_from(opts)
                .unwrap_err()
                .to_string()
                .contains("missing persistent x25519 key")
        );
    }

    #[test]
    fn key_file_with_x25519_uses_provided() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!("ESPRESSO_NODE_PRIVATE_STAKING_KEY={}", staking_tb64(&keys)),
            format!("ESPRESSO_NODE_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
            format!("ESPRESSO_NODE_PRIVATE_X25519_KEY={}", x25519_tb64(&keys)),
        ]);
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        let result = KeySet::try_from(opts).unwrap();
        assert_eq!(result.x25519, keys.x25519);
    }

    #[test]
    fn explicit_network_key_overrides_key_file() {
        let file_keys = generate_keys();
        let explicit = generate_keys();
        let f = write_key_file(&[
            format!(
                "ESPRESSO_NODE_PRIVATE_STAKING_KEY={}",
                staking_tb64(&file_keys)
            ),
            format!("ESPRESSO_NODE_PRIVATE_STATE_KEY={}", state_tb64(&file_keys)),
            format!(
                "ESPRESSO_NODE_PRIVATE_X25519_KEY={}",
                x25519_tb64(&file_keys)
            ),
        ]);
        let keys = KeySet::try_from(KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: Some(x25519_tb64(&explicit)),
        })
        .unwrap();
        assert_eq!(keys.staking, file_keys.staking);
        assert_eq!(keys.state, file_keys.state);
        assert_eq!(keys.x25519, explicit.x25519);
    }

    #[test]
    fn key_file_with_malformed_x25519_fails() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!("ESPRESSO_NODE_PRIVATE_STAKING_KEY={}", staking_tb64(&keys)),
            format!("ESPRESSO_NODE_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
            "ESPRESSO_NODE_PRIVATE_X25519_KEY=not-a-valid-key".to_string(),
        ]);
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        assert!(
            KeySet::try_from(opts)
                .unwrap_err()
                .to_string()
                .contains("malformed")
        );
    }

    #[test]
    fn key_file_with_legacy_sequencer_names() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!(
                "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY={}",
                staking_tb64(&keys)
            ),
            format!("ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
            format!(
                "ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY={}",
                x25519_tb64(&keys)
            ),
        ]);
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        let result = KeySet::try_from(opts).unwrap();
        assert_eq!(result.staking, keys.staking);
        assert_eq!(result.x25519, keys.x25519);
    }
}
