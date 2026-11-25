## Inputs

- PR description: `pr_view`. Diff: `pr_diff`, also in `./tmp/pr.diff`. The diff is the scope.
- Checkout: the PR head. Read and Grep it for surrounding code, callers, tests.
- Earlier review comments: `./tmp/pr-review-comments.json`.

## Scope

Perform static review only; CI runs build, lint, format, test, serialization and API compatibility gates separately.
Review whether tests are adequate, not whether they pass.

## Prior threads

Read `./tmp/pr-review-comments.json` before anything else.

- Skip findings any reviewer already raised.
- Own threads: author `claude`, or author `github-actions` with a root body starting with `<!-- claude-gh -->`. For each
  unresolved one: `reply` (comment_id = root comment `databaseId`) on what changed; `resolve_thread` if fixed, explained
  or answered. Never resolve others' threads.

## Findings

- Call `candidate` the moment a possible defect is noticed, before investigating further.
- Keep every candidate with a nameable failure scenario, including half-believed ones. Dropping those is the main cause
  of missed bugs.
- Before posting, call `verdict` for every candidate: `wrong` only when quoted code shows the candidate misreads it;
  `duplicate` when another candidate covers it; otherwise `kept`.
- Low impact, "not a regression", "acceptable", "unlikely" or "probably fine" are not grounds to discard: keep the
  candidate and lower its severity to concern or nit.
- Post every kept candidate: max 10, most severe first, correctness before cleanup.
- Each: `path`, `line` (RIGHT side of the diff), one-line summary, failure scenario (inputs/state -> wrong result).
- Severity:
  - **blocking**: security, consensus or protocol correctness, data loss or state not persisted, breaking
    API/serialization/upgrade/config compatibility. A write the existing path performs and the new path skips or can
    skip is blocking, even if a restart or later catchup would repair it. Also blocking: a secret leaving the host, a
    task that stalls or retries forever, unbounded memory growth, a node that cannot restart.
  - **should fix**: bugs, missing tests for changed behavior, missing operator note, a storage backend left out.
  - **concern**: design or architecture trade-off.
  - **nit**: small cleanup.
- Do not report: intentional TODOs, micro-optimizations, anything CI checks.

## Output

1. One `inline_comment` per finding:

   ```markdown
   **<severity>**: <title>

   <Failure scenario, 1-3 sentences.>

   <Fix: `suggestion` block if it replaces the line, else one sentence.>
   ```

2. Last, one `comment`:

   ```markdown
   <1-3 lines: what the PR does.>

   - **<severity>**: <few words> ([path:line](https://github.com/<repo>/blob/<head sha>/<path>#L<line>))

   Checked: <what was verified, e.g. callers of X, failure paths of Y, storage round trip>.
   ```

   No findings: `No issues found.` plus the `Checked:` line. No praise, no restating the diff.
