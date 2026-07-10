# SP1 zkVM proof of concept

Verifies a real decaf testnet leaf and its quorum certificate (see `program/fixtures/README.md` for provenance) inside
the SP1 zkVM, using the `light-client` crate's quorum verification with `espresso-types` built with
`default-features = false`.

The guest targets `riscv64im-succinct-zkvm-elf` (SP1 v6, matching espresso-stack's zk-reader). The 32-bit SP1 target
cannot verify decaf data: `hotshot_types::data::serialize_signature2` feeds the bincode encoding of the signer
`BitVec<usize>` into QC and leaf commitments, making the 64-bit word width of the committing platform
consensus-critical.

## Build the guest

Requires the SP1 toolchain (`sp1up`). On NixOS the toolchain binaries need patching; find the toolchain id with
`ls ~/.sp1/toolchains`, then:

```sh
TC=$HOME/.sp1/toolchains/<id>
# Interpreter: take the loader from any nix-built binary, e.g. `patchelf --print-interpreter $(command -v cargo)`.
patchelf --set-interpreter "$(patchelf --print-interpreter "$(command -v cargo)")" \
    "$TC/bin/rustc" "$TC/lib/rustlib/x86_64-unknown-linux-gnu/bin/rust-lld"
# rustc also needs glibc, zlib and libgcc plus the toolchain's own libs on its rpath, e.g.:
# patchelf --set-rpath "<glibc>/lib:<zlib>/lib:<gcc-lib>/lib:$TC/lib" "$TC/bin/rustc"
```

The RUSTFLAGS reproduce what `sp1-build` v6 passes:

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

The guest ELF is loaded from `SP1_ELF` if set, otherwise from
`target/nix/riscv64im-succinct-zkvm-elf/release/espresso-sp1-program` at the repo root. That default only exists because
the repo's nix shell sets `CARGO_TARGET_DIR=target/nix`; outside the shell, point `SP1_ELF` at the built ELF.

## Journal layout

1. `u64` block height
2. 32 bytes leaf commitment (recomputed on the guest)
3. 32 bytes sha256 of the raw stake table input
4. 32 bytes supermajority threshold (big endian)
5. `u64` epoch

The proof does not bind the epoch to its stake table: a verifier must check `stake_table_digest` against a trusted,
byte-exact snapshot of the stake table for the journal's epoch (the JSON is not canonicalized, so only exact bytes
match). The guest also handles only current-epoch (mid-epoch) QCs; epoch-transition leaves additionally require the
next-epoch quorum check (`StakeTableQuorum::verify_static`).

## Host tests

`cargo test` in `program/` runs the positive verification against the real fixtures plus negative controls (crafted
genesis-view QC, corrupted signature, swapped signature, zeroed stake, truncated stake table, wrong leaf commitment,
tampered header).
