# Decaf testnet fixtures

Fetched 2026-07-08 from the decaf query service, https://query.decaf.testnet.espresso.network:

- `leaf_query_data.json`: `GET /v1/availability/leaf/10541613`. A `hotshot-query-service-types`
  `availability::LeafQueryData<SeqTypes>` (Leaf2 plus QuorumCertificate2). View number 11992982,
  `qc.data.epoch = Some(3514)`, `qc.data.block_number = 10541613`, header protocol version 0.5. The block is mid-epoch
  (offset 2613 of 3000), so no epoch-transition QC is involved.
- `stake_table.json`: `GET /v0/node/stake-table/3514`. A JSON array of 51 `hotshot_types` `PeerConfig<SeqTypes>`
  entries, the active stake table for epoch 3514.
