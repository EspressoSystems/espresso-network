# Recorded legacy chains

Chains recorded from legacy consensus, the protocol before 0.6. Mainnet and decaf still serve data from before 0.6: old
headers, VID schemes, reward trees, state certificates and the stake table contract's V1 and V2 event history. The node
runs only the new protocol, so no test network can produce that data any more; these recordings keep it testable.
`LegacyChain::replay` in `crates/espresso/node/src/legacy_chain.rs` starts a query node with no consensus on one of
them: a follower fed the recorded blocks, on an anvil that has mined the recorded L1 transactions again. The node then
derives the stake table, rewards and merklized state with the code a live node uses, checking every derived root against
its header.

**These recordings cannot be regenerated: the node no longer runs legacy consensus.** A change to how any recorded type
deserializes has to keep reading them, as it has to keep reading the `data/v*` reference vectors.

## Chains

Every chain has transactions alternating between namespaces 101 and 102. All but `v2-v3` have 5 nodes and epochs of 10
blocks.

| Chain   | Protocol                                                                                 | Stake table contract                                                                                                                             |            Blocks | Epochs |       State certs |   Size |
| ------- | ---------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------: | -----: | ----------------: | -----: |
| `v3`    | 0.3 from genesis                                                                         | V2                                                                                                                                               |                81 |      8 |                 8 | 0.7 MB |
| `v4`    | 0.4 from genesis                                                                         | V1, upgraded to V2 after epoch 2 with every commission raised by 1%, then to V3 after epoch 4 with a network config registered for one validator |               152 |     15 |                15 | 1.0 MB |
| `v5`    | 0.5 from genesis                                                                         | V2                                                                                                                                               |                81 |      8 |                 8 | 0.8 MB |
| `v2-v3` | 0.2 from genesis, upgraded to 0.3 at block 321 as mainnet was, with epochs of 200 blocks | V2, deployed with the upgrade                                                                                                                    | 1001 (332 at 0.2) |      5 | 4 (epochs 2 to 5) | 4.3 MB |

The `v2-v3` chain is a single node: legacy consensus decides the 0.3 upgrade only on one node, and only with epochs
longer than the upgrade takes. The `v4` chain's contract history is what mainnet's contract went through. The network
config it registered is in `network-config-update.json`, so a test can look for that validator, key and address in the
stake table the replayed node derives.

## Files

| File                         | Contents                                                                                                                                                                                                                                                                                                                    |
| ---------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `genesis.toml`               | The genesis the recorded validators ran with, as a genesis file.                                                                                                                                                                                                                                                            |
| `network-config.json`        | The HotShot network config, which the replayed node loads from persistence since it has no peer to fetch it from.                                                                                                                                                                                                           |
| `blocks.json`                | Every block: leaf, block and VID common data, as the query service serves them.                                                                                                                                                                                                                                             |
| `state-certs.json`           | The light client state certificate of every finished epoch.                                                                                                                                                                                                                                                                 |
| `network-config-update.json` | The validator, x25519 key and address `update_network_config` registered on the V3 contract (`v4` only).                                                                                                                                                                                                                    |
| `l1-blocks.json`             | Every L1 block's timestamp and signed transactions, indexed by block number; block 0 is anvil's genesis block and has none. The replay mines them into a fresh anvil one block at a time, so contract addresses, logs and block numbers come out as recorded. Block hashes do not, and nothing the node runs compares them. |

## Recording

`record_legacy_chains` recorded them. It runs a `TestNetwork` on legacy consensus against an anvil that mines a block a
second, drives the contract upgrades from the test, reads the chain back through the query API and the first node's
persistence, and exports the L1 block by block with `eth_getBlockByNumber` and `eth_getRawTransactionByHash`. It needs
legacy consensus, which the node no longer runs, so it is kept off `main`: it is
`crates/espresso/node/src/legacy_chain/record.rs` at the `legacy-chain-recorder` tag (commit `319dfd916d5`, also the
head of the branch of the same name), `#[ignore]`d:

```sh
git switch --detach legacy-chain-recorder
cargo test -p espresso-node --features embedded-db --lib record_legacy_chains -- --ignored
```

`LEGACY_CHAINS=v4,v5` records only the chains named.

The L1 recording is standard signed transactions and timestamps, replayed into a fresh anvil, so no tool version is
baked into the fixtures.

## Outstanding

- v1 reward proofs for 0.3 heights: the state loop stores no v1 paths, so `v3` balances are not queryable (#5170).
- Nothing exercises a full sync of mainnet or decaf history. A scheduled job that syncs a node from genesis would cover
  the pre-0.6 data these recordings approximate.
