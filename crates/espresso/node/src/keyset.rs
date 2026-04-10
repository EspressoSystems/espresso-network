use std::{collections::HashMap, path::PathBuf};

use alloy::signers::local::coins_bip39::{English, Mnemonic};
use anyhow::{Context, bail};
use clap::Parser;
use derivative::Derivative;
use hotshot::types::{BLSPrivKey, BLSPubKey, SignatureKey};
use hotshot_types::{
    light_client::{StateKeyPair, StateSignKey},
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
    /// * ESPRESSO_NODE_PRIVATE_X25519_KEY (optional)
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

impl TryFrom<KeySetOptions> for KeySet {
    type Error = anyhow::Error;

    fn try_from(opt: KeySetOptions) -> Result<Self, Self::Error> {
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
                staking = Some(read_from_key_file(
                    &vars,
                    "ESPRESSO_NODE_PRIVATE_STAKING_KEY",
                )?);
            }
            if state.is_none() {
                state = Some(read_from_key_file(
                    &vars,
                    "ESPRESSO_NODE_PRIVATE_STATE_KEY",
                )?);
            }
            // Inlined instead of using read_from_key_file because we need to tolerate
            // a missing key (falls through to random generation) but still fail on malformed.
            if x25519.is_none()
                && let Some(raw) = vars.get("ESPRESSO_NODE_PRIVATE_X25519_KEY")
            {
                x25519 = Some(
                    TaggedBase64::parse(raw)
                        .and_then(|tb64| tb64.try_into())
                        .context("key file has malformed ESPRESSO_NODE_PRIVATE_X25519_KEY")?,
                );
            }
        }

        let (Some(staking), Some(state)) = (staking, state) else {
            bail!("neither mnemonic, key file nor full set of private keys was provided")
        };

        // TODO: remove this fallback once the network upgrades to CLIQUENET_VERSION and x25519
        // keys become required. For now, generate a random key so existing deployments without an
        // x25519 key configured can still start.
        let x25519 = match x25519 {
            Some(key) => key,
            None => {
                tracing::warn!(
                    "No x25519 key provided, generating a random ephemeral key. A persistent key \
                     (via ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY or mnemonic) will be required for \
                     the Cliquenet protocol upgrade."
                );
                x25519::Keypair::generate()
                    .context("generating random x25519 key")?
                    .secret_key()
            },
        };

        Ok(Self {
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
    env: &str,
) -> anyhow::Result<T> {
    TaggedBase64::parse(vars.get(env).context(format!("key file missing {env}"))?)
        .context(format!("key file has malformed {env}"))?
        .try_into()
        .context(format!("key file has malformed {env}"))
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn generate_keys() -> KeySet {
        let mnemonic: Mnemonic<English> = Mnemonic::new(&mut rand::rngs::OsRng);
        KeySet::try_from(KeySetOptions {
            mnemonic: Some(mnemonic),
            index: None,
            key_file: None,
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        })
        .unwrap()
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
    fn env_vars_without_x25519_succeeds() {
        let keys = generate_keys();
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: None,
            private_staking_key: Some(staking_tb64(&keys)),
            private_state_key: Some(state_tb64(&keys)),
            private_x25519_key: None,
        };
        KeySet::try_from(opts).unwrap();
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
    fn key_file_without_x25519_succeeds() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!(
                "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY={}",
                staking_tb64(&keys)
            ),
            format!("ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
        ]);
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: Some(f.path().to_path_buf()),
            private_staking_key: None,
            private_state_key: None,
            private_x25519_key: None,
        };
        KeySet::try_from(opts).unwrap();
    }

    #[test]
    fn key_file_with_x25519_uses_provided() {
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
        assert_eq!(result.x25519, keys.x25519);
    }

    #[test]
    fn key_file_with_malformed_x25519_fails() {
        let keys = generate_keys();
        let f = write_key_file(&[
            format!(
                "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY={}",
                staking_tb64(&keys)
            ),
            format!("ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY={}", state_tb64(&keys)),
            "ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY=not-a-valid-key".to_string(),
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
}
