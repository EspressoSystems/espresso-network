# Maintain AGENTS.md

`AGENTS.md` (repo root, `CLAUDE.md` is a symlink to it) is loaded into every agent session. The docs it references are
read on demand. Both are token cost paid by every session.

Files in scope: `AGENTS.md`, `doc/agents/*.md`, and every doc referenced from those.

## Rules

1. `AGENTS.md` holds only facts every session needs.
2. Everything else goes in a file under `doc/agents/` or `doc/`, referenced from `AGENTS.md` by one line saying when to
   read it.
3. Facts only. No prose, no hedging, no marketing, no emoji, no summaries of what changed. Minimal markdown: headings,
   bullets, `path:line`, code spans.
4. Reasoning, evidence and verification never appear in the md files. They go in the report (see below).

## Checks

- Stale: every path, symbol, route, command, contract name, version claim still exists. Verify against the code,
  `justfile`, `data/genesis/*.toml`, `contracts/`.
- Wrong: claims about the running network. The runner has network access; query the public query services
  `https://query.main.net.espresso.network` and `https://query.decaf.testnet.espresso.network`. `/v1/status/metrics`
  (Prometheus: build info, view, epoch, height) and `/v1/config/runtime` carry the most per request. Paths without the
  `/v1` prefix redirect, so `curl -sL`.
- Misplaced: content not needed by every session moves out of `AGENTS.md`, leaving a trigger line.
- Duplicated: same fact in two files, keep one.
- Missing: architectural changes since the docs in scope were last touched
  (`git log --oneline -20 -- AGENTS.md doc/agents/`, then `git log --oneline --since` that date over `crates/`,
  `contracts/`) that every session needs.
- Bloat: rule 3 violations, and sections a session would never use.

## Steps

1. Run the checks. Record `path:line` or command output as evidence for each finding.
2. Edit the files in scope. Drop any change that is not evidenced.
3. Write `./tmp/agents-md-report.md`: one entry per change, each stating what changed, why, and the evidence
   (`path:line`, commit sha, command output).
4. No changes: revert any edits, write nothing to `./tmp/`, and stop.

Do not commit, push, or open a PR. The workflow does that from the working tree.
