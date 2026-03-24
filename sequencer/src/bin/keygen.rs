//! Utility program to generate keypairs

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use alloy::{hex, signers::local::coins_bip39::Mnemonic};
use clap::{Parser, ValueEnum};
use derive_more::Display;
use hotshot::types::SignatureKey;
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSPubKey, x25519};
use rand::{SeedableRng, rngs::StdRng};
use sequencer::keyset::{KeySet, KeySetOptions};
use sequencer_utils::logging;
use tagged_base64::TaggedBase64;
use tracing::info_span;

#[derive(Clone, Copy, Debug, Display, Default, ValueEnum)]
enum Scheme {
    #[default]
    #[display("all")]
    All,
    #[display("bls")]
    Bls,
    #[display("schnorr")]
    Schnorr,
    #[display("x25519")]
    X25519,
}

impl Scheme {
    fn r#gen(self, keys: &KeySet, output: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Self::All => {
                Self::Bls.r#gen(keys, output)?;
                Self::Schnorr.r#gen(keys, output)?;
                Self::X25519.r#gen(keys, output)?;
            },
            Self::Bls => {
                let priv_key = &keys.staking;
                let pub_key = BLSPubKey::from_private(priv_key);
                let priv_key = priv_key.to_tagged_base64()?;
                writeln!(output, "ESPRESSO_SEQUENCER_PUBLIC_STAKING_KEY={pub_key}")?;
                writeln!(output, "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY={priv_key}")?;
                tracing::info!(%pub_key, "generated staking key")
            },
            Self::Schnorr => {
                let key_pair = StateKeyPair::from_sign_key(keys.state.clone());
                let priv_key = key_pair.sign_key_ref().to_tagged_base64()?;
                writeln!(
                    output,
                    "ESPRESSO_SEQUENCER_PUBLIC_STATE_KEY={}",
                    key_pair.ver_key()
                )?;
                writeln!(output, "ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY={priv_key}")?;
                tracing::info!(pub_key = %key_pair.ver_key(), "generated state key");
            },
            Self::X25519 => {
                let kp = x25519::Keypair::from(keys.x25519.clone());
                let sk = TaggedBase64::try_from(kp.secret_key())?;
                let pk = kp.public_key();
                writeln!(output, "ESPRESSO_SEQUENCER_PUBLIC_X25519_KEY={pk}")?;
                writeln!(output, "ESPRESSO_SEQUENCER_PRIVATE_X25519_KEY={sk}")?;
                tracing::info!(pub_key = %pk, "generated x25519 key");
            },
        }
        Ok(())
    }
}

/// Utility program to generate keypairs
///
/// With no options, this program generates the keys needed to run a single instance of the Espresso
/// sequencer. Options can be given to control the number or type of keys generated.
///
/// Generated secret keys are written to a file in .env format, which can directly be used to
/// configure a sequencer node. Public information about the generated keys is printed to stdout.
#[derive(Clone, Debug, Parser)]
struct Options {
    #[clap(flatten)]
    key_options: KeySetOptions,

    /// Signature scheme to generate.
    ///
    /// Sequencer nodes require both a BLS key (called the staking key) and a Schnorr key (called
    /// the state key). By default, this program generates these keys in pairs, to make it easy to
    /// configure sequencer nodes, but this option can be specified to generate keys for only one of
    /// the signature schemes.
    #[clap(long, default_value = "all")]
    scheme: Scheme,

    /// Number of setups to generate.
    ///
    /// Default is 1.
    #[clap(long, short = 'n', name = "N", default_value = "1")]
    num: usize,

    /// Write private keys to .env files under DIR.
    ///
    /// DIR must be a directory. If it does not exist, one will be created. Private key setups will
    /// be written to files immediately under DIR, with names like 0.env, 1.env, etc. for 0 through
    /// N - 1. The random seed used to generate the keys will also be included
    /// in the .env file as comment at the top
    /// If not provided, keys will be printed to stdout.
    #[clap(short, long, name = "OUT")]
    out: Option<PathBuf>,

    #[clap(flatten)]
    logging: logging::Config,
}

fn main() -> anyhow::Result<()> {
    let mut opts = Options::parse();
    opts.logging.init();

    tracing::debug!(
        "Generating {} keypairs with scheme {}",
        opts.num,
        opts.scheme
    );

    if opts.key_options.mnemonic.is_none() {
        opts.key_options.mnemonic = Some(Mnemonic::new(&mut StdRng::from_entropy()));
    }

    if let Some(ref out_dir) = opts.out {
        fs::create_dir_all(out_dir)?;
    }

    for index in 0..opts.num {
        opts.key_options.index = Some(index as u64);

        let span = info_span!("gen", index);
        let _enter = span.enter();
        tracing::info!("generating new key set");

        let mut output = if let Some(ref out_dir) = opts.out {
            let path = out_dir.join(format!("{index}.env"));
            let mut file = File::options()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&path)?;

            // Write the mnemonic and index as a comment at the top
            if let Some(mnemonic) = &opts.key_options.mnemonic {
                writeln!(file, "# Mnemonic: {}", hex::encode(mnemonic.to_phrase()))?;
                writeln!(file, "# Index: {index}")?;
            }
            Box::new(file) as Box<dyn Write>
        } else {
            Box::new(std::io::stdout())
        };

        let keys = KeySet::try_from(opts.key_options.clone())?;
        opts.scheme.r#gen(&keys, &mut output)?;

        if let Some(ref out_dir) = opts.out {
            tracing::info!(
                "private keys written to {}",
                out_dir.join(format!("{index}.env")).display()
            );
        }
    }

    Ok(())
}
