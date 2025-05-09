# staking-cli Developer Docs

The staking-cli can be used to fund the stake table on L1 for our testnet and demos.

```
cargo run --bin staking-cli -- stake-for-demo --help

Usage: staking-cli stake-for-demo [OPTIONS]

Options:
      --num-validators <NUM_VALIDATORS>
          The number of validators to register.

          The default (5) works for the local native and docker demos.

          [default: 5]

      --delegation-config <DELEGATION_CONFIG>
          [default: VariableAmounts]
          [possible values: equal-amounts, variable-amounts, multiple-delegators]

  -h, --help
          Print help (see a summary with '-h')
```

Currently supported are these delegation configurations:

1. Equal amounts: each validator self delegates an equal amount. Leading to uniform staking weights.
2. Variable amounts: validator delegate 100, 200, ..., 500 ESP tokens in order. This is currently the default because it
   used to be the only option.
3. Multiple delegators: Like 2, but also adds a randomly chosen number of other delegators to each validator.
