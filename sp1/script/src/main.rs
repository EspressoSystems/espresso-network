use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};

use anyhow::{Context, Result};
use clap::Parser;
use sp1_sdk::{
    Elf, ProvingKey, SP1PublicValues, SP1Stdin,
    blocking::{ProveRequest, Prover, ProverClient},
};

/// Execute (and optionally prove) the SP1 guest against the decaf fixtures.
#[derive(Parser)]
struct Args {
    /// Additionally generate and verify a core proof (default: execute only).
    #[clap(long)]
    prove: bool,
}

fn manifest_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(relative)
}

fn load_elf() -> Result<Elf> {
    let path = match std::env::var("SP1_ELF") {
        Ok(path) => PathBuf::from(path),
        Err(_) => manifest_path(
            "../../target/nix/riscv64im-succinct-zkvm-elf/release/espresso-sp1-program",
        ),
    };
    let path = path
        .canonicalize()
        .with_context(|| format!("guest ELF not found at {}", path.display()))?;
    println!("guest ELF: {}", path.display());
    let bytes = fs::read(&path).with_context(|| format!("reading guest ELF {}", path.display()))?;
    Ok(bytes.into())
}

fn print_journal(public_values: &mut SP1PublicValues) {
    let height: u64 = public_values.read();
    let mut commitment = [0u8; 32];
    public_values.read_slice(&mut commitment);
    let mut stake_table_digest = [0u8; 32];
    public_values.read_slice(&mut stake_table_digest);
    let mut threshold = [0u8; 32];
    public_values.read_slice(&mut threshold);
    let epoch: u64 = public_values.read();

    println!("journal:");
    println!("  height:             {height}");
    println!("  leaf commitment:    0x{}", hex::encode(commitment));
    println!(
        "  stake table sha256: 0x{}",
        hex::encode(stake_table_digest)
    );
    println!("  threshold (be):     0x{}", hex::encode(threshold));
    println!("  epoch:              {epoch}");
}

fn main() -> Result<()> {
    sp1_sdk::utils::setup_logger();
    let args = Args::parse();

    let leaf = fs::read(manifest_path("../program/fixtures/leaf_query_data.json"))
        .context("reading leaf fixture")?;
    let stake_table = fs::read(manifest_path("../program/fixtures/stake_table.json"))
        .context("reading stake table fixture")?;
    let elf = load_elf()?;

    let mut stdin = SP1Stdin::new();
    stdin.write_vec(leaf);
    stdin.write_vec(stake_table);

    let client = ProverClient::from_env();

    let start = Instant::now();
    let (mut public_values, report) = client
        .execute(elf.clone(), stdin.clone())
        .run()
        .context("executing guest")?;
    println!("executed in {:.2?}", start.elapsed());
    println!("total instructions: {}", report.total_instruction_count());
    println!("total syscalls:     {}", report.total_syscall_count());
    print_journal(&mut public_values);

    if args.prove {
        let start = Instant::now();
        let pk = client.setup(elf).context("setup")?;
        println!("setup in {:.2?}", start.elapsed());

        let start = Instant::now();
        let mut proof = client.prove(&pk, stdin).core().run().context("proving")?;
        println!("proved (core) in {:.2?}", start.elapsed());

        let start = Instant::now();
        client
            .verify(&proof, pk.verifying_key(), None)
            .map_err(|err| anyhow::anyhow!("verifying proof: {err:?}"))?;
        println!("verified in {:.2?}", start.elapsed());

        print_journal(&mut proof.public_values);

        let out_dir = manifest_path("../../tmp");
        fs::create_dir_all(&out_dir)?;
        let out = out_dir.join("espresso-sp1-proof.bin");
        proof.save(&out).context("saving proof")?;
        println!("proof written to {}", out.display());
    }

    Ok(())
}
