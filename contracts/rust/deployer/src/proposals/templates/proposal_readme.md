# StakeTable V2 -> V3 Upgrade ({{NETWORK_LABEL}})

## Proposal metadata

- network: {{NETWORK}} (chainId={{CHAIN_ID}})
- source commit: {{SOURCE_COMMIT}}
- proxy: {{PROXY}}
- impl: {{IMPL}}
- timelock: {{TIMELOCK}} (OpsTimelock)
- salt: {{SALT}}
- delay: {{DELAY}} seconds
- predecessor: 0x0 (none)

## Safe hashes

Hashes are precomputed below assuming `operation=0` (single-tx direct call). Confirm the values match the Ledger display
and the Safe UI before signing. The nonces reflect the Safe's current on-chain nonce at proposal-generation time;
recompute with `verify-proposal` if additional transactions have been queued since then.

### Schedule (nonce={{SCHEDULE_NONCE}})

- Safe: {{SCHEDULE_SAFE}}
- domain: {{SCHEDULE_DOMAIN}}
- message: {{SCHEDULE_MESSAGE}}
- safe_tx: {{SCHEDULE_SAFE_TX}}

### Execute (nonce={{EXECUTE_NONCE}})

- Safe: {{EXECUTE_SAFE}}
- domain: {{EXECUTE_DOMAIN}}
- message: {{EXECUTE_MESSAGE}}
- safe_tx: {{EXECUTE_SAFE_TX}}

## Verify and recompute

Run this command to verify the proposal and recompute hashes (e.g. if the nonce has advanced):

```
deploy verify-proposal \
  --contract stake-table-v3 \
  --input {{SCHEDULE_PATH}} \
  --input {{EXECUTE_PATH}} \
  --safe {{VERIFY_SAFE}} \
  --nonce {{VERIFY_NONCE}}
```

Confirm all rows PASS and that domain/message/safe_tx hashes match the Ledger display.

## Signer flow

1. `git checkout {{SOURCE_COMMIT}}`
2. Run the verify command above with the correct Safe address and nonce.
3. Confirm all PASS and hashes match Ledger display.
4. Import `schedule.json` into Safe app, confirm Ledger domain+message+safe_tx hashes match.
5. Sign and submit.
6. After the delay elapses, repeat with `execute.json` (nonce N+1).
