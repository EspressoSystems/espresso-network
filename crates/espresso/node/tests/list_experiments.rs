//! `espresso-node --list-experiments` is parsed by the bench harness from stdout.

use std::process::Command;

use espresso_utils::experiment::Experiment;

/// The flag prints one JSON line and exits 0 before module parsing and the genesis load.
#[test]
fn list_experiments_prints_names_and_exits_before_startup() {
    let output = Command::new(env!("CARGO_BIN_EXE_espresso-node"))
        .current_dir(env!("CARGO_TARGET_TMPDIR"))
        .env_clear()
        .args(["--list-experiments", "--", "bad", "module"])
        .output()
        .expect("spawn espresso-node");

    let expected = format!("{}\n", serde_json::to_string(&Experiment::names()).unwrap());
    assert_eq!(String::from_utf8_lossy(&output.stdout), expected);
    assert!(
        output.status.success(),
        "status {:?}, stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
}
