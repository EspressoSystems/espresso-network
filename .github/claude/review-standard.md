# PR review

Goal: find every real defect in the PR. Recall over precision.

- Steps 2-9 each collect up to 6 candidates. Do not judge them until Findings.

## Procedure

1. Map the change: what it touches, what it claims to fix or add.
2. Hunks. For each hunk, Read the whole enclosing function. Per line: which input, state, ordering or concurrency makes
   it wrong: wrong conditions or arithmetic, panics reachable from external input, work that blocks or waits on
   something that is not running yet, tasks never stopped.
3. Removed behavior and parity.
   - For each deleted or replaced line: name the invariant it enforced; find where the new code re-establishes it. Not
     found: candidate.
   - If the PR adds a path for something an existing path already does (another protocol version, another sync route,
     another backend, a sibling handler): Read the existing path. List its side effects in order (writes, in-memory
     updates, sends, spawned tasks). For each, find the same effect in the new path at the same position relative to
     fallible calls. Missing, conditional, or moved after an early return: candidate.
4. Failure paths. For each early return or error branch between two side effects (storage write, in-memory update,
   network send, spawned task):
   - What state is left: written, half-written, cached in memory only?
   - Does any later path retry or repair it, or does an early return or cache hit skip it forever?
   - Is the operation idempotent when retried or replayed at startup?
5. Callers and callees. Grep every changed symbol. Per call site: new precondition, changed return or error, changed
   ordering or timing. Per callee changed in the same PR: still safe here?
   - For each new or changed condition, check or `ensure!`: find where its inputs come from and list every value they
     take, including `None`, defaults, and data written before an upgrade or by the legacy path. A real value that now
     fails or passes: candidate.
6. Invariants untested by CI:
   - Storage: everything written can be read back, by this code and by the release still running on other nodes. No test
     covers this round trip; check it by reading the write and read paths. Every storage backend gets the change.
   - Upgrades: behavior gated on the right protocol version; both sides of a transition handled.
   - Boundaries: first/last item of an epoch or range, state before a feature activates, empty or genesis state.
   - Operator action needed: a `doc/operator-notes/*.md` fragment is present.
   - Secrets: can anything secret (keys, tokens, passwords, credentials in URLs) leave the process or reach a less
     trusted reader through this change? Follow error rendering and struct dumps, not only explicit fields.
7. Design at the type level: would better types make this defect or this runtime check impossible?
8. Tests. Would any test fail without the change? Changed behavior without a test that pins it: candidate.
9. Cleanup, only with a concrete cost: code duplicating an existing helper (name it); derivable state; dead code;
   repeated I/O or work on a hot path; a special case where fixing the shared mechanism is simpler (name the change).
