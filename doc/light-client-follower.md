# Light-client follower

A follower is an Espresso query node without stake. It does not run consensus; it follows the chain through the
[light client](../crates/light-client) and serves the same query API a staked query node serves: availability, node,
catchup, merklized fee/block/reward state, reward claims, stake tables, state certificates, config, explorer,
light-client proofs, submit.

## Running one

```sh
espresso-node \
  --l1-provider-url <L1 RPC URL> \
  --genesis-file data/genesis/decaf.toml \
  -- storage-sql --postgres-host ... \
  -- http --port 24000 \
  -- query --peers https://query.decaf.testnet.espresso.network \
  -- follower -- catchup -- config -- submit -- light-client -- explorer
```

The `follower` module selects follower mode. Its upstreams are the query module's `--peers` (`ESPRESSO_NODE_API_PEERS`),
which must be set. The follower needs:

- the `storage-sql`, `http` and `query` modules: merklized state, rewards and the explorer need SQL storage;
- an L1 RPC, for fee deposits and the stake table, like any node;
- upstream query nodes that serve the `light-client`, `catchup`, `config` and (for `submit`) `submit` modules.

It does not need staking keys, an orchestrator or cliquenet; those options are ignored. `--state-peers` and
`--config-peers` default to the upstreams. Telemetry is disabled with a warning: a follower has no staking key to
identify itself with.

Tuning, as options of the `follower` module:

- `--poll-interval` (`ESPRESSO_NODE_FOLLOWER_POLL_INTERVAL`, 1s): how often to poll the upstreams for new blocks.
- `--max-blocks-per-poll` (`ESPRESSO_NODE_FOLLOWER_MAX_BLOCKS_PER_POLL`, 10): how many blocks one poll ingests. Older
  heights are backfilled by the query service, so a fresh follower serves the tip promptly and fills history in the
  background. Before it stores a block of epoch `E`, the follower checks that the stake table of `E+1` is held, and
  catches it up from storage, the LC contract and the upstreams if not.
- `--light-client-genesis <file>` (`ESPRESSO_NODE_LIGHT_CLIENT_GENESIS`): pin the light client's root of trust to a
  reviewed TOML file (see `crates/light-client-query-service/genesis/`) instead of deriving it from the network config
  fetched from the peers.

Against the native demo (`just demo-native`), node 0 on port 24000 serves everything a follower needs:
`-- query --peers http://localhost:24000 -- follower`.

## How it works

```
upstreams ──HTTP──▶ LightClient (verify) ──▶ follow loop ──▶ query database
                                                 │                │
                                                 │                └─▶ merklized state loop (fee, block, reward trees)
                                                 ├─▶ epoch root: stake table two epochs out, from L1
                                                 └─▶ transition block: DRB for the next epoch
```

Each poll fetches the verified chain height, then for every new block the leaf with its finality proof, the payload and
VID common, and the cert2 that finalizes it, and appends them to the query database. The merklized state loop that every
SQL query node runs recomputes the fee, block and reward trees from the stored chain and checks every root against the
header, so the state, reward and catchup endpoints need nothing from consensus.

At each epoch root the follower derives the stake table two epochs out from L1, exactly as a validator does, and at each
transition block it supplies the DRB the leaf carries. Stake tables, validators and block rewards come from that
coordinator; state certificates are fetched from an upstream on demand and validated against the L1-derived stake table.

`submit` forwards transactions to the first upstream that accepts them, after the usual block size check.

## Trust model

Verified from the upstreams: leaves (finality proof against the stake table of the block's epoch, rooted at the genesis
stake table and each epoch root's `next_stake_table_hash`), headers (Merkle inclusion against a verified leaf), payload
and VID common (against the header), cert2 (threshold signature), state certificates (`validate_state_cert`), merklized
state (recomputed locally).

Trusted like a validator: the L1 RPC, and the network config on first start (persisted, else fetched from
`--config-peers`). Pin `--light-client-genesis` to stop trusting the peers for the root of trust.

Not verified, liveness only: the upstreams' claimed block height and submit forwarding.

## What a follower cannot serve

| Endpoint               | Behaviour                                                         |
| ---------------------- | ----------------------------------------------------------------- |
| `state-signature/*`    | 404: only a validator signs light client states                   |
| `status/keys`          | 404: no validator keys                                            |
| `node/participation/*` | empty: participation is derived from votes                        |
| `node/vid/share/*`     | 404 when not stored: no request-response protocol to fetch shares |

Everything else is served from the same storage a staked query node fills from its decides.

## Cold start

A fresh follower starts serving from the tip and backfills history like any fresh query node. The merklized state loop
replays from genesis, so state, reward and catchup endpoints lag until the backfill completes; restore the database from
a snapshot to skip it. A restart resumes from the height the database has reached.
