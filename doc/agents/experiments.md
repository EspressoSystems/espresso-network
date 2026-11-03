# Experiments

Experiments are node features that are off unless switched on at startup. The switch lives in
`crates/espresso/utils/src/experiment.rs`. `main` currently declares no variants; bench branches (e.g.
`bench-experimental`) add theirs.

## Adding an experiment

1. Add a variant to `Experiment` with a one-line doc. Its kebab-case name (`BlobDir` -> `blob-dir`) is the name in
   `ESPRESSO_NODE_EXPERIMENTS` and in `--list-experiments`.
2. Gate the code on `Experiment::X.enabled()`. Where `Options` is in scope, pass `&opt.experiments` and call
   `contains(Experiment::X)` instead, so tests need no global install.
3. A setting that carries a value (a path, a size) stays an ordinary clap option; the variant gates whether it applies.
4. Do not add ad-hoc env vars (`std::env::var` reads) for experiment switches.

## Off means unchanged

A reviewer must be able to see from the diff that the node behaves as before when the experiment is off:

- Gate once, at the outermost point that selects the new behavior (e.g. which backend to build), not in many places.
- The off branch is the existing code, unmoved and unedited. New code goes in the on branch or in its own module.
- Inputs only the experiment uses (options, env, files) are read inside the on branch.
- Existing tests cover the off path: `enabled()` is false unless `install` runs. On-path tests pass an
  `EnabledExperiments` where the code takes one; tests that call `install` rely on nextest's process per test.

Changes to the switch itself land on `main` first and are cherry-picked to bench branches.

## Enabling

- `ESPRESSO_NODE_EXPERIMENTS` / `--experiments`: comma list of names, or `all`. Unset or empty enables none.
- An unknown name, or `all` next to other names, exits 2 at startup.
- A node with a non-empty set logs one `experiments enabled` warning at startup.

## Listing

`--list-experiments` prints the binary's experiments as a JSON array and exits 0, before module parsing and genesis
load. In the image, call the binary directly; the entrypoint script prints to stdout:

```bash
docker run --rm --entrypoint /bin/espresso-node-postgres <image> --list-experiments
```

Binaries older than the switch exit 2 (unknown flag).

## Benchmarks

`scripts/network-bench/aws-bench` sets `ESPRESSO_NODE_EXPERIMENTS=all` on every node and runs `--list-experiments`
locally during preflight. Exit 2 records `[]`. The run summary lists the enabled experiments.
`--node-env ESPRESSO_NODE_EXPERIMENTS=a,b` selects a subset; names the image does not list are refused before any AWS
write.
