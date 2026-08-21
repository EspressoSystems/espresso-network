//! The recorder and the replay agree on what a dropped input is called.
//!
//! `NewProtocolDiff.Corpus.unmodelledInputs` lists the dropped inputs that put a
//! trace outside the specification rather than in disagreement with it, and it
//! matches them by name against the `# dropped input:` lines this crate writes.
//! Two lists in two languages, joined by string equality and nothing else: an
//! entry that names an input the recorder never drops excuses nothing, and a
//! rename on this side silently stops excusing what it used to.
//!
//! Both were true. `EpochRootCertificates` sat in that list while the recorder
//! translated the input into a `certificate1` step, so the entry was dead and no
//! build minded. This test is what would have minded.
//!
//! The check is one-way by design: the recorder drops more than the replay
//! excuses. `Stored` and `BlockBuilt` are dropped because the model has no such
//! input, but their outputs ride along on the next written step, so a divergence
//! near one is a disagreement rather than a boundary of the model.
//!
//! Matching is by prefix, because `Corpus.unmodelledDropped` matches that way:
//! it asks whether the recorded line starts with the entry's name. So a rename
//! that keeps the prefix — `DrbResult` to `DrbResultV2` — stays excused here and
//! at replay time alike, and one that does not keeps neither.

/// This crate's recorder, read as text: the `Dropped(...)` strings are the
/// authoritative set, and reading them is cheaper than constructing every input.
const TRACE_RS: &str = include_str!("../trace.rs");

/// The replay's list of excuses. A moved file breaks the build, which is the
/// loudest failure available and better than a check that quietly stops looking.
const CORPUS_LEAN: &str = include_str!("../../../../../lean/new-protocol-diff/NewProtocolDiff/Corpus.lean");

/// Every `Dropped("…")` name the recorder can write.
fn dropped_kinds() -> Vec<String> {
    TRACE_RS
        .split("Dropped(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .collect()
}

/// Every name in `unmodelledInputs`, which is a list of `("Name", "reason")`.
fn excused_kinds() -> Vec<String> {
    let list = CORPUS_LEAN
        .split("def unmodelledInputs")
        .nth(1)
        .expect("Corpus.lean no longer defines unmodelledInputs");
    let list = list.split("def ").next().unwrap_or(list);
    list.split("(\"")
        .skip(1)
        .filter_map(|rest| rest.split('"').next())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect()
}

#[test]
fn every_excused_input_is_one_the_recorder_drops() {
    let dropped = dropped_kinds();
    assert!(
        !dropped.is_empty(),
        "found no Dropped(..) strings in trace.rs; this test is no longer reading it"
    );
    let excused = excused_kinds();
    assert!(
        !excused.is_empty(),
        "found no entries in unmodelledInputs; this test is no longer reading Corpus.lean"
    );
    for kind in &excused {
        assert!(
            dropped.iter().any(|d| d == kind || d.starts_with(kind)),
            "`{kind}` is excused by the replay but never dropped by the recorder, \
             so it excuses nothing: either drop it in trace.rs or remove the entry \
             from unmodelledInputs. Dropped: {dropped:?}"
        );
    }
}
