# L1 provider requirements

What an Espresso node reads from its L1 JSON-RPC provider (`ESPRESSO_L1_PROVIDER`, optional `ESPRESSO_L1_WS_PROVIDER`),
for mainnet and decaf with the genesis files in `data/genesis/`.

## Summary

- Archive state is not needed. No `eth_call` or `eth_getStorageAt` targets a block older than the latest finalized one.
- Old block headers are not needed. The genesis L1 block is pinned in the genesis file.
- Historical logs of two contracts are needed:

| Network | Contract    | Address                                      | Logs from L1 block | Logs to L1 block | Needed by                         |
| ------- | ----------- | -------------------------------------------- | ------------------ | ---------------- | --------------------------------- |
| mainnet | StakeTable  | `0xcef474d372b5b09defe2af187bf17338dc704451` | 24348377           | finalized        | all nodes, fresh store            |
| mainnet | FeeContract | `0x9fce21c3f7600aa63392a5f5713986b39bb98884` | 21116304           | 0.6 upgrade      | unpruned query nodes, empty state |
| decaf   | StakeTable  | `0x40304fbe94d5e7d1492dd90c53a2d63e8506a037` | 8077808            | finalized        | all nodes, fresh store            |
| decaf   | FeeContract | `0x42835083fd1d3fc5d799b5f6815ae4bf2623e6d0` | 6758038            | 0.6 upgrade      | unpruned query nodes, empty state |

- StakeTable start: `initializedAtBlock()` on the contract.
- FeeContract start: the block after the genesis `[l1_finalized]` block. The range ends at the `l1_finalized` of the
  last header before the 0.6 upgrade.

## Requirements by path

| Path                                    | RPC methods                                                                                                              | History                                     | When                                   |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------- | -------------------------------------- |
| Head and finalized tracking             | `eth_getBlockByNumber(latest, finalized)`, `eth_blockNumber`, `eth_subscribe newHeads` or `eth_newBlockFilter` + polling | head                                        | always                                 |
| Fee contract proxy check                | `eth_getStorageAt` at `latest`                                                                                           | none                                        | every start                            |
| Stake table events, fresh store         | `eth_call initializedAtBlock()`, `eth_getLogs` in chunks                                                                 | StakeTable deployment block to finalized    | first start                            |
| Stake table events, existing store      | `eth_getLogs`                                                                                                            | last stored L1 block to finalized           | every start, every 60 min              |
| Proposal validation                     | `eth_getBlockByHash`, `eth_getBlockByNumber`                                                                             | recent finalized blocks                     | steady state                           |
| Light client state                      | `eth_call` at `latest` or at the finalized block number                                                                  | about 100 blocks of state                   | query nodes                            |
| Fee deposits, state replay from genesis | `eth_getLogs Deposit` in chunks                                                                                          | genesis L1 block to the last pre-0.6 header | archive or unpruned query, empty state |

## Behavior when history is missing

- Stake table scan: each chunk retries for `ESPRESSO_L1_EVENTS_MAX_RETRY_DURATION` (20 min), then the node panics. The
  events are stored with their L1 block cursor, so the full scan runs once per fresh store. Peers do not supply events.
- Fee deposit replay: retries forever and logs warnings. The state replay stalls.

## Relevant options

- `ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE` (10000): block range per `eth_getLogs`. Lower it for providers with smaller
  limits.
- `ESPRESSO_L1_EVENTS_MAX_RETRY_DURATION` (20 min): stake table scan retry limit per chunk.
- `ESPRESSO_L1_PROVIDER`: comma-separated list; the node fails over between providers.
