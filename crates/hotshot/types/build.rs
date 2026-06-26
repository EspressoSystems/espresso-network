use ark_bn254::Bn254;
use ark_serialize::CanonicalSerialize;
use jf_pcs::prelude::UnivariateUniversalParams;
use std::{env, fs, path::Path};

/// For `STAKE_TABLE_CAPACITY=200`, the light client prover (a.k.a. `hotshot-state-prover`)
/// would need to generate proof for a circuit of slightly below 2^20 gates.
/// Thus we need to support this upperbounded degree in our Structured Reference String (SRS),
/// the `+2` is just an artifact from the jellyfish's Plonk proof system.
#[allow(clippy::cast_possible_truncation)]
const SRS_DEGREE: usize = 2u64.pow(20) as usize + 2;

fn main() {
    println!("cargo::rerun-if-changed=src/vid/advz.rs");

    // Generate the KZG SRS and safe it to a file in the target dir. This allows the compiled
    // library to embed the serialized SRS directly, obviating the need to fetch it from the network
    // or file system at runtime.
    let srs = ark_srs::kzg10::aztec20::setup(SRS_DEGREE).expect("Aztec SRS failed to load");
    let params = UnivariateUniversalParams::<Bn254> {
        powers_of_g: srs.powers_of_g,
        h: srs.h,
        beta_h: srs.beta_h,
        powers_of_h: vec![srs.h, srs.beta_h],
    };

    let out_dir = env::var("OUT_DIR").unwrap();
    let out = Path::new(&out_dir).join("kzg_srs.bin");
    let mut bytes = vec![];
    params.serialize_compressed(&mut bytes).unwrap();
    fs::write(out, bytes).unwrap();
}
