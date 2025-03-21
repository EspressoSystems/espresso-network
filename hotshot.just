test-ci-rest *ARGS:
  echo Running unit tests
  RUST_LOG=error cargo nextest run -E 'not (test(tests_1) | test(tests_2) | test(tests_3) | test(tests_4) | test(tests_5) | test(tests_6))' --profile hotshot --lib --bins --tests --benches --workspace --no-fail-fast {{ARGS}}

test-ci-1:
  echo Running integration test group 1
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_1 --no-fail-fast

test-ci-2:
  echo Running integration test group 2
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_2 --no-fail-fast

test-ci-3:
  echo Running integration test group 3
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_3 --no-fail-fast

test-ci-4:
  echo Running integration test group 4
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_4 --no-fail-fast

test-ci-5:
  echo Running integration test group 5
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_5 --no-fail-fast

test-ci-6-1:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:1/6

test-ci-6-2:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:2/6

test-ci-6-3:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:3/6

test-ci-6-4:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:4/6

test-ci-6-5:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:5/6

test-ci-6-6:
  echo Running integration test group 6
  RUST_LOG=error cargo nextest run --profile hotshot --test tests_6 --no-fail-fast --partition hash:6/6

# Usage:
#
#   just test memoryimpl_::test_success
#
# To display logs from a test run:
#
#   just test memoryimpl_::test_success --nocapture
test *ARGS:
  echo Running test {{ARGS}}
  cargo nextest run --profile hotshot {{ARGS}}

example *ARGS:
  cargo run --package hotshot-examples --example {{ARGS}}
