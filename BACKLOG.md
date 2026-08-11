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

- [ ] NP-2 (Medium, runtime, correctness): `EpochManager::request_drb_result` returns without spawning anything when
      `completed_drb_requests` already holds the epoch (`epoch.rs:161-163`), and that set is only pruned below the
      current epoch by `gc`. `ConsensusInput::DrbResult` is produced solely from `EpochManager::next()`
      (`coordinator.rs:632-640`), so once an epoch is marked completed the re-request lever consensus relies on is
      permanently a no-op: `maybe_propose` and `maybe_vote_1` emit `RequestDrbResult` on every retry
      (`consensus.rs:1131`, `consensus.rs:1661`, whose own comment expects the retry to kick catchup) and never receive
      the result, leaving the node unable to build or vote on an epoch-transition proposal. `handle_leaf_decided` marks
      an epoch completed while emitting nothing to consensus (`epoch.rs:134-144`), so the two views of the DRB can
      diverge. Acceptance: a unit test that delivers a `DrbResult` for epoch E once, then calls `request_drb_result(E)`
      again and asserts `next()` yields a `DrbResult` for E within a bounded wait; it must fail on the unfixed code.

## Later

- [ ] NP-3 (Low, runtime, correctness): `VidFragmentAccumulator::accept` inserts each namespace piece into the pending
      entry as it iterates and only then hits the out-of-range or duplicate check (`vid/fragments.rs:94-106`), so a
      rejected multi-piece fragment leaves its earlier pieces buffered; a later honest fragment covering those indices
      is then rejected as `DuplicateIndex` and the view's share can never complete. Only the view leader can send
      fragments (`coordinator.rs:1080-1092`) and it can already withhold them, which is why this is Low rather than a
      liveness attack. Acceptance: a test feeds a fragment whose pieces are `[index 0, out-of-range index]`, asserts the
      error, then feeds the honest fragment for index 0 and asserts it is accepted; it must fail on the unfixed code.
- [ ] NP-4 (Low, runtime, code quality): `BlockBuilder` decrements its byte counters with unchecked `-=` in
      `on_dedup_manifest` (`block.rs:301`) and `on_view_changed` (`block.rs:329`) while `on_block_reconstructed` uses
      `saturating_sub` (`block.rs:340`); the counters are what enforce `max_leader_bytes`/`max_retry_bytes`, so a wrap
      would silently disable the caps rather than fail loudly. Acceptance:
      `grep -n 'total_bytes -=' crates/hotshot/new-protocol/src/block.rs` prints nothing and the crate battery still
      passes.

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
