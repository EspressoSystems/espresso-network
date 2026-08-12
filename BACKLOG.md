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

- [ ] NP-7 (Medium, test, testing): the four message kinds NP-1 widened the epoch bound to cover - `VidShareFragment`,
      `VidShareBroadcast`, `DedupManifest` and `ProposalFetch::Response` - have no test on either side of the ceiling;
      `tests/intake.rs` builds nine shapes and its `set_claimed_epoch` panics on anything else, so a wrong arm in
      `Message::max_claimed_epoch` (`message.rs:495-512`) would reopen the NP-1 hole silently. Acceptance: each of the
      four is exercised at the ceiling and one past it, admitted and dropped respectively, and each test fails if its
      arm of `max_claimed_epoch` is made to return `None`.

## Later

- [ ] NP-8 (Low, build-ci, testing): the per-iteration Verify command excludes the restart, failure and network shards,
      which is where NP-1's liveness regression lived, and nothing enforces PLAN.md's own instruction to run the full
      four-shard suite before declaring convergence. Make the requirement mechanical rather than a sentence in prose:
      either promote the full suite to the Verify command, or add a convergence-only gate that runs it. It must run the
      four CI shards as separate filtered runs, not as one unfiltered run: run serially on one machine, all 217 tests
      saturate it and `tests::restarts::restart_all_nodes_at_epoch_boundary` exceeds its `max_runtime` at 305s though it
      passes alone in 71s, so an unfiltered run reports a failure that is only contention. Acceptance: the procedure a
      future run follows cannot reach a convergence declaration without a green run of each of the four shards recorded
      in the JOURNAL entry that declares.
- [ ] NP-9 (Low, docs, correctness): the Settled classes enumeration for the epoch-bound class (BACKLOG.md) does not
      establish what it claims. Its grep is scoped to `crates/hotshot/new-protocol/src`, so it misses a reachable lookup
      outside the crate (`proposal.rs:313` -> `contracts/rust/adapter/src/light_client.rs:173`
      `membership_for_epoch(state_cert.epoch())`); it calls `Coordinator::leader` locally-derived when two callers feed
      a wire epoch (`fragment.data.epoch`, `manifest.epoch`); and every line number in it is stale. Both sites are in
      fact bounded at intake, so no hole results - the defect is that the enumeration does not prove it. Acceptance: the
      enumeration command covers every crate a new-protocol epoch can reach a stake-table lookup through, each site is
      classified with a line number that resolves at the recorded commit, and the two misclassified sites are described
      by what actually bounds them.

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
