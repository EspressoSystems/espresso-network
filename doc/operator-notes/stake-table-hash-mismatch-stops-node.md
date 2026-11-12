### A stake table hash mismatch stops the node

Urgency: Low

- The node now stops (process abort) when the decided stake table hash does not match the table it derived from L1,
  where it used to log a panic and keep running. A table without a stored hash is not compared.
- The cause is the node's L1 RPC provider.
- Restart the node after fixing the L1 RPC provider.
