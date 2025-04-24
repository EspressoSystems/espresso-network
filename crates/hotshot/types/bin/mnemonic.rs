use std::env;

use hotshot_types::{signature_key::BLSPubKey, utils::mnemonic};

pub fn main() {
    let args: Vec<String> = env::args().collect();

    let keys: Vec<_> = args[1..].to_vec();

    println!("\nKeys:\n");

    for key in &keys {
        println!("{}", key);
    }

    println!("\nMnemonics:\n");

    for key in keys {
        let mnemonic = mnemonic(
            BLSPubKey::try_from(&tagged_base64::TaggedBase64::parse(&key).unwrap()).unwrap(),
        );

        println!("{}", mnemonic);
    }
}
