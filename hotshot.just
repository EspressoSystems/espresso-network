# Excluding packages with `--exclude` instead of the filterset prevents nextest from compiling them.
test-ci-rest *args:
  echo Running unit tests
  RUST_LOG=error cargo nextest run \
  --exclude "espresso-*" \
  --exclude client \
  --exclude hotshot-query-service \
  --exclude hotshot-state-prover \
  --exclude hotshot-testing \
  --exclude node-metrics \
  --exclude staking-cli \
  --exclude tests \
  --exclude diff-test-hotshot \
  --exclude gen-vk-contract \
  --exclude hotshot-contract-adapter \
  --profile hotshot --lib --bins --tests --benches --workspace {{args}}
