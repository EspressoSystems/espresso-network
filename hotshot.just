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

test-ci-1 *args:
  echo Running integration test group 1
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_1 {{args}}

test-ci-2 *args:
  echo Running integration test group 2
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_2 {{args}}

test-ci-3 *args:
  echo Running integration test group 3
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_3 {{args}}

test-ci-4 *args:
  echo Running integration test group 4
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_4 {{args}}

test-ci-5 *args:
  echo Running integration test group 5
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_5 {{args}}

test-ci-6-1 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:1/6 {{args}}

test-ci-6-2 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:2/6 {{args}}

test-ci-6-3 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:3/6 {{args}}

test-ci-6-4 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:4/6 {{args}}

test-ci-6-5 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:5/6 {{args}}

test-ci-6-6 *args:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test tests_6 --partition hash:6/6 {{args}}

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
