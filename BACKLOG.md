# Backlog

Ledger, not narrative. Top unblocked item is next. Markers: [ ] open, [b] blocked.

Rules:

- One line per item:
  `- [ ] <ID> (<Severity>, <class>, <dimension>): <finding>. Acceptance: <runnable command or observable fact>.`
- Class is one of runtime, test, build-ci, docs, dev-tooling, chosen by the files the fix will touch; a line without one
  is read as runtime. Within a section, order by severity first, then runtime before the other classes.
- A finished task is deleted from its section and recorded as one line in the JOURNAL entry that closed it. No done
  markers accumulate here.
- Run context, audit scores, and DONE annotations live in JOURNAL.md only. No prose sections and no headings beyond the
  ones below, ever.

## Now

## Next

## Later

- [ ] NP-5 (Low, runtime, performance): the proactive DRB prefetch at `coordinator.rs:983-986` runs on every
      `ConsensusOutput::ViewChanged`, and since NP-2 removed `request_drb_result`'s completed-epoch short-circuit
      (`epoch.rs:180-184` now dedups in-flight requests only) it spawns a task per view change once the epoch is known,
      instead of stopping after the first success. Each one resolves immediately from membership and delivers a
      `DrbResult`, so every view change also costs a full `Consensus::apply` pass plus the five `retry_pending_votes`
      calls and the `cert_verifiers.retry_pending` sweep at `coordinator.rs:634-639`. This is a regression this run
      introduced: the prefetch wants "once per epoch", which is the coordinator's business to remember, not the epoch
      manager's. Acceptance: a test drives several view changes within one epoch and asserts at most one DRB request is
      made for the successor epoch, while a separate request for an already-delivered epoch is still answered (the NP-2
      property must survive); it must fail on the unfixed code.
- [ ] NP-6 (Low, runtime, code quality): `coordinator/timer.rs` carries two defects in 66 lines. `reset_with`
      (`timer.rs:42`) is dead - `grep -rn 'reset_with\b' crates/ --include='*.rs'` finds only its definition. And
      `Timer::poll` (`timer.rs:58-65`) returns `Poll::Pending` without registering a waker once `done` is set, so a
      second await of the same `Timer` never wakes; the crate's only use is inside a `select!` whose other branches wake
      the task, which is why nothing hangs today, but `Timer` is `pub` in a `pub mod`. Acceptance: the grep finds no
      `reset_with`, and a test awaits an already-fired `Timer` under a short `tokio::time::timeout` and observes it
      still pending rather than hanging the runtime, then resets it and observes it fire; it must fail on the unfixed
      code.

## Proposed

Items needing a user decision before any work, one plain line each, never a checkbox task: envelope changes, audit
escalations, challenges to a settled class. Never worked without explicit user approval and never counted against
convergence.

## Settled classes

One line per class: the idiom or defect class, the surface it applies to, and how it was settled - fixed class-complete
with its enumerating check, or declined with the reason. Audits must not file findings inside a settled class unless its
implementing code changed after settlement.

- unbounded peer-claimed epoch reaching a stake-table lookup, on every `hotshot-new-protocol` path that reads an epoch
  off the wire: fixed class-complete at two boundaries, `Coordinator::on_network_message` (via
  `Message::max_claimed_epoch`, which covers every epoch a message carries, not just its top-level one) and
  `verify_new_protocol_leaf_chain` (chain-reachability bound before the lookup). Enumerating check:
  `grep -rn 'membership_for_epoch\|stake_table_for_epoch' crates/hotshot/new-protocol/src --include='*.rs' | grep -v tests`;
  its 14 call sites are each either fed a locally-derived epoch (block.rs:177, coordinator.rs:1478 and 1522,
  vote.rs:347, consensus.rs:1851/2321/2335/2354, utils.rs:134) or bounded at one of those two boundaries
  (cert_verifier.rs:210 and 387, proposal.rs:325, vote.rs:317, epoch.rs:186, utils.rs:86).

## Declined

Findings judged not worth fixing, one line each with the reason. Audits must not re-file these.

## Converged

One line per convergence, appended, never rewritten: Converged: <full commit hash> - <date>. The ratchet reads the
latest line here.
