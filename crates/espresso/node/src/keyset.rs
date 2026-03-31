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
    /// The file should follow the .env format, with two keys:
    /// * ESPRESSO_NODE_PRIVATE_STAKING_KEY
    /// * ESPRESSO_NODE_PRIVATE_STATE_KEY
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
            if x25519.is_none() {
                x25519 = Some(read_from_key_file(
                    &vars,
                    "ESPRESSO_NODE_PRIVATE_X25519_KEY",
                )?);
            }
        }

        let (Some(staking), Some(state), Some(x25519)) = (staking, state, x25519) else {
            bail!("neither mnemonic, key file nor full set of private keys was provided")
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
