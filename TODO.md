# TODO

## x25519 public key format decision

Need team input on the serialization format for x25519 public keys in the staking CLI.

Options:

- **bs58** (current): matches keygen.rs output, used by `x25519::PublicKey` Display impl
- **tagged base64**: would match BLS/Schnorr key format, `X25519_PK` tag impl already exists in hotshot_types

Affected:

- `staking-cli/src/parse.rs`: `parse_x25519_key` parser
- `crates/espresso/node/src/bin/keygen.rs`: public key output format
- Any external tooling that reads/writes x25519 keys

If switching to tagged base64: update both keygen.rs output and CLI parser together.
