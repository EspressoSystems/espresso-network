# Journal

Append-only. One primary entry per iteration; SALVAGE and ROTATION entries are additional. Never rewrite past entries
(filling the current entry's Checkpoint field is completion, not a rewrite).

Heading grammar, exactly (fenced and indented here so this example is never mistaken for an entry by anything that
counts or rotates them):

```
  ## iter <i>/<N> | <run-id> | <YYYY-MM-DD> | <task-id or AUDIT or EVALUATOR or RATCHET or WRAPUP or SALVAGE or ROTATION> | <done|blocked|audit|converged|salvage|rotation>
```

Write a real heading at column zero, never indented: the indentation above belongs to the example alone, and an indented
heading is invisible to the rotation anchor and to the archive counter, so the entry under it is not counted and not
rotated.

SALVAGE entries take status salvage; ROTATION entries take status rotation. An EVALUATOR entry records an evaluator-gate
iteration: status audit when the run continues after the verdict, blocked on a terminal second REJECT, converged when
that same iteration declares.

run-id is the first 8 characters of the session id, a hyphen, then the HHMMSS of started_at from the loop state
frontmatter, so two runs in one session are told apart. Body fields, in order: Task, Changed, Checkpoint (the jeffy
checkpoint commit hash, or none with the reason), Verification, Learnings, Next.

The closing entry that declares convergence carries the evaluator verdict in its Verification field:
`Evaluator: PASS - <one-line summary>`, or `Evaluator: unavailable (<reason>)`. An earlier EVALUATOR entry records its
own verdict the same way and never stands in for the closing one: the Stop hook reads the closing entry alone, so a run
that gates early and keeps working re-invokes the gate at the declaration.

Closed tasks are recorded here as one line each (ID, title, closing evidence), because BACKLOG.md deletes them.
Rotation: when this file exceeds 500 lines, move all but the last 10 entries to the end of JOURNAL-archive.md, appending
to whatever that file already holds and never overwriting it, because the archive accumulates across every rotation and
every run; create it only when it does not already exist, and record the rotation as a ROTATION entry.

## iter 1/10 | 29c94e91-175223 | 2026-08-11 | AUDIT | audit

Task: bootstrap the Improvement-mode state files and run the first audit, scoped by the run's focus directive to the new
protocol.

Changed: PLAN.md (Operating envelope surfaces, Surface inventory, Verify command, Lessons), BACKLOG.md (4 findings),
JOURNAL.md (this entry), .gitignore (loop state file).

Checkpoint: 4b54252cf130095b950e33f52684cba2b70d5eb2. This iteration changed only the three state files and .gitignore,
which is what an audit iteration is: four BACKLOG.md items went from absent to open, so it is not a stall.

Verification: Verify command green - `187 tests run: 187 passed, 15 skipped` in 236s, exit 0. The audit is read-based
over `crates/hotshot/new-protocol/src/**` at 64c5cdc6d0a plus that battery as the executed check. Scope: the focus
directive fixes the surface at the `hotshot-new-protocol` crate; `crates/cliquenet` is a separate crate and out of
scope. Surface inventory: 21 of 26 rows swept. The 5 unswept rows are coordinator:event-loop,
coordinator:consensus-output, coordinator:client-api, coordinator:submodules, logging - the scores below claim only the
21 swept rows and say nothing about those 5. Scores: security High (NP-1), correctness Medium (NP-2), testing Medium
(both NP-1's guard asymmetry and NP-2's dead retry lever are untested paths; no separate task, they are the same root
causes), error handling Low (NP-3), code quality Low (NP-4), architecture None, performance None, documentation None,
developer experience None, observability None. Dependency hygiene not assessed this iteration. UX and accessibility do
not apply: the crate has no user-facing surface. Closeout has NOT begun - this audit found one High and one Medium.

Evidence for the two that matter. NP-1: `Coordinator::on_network_message` guards `Certificate1`, `Certificate2` and
`EpochChange` with `is_epoch_too_far_ahead` (coordinator.rs:1180, 1204, 1315) but leaves `Proposal` (1053),
`TimeoutCertificate` (1279), `HighQc` (1295), the `CatchupEvidence` carried inside `TimeoutVote` (1230-1254) and the
three vote arms (via vote.rs:315-318) ungated, and `verify_new_protocol_leaf_chain` looks up the stake table for an
epoch derived from an unverified `cert2.data.block_number` before validating that certificate (utils.rs:54-58); each of
those paths passes an unverified, peer-chosen epoch to `EpochMembershipCoordinator::membership_for_epoch` or
`stake_table_for_epoch`, which have a lower bound (`epoch < first_epoch`) but no upper bound
(epoch_membership.rs:157-186, 198-232) and spawn a catchup whose first loop walks back one epoch at a time from the
claimed epoch, doing a `load_stake_table` await and inserting a `catchup_map` entry plus a broadcast channel per
iteration (epoch_membership.rs:339-385). `Proposal` is also the only view-keyed message without `is_view_too_far_ahead`;
the guard appears at 1058 but only decides whether to update the `proposal_received_at` metric. Senders are
authenticated validators (network.rs:121-136), which is why this is a Byzantine-validator finding rather than an
open-internet one - and Byzantine validators are exactly the envelope a BFT protocol is built for. NP-2:
`ConsensusInput::DrbResult` has a single producer, `EpochManager::next()` at coordinator.rs:632-640;
`request_drb_result` short-circuits on `completed_drb_requests` (epoch.rs:161-163) and that set is pruned only below the
current epoch, so consensus's only retry lever (consensus.rs:1131 and 1661) is permanently inert once an epoch has been
marked completed - and `handle_leaf_decided` marks an epoch completed while emitting nothing to consensus
(epoch.rs:134-144), so the two copies of the DRB can diverge in the first place.

Two hypotheses were chased and refuted, recorded so a later audit does not re-file them: `storage.rs:317` drops
`next_epoch_justify_qc` when persisting a proposal, but `From<Proposal> for Leaf2` drops it identically
(new_protocol/proposal.rs:96), so restart-seeded proposals still commit to the same leaf; and a validated
`EpochChangeMessage` cannot carry a proposal that disagrees with its certificates, because `well_formed` binds
`proposal_commitment(&self.proposal)` to `cert1.data.leaf_commit` (message.rs:201).

Learnings: cargo and just must run under `nix develop --command`, and nix needs `~/.cache/nix` write access, so those
commands run with the agent sandbox disabled. The crate battery is slow enough (the full four-shard suite was still
running after 35 minutes) that the per-iteration Verify command is CI's own `standard` shard and the full suite is
reserved for the convergence check. I piped the first battery run through `tail` and got a 0-byte log and an exit status
belonging to `tail` rather than to nextest - the Method's own rule against that, learned the direct way; the run was
re-issued with redirection.

Next: NP-1, the intake admission guards.
