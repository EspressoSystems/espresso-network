# AGENTS.md

Espresso Network is a confirmation layer for Ethereum rollups. `crates/espresso/` is the application, `crates/hotshot/`
the generic BFT consensus library it is built on, `contracts/` the Solidity L1 side.

## Where to look

- `doc/agents/rust.md` - before running cargo or writing Rust: commands, conventions, storage, tests, adding an API
  endpoint
- `doc/agents/refinery-migrations.md` - before writing a SQL migration that touches existing rows
- `doc/agents/solidity.md` - before touching `contracts/`
- `API.md` - the v1 and v2 APIs, and how to add a v2 endpoint
- `doc/agents/architecture.md` - transaction and block flow, L1 reads, stake table, consensus upgrades
- `doc/agents/protocol-versions.md` - what each version changed, and what a network runs
- `doc/agents/live-chains.md` - querying mainnet or decaf over the query-service API
- `doc/agents/rewards.md` - reward accumulation and the L1 claim path
- `doc/cargo-features.md` - feature gates for zkVM builds and which functions panic without them
- `doc/upgrades.md` - configuring and running a consensus upgrade
- `doc/software-releases.md` - release branches, `MAJOR.MINOR.PHASE.PATCH` tags, the tracker issue and its commands
- `doc/espresso-dev-node.md` - single-process dev node, for rollup integration work
- `doc/light-client-follower.md` - a query node without stake that follows the chain through the light client
- `doc/pup.md` - `pup` Datadog CLI, for logs and metrics of Espresso's own infrastructure

## Test-first workflow

Every code change follows these steps. The tests describe the planned behavior before any code exists. Only the user can
change them. Formatting a test with `cargo fmt` does not count as a change to it.

- When no test can check the change, skip step 1 and say so in the report. For example: a dependency bump, CI config, or
  a log message.
- When the user asks for an edit to a test, that request is the approval for that edit.

### 1. A subagent writes the tests first

- When the change adds public items, the main agent first adds them as stubs: the planned signatures, with `todo!()`
  bodies. Then each new test compiles and fails on its own.
- Then the main agent spawns a subagent to write the tests. No other implementation code exists yet.
- Brief it with the planned behavior: the public signatures, inputs, expected results, edge cases, and error cases. Do
  not give it the planned implementation, so the tests check what the change must do, not how it does it.
- Tests use the public API of the module, not private details. Then the implementation can change without changing the
  tests. When a bug is in a private function that no focused test can reach through the public API, test that function
  in the module's `#[cfg(test)]` block.
- For a bug fix, the first test reproduces the bug.
- For a pure refactor, the subagent checks that existing tests cover the code, and adds tests where they do not. These
  tests pass before the refactor and must still pass after it.
- For any other change, run the new tests before you implement. Each must fail for the reason the change addresses: the
  missing behavior or the bug. A typo or a missing import does not count. Read the failure message to check. A test that
  already passes does not test the change: send it back to the subagent to fix.
- Then stop and show the user the new tests: the name of each and one line on what it checks. Start step 2 only after
  the user answers.

### 2. Implement without changing the tests

- The tests from step 1 are fixed. Do not edit, delete, rename, or `#[ignore]` them. Do not weaken them: no changes to
  assertions, expected values, inputs, or timeouts.
- When a new test fails, fix the implementation, not the test.
- The change is done when the new tests pass and the existing tests of the changed packages still pass
  (`cargo nextest run -p <package>`). CI runs the rest.
- A test that also fails on `main` does not block the change. Say so in the report. Issue #4585 lists flaky and failing
  tests on `main`.

### 3. Changing the new tests needs the user

- If the main agent thinks a test from step 1 is wrong, it stops and asks the user. For example: the expected value is
  wrong, the test checks the wrong thing, or the plan changed.
- Tell the user which test, what is wrong, and the exact edit you propose. Do not edit the test until the user answers.
- A subagent that thinks a test is wrong reports it to the main agent. It does not edit the test.

### 4. Changing existing tests needs the user

- An existing test is any test code or test data that existed before the task started: `#[cfg(test)]` modules, `tests/`,
  `slow-tests/`, `contracts/test/`, the crates `crates/test-utils`, `crates/test-server` and
  `crates/hotshot/example-types`, test helpers and fixtures (also those behind `feature = "testing"`), the `reference`
  vectors in `data/`, `data/insta_snapshots/`, and `.gas-snapshot`.
- If the change needs an edit to an existing test, flag it and let the user decide. Do not edit the test until the user
  answers.
- Tell the user the test name and `file:line`, why the change breaks it, and the options. Usually: change the
  implementation so the test passes as it is, or edit the test (show the edit).
- This includes small mechanical edits, such as a renamed function or a new argument. List them all in one message, so
  the user can decide once.
- This includes regenerating `reference` files and commitment constants after a type change (see `doc/agents/rust.md`).
- A subagent that needs an edit to an existing test reports it to the main agent. It does not edit the test.

### Subagents and reports

- When you brief a subagent that writes or changes code, include these rules in the brief.
- In the final report, list the new tests. Also list every existing test you changed, and when the user approved it.

## Writing Reviewable Code

Reviewing is the bottleneck. Default to changes that minimize reviewer time.

### Diff shape

- Each PR is one self-contained change. Split larger work into a stack.
- Separate refactors and renames from behavior changes. Every commit compiles and passes tests.
- Delete code in its own commit.
- Don't spread small edits across many files when one file would do.

### Reading flow

- Order code top-down: public API first, helpers below. Reviewers read in declaration order.
- Each function fits on one screen. Extract named sub-steps; don't use comments to mark sections.
- Comment the _why_ only when it isn't visible from types or code. Don't narrate the _what_.

### Commits and PRs

- Subject: imperative, scoped, under 70 chars (`feat(stake-table): ...`).
- When work is ready, suggest an updated PR description capturing the final changeset.
- Tell the reviewer where to focus.
- Link the regression test or `reference` test when touching serializable types.

## Key files

- `justfile` - build/test/deploy commands
- `data/genesis/*.toml` - genesis configs, including the protocol version each network runs
- `data/v*/` - reference serialization test vectors
