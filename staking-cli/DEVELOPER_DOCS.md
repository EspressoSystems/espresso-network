# staking-cli Developer Docs

The staking-cli can be used to fund the stake table on L1 for our testnet and demos.

<!-- markdown-toc start - Don't edit this section. Run M-x markdown-toc-refresh-toc -->
**Table of Contents**

- [staking-cli Developer Docs](#staking-cli-developer-docs)
  - [Demo Commands](#demo-commands)
    - [`demo stake`](#demo-stake)
    - [`demo delegate`](#demo-delegate)
    - [`demo undelegate`](#demo-undelegate)
    - [`demo churn`](#demo-churn)
  - [Delegation Configurations](#delegation-configurations)
  - [Deprecated Commands](#deprecated-commands)

<!-- markdown-toc end -->

## Demo Commands

The `demo` subcommand provides tools for testing and demonstration purposes.

### `demo stake`

Register validators and create delegators for testing.

    staking-cli demo stake --num-validators 5

Options:

- `--num-validators`: Number of validators to register (default: 5)
- `--num-delegators-per-validator`: Number of delegators per validator (default: random 2-5, max: 100,000)
- `--delegation-config`: Delegation configuration mode (default: variable-amounts)
  - `equal-amounts`: All validators have equal delegation amounts
  - `variable-amounts`: Variable delegation amounts per validator
  - `multiple-delegators`: Multiple delegators per validator
  - `no-self-delegation`: Validators do not self-delegate

Example:

    staking-cli demo stake --num-validators 10 --num-delegators-per-validator 50

### `demo delegate`

Mass delegate to existing validators with deterministic delegator generation.

    staking-cli demo delegate \
      --validators 0xAAA,0xBBB,0xCCC \
      --delegator-start-index 0 \
      --num-delegators 100 \
      --min-amount 100 \
      --max-amount 500

Options:

- `--validators`: Comma-separated validator addresses to delegate to
- `--delegator-start-index`: Starting index for deterministic delegator generation
- `--num-delegators`: Number of delegators to create
- `--min-amount`: Minimum delegation amount in ESP
- `--max-amount`: Maximum delegation amount in ESP
- `--batch-size`: Number of transactions to submit per batch (default: all at once)
- `--delay`: Delay between batches (e.g., "1s", "500ms"); requires `--batch-size`

Delegators are distributed round-robin across validators.

### `demo undelegate`

Mass undelegate from validators. Queries on-chain delegation amounts and undelegates everything.

    staking-cli demo undelegate \
      --validators 0xAAA,0xBBB \
      --delegator-start-index 0 \
      --num-delegators 100

Options:

- `--validators`: Comma-separated validator addresses to undelegate from
- `--delegator-start-index`: Starting index for delegator generation
- `--num-delegators`: Number of delegators
- `--batch-size`: Number of transactions to submit per batch (default: all at once)
- `--delay`: Delay between batches (e.g., "1s", "500ms"); requires `--batch-size`

Skips delegators with zero delegation to a validator.

### `demo churn`

Continuous delegation/undelegation activity forever. Useful for testing stake table changes.

    staking-cli demo churn \
      --validator-start-index 20 \
      --num-validators 5 \
      --delegator-start-index 0 \
      --num-delegators 50 \
      --min-amount 100 \
      --max-amount 500 \
      --delay 2s

Options:

- `--validator-start-index`: Starting mnemonic index for validators (default: 20)
- `--num-validators`: Number of validators to target
- `--delegator-start-index`: Starting index for delegator generation
- `--num-delegators`: Number of delegators in the pool
- `--min-amount`: Minimum delegation amount in ESP
- `--max-amount`: Maximum delegation amount in ESP
- `--delay`: Delay between operations (default: "1s")

The churn loop picks random delegators and either delegates (if idle) or undelegates (if delegated).

## Delegation Configurations

Currently supported delegation configurations for `demo stake`:

1. Equal amounts: each validator self delegates an equal amount. Leading to uniform staking weights.
2. Variable amounts: validators delegate 100, 200, ..., 500 ESP tokens in order. This is currently the default because it used to be the only option.
3. Multiple delegators: Like 2, but also adds a randomly chosen number of other delegators to each validator.
4. No self-delegation: Validators do not self-delegate, only external delegators.

## Deprecated Commands

The `stake-for-demo` command is deprecated and will be removed in a future release.
Use `demo stake` instead.
