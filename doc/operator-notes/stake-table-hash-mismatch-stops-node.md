### A stake table hash mismatch stops the node

Urgency: Low

- The node now stops (process abort) when the decided stake table hash does not match the table it derived from L1.
  Before, it logged a panic and kept running.
- The cause is the node's L1 RPC provider.
- Restart the node after fixing the L1 RPC provider.
