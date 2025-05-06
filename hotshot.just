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

test-ci test *args:
  echo Running integration test group {{test}}
  RUST_LOG=error cargo nextest run -p hotshot-testing --profile hotshot --test {{test}} {{args}}

test-ci-all *args:
    just hotshot::test-ci test_epoch_success_catchup_types {{args}}
    just hotshot::test-ci test_epoch_success_overlap_2_f {{args}}
    just hotshot::test-ci test_epoch_success_overlap_2f_plus_1 {{args}}
    just hotshot::test-ci test_epoch_success_overlap_3f {{args}}
    just hotshot::test-ci test_epoch_success_overlap_dynamic {{args}}
    just hotshot::test-ci test_epoch_success_overlap_f {{args}}
    just hotshot::test-ci test_epoch_success_overlap_f_plus_1 {{args}}
    just hotshot::test-ci test_epoch_success_types {{args}}
    just hotshot::test-ci test_epoch_success_types_randomized_leader {{args}}
    just hotshot::test-ci tests_1 {{args}}
    just hotshot::test-ci tests_10 {{args}}
    just hotshot::test-ci tests_2 {{args}}
    just hotshot::test-ci tests_3 {{args}}
    just hotshot::test-ci tests_4 {{args}}
    just hotshot::test-ci tests_5 {{args}}
    just hotshot::test-ci tests_6 {{args}}
    just hotshot::test-ci tests_7 {{args}}
    just hotshot::test-ci tests_8 {{args}}
    just hotshot::test-ci tests_epoch_1 {{args}}
    just hotshot::test-ci tests_epoch_3 {{args}}
    just hotshot::test-ci tests_epoch_4 {{args}}
    just hotshot::test-ci tests_epoch_5 {{args}}
    just hotshot::test-ci tests_epoch_6 {{args}}
    just hotshot::test-ci tests_epoch_7 {{args}}
    just hotshot::test-ci tests_epoch_8 {{args}}

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
