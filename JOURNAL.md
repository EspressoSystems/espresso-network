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

## iter 2/10 | 29c94e91-175223 | 2026-08-11 | NP-1 | done

Task: NP-1 (High, runtime, security) - an unverified, peer-chosen epoch number reaching a stake-table lookup, whose cost
is linear in that number. Closed.

Changed: crates/hotshot/new-protocol/src/{message.rs,coordinator.rs,utils.rs} (the fix),
{cert_verifier.rs,vote.rs,epoch.rs,proposal.rs} (test-only `pending_count` accessors), tests.rs and the new
tests/intake.rs (acceptance), PLAN.md, BACKLOG.md, JOURNAL.md.

Checkpoint: pending

Verification: three acceptance checks, each first run against the unfixed code. (1)
`tests::intake::intake_drops_messages_claiming_an_epoch_past_the_ceiling` and
`intake_drops_proposals_too_far_ahead_in_view` fail on the unfixed tree -
`proposal: message claiming epoch 5 was admitted past the ceiling` and `proposal 31 views ahead was admitted` - and pass
on the fixed one; their controls `intake_admits_messages_claiming_an_epoch_at_the_ceiling` and
`intake_admits_proposals_within_the_view_bound` pass both ways, which is what makes the pair a discrimination rather
than a blanket drop. (2) `utils::test::test_verify_new_protocol_leaf_chain_rejects_unreachable_height_before_lookup`
fails unfixed with the defect stated in its own words -
`no stake table available for epoch 1001: ": Stake table for epoch EpochNumber(1001) unavailable. Starting catchup"` -
i.e. a peer-supplied cert2 had just started catchup for epoch 1001 on a chain 10 blocks long. (3) The enumeration
`grep -rn 'membership_for_epoch\|stake_table_for_epoch' crates/hotshot/new-protocol/src --include='*.rs' | grep -v tests`
returns 14 call sites, each now either fed a locally-derived epoch or bounded at one of the two boundaries; recorded
under Settled classes with the split. To run the checks against unfixed code the fixed files were copied aside and
restored afterwards, never `git checkout`. Verify command green: 192 tests run, 192 passed, 15 skipped, exit 0.

Contract preserved. `Coordinator::on_network_message`: within the ceiling every arm behaves exactly as before; the
boundary only extends to all message kinds the rule `Certificate1`, `Certificate2` and `EpochChange` already followed,
and it is deliberately computed over _every_ epoch a message carries rather than its top-level one, because a proposal's
`justify_qc`, `view_change_evidence` and `state_cert` are each resolved against their own epoch's committee. That
widened the fix past the sites the finding listed: `VidShareFragment`, `VidShareBroadcast`, `DedupManifest` and
`ProposalFetch::Response` reach `Coordinator::leader` or the proposal validator with an epoch too, and are now covered.
`verify_new_protocol_leaf_chain`: the accept set is unchanged. The walk steps down exactly one height per accepted leaf,
so a chain failing the new bound could never have reached `expected_height` and already returned "expected height was
not found"; the bound only moves that rejection ahead of the stake-table lookup. Moving the three cert2-to-chain binding
checks above the signature check likewise preserves the accept set and only changes which error a bad input gets.

Docs updated in the same iteration, per change discipline: `EPOCH_CHANGE_LOOKAHEAD` said "epoch changes claiming an
epoch further ahead than this are dropped" and now describes every message, and `verify_new_protocol_leaf_chain` gained
its new precondition. Surface inventory: coordinator:network-intake, message and utils flipped back to unswept, because
their production code changed. cert_verifier, vote, epoch and proposal did not flip: they gained only `#[cfg(test)]`
accessors, so the earlier sweep still certifies their production behaviour - the rule exists so a sweep never certifies
code it did not see, and no production code changed there. 19 of 26 rows swept.

Correction to the previous entry, recorded here rather than by rewriting it: that entry says 21 of 26 rows swept with 5
unswept, but it was written before the logging and utils rows were swept later in the same iteration; the true figure at
that checkpoint was 22 of 26 with 4 unswept, which is what the loop state reported at the start of this iteration.

Learnings: `EpochNumber::genesis()` is 1 while `ViewNumber::genesis()` is 0 (`data.rs:142` and `161`), so a bound
computed from 0 is off by one - the first version of the intake test offered epoch 4 as "past the ceiling" when the
ceiling was exactly 4, and the guard correctly admitted it. Deriving the constant from `EpochNumber::genesis()` in the
test rather than writing a literal is what caught it.

Next: NP-2, the DRB re-request lever that is permanently a no-op once an epoch is marked completed.
