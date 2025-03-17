use std::str::FromStr;

use anyhow::bail;
use clap::Parser;
use espresso_types::{PrivKey, PubKey};
use hotshot::{traits::implementations::derive_libp2p_peer_id, types::BLSPubKey};
use hotshot_types::{
    light_client::{StateKeyPair, StateSignKey},
    traits::signature_key::SignatureKey,
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
        if let Ok(key) = TaggedBase64::parse(s)?.try_into() {
            Ok(Self::Bls(key))
        } else if let Ok(key) = TaggedBase64::parse(s)?.try_into() {
            Ok(Self::Schnorr(key))
        } else {
            bail!("unrecognized key type")
        }
    }
}

/// Get the public key corresponding to a private key.
#[derive(Clone, Debug, Parser)]
struct Options {
    /// The private key to get the public key for.
    key: PrivateKey,

    /// Whether or not to derive the libp2p peer ID from the private key.
    #[arg(long, short)]
    libp2p: bool,
}

fn main() {
    let opt = Options::parse();

    match (opt.libp2p, opt.key) {
        // Non-libp2p
        (false, PrivateKey::Bls(key)) => println!("{}", PubKey::from_private(&key)),
        (false, PrivateKey::Schnorr(key)) => {
            println!("{}", StateKeyPair::from_sign_key(key).ver_key())
        },

        // Libp2p
        (true, PrivateKey::Bls(key)) => {
            println!(
                "{}",
                derive_libp2p_peer_id::<BLSPubKey>(&key).expect("Failed to derive libp2p peer ID")
            );
        },
        (true, _) => {
            eprintln!("Key type unsupported for libp2p peer ID derivation");
        },
    }
}
