# Recorded legacy chains

Chains recorded from legacy consensus, the protocol before 0.6. Mainnet and decaf still serve data
from before 0.6: old headers, VID schemes, reward trees, state certificates and the stake table
contract's V1 and V2 event history. Once legacy consensus is deleted no test network can produce
that data, so these recordings keep it testable. `LegacyChain::replay` in
`crates/espresso/node/src/legacy_chain.rs` starts a query node with no consensus on one of them: a
follower fed the recorded blocks, on an anvil restored from the recorded L1 state. The node then
derives the stake table, rewards and merklized state with the code a live node uses, checking every
derived root against its header.

**These recordings cannot be regenerated once legacy consensus is deleted.** A change to how any
recorded type deserializes has to keep reading them, as it has to keep reading the `data/v*`
reference vectors.

## Chains

Every chain has 5 nodes, epochs of 10 blocks, and transactions alternating between namespaces 101
and 102.

| Chain | Protocol | Stake table contract | Blocks | Epochs | State certs | Size |
|-------|----------|----------------------|-------:|-------:|------------:|-----:|
| `v3`  | 0.3 from genesis | V2 | 81 | 8 | 8 | 2.4 MB |
| `v4`  | 0.4 from genesis | V1, upgraded to V2 after epoch 2 with every commission raised by 1%, then to V3 after epoch 4 with a network config registered for one validator | 152 | 15 | 15 | 2.6 MB |
| `v5`  | 0.5 from genesis | V2 | 81 | 8 | 8 | 2.4 MB |

The `v4` chain's contract history is what mainnet's contract went through. The network config it
registered is in `network-config-update.json`, so a test can look for that validator, key and
address in the stake table the replayed node derives.

## Files

| File | Contents |
|------|----------|
| `genesis.toml` | The genesis the recorded validators ran with, as a genesis file. |
| `network-config.json` | The HotShot network config, which the replayed node loads from persistence since it has no peer to fetch it from. |
| `blocks.json` | Every block: leaf, block and VID common data, as the query service serves them. |
| `state-certs.json` | The light client state certificate of every finished epoch. |
| `network-config-update.json` | The validator, x25519 key and address `update_network_config` registered on the V3 contract (`v4` only). |
| `l1-state.json` | The anvil state dump, restored with `anvil --load-state`. It holds the chain's blocks, transactions, logs and latest accounts, but no per-block historical state, which nothing the node runs reads. |

## Recording

`record_legacy_chains` in `crates/espresso/node/src/legacy_chain/record.rs` recorded them, with
anvil 1.8.1 (foundry). It runs a `TestNetwork` on legacy consensus against an anvil that mines a
block a second and dumps its state, drives the contract upgrades from the test, and reads the chain
back through the query API and the first node's persistence. It is `#[ignore]`d:

```sh
cargo test -p espresso-node --features embedded-db --lib record_legacy_chains -- --ignored
```

`LEGACY_CHAINS=v4,v5` records only the chains named.

Anvil is not pinned. If a foundry upgrade changes the `--load-state` format, these dumps have to be
converted, not re-recorded.
