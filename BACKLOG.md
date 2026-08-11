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
