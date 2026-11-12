//! Generate stake table cases, run them through the node, and write them for the Lean
//! checker.
//!
//!     STAKE_TABLE_CORPUS=<directory> cargo test -p espresso-types \
//!         --lib stake_table_corpus -- --ignored
//!
//! Writes one `<kind>.jsonl` file per kind of case into the directory (by default
//! `stake-table-corpus` under the system's temporary directory): events replayed
//! and the validators selected from them, block rewards, payouts, log orders, block
//! range chunks and leader schedules. Each line is a [`case::Case`]: an input and
//! the node's result. `lake exe check <directory>` in `lean/stake-table-diff` runs the
//! Lean implementation on every input and compares.
//!
//! Nothing here depends on Lean. The test fails only if the generator could not
//! produce the cases it is meant to, for example if some outcome never turned up.

mod case;
mod generate;

use std::{
    fs::{self, File},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};

use self::case::Case;

#[test]
#[ignore = "writes the corpus that lean/stake-table-diff checks; run it with --ignored"]
fn stake_table_corpus() -> Result<()> {
    let dir = std::env::var_os("STAKE_TABLE_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("stake-table-corpus"));
    fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut replays = generate::replays()?;
    let mut schedules = generate::schedules();
    for case in generate::selections()? {
        match case {
            Case::Replay { .. } => replays.push(case),
            _ => schedules.push(case),
        }
    }
    write(&dir, "replay", &replays)?;
    write(&dir, "schedule", &schedules)?;
    write(&dir, "reward", &generate::rewards())?;
    write(&dir, "payout", &generate::payouts()?)?;
    write(&dir, "order", &generate::orders()?)?;
    write(&dir, "chunks", &generate::chunks())?;
    Ok(())
}

/// Write `cases` to `<dir>/<kind>.jsonl`, one per line.
fn write(dir: &Path, kind: &str, cases: &[Case]) -> Result<()> {
    let path = dir.join(format!("{kind}.jsonl"));
    let mut out = BufWriter::new(
        File::create(&path).with_context(|| format!("creating {}", path.display()))?,
    );
    for case in cases {
        serde_json::to_writer(&mut out, case)?;
        out.write_all(b"\n")?;
    }
    out.flush()?;
    println!("{}: {} cases", path.display(), cases.len());
    Ok(())
}
