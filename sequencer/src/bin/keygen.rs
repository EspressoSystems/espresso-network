//! Utility program to generate keypairs

use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use alloy::hex;
use anyhow::anyhow;
use clap::{Parser, ValueEnum};
use derive_more::Display;
use hotshot::types::SignatureKey;
use hotshot_types::{light_client::StateKeyPair, signature_key::BLSPubKey};
use rand::{RngCore, SeedableRng};
use sequencer_utils::logging;
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
}

impl Scheme {
    fn gen(self, seed: [u8; 32], index: u64, env_file: &mut impl Write) -> anyhow::Result<()> {
        match self {
            Self::All => {
                Self::Bls.gen(seed, index, env_file)?;
                Self::Schnorr.gen(seed, index, env_file)?;
            },
            Self::Bls => {
                let (pub_key, priv_key) = BLSPubKey::generated_from_seed_indexed(seed, index);
                let priv_key = priv_key.to_tagged_base64()?;
                writeln!(env_file, "ESPRESSO_SEQUENCER_PUBLIC_STAKING_KEY={pub_key}")?;
                writeln!(
                    env_file,
                    "ESPRESSO_SEQUENCER_PRIVATE_STAKING_KEY={priv_key}"
                )?;
                tracing::info!(%pub_key, "generated staking key")
            },
            Self::Schnorr => {
                let key_pair = StateKeyPair::generate_from_seed_indexed(seed, index);
                let priv_key = key_pair.sign_key_ref().to_tagged_base64()?;
                writeln!(
                    env_file,
                    "ESPRESSO_SEQUENCER_PUBLIC_STATE_KEY={}",
                    key_pair.ver_key()
                )?;
                writeln!(env_file, "ESPRESSO_SEQUENCER_PRIVATE_STATE_KEY={priv_key}")?;
                tracing::info!(pub_key = %key_pair.ver_key(), "generated state key");
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
    /// Seed for generating keys.
    ///
    /// If not provided, a random seed will be generated using system entropy.
    #[clap(long, short = 's', value_parser = parse_seed)]
    seed: Option<[u8; 32]>,

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
    /// N - 1. The random seed used to generate the keys will also be written to a file in DIR
    /// called .seed.
    #[clap(short, long, name = "OUT")]
    out: PathBuf,

    #[clap(flatten)]
    logging: logging::Config,
}

fn parse_seed(s: &str) -> Result<[u8; 32], anyhow::Error> {
    let bytes = hex::decode(s)?;
    bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| anyhow!("invalid seed length: {} (expected 32)", bytes.len()))
}

fn gen_default_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    let mut rng = rand_chacha::ChaChaRng::from_entropy();
    rng.fill_bytes(&mut seed);

    seed
}

fn main() -> anyhow::Result<()> {
    let opts = Options::parse();
    opts.logging.init();

    tracing::debug!(
        "Generating {} keypairs with scheme {}",
        opts.num,
        opts.scheme
    );

    // Create output dir if necessary.
    fs::create_dir_all(&opts.out)?;

    let seed = opts.seed.unwrap_or_else(|| {
        tracing::debug!("No seed provided, generating a random seed");
        gen_default_seed()
    });
    fs::write(opts.out.join(".seed"), hex::encode(seed))?;

    for index in 0..opts.num {
        let span = info_span!("gen", index);
        let _enter = span.enter();
        tracing::info!("generating new key set");

        let path = opts.out.join(format!("{index}.env"));
        let mut file = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        opts.scheme.gen(seed, index as u64, &mut file)?;

        tracing::info!("private keys written to {}", path.display());
    }

    Ok(())
}
