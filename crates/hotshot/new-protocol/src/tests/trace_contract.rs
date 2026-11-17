//! The recorder and the trace checker agree on what they exchange by name.
//!
//! A trace is JSON the checker reads with readers derived from its own types
//! (`NewProtocolDiff.Trace`), so a name the recorder writes and the checker
//! does not know fails the check of every trace. The reverse does not: a
//! constructor the recorder never writes is a case the checker handles and no
//! trace has. `stateValidationFailed` and `drbResult` excuse a node's wait,
//! so a rename on this side alone would silently stop excusing it.
//!
//! The checker also copies one rule of this crate: which blocks wait for the
//! next epoch's DRB result (`NewProtocolDiff.transitionWindow`). The two have
//! to name the same blocks, or a node waiting for a DRB result is read as late.

use hotshot_types::utils::is_epoch_transition;

/// This crate's recorder, read as text.
const TRACE_RS: &str = include_str!("../trace.rs");

/// The source of the actions consensus holds back, which names their kinds.
const RECORDED_RS: &str = include_str!("../consensus/recorded.rs");

/// The checker's trace types. A moved file breaks the build, which is the
/// loudest failure available and better than a check that quietly stops looking.
const TRACE_LEAN: &str =
    include_str!("../../../../../lean/new-protocol-diff/NewProtocolDiff/Trace.lean");

/// The checker's step check, which holds its copy of the transition window.
const PROMPT_CHECK_LEAN: &str =
    include_str!("../../../../../lean/new-protocol-diff/NewProtocolDiff/PromptCheck.lean");

/// The constructors of the checker's inductive type `name`.
fn constructors(name: &str) -> Vec<String> {
    let body = TRACE_LEAN
        .split(&format!("inductive {name} where"))
        .nth(1)
        .unwrap_or_else(|| panic!("Trace.lean no longer defines {name}"));
    let body = body.split("deriving").next().unwrap_or(body);
    let found: Vec<String> = body
        .lines()
        .filter_map(|l| l.trim_start().strip_prefix("| "))
        .filter_map(|l| l.split([' ', '(']).next())
        .map(str::to_string)
        .collect();
    assert!(!found.is_empty(), "found no constructors of {name}");
    found
}

/// Each of the checker's types, the source that writes its constructors,
/// and how a constructor's name appears where it is written, with no
/// whitespace: the formatter decides where these calls break.
const WRITTEN: [(&str, &str, &[&str]); 4] = [
    ("Dropped", TRACE_RS, &["tagged(\"{}\""]),
    ("Write", TRACE_RS, &["tagged(\"{}\",obj("]),
    ("ActionKind", TRACE_RS, &["=>\"{}\","]),
    ("HeldKind", RECORDED_RS, &["=>\"{}\","]),
];

#[test]
fn every_name_the_checker_reads_is_one_the_recorder_writes() {
    for (ty, source, forms) in WRITTEN {
        let source: String = source.chars().filter(|c| !c.is_whitespace()).collect();
        for name in constructors(ty) {
            assert!(
                forms
                    .iter()
                    .any(|form| source.contains(&form.replace("{}", &name))),
                "the checker's `{ty}.{name}` is never written by this crate"
            );
        }
    }
}

#[test]
fn the_checker_copies_the_transition_window() {
    let window: u64 = PROMPT_CHECK_LEAN
        .split("def transitionWindow : Nat := ")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("PromptCheck.lean no longer defines transitionWindow as a number");
    // The crate's rule subtracts the window from the height, so it has no
    // answer for epochs shorter than the window.
    for height in window..=12 {
        for block in 0..=3 * height {
            assert_eq!(
                is_epoch_transition(block, height),
                block != 0 && (block % height >= height - window || block % height == 0),
                "block {block} of epochs of {height} blocks"
            );
        }
    }
    assert!(
        !is_epoch_transition(7, 0),
        "with no epochs, no block is in a transition"
    );
}
