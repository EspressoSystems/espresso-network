### Retired consensus overrides

Urgency: Low

Remove `--builder-urls` / `ESPRESSO_BUILDER_URLS` from node launch settings, and
`--builder-urls` / `ESPRESSO_ORCHESTRATOR_BUILDER_URLS` and
`--builder-timeout` / `ESPRESSO_ORCHESTRATOR_BUILDER_TIMEOUT` from orchestrator settings.
Also remove orchestrator `--next-view-timeout`, `--start-delay`, `--timeout-ratio`, and
`--round-start-delay` (or their `ESPRESSO_ORCHESTRATOR_*` environment equivalents).
These timing options do not control the new consensus protocol.

The options remain accepted with a warning, but no longer overwrite network configuration metadata.
Leaders already build blocks in-process, so these options have no effect on block production or consensus timing.

Historical builder, next-view timeout, view-sync, bootstrap-node, GPU fixed-leader and proposal request-delay fields
remain in saved/network configuration and API responses for compatibility. They do not control the
new consensus protocol. The node runtime-config API continues reporting supplied deprecated builder
URLs for compatibility; this does not indicate an active external builder.
