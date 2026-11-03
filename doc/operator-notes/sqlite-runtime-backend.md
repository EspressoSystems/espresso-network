### SQLite is now chosen at runtime

Urgency: Low

The node picks SQLite or Postgres at startup, so there is one binary for both.

- Docker: nothing to do. `ESPRESSO_NODE_EMBEDDED_DB=true` still selects SQLite. `/bin/espresso-node-sqlite` still
  exists, but it is deprecated and prints a warning.
- Built from source: the `espresso-node-sqlite` package and binary are gone. Build `espresso-node` instead, and run it
  with `storage-sql` and `--embedded-db` (or `ESPRESSO_NODE_EMBEDDED_DB=true`). Keep `ESPRESSO_NODE_STORAGE_PATH` as it
  was, so the node opens the same database.
- `ESPRESSO_NODE_EMBEDDED_DB` accepts the usual boolean spellings. `false`, `0`, `no`, `off` and an empty value select
  Postgres. Other values select SQLite.
