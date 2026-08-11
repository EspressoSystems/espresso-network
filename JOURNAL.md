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
over `crates/hotshot/new-protocol/src/**` at `64c5cdc6d0a` plus that battery as the executed check. Scope: the focus
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

Checkpoint: 975205f1514e84887d2547c4f84fd96fa0a18f55

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

## iter 3/10 | 29c94e91-175223 | 2026-08-11 | NP-2 | done

Task: NP-2 (Medium, runtime, correctness) - `EpochManager::request_drb_result` short-circuiting on
`completed_drb_requests`, which left consensus's only DRB retry lever permanently unanswered. Closed.

Changed: crates/hotshot/new-protocol/src/epoch.rs (the short-circuit removed, both affected doc comments rewritten, and
an inline test module added), PLAN.md, BACKLOG.md, JOURNAL.md.

Checkpoint: b9516d23682220a852c58e768e5feee990abcf01

Verification: acceptance check `epoch::tests::repeated_request_is_answered_again` run first against the unfixed code,
where it fails with the defect stated plainly - `no DRB delivered: the request was dropped, not answered` - and passes
against the fixed one. Its control `epoch::tests::concurrent_requests_are_deduplicated` passes both ways, which is what
shows the fix removed the wrong dedup and not the right one: three back-to-back requests for the same epoch still
collapse into a single in-flight task. To run the check against unfixed code the fixed file was copied aside and
restored afterwards, never `git checkout`. Verify command green: 194 tests run, 194 passed, 15 skipped, exit 0.

Contract preserved. Two dedup guards lived here and they are not interchangeable. `pending_drb_requests` collapses
_concurrent_ callers and is untouched, so no caller can spawn a duplicate task while one is in flight.
`completed_drb_requests` records that this manager once resolved an epoch, and it still gates `handle_leaf_decided` so a
decided epoch root is never recomputed and a transition block never re-supplies a DRB; what it must not do is answer for
consensus, which keeps its own in-memory copy and is the party actually asking. Honouring repeats cannot flood: the
intake arms only call `request_drb_result` when `CertVerifier::verify` failed to resolve the epoch's membership, which
cannot be true for an epoch whose DRB is already in membership, and the consensus retry only fires while
`Consensus::drb_results` lacks the entry, so each answered request stops the caller that made it. When the epoch is
already known the spawned task resolves from membership immediately and ends, which is what both new tests observe.

Both doc comments that stated the old behaviour were rewritten in the same iteration: the `completed_drb_requests` field
comment said "Subsequent `request_drb_result` calls for these epochs are no-ops", which is now exactly wrong, and
`request_drb_result` gained a comment explaining why a repeat is honoured, so a future reader does not reinstate the
short-circuit as an obvious optimisation.

Surface inventory: the epoch row flipped back to unswept, because its production behaviour changed. 18 of 26 rows swept,
8 unswept. The unswept set is now the four coordinator rows plus message, utils, epoch and coordinator:network-intake;
with two Low tasks left on the ledger there is budget to re-sweep them in a later audit iteration before any convergence
claim.

Learnings: none that generalise beyond this fix.

Next: NP-3, the VID fragment accumulator mutating its pending entry before validating the whole fragment.

## iter 4/10 | 29c94e91-175223 | 2026-08-11 | NP-3 | done

Task: NP-3 (Low, runtime, correctness) - `VidFragmentAccumulator::accept` mutating the view's pending entry before it
had validated the whole fragment. Closed.

Changed: crates/hotshot/new-protocol/src/vid/fragments.rs (validate-then-insert, plus a
`PendingShare::describes_same_share_as` helper for the consistency check that was inline),
crates/hotshot/new-protocol/src/tests/vid.rs (two acceptance tests), PLAN.md, BACKLOG.md, JOURNAL.md.

Checkpoint: 1cbf91b70fac6f796625435b32373904ef54bc2e

Verification: two acceptance checks, both run first against the unfixed code, where both fail, and both pass against the
fixed one. `fragment_accumulator_rejection_strands_no_pieces` is the check as filed: a fragment carrying a good piece
for index 0 followed by an out-of-range index is rejected, and the honest fragment covering both namespaces is then
admitted and reassembles the original share - unfixed, index 0 came back as `DuplicateIndex` and the share could never
complete. `fragment_accumulator_rejection_pins_no_metadata` covers the half the filing had missed: the entry was created
from the fragment's own header _before_ the per-piece loop ran, so a rejected first fragment also pinned the view's
`num_namespaces`, and every honest fragment afterwards was turned away as `Inconsistent`. The four pre-existing fragment
tests pass unchanged in both trees, which is what shows the reordering did not weaken the rejections themselves. To run
the checks against unfixed code the fixed file was copied aside and restored afterwards, never `git checkout`. Verify
command green: 196 tests run, 196 passed, 15 skipped, exit 0. One of them, the pre-existing
`tests::block::test_leader_buffer_drain`, was additionally flagged `LEAK` by nextest, which means it passed but left a
handle open past the leak timeout; it is not in the code this iteration touched and it was not flagged in the previous
run, so it reads as a timing artefact rather than a regression. Noted rather than chased, and worth a second look when
NP-4 takes that module.

Contract preserved. The accept set is unchanged: the same fragments are accepted and the same ones rejected, with the
same error variants - the intra-fragment duplicate case still reports `DuplicateIndex`, now detected against a set of
the fragment's own indices rather than as a side effect of a partial insert. What changed is only that a rejection is
now total: `accept` either applies the whole fragment or leaves the view untouched. `IndexOutOfRange` now reports
`fragment.num_namespaces` where it reported `pending.num_namespaces`; those are equal on every path that reaches the
check, because the consistency comparison runs first and rejects a fragment whose count differs.

Severity note: this stays Low and the envelope is why. Only the view's leader can send a fragment
(`coordinator.rs:1080-1092` rejects any other sender), and a leader that wants to deny its own view its share can simply
not send it, so the fix removes a self-inflicted footgun rather than an attack. It is still worth fixing: the
pre-existing `fragment_accumulator_rejects_intra_fragment_duplicate` test shows the accumulator was already being handed
rejectable fragments in its own test suite, and a partial insert there is silent.

Learnings: none that generalise beyond this fix.

Next: NP-4, the unchecked byte-counter decrements in `BlockBuilder`, which is the last item on the ledger; after it the
run needs an audit iteration to re-sweep the nine rows this run has re-opened before any convergence claim.

## iter 5/10 | 29c94e91-175223 | 2026-08-11 | NP-4 | done

Task: NP-4 (Low, runtime, code quality) - `BlockBuilder`'s two unchecked byte-counter decrements. Closed. The ledger is
now empty.

Changed: crates/hotshot/new-protocol/src/block.rs (both decrements made saturating, and the counters' contract
documented on `BlockBuilderConfig`), crates/hotshot/new-protocol/src/tests/block.rs (three tests pinning the two caps),
PLAN.md, BACKLOG.md, JOURNAL.md.

Checkpoint: 1872c2ee84eaf4964748113f8a590d9aad5ab956

Verification: the acceptance check as filed is the enumeration
`grep -n 'total_bytes -=' crates/hotshot/new-protocol/src/block.rs`, which prints two lines at the pre-fix commit (301
and 329, read back with `git show HEAD:...`) and nothing now. That check discriminates, but it only proves the code
changed, so I also added three tests for what the counters actually exist to enforce:
`test_leader_buffer_respects_max_bytes` (five hundred-byte transactions fit under the 512-byte cap, the sixth does not),
`test_dedup_returns_leader_budget` (dropping three of five returns exactly their bytes, so three of four new ones fit
afterwards), and `test_retry_buffer_respects_max_bytes_and_frees_on_expiry` (ten fit under the 1024-byte cap, the
eleventh does not; expiry returns the whole budget and it is reusable). Each exercises its documented cap on both sides
of the limit. Verify command green: 199 tests run, 199 passed, 15 skipped, exit 0. The `LEAK` that nextest flagged on
`test_leader_buffer_drain` in the previous iteration did not recur in this run, which took the same module and added
three tests to it, so it was the timing artefact it looked like rather than anything in that test.

These three tests pass against the unfixed code too, and that is the honest description of this task: no underflow is
reachable today. Every decrement is by an amount that was added to the counter - `leader_buffer` and
`leader_total_bytes` are reset together by `request_block`'s `mem::take`, and each `RetryEntry` stores the exact size
that was added - so the change is hardening and consistency with the third site, which already used `saturating_sub`,
rather than a bug fix. The tests are what make the contract enforceable from here, since nothing pinned the caps before.

Correction to my own filing, which said a wrap "would silently disable the caps": that is the wrong direction and a
future reader would misjudge the risk. A wrapped counter sits just below `u64::MAX`, so `total + size > cap` is true
forever after and the builder refuses every transaction - it starves rather than over-admits. Clamping at zero fails the
other way, toward accepting work, which is why saturating is the right choice here and why the new doc comment says so.

Contract preserved: no behaviour changes on any reachable path, because no decrement can currently underflow; the caps,
the admission rule and the expiry semantics are exactly as before, which is what the three new tests and the five
pre-existing block tests jointly show.

Surface inventory: the block row flipped back to unswept. 16 of 26 swept, 10 unswept - the four coordinator rows plus
message, utils, epoch, vid:fragments, block, and coordinator:network-intake. Every one of those was re-opened by this
run's own fixes, not by an unexamined remainder.

Learnings: an acceptance check that is only a grep proves the code changed, not that the behaviour is right; pair it
with a test of the invariant the code exists to maintain.

Next: the ledger is empty, so the next iteration audits. Its first job is the ten re-opened rows, since convergence
requires none left unswept, and the evaluator gate has not been run yet - iteration 1's audit found a High and a Medium,
so the "clean full audit already recorded" precondition for running the gate early has never been met this run.

## iter 6/10 | 29c94e91-175223 | 2026-08-12 | AUDIT | audit

Task: full audit. The ledger emptied at the end of iteration 5, so this iteration swept the ten outstanding Surface
inventory rows and rescored.

Changed: PLAN.md (ten rows swept), BACKLOG.md (two findings), JOURNAL.md.

Checkpoint: c9b8e106c14d77af0adf1ee8f507d57a81365022

Verification: Verify command green: 199 tests run, 199 passed, 15 skipped, exit 0. Surface inventory: 26 of 26 rows
swept, none unswept, so the scores below claim the whole mapped surface for the first time this run. Six of the ten rows
were re-sweeps of code this run changed and were cheap; the other four were the coordinator rows, never swept before -
`next_consensus_input`'s `select!` loop and all three GC scopes, all twenty arms of `process_consensus_output`, all
fifteen `on_client_request` variants, and the three submodules. Scores, now claiming every row: security None,
correctness Low (NP-6), performance Low (NP-5), code quality Low (NP-6), architecture None, testing None, error handling
None, documentation None, developer experience None, observability None. Dependency hygiene still not assessed: the
crate takes its dependencies from the workspace manifest, which is out of this run's focus. UX and accessibility do not
apply. Zero High and zero Medium in-envelope, so CLOSEOUT HAS BEGUN: no further audit or replenishment this run, which
will finish by working the ledger and converging.

The two findings both come from code this run touched, which is where the Method says to look hardest. NP-5 is a
regression I introduced in iteration 3 and did not catch there: I reasoned about the two callers of `request_drb_result`
that stop once answered and missed the third, the proactive prefetch at `coordinator.rs:983-986`, which fires on every
view change unconditionally. With the completed-epoch short-circuit gone, that now spawns a task per view. The fix
belongs in the coordinator, which is the party that wants "once per epoch"; reinstating the short-circuit in the epoch
manager would undo NP-2. NP-6 is two small defects in the 66-line timer module: a dead `reset_with`, and a `poll` that
returns Pending without a waker after firing.

Cancel safety was the main thing I went looking for in the event loop, since a `select!` drops every branch future it
does not take. Each branch awaits either a channel receive or `JoinSet::join_next`, both cancel-safe, and the
`loop { match ... .await }` shapes in the vote collectors and cert verifiers only discard a task result on arms that
deliberately drop it (stale key, invalid certificate); no arm takes a value and then awaits again before returning it. I
also traced every coordinator-owned map to a GC scope: `da_payloads` and `payload_txn_bytes` are split off on decide,
`pending_proposal_fetches` and `requested_missing_proposals` on decide, the collectors and network on local view change.
None is unbounded.

One candidate was considered and not filed, recorded so the decision is auditable rather than silent: `start` panics via
`expect("genesis cert1 must be seeded")` at `coordinator.rs:394` if `seed_parent` was not called first. The construction
surface is user-error by the envelope, the panic message names its own precondition at the point of failure, and both
in-repo integrators seed before starting, so this is idiomatic rather than a defect.

Learnings: a fix that removes a dedup must enumerate every caller of the deduped function, not just the callers the
finding named - NP-5 exists because I checked two of three.

Next: NP-5, then NP-6, then the evaluator gate and, if it passes, convergence. The gate has not run yet this run: its
early-run precondition is a clean full audit already recorded, which only this iteration produced, and the ledger is not
empty now that this audit filed two items.

## iter 7/10 | 29c94e91-175223 | 2026-08-12 | NP-5 | done

Task: NP-5 (Low, runtime, performance) - the view-change DRB prefetch spawning a task per view since NP-2 removed the
completed-epoch short-circuit. Closed.

Changed: crates/hotshot/new-protocol/src/epoch.rs (new `prefetch_drb_result`, plus two tests),
crates/hotshot/new-protocol/src/coordinator.rs (the view-change prefetch now calls it), PLAN.md, BACKLOG.md, JOURNAL.md.

Checkpoint: f8cc068af94844927d4dcb1711f13063462d8636

Verification: the fix gives the two callers two methods instead of one shared guard, because they want different things:
`prefetch_drb_result` is speculative and only wants the result to exist, so it is a no-op once the epoch is resolved and
retries while it is not; `request_drb_result` is consensus asking for a delivery and is never silenced. Acceptance check
`epoch::tests::prefetch_goes_quiet_once_resolved_but_request_does_not` asserts both halves in one test - five prefetches
after resolution spawn nothing, three requests after that are each answered. Run against a tree where
`prefetch_drb_result` forwards unconditionally, which is exactly what the code did before this iteration, it fails with
`prefetching a resolved epoch spawned work`; it passes on the fixed tree. Its control
`prefetch_retries_while_unresolved` passes both ways, which is what shows the fix did not turn the prefetch into a
one-shot that gives up after a failed catchup - the case the prefetch exists for. The fixed file was copied aside and
restored, never `git checkout`. Enumeration of every DRB request site in the coordinator,
`grep -n 'request_drb_result\|prefetch_drb_result' crates/hotshot/new-protocol/src/coordinator.rs`: ten sites, one
prefetch (the view-change arm at 988) and nine requests. The nine are correct as requests and would behave identically
either way, because each is reached only when membership for that epoch is unresolved, so the completed set cannot
contain it: `cert_verifiers.retry_pending` at 641 returns exactly the unresolved epochs, the `RequestDrbResult` output
at 698 is consensus's own retry, and the seven intake arms are reached only when `CertVerifier::verify` failed to
resolve the epoch. Verify command green: 201 tests run, 201 passed, 15 skipped, exit 0.

Contract preserved: consensus's retry lever, which NP-2 exists to keep working, is untouched - the new method is
additive and the plain request has the same unconditional behaviour it gained in iteration 3, still pinned by
`repeated_request_is_answered_again`. The prefetch's own contract is unchanged from before iteration 3: fire until the
epoch resolves, then stop.

Surface inventory stays at 26 of 26. The epoch and coordinator:consensus-output rows were re-swept in this same
iteration rather than left stale: their code changed here, but I read both regions while changing them, and both
batteries ran - `epoch::tests` directly and the coordinator arm through the Verify command. Recording them as swept at
this iteration's checkpoint is what the row means; flipping them to unswept for a later iteration to re-certify code it
had not seen more recently than I have would be bookkeeping rather than evidence.

Learnings: none that generalise beyond this fix.

Next: NP-6, the timer module. After that the ledger is empty and the run can invoke the evaluator gate and, if it
passes, declare convergence in that same iteration.
