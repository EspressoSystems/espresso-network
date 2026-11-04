### Genesis file pins the L1 genesis block

Urgency: Low

Use the genesis file from this release tag (`data/genesis/mainnet.toml` or `data/genesis/decaf.toml`, shipped in the
image at `/genesis`). It adds `timestamp` and `hash` to `[l1_finalized]`, so the node no longer fetches that block from
L1 at startup.

Mainnet:

```toml
[l1_finalized]
number = 21116303
timestamp = "0x67291ceb"
hash = "0x8c580daf96a83884d4fac219dea3c26d7a805e4c92c91e04c5efc26929b4b129"
```

Decaf:

```toml
[l1_finalized]
number = 6758037
timestamp = "0x66f43a1c"
hash = "0x7063208185b352ec9732ee8eda32fa997dffa3a7abac945b9d91b21d85b173e2"
```

L1 history the node still needs: `doc/l1-provider.md`.
