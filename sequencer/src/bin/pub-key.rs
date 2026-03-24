use clap::{Parser, ValueEnum};
use derive_more::Display;
use espresso_types::PubKey;
use hotshot::{traits::implementations::derive_libp2p_peer_id, types::BLSPubKey};
use hotshot_types::{light_client::StateKeyPair, traits::signature_key::SignatureKey, x25519};
use sequencer::keyset::{KeySet, KeySetOptions};

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
    #[display("libp2p")]
    Libp2p,
}

impl Scheme {
    fn print(self, keys: &KeySet) {
        match self {
            Scheme::All => {
                for scheme in [Scheme::Bls, Scheme::Schnorr, Scheme::X25519, Scheme::Libp2p] {
                    scheme.print(keys);
                }
            },
            Scheme::Bls => println!("{}", PubKey::from_private(&keys.staking)),
            Scheme::Schnorr => println!(
                "{}",
                StateKeyPair::from_sign_key(keys.state.clone()).ver_key()
            ),
            Scheme::X25519 => println!(
                "{}",
                x25519::Keypair::from(keys.x25519.clone()).public_key()
            ),
            Scheme::Libp2p => println!(
                "{}",
                derive_libp2p_peer_id::<BLSPubKey>(&keys.staking)
                    .expect("Failed to derive libp2p peer ID")
            ),
        }
    }
}

/// Print the public keys for the configured node.
///
/// This command takes the same options/env vars pertaining to private keys as the main node, and
/// prints the public keys corresponding to the configured private keys.
#[derive(Clone, Debug, Parser)]
struct Options {
    #[clap(flatten)]
    key_set: KeySetOptions,

    /// Which key to print.
    #[clap(short, long)]
    scheme: Scheme,
}

fn main() {
    let opt = Options::parse();
    let keys = KeySet::try_from(opt.key_set).unwrap();
    opt.scheme.print(&keys);
}
