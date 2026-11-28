### Configure a persistent validator network key

Urgency: High

Before upgrading, validators must configure a persistent x25519 network key. Startup now fails if
`ESPRESSO_NODE_PRIVATE_X25519_KEY` is absent from both the environment and the configured key file, unless
`ESPRESSO_NODE_KEY_MNEMONIC` derives it. Validators already using a mnemonic or a key file containing all three keys
need no changes.

For older two-key setups, add the private x25519 key corresponding to the public key registered on-chain to the
existing configuration. If a new network identity is needed, generate it with
`keygen --scheme x25519 --out network-keys`, persist the private key from `network-keys/0.env` alongside the existing
staking and state keys, and register its public key and dialable address with `staking-cli update-network-config --x25519-key <PUBLIC_KEY> --p2p-addr <HOST:PORT>`.
Allow the stake-table update to become active before restarting with the new identity. Preserve the existing
staking and state keys.

Light-client followers do not require a network key. `pub-key --scheme bls` and `pub-key --scheme schnorr` continue
to work with older two-key configurations.
