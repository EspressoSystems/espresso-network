### A node whose state started from a snapshot no longer starts with `--archive`

Urgency: Medium

- A node whose merklized state started from a recent epoch boundary now refuses to start with `--archive`
  (`ESPRESSO_NODE_ARCHIVE`). Such a node ran with `--prune` on 0.7.0.5 or later and started with no merklized state,
  either fresh or after its state loop was stuck. Before, it started anyway, with incomplete trees below that boundary.
- Check before upgrading: a node in this state runs with `--archive` and logs
  `merklized state started from a boundary snapshot, not genesis, so its trees are incomplete` at every start.
- To keep it running, remove `--archive`, or set `--prune` again.
- To make it a full archive node, reset its storage, so the state is rebuilt from genesis.
