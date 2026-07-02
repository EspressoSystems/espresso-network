# Committed upgrade proposals

## Directory convention

```
contracts/deployments/proposals/
  <network>/
    <YYYYMMDD>-<slug>/
      contract        # one line: the --contract kind (e.g. stake-table-v3)
      schedule.json   # Safe-tx-builder batch for the timelock schedule call
      execute.json    # Safe-tx-builder batch for the timelock execute call
      README.md       # source commit, impl, salt, delay, expected Safe hashes
```

For multisig-direct upgrades (no timelock) a single `upgrade.json` replaces the two files.

The `contract` file contains the `--contract` value accepted by `deploy verify-proposal` (one of: `stake-table-v2`,
`stake-table-v3`, `esp-token-v2`, `fee-contract`, `reward-claim`).

## Verification

Each proposal is verified by the CI job on PRs touching `contracts/deployments/proposals/**`:

```
deploy verify-proposal contracts/deployments/proposals/<network>/<date>-<slug>
```

The CI job reads the contract kind and both JSON files from the proposal directory. The job forks the network RPC
(Sepolia for decaf/mainnet). A proposal that fails bytecode or governance checks cannot merge.

## Signer flow

1. Check out the commit recorded in the proposal README.
2. Run `deploy verify-proposal <dir> --safe <safe> --nonce <nonce>`.
3. Confirm all rows PASS and Safe hashes match the README values.
4. Import the JSON file(s) into the Safe app.
5. Confirm the Ledger displays the same domain+message+safe_tx hashes.
6. Sign and submit.

## Networks

- `decaf/` — Decaf testnet (Sepolia chainId=11155111)
- `mainnet/` — Espresso mainnet (Ethereum mainnet chainId=1)
