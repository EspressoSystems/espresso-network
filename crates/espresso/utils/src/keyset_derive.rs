use alloy::signers::local::coins_bip39::{English, Mnemonic};
use anyhow::Context;
use hotshot::types::{BLSPrivKey, BLSPubKey, SignatureKey};
use hotshot_types::{
    light_client::{StateKeyPair, StateSignKey},
    x25519,
};

pub struct DerivedKeys {
    pub staking: BLSPrivKey,
    pub state: StateSignKey,
    pub x25519: x25519::SecretKey,
}

pub fn derive_keys_from_mnemonic(
    mnemonic: &Mnemonic<English>,
    index: u64,
) -> anyhow::Result<DerivedKeys> {
    let entropy = mnemonic.to_seed(None).context("invalid mnemonic")?;

    let seed = blake3::derive_key("espresso staking key", &entropy);
    let staking = BLSPubKey::generated_from_seed_indexed(seed, index).1;

    let seed = blake3::derive_key("espresso state key", &entropy);
    let state = StateKeyPair::generate_from_seed_indexed(seed, index)
        .0
        .sign_key();

    let seed = blake3::derive_key("espresso x25519 key", &entropy);
    let x25519 = x25519::Keypair::generated_from_seed_indexed(seed, index)
        .context("generating x25519 key from mnemonic")?
        .secret_key();

    Ok(DerivedKeys {
        staking,
        state,
        x25519,
    })
}
