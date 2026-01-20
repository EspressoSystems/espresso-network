use std::str::FromStr;

use anyhow::bail;
use clap::Parser;
use espresso_types::{PrivKey, PubKey};
use hotshot::types::SignatureKey;
use hotshot_types::{
    light_client::{StateKeyPair, StateSignKey},
};
use tagged_base64::TaggedBase64;

#[derive(Clone, Debug)]
enum PrivateKey {
    Bls(PrivKey),
    Schnorr(StateSignKey),
}

impl FromStr for PrivateKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let tb64 = TaggedBase64::parse(s)?;
        if let Ok(key) = tb64.clone().try_into() {
            Ok(Self::Bls(key))
        } else if let Ok(key) = tb64.try_into() {
            Ok(Self::Schnorr(key))
        } else {
            bail!("unrecognized key type")
        }
    }
}

/// Get the public key corresponding to a private key.
#[derive(Clone, Debug, Parser)]
pub struct Options {
    /// The private key to get the public key for.
    key: PrivateKey,

    // Whether or not to derive the libp2p peer ID from the private key.
    #[clap(long, short)]
    libp2p: bool,
}

pub fn run(opt: Options) {
    match opt.key {
        // Non-libp2p
        PrivateKey::Bls(key) => println!("{}", PubKey::from_private(&key)),
        PrivateKey::Schnorr(key) => println!("{}", StateKeyPair::from_sign_key(key).ver_key()),
    }
}
