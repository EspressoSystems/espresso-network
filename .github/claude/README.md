# Claude on GitHub

## Files

| File                    | Purpose                                                                           |
| ----------------------- | --------------------------------------------------------------------------------- |
| `claude_gh.py`          | The CLI the workflows call.                                                       |
| `review.md`             | Shared part of the `review` job: inputs, prior threads, findings, output.         |
| `review-standard.md`    | Review procedure, placed before `review.md` on regular PRs.                       |
| `review-backport.md`    | Placed before `review.md` instead on backport PRs (branch `backport-<n>-to-*`).   |
| `mention.md`            | Prompt for `@claude` mentions.                                                    |
| `code-review.md`        | Appended to the system prompt of the `/code-review` comment command.              |
| `backport-resolve.md`   | Prompt for resolving backport conflicts (`.github/workflows/backport.yml`).       |
| `maintain-agents-md.md` | Prompt for the weekly AGENTS.md audit (`.github/workflows/claude-agents-md.yml`). |
| `eval/`                 | Offline evaluation of the `review` job on PRs with known bugs.                    |
| `tests/`                | Unit tests for `claude_gh.py` and the eval's read-only GitHub. No network.        |

Commands: `just claude::fmt`, `just claude::lint`, `just claude::check`, `just claude::test`.

## PR review in CI

`.github/workflows/claude-review.yml` runs on non-draft, non-fork PRs when opened, reopened or marked ready; an OWNER or
MEMBER `/review` comment on a PR re-runs it on the current head via `claude.yml`. It calls `claude-job.yml`, which also
serves `claude.yml` (`@claude` mentions, `/review` and `/code-review`): `start` posts a status comment with the write
token. `run` fetches team review comments into `tmp/pr-review-comments.json` (plus original-PR context for backports)
and runs `claude_gh.py run review` with a read token and `ANTHROPIC_API_KEY`; Claude reads the diff and checkout and
records inline comments, candidates with verdicts, and a summary. `apply` posts the record with the write token and
edits the status comment with the summary and cost footer. All three jobs run `.github/claude` from one commit: the
merge commit for PR-open reviews, the default branch head for `claude.yml` (including `/review`).

## CI threat model

Applies to `backport.yml`, `claude-agents-md.yml` and `claude-job.yml`.

Inputs:

- Trusted: the workflow files, prompts and scripts at the ref the workflow runs from.
- Untrusted: the PR checkout (also from forks), merged code and docs Claude reads (can carry a prompt injection), and
  every model output.
- A malicious change that is reviewed and merged is out of scope.

Rules:

- Before review (PR open, `/review`, `/code-review` including forks, `@claude`), Claude only reads and records actions.
  Bash runs only on reviewed code (backport, scheduled agents-md) or same-repo PRs whose author already has write
  access.
- Bash jobs run Claude, and git or cargo on a tree Claude touched, only in `scripts/ci-sandbox`: no host processes,
  cleared env except PATH, HOME and named `--env` vars, empty home, nothing outlives it.
- Claude runs with `--setting-sources user --strict-mcp-config` in the Bash jobs: a committed `.claude/settings.json`
  (hooks), `.mcp.json`, skills or agents are not loaded. `backport.yml` still loads the tree's CLAUDE.md and AGENTS.md,
  which are reviewed, merged code.
- `zizmor.yml` audits `.github/` on every PR; `lint.yml` rejects invisible Unicode in tracked files.
- No write-token job checks out, executes or runs git against a tree Claude touched.
- Write-token jobs read an artifact only after a format-specific check.

| Workflow               | Job        | Token | Claude | Sandbox      | Handoff                  | Consumer check                                                |
| ---------------------- | ---------- | ----- | ------ | ------------ | ------------------------ | ------------------------------------------------------------- |
| `backport.yml`         | `backport` | write | no     | none         | `needs_claude` output    | none: PR numbers and branches come from `gh pr view`          |
| `backport.yml`         | `resolve`  | read  | yes    | `ci-sandbox` | bundles, shas, summaries | `push`: `git bundle verify`, sha equals the one cargo checked |
| `backport.yml`         | `push`     | write | no     | none         | reads artifact           | no conflict markers; pushes only the verified sha             |
| `claude-agents-md.yml` | `audit`    | read  | yes    | `ci-sandbox` | patch, report            | `propose`: paths only `AGENTS.md`, `doc/`; text hunks only    |
| `claude-agents-md.yml` | `propose`  | write | no     | none         | reads artifact           | `git apply` after the check, then `create-pull-request`       |
| `claude-job.yml`       | `start`    | write | no     | none         | status comment id        | `apply` edits only that comment                               |
| `claude-job.yml`       | `run`      | read  | yes    | none         | `actions.json` record    | `apply`: `claude_gh.py` validates the record                  |
| `claude-job.yml`       | `apply`    | write | no     | none         | reads artifact           | each action validated; allowed actions per job                |

Accepted risks:

- Claude can leak read context through comment text. In the Bash jobs the network stays open, so `ANTHROPIC_API_KEY` can
  also leave through any host.
- The sandboxed process's own environment is readable by itself, so every `--env` var is exposed to Claude.
- `backport.yml` `push` accepts any file content on the backport branch. Author and scope are not checked; human review
  of the backport PR is the control.
- `--share-net` leaves the host loopback reachable from the sandbox.
- `ANTHROPIC_API_KEY` is in Claude's environment in every Claude job. An injection could read it (for example through an
  aliased `/proc` path the deny rule misses) and post it encoded in a comment. A dedicated CI workspace with a spend
  limit bounds the loss.

## Evaluating review changes

`eval/eval.py` replays the `review` job on the PRs in `eval/cases.toml`, using the working tree's `claude_gh.py` and
`review*.md`.

- Never posts or modifies anything on GitHub: only the `run` step executes, against a GitHub whose write methods raise.
- Needs no local state: all files go under `tmp/claude-eval/`, worktrees under `tmp/claude-eval/wt/<pr>`.

### Prerequisites

- GitHub read access: `GH_TOKEN` if set, else `gh auth token`, else unauthenticated (60 requests per hour, enough for a
  few PRs).
- Claude: `ANTHROPIC_API_KEY`, or an existing `claude` login.
- Optional `CLAUDE_CONFIG_DIR`: a login without personal settings and CLAUDE.md:
  `CLAUDE_CONFIG_DIR=$PWD/tmp/claude-config claude /login`, then `export CLAUDE_CONFIG_DIR=$PWD/tmp/claude-config`.
- Optional `CLAUDE_CLI_PATH`: path to a `claude` binary. Needed on NixOS, where the SDK's bundled CLI does not run.

### Commands

```sh
just claude::eval                       # all cases, opus
just claude::eval --model fable 4670    # one case, another model
just claude::eval --prepare-only 4456   # worktree and event only, no Claude run, no cost
just claude::eval --out tmp/claude-eval/my-run 4456
just claude::eval --jobs 3              # at most 3 concurrent reviews (default 10)
```

### Results

`tmp/claude-eval/<UTC time>-<model>/` (or `--out`, which must not exist):

- `<pr>/event.json`, `<pr>/log.txt`, `<pr>/exit`, `<pr>/actions.json` (the record).
- `report.md`: per case the expected bug, cost and duration, the summary's findings, wrong candidates with evidence, and
  unjudged candidates.

### Scoring

Per case in `report.md`, compare findings and wrong candidates with `Bug`:

- caught: a finding names the bug.
- partial: a finding points at the right code with the wrong failure, or the bug was a candidate judged wrong (check the
  evidence).
- missed: neither.

### Adding a case

Append a `[[case]]` to `eval/cases.toml`: `pr` (introduced the bug), `fixed_by` (fix PR numbers), `bug` (one sentence:
what a reviewer of `pr` should notice). Keep only PRs whose diff introduced the bug, confirmed from the fix PR.

### Cost

Per PR: $0.6-2.5 with opus, about 2x with fable.

### Known limitations

- Prior review comments are not replayed (empty `tmp/pr-review-comments.json`): thread handling (`## Prior threads` in
  `review.md`) and skipping existing feedback are not evaluated.
- The PR title and body stay visible to the reviewer and may describe the bug or a later fix (a PR that fixes one bug
  and introduces another).
- One run per PR is noisy; results vary between runs. Compare over several runs before concluding.
