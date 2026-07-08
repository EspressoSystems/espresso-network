# SP1 zkVM proof of concept

Verifies a real decaf testnet leaf and its quorum certificate (see `program/fixtures/README.md` for provenance) inside
the SP1 zkVM, using the `light-client` crate's quorum verification with `espresso-types` built with
`default-features = false`.

The guest targets `riscv64im-succinct-zkvm-elf` (SP1 v6, matching espresso-stack's zk-reader). The 32-bit SP1 target
cannot verify decaf data: `hotshot_types::data::serialize_signature2` feeds the bincode encoding of the signer
`BitVec<usize>` into QC and leaf commitments, making the 64-bit word width of the committing platform
consensus-critical.

## Build the guest

Requires the SP1 toolchain (`sp1up`); on NixOS, patchelf `rustc` and `rust-lld`. The RUSTFLAGS reproduce what
`sp1-build` v6 passes.

```sh
cd program
RUSTC=$HOME/.sp1/toolchains/<id>/bin/rustc \
CARGO_TARGET_RISCV64IM_SUCCINCT_ZKVM_ELF_RUSTFLAGS='-C passes=lower-atomic -C link-arg=--image-base=2013265920 -C panic=abort --cfg getrandom_backend="custom" -C llvm-args=-misched-prera-direction=bottomup -C llvm-args=-misched-postra-direction=bottomup' \
cargo build --release --target riscv64im-succinct-zkvm-elf
```

## Run the host script

```sh
cd script
cargo run --release              # execute only, prints cycle count + journal
cargo run --release -- --prove   # additionally setup + core proof + verify
```

The guest ELF is loaded from `SP1_ELF` or the default cargo target directory.

## Journal layout

1. `u64` block height
2. 32 bytes leaf commitment (recomputed on the guest)
3. 32 bytes sha256 of the raw stake table input
4. 32 bytes supermajority threshold (big endian)
5. `u64` epoch

## Host tests

`cargo test` in `program/` runs the positive verification against the real fixtures plus negative controls (corrupted
signature, swapped signature, zeroed stake, truncated stake table, wrong leaf commitment, tampered header).
