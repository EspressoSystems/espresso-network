# L1 provider requirements

What an Espresso node reads from its L1 JSON-RPC provider (`ESPRESSO_L1_PROVIDER`, optional `ESPRESSO_L1_WS_PROVIDER`),
for mainnet and decaf with the genesis files in `data/genesis/`.

## Summary

- Archive state is not needed. No `eth_call` or `eth_getStorageAt` targets a block older than the latest finalized one.
- Old block headers are not needed. The genesis L1 block is pinned in the genesis file.
- Historical logs of two contracts are needed, listed below.

## Historical logs

StakeTable, needed by all nodes on a fresh store. Logs from `initializedAtBlock()` to the latest finalized block:

| Network | Address                                      | From L1 block |
| ------- | -------------------------------------------- | ------------- |
| mainnet | `0xcef474d372b5b09defe2af187bf17338dc704451` | 24348377      |
| decaf   | `0x40304fbe94d5e7d1492dd90c53a2d63e8506a037` | 8077808       |

FeeContract, needed by unpruned query nodes with an empty state store. `Deposit` logs from the block after the genesis
`[l1_finalized]` block to the `l1_finalized` of the last header before the 0.6 upgrade:

| Network | Address                                      | From L1 block |
| ------- | -------------------------------------------- | ------------- |
| mainnet | `0x9fce21c3f7600aa63392a5f5713986b39bb98884` | 21116304      |
| decaf   | `0x42835083fd1d3fc5d799b5f6815ae4bf2623e6d0` | 6758038       |

## Requirements by path

- Head and finalized tracking, always: `eth_getBlockByNumber(latest, finalized)`, `eth_blockNumber`, and
  `eth_subscribe newHeads` or `eth_newBlockFilter` polling. Head only.
- Fee contract proxy check, every start: `eth_getStorageAt` at `latest`.
- Stake table events, first start: `eth_call initializedAtBlock()`, then `eth_getLogs` in chunks from the deployment
  block to finalized.
- Stake table events, every start and every 60 min: `eth_getLogs` from the last stored L1 block to finalized.
- Proposal validation: `eth_getBlockByHash`, `eth_getBlockByNumber` on recent finalized blocks.
- Light client state, query nodes: `eth_call` at `latest` or at the finalized block number, about 100 blocks of state.
- Fee deposits, unpruned query nodes replaying state from genesis: `eth_getLogs Deposit` in chunks over the FeeContract
  range above.

## Behavior when history is missing

- Stake table scan: each chunk retries for `ESPRESSO_L1_EVENTS_MAX_RETRY_DURATION` (20 min), then the node panics. The
  events are stored with their L1 block cursor, so the full scan runs once per fresh store. Peers do not supply events.
- Fee deposit replay: retries forever and logs warnings. The state replay stalls.

## Relevant options

- `ESPRESSO_L1_EVENTS_MAX_BLOCK_RANGE` (10000): block range per `eth_getLogs`. Lower it for providers with smaller
  limits.
- `ESPRESSO_L1_EVENTS_MAX_RETRY_DURATION` (20 min): stake table scan retry limit per chunk.
- `ESPRESSO_L1_PROVIDER`: comma-separated list; the node fails over between providers.
