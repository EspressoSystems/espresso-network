use anyhow::Context;
use clap::{Parser, ValueEnum};
use derive_more::Display;
use espresso_node::keyset::{KeySet, KeySetOptions};
use espresso_types::PubKey;
use hotshot_types::{light_client::StateKeyPair, traits::signature_key::SignatureKey, x25519};

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
    fn public_keys(self, options: KeySetOptions) -> anyhow::Result<Vec<String>> {
        Ok(match self {
            Scheme::All => {
                let keys = KeySet::try_from(options)?;
                vec![
                    PubKey::from_private(&keys.staking).to_string(),
                    StateKeyPair::from_sign_key(keys.state)
                        .ver_key()
                        .to_string(),
                    x25519::Keypair::from(keys.x25519).public_key().to_string(),
                ]
            },
            Scheme::Bls => vec![PubKey::from_private(&options.staking_key()?).to_string()],
            Scheme::Schnorr => vec![
                StateKeyPair::from_sign_key(options.state_key()?)
                    .ver_key()
                    .to_string(),
            ],
            Scheme::X25519 => vec![
                x25519::Keypair::from(options.x25519_key()?)
                    .public_key()
                    .to_string(),
            ],
        })
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

fn main() -> anyhow::Result<()> {
    let opt = Options::parse();
    for key in opt
        .scheme
        .public_keys(opt.key_set)
        .context("loading keys")?
    {
        println!("{key}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use alloy::signers::local::coins_bip39::{English, Mnemonic};

    use super::*;

    #[test]
    fn signing_schemes_do_not_require_network_key() {
        let mnemonic = Mnemonic::<English>::new_from_phrase(
            "test test test test test test test test test test test junk",
        )
        .unwrap();
        let keys = KeySet::from_mnemonic(mnemonic, None).unwrap();
        let opts = KeySetOptions {
            mnemonic: None,
            index: None,
            key_file: None,
            private_staking_key: Some(keys.staking.to_tagged_base64().unwrap()),
            private_state_key: Some(keys.state.to_tagged_base64().unwrap()),
            private_x25519_key: None,
        };

        assert_eq!(
            Scheme::Bls.public_keys(opts.clone()).unwrap(),
            vec![PubKey::from_private(&keys.staking).to_string()]
        );
        assert_eq!(
            Scheme::Schnorr.public_keys(opts.clone()).unwrap(),
            vec![
                StateKeyPair::from_sign_key(keys.state)
                    .ver_key()
                    .to_string()
            ]
        );
        assert!(Scheme::X25519.public_keys(opts.clone()).is_err());
        assert!(Scheme::All.public_keys(opts).is_err());
    }
}
