# Excluding packages with `--exclude` instead of the filterset prevents nextest from compiling them.
# We should find a better way to handle this for example by having consistent crate names.
test-ci-rest *args:
  echo Running unit tests
  RUST_LOG=error cargo nextest run \
  --exclude "marketplace-*" \
  --exclude "sequencer*" \
  --exclude builder \
  --exclude client \
  --exclude espresso-* \
  --exclude hotshot-builder-core-refactored \
  --exclude hotshot-query-service \
  --exclude hotshot-state-prover \
  --exclude hotshot-testing \
  --exclude node-metrics \
  --exclude staking-cli \
  --exclude tests \
  --exclude diff-test-hotshot \
  --exclude gen-vk-contract \
  --profile hotshot --lib --bins --tests --benches --workspace {{args}}

# Build and run an integration test target
test-ci test *args:
  echo Running integration test group {{test}}
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test {{test}} {{args}}

# Run all hotshot integration tests
test-all *args:
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot {{args}}

# List all integration test targets in the hotshot-testing crate
list-tests:
    cargo metadata --no-deps --format-version 1 | jq -r '.packages[] | select(.name == "hotshot-testing") | .targets[] | select(.kind[] == "test") | .name'

# Build the github action matrix from integration test targets
matrix:
    #!/usr/bin/env bash
    targets=$(just hotshot::list-tests)

    # Prefix with "test-ci" and make a JSON array
    test_suites=$(printf '%s\n' $targets | jq -R . | jq -s 'map("test-ci " + .)')

    # Add the extra "test-ci-rest"
    test_suites=$(echo "$test_suites" | jq '. + ["test-ci-rest"]')

    echo "{\"test_suites\": $test_suites}" | jq -c

# Usage:
#
#   just test memoryimpl_::test_success
#
# To display logs from a test run:
#
#   just test memoryimpl_::test_success --nocapture
test *args:
  echo Running test {{args}}
  cargo nextest run -p hotshot-testing --profile hotshot {{args}}

example *args:
  cargo run --package hotshot-examples --example {{args}}
