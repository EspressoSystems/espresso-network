# Stake Table: X25519 Key and P2P Address Registration

## 1. Overview and Motivation

Validators need to discover each other's network addresses and encryption keys for direct communication required by fast
finality. Currently `RegisteredValidator` in Rust has `x25519_key: Option<x25519::PublicKey>` and
`p2p_addr: Option<NetAddr>` but both are always `None` because no contract events populate them.

This change adds x25519 key and p2p address to the stake table contract so validators can register and update their
network configuration on-chain. The stake table becomes the single source of truth for peer discovery.

No UI changes. Only stake table contract, Rust event processing, and staking CLI.

## 2. Contract Changes (StakeTableV3)

### Versioning

- `StakeTableV3 is StakeTableV2`
- `reinitializer(3)`
- `getVersion()` returns `(3, 0, 0)`
- If the redelegation feature (which also targets V3) lands first, this becomes V4 or the two features are combined into
  V3.

### New storage

```solidity
mapping(bytes32 x25519Key => bool used) public x25519Keys;
```

Unlike `blsKeys` and `schnorrKeys` which store hashes of multi-field keys, x25519 keys are already 32 bytes so no
hashing is needed. Same uniqueness pattern: a single bit per key. No per-validator key storage, no cleanup on rotation.
Old keys stay marked as used permanently. This is acceptable because key operations are rare for the ~100 active
validators.

No storage for p2p address. It is event-sourced only, same pattern as `metadataUri`.

### `initializeV3()`

Minimal. No data migration needed. Sets reinitializer(3).

### New functions

Three functions, one per intent. Each function name makes the caller's intent explicit so the contract can validate
accordingly.

#### 2.1 `registerValidatorV3`

```solidity
function registerValidatorV3(
    BN254.G2Point memory blsVK,
    EdOnBN254.EdOnBN254Point memory schnorrVK,
    BN254.G1Point memory blsSig,
    bytes memory schnorrSig,
    uint16 commission,
    string memory metadataUri,
    bytes32 x25519Key,
    string memory p2pAddr
) external virtual whenNotPaused
```

For new registrations after the upgrade. Both x25519 key and p2p address are required.

**Checks (in addition to all existing `registerValidatorV2` checks):**

- `x25519Key != bytes32(0)` (non-zero)
- `!x25519Keys[x25519Key]` (unique, never used before)
- `bytes(p2pAddr).length > 0` (non-empty)
- `bytes(p2pAddr).length <= MAX_P2P_ADDR_LENGTH` (max 512 bytes)

**State changes (in addition to existing V2 state changes):**

- `x25519Keys[x25519Key] = true`

**Emits:** `ValidatorRegisteredV3`

Deprecates `registerValidatorV2`. V3 overrides `registerValidatorV2` to revert with `DeprecatedFunction()`, same pattern
as V2 deprecating V1's `registerValidator`.

#### 2.2 `setNetworkConfig`

```solidity
function setNetworkConfig(
    bytes32 x25519Key,
    string memory p2pAddr
) external virtual whenNotPaused
```

Sets or updates the x25519 key and p2p address for a validator.

Primary intent: initial configuration for validators registered before the V3 upgrade.

Also usable for x25519 key rotation independent of consensus keys. A validator who only wants to change their p2p
address (without rotating x25519) should use `updateP2pAddr` instead, since `setNetworkConfig` requires a new, unused
x25519 key.

**Checks:**

- `ensureValidatorActive(msg.sender)`
- `x25519Key != bytes32(0)`
- `!x25519Keys[x25519Key]` (must be a key never used before)
- `bytes(p2pAddr).length > 0`
- `bytes(p2pAddr).length <= MAX_P2P_ADDR_LENGTH`

**State changes:**

- `x25519Keys[x25519Key] = true`

**Emits:** `NetworkConfigUpdated(msg.sender, x25519Key, p2pAddr)`

**Why not one-time only?** Without per-validator storage of the current x25519 key, we cannot enforce one-time. More
importantly, making it repeatable avoids locking out a validator who makes an error. x25519 key uniqueness still
prevents accidental reuse.

#### 2.3 `updateP2pAddr`

```solidity
function updateP2pAddr(
    string memory p2pAddr
) external virtual whenNotPaused
```

Updates only the p2p address. Intent: operational change (server migration, IP change) without touching cryptographic
keys.

**Checks:**

- `ensureValidatorActive(msg.sender)`
- `bytes(p2pAddr).length > 0`
- `bytes(p2pAddr).length <= MAX_P2P_ADDR_LENGTH`

**State changes:** None.

**Emits:** `NetworkConfigUpdated(msg.sender, bytes32(0), p2pAddr)`

The `bytes32(0)` x25519 key in the event signals "unchanged". This is unambiguous because `setNetworkConfig` and
`registerValidatorV3` both enforce `x25519Key != bytes32(0)`. Zero is never a valid key value.

### Why three functions instead of one

A single `updateNetworkConfig(x25519Key, p2pAddr)` with sentinel values (zero = keep current) creates ambiguity: the
contract cannot distinguish "I want to update both" from "I only want to update one". With separate functions, intent is
explicit:

| Intent                           | Function              | Contract validates                      |
| -------------------------------- | --------------------- | --------------------------------------- |
| New registration (post-upgrade)  | `registerValidatorV3` | Both required, full registration checks |
| Set/rotate x25519 key + p2p addr | `setNetworkConfig`    | Both required, x25519 uniqueness        |
| Change p2p addr only             | `updateP2pAddr`       | p2p non-empty only                      |

This also lets us add different access control or rate limiting per intent in the future without refactoring.

### Why only 2 new event types

Minimizing event types reduces complexity in the Rust event filter and handler code. All consensus nodes must watch and
process every event type.

| Event                   | Emitted by                          | New?                                |
| ----------------------- | ----------------------------------- | ----------------------------------- |
| `ValidatorRegisteredV3` | `registerValidatorV3`               | Yes (replaces V2 in same code path) |
| `NetworkConfigUpdated`  | `setNetworkConfig`, `updateP2pAddr` | Yes                                 |

`NetworkConfigUpdated` is shared between `setNetworkConfig` and `updateP2pAddr`. The x25519 key field being `bytes32(0)`
tells the Rust handler to skip updating the key. This avoids a third event type for a relatively minor distinction.

### New events

```solidity
event ValidatorRegisteredV3(
    address indexed account,
    BN254.G2Point blsVK,
    EdOnBN254.EdOnBN254Point schnorrVK,
    uint16 commission,
    BN254.G1Point blsSig,
    bytes schnorrSig,
    string metadataUri,
    bytes32 x25519Key,
    string p2pAddr
);

event NetworkConfigUpdated(
    address indexed validator,
    bytes32 x25519Key,
    string p2pAddr
);
```

### New errors

```solidity
error InvalidX25519Key();       // x25519 key is bytes32(0)
error X25519KeyAlreadyUsed();   // x25519 key previously registered
error InvalidP2pAddr();         // p2p addr empty or exceeds max length
```

### Constants

```solidity
uint256 public constant MAX_P2P_ADDR_LENGTH = 512;
```

### Validation details

**x25519 key:**

- 32 bytes (`bytes32` in Solidity). Matches the x25519 public key size.
- Non-zero check: prevents accidentally sending an uninitialized value.
- Uniqueness: same pattern as BLS and Schnorr keys. Prevents two validators from claiming the same encryption key which
  would cause communication issues.
- No signature verification: x25519 is a Diffie-Hellman key, not a signing key. There is no standard signature scheme
  for x25519. The validator signs the transaction with their Ethereum key which proves they control the account.

**p2p address:**

- Variable-length string. Format is `host:port` where host is an IP address or hostname. This matches the Rust `NetAddr`
  type which parses `rsplit_once(':')` then tries IP, falling back to hostname.
- Minimal structural validation in the contract via an internal `validateP2pAddr` helper (same pattern as
  `validateMetadataUri`). The goal is to catch common accidental errors at registration time rather than letting them
  propagate to network timeouts later. Full format validation (valid IP octets, valid DNS labels) is not practical in
  Solidity and is left to the Rust side.
- No uniqueness enforcement. Unlike cryptographic keys, network addresses can legitimately be reused (e.g. validator
  migrates away from an address, another validator later uses it). The address could also be shared behind a load
  balancer or relay.
- Max length 512 bytes. Generous upper bound for `hostname:port` (max DNS name is 253 chars + colon
  - 5 digit port).

`validateP2pAddr` is `public pure` so clients can call it to check addresses before submitting transactions, and so it
can be directly unit-tested.

Checks:

1. Length > 0 and <= `MAX_P2P_ADDR_LENGTH`
2. Contains at least one `:` (find the last occurrence, matching Rust's `rsplit_once(':')`)
3. Host part (before last `:`) is non-empty
4. Port part (after last `:`) is non-empty, digits only, parses to uint16 in range 1-65535

```solidity
function validateP2pAddr(string memory p2pAddr) public pure {
    bytes memory b = bytes(p2pAddr);
    if (b.length == 0 || b.length > MAX_P2P_ADDR_LENGTH) {
        revert InvalidP2pAddr();
    }

    // Find last ':' (same as Rust's rsplit_once)
    uint256 colonIdx = type(uint256).max;
    for (uint256 i = b.length; i > 0; i--) {
        if (b[i - 1] == ":") {
            colonIdx = i - 1;
            break;
        }
    }

    // Must have a colon with non-empty host before it
    if (colonIdx == type(uint256).max || colonIdx == 0) {
        revert InvalidP2pAddr();
    }

    // Parse port: digits only, 1-65535
    uint256 port = 0;
    uint256 portLen = b.length - colonIdx - 1;
    if (portLen == 0 || portLen > 5) {
        revert InvalidP2pAddr();
    }
    for (uint256 i = colonIdx + 1; i < b.length; i++) {
        uint8 c = uint8(b[i]);
        if (c < 0x30 || c > 0x39) { // not a digit
            revert InvalidP2pAddr();
        }
        port = port * 10 + (c - 0x30);
    }
    if (port == 0 || port > 65535) {
        revert InvalidP2pAddr();
    }
}
```

This catches: missing port, port zero, port out of range, non-numeric port, missing host, empty string, passing a
multiaddr (`/ip4/...` has no `:`-delimited port). It does not validate that the host is a valid IP or DNS name.

## 3. Rust Code Changes

### No new consensus version

Same approach as other V2 contract events. Strict deployment sequence (Section 6) ensures all nodes support the new
events before the contract emits them. No protocol version bump required.

### Files to change

**`crates/espresso/types/src/v0/v0_3/stake_table.rs`**

Add two variants to `StakeTableEvent`:

```rust
pub enum StakeTableEvent {
    // ... existing variants ...
    RegisterV3(ValidatorRegisteredV3),
    NetworkConfigUpdate(NetworkConfigUpdated),
}
```

**`crates/espresso/types/src/v0/impls/stake_table.rs`**

- Regenerate Rust bindings with V3 ABI (`just gen-bindings`). This produces `StakeTableV3Events` which is a superset of
  V2 events. Replace `StakeTableV2Events` with `StakeTableV3Events` everywhere.

- Update `TryFrom<StakeTableV3Events>` impl:
  - Map `ValidatorRegisteredV3` to `StakeTableEvent::RegisterV3`
  - Map `NetworkConfigUpdated` to `StakeTableEvent::NetworkConfigUpdate`

- `apply_event` handler for `RegisterV3`: same as `RegisterV2` handler but sets `x25519_key` and `p2p_addr` on the
  `RegisteredValidator`. Parse x25519 key from `bytes32`, parse p2p addr from string via `NetAddr::from_str`.

- `apply_event` handler for `NetworkConfigUpdate`:
  - Look up validator, error if not found.
  - If `x25519Key != bytes32(0)`: validate uniqueness against `used_x25519_keys`, parse and set `x25519_key` on
    validator, add to used set.
  - Parse and set `p2p_addr` from the string field.

- Add `used_x25519_keys: HashSet<x25519::PublicKey>` to `StakeTableState`. This mirrors the contract's uniqueness check.
  The contract enforces uniqueness on L1 but the Rust side replays events from L1 and must independently validate them
  (same as BLS and Schnorr key tracking).

- Event filter: add `ValidatorRegisteredV3::SIGNATURE` and `NetworkConfigUpdated::SIGNATURE` to the topic filter list.

**`contracts/rust/adapter/src/stake_table.rs`**

- Add authentication for `ValidatorRegisteredV3`: same BLS + Schnorr verification as V2, x25519 and p2p addr are not
  authenticated via signatures (x25519 has no signature scheme, p2p addr is operational data).

### Commitment considerations

The `Committable` impl for `RegisteredValidator` in `crates/espresso/types/src/v0/v0_3/stake_table.rs` currently has
x25519 and p2p addr commented out. Uncommenting these is a commitment-breaking change.

Options:

1. Gate behind a protocol version: only include in commitment for blocks after the version that activates this feature.
2. Keep them out of the commitment: x25519 and p2p addr are network-layer data, not consensus state. Validators don't
   need to agree on these values for consensus to work.

Recommend option 2 for now. These values are for peer discovery, not for consensus validation. If needed later, option 1
can be added.

### Epoch activation delay

Stake table changes take 2-3 epochs to become active in consensus. This applies to x25519 and p2p addr updates as well
since they are fields on `RegisteredValidator` which is part of the epoch stake table.

This is acceptable. Validators will register their network config well before fast finality is activated. For ongoing
updates (e.g. IP change), the epoch delay means there is a window where the old address is still in use. This is the
same situation as key rotation today.

## 4. Staking CLI Changes

**File:** `staking-cli/src/lib.rs`

Add two new commands:

```
set-network-config --x25519-key <KEY> --p2p-addr <ADDR>
update-p2p-addr --p2p-addr <ADDR>
```

- `set-network-config`: calls `stake_table.setNetworkConfig(x25519Key, p2pAddr)`
- `update-p2p-addr`: calls `stake_table.updateP2pAddr(p2pAddr)`
- Update `registerValidatorV3` command (or extend existing register command) to include `--x25519-key` and `--p2p-addr`
  flags.
- Add event display formatting for `ValidatorRegisteredV3` and `NetworkConfigUpdated`.

## 5. Deployment and Upgrade

### Rust deployer changes

**Crate:** `contracts/rust/deployer/`

Add V3 upgrade support following the V2 pattern:

**`src/lib.rs`:**

- `prepare_stake_table_v3_upgrade()`: verify proxy is at V2, encode `initializeV3()` calldata. No data migration needed.
- `upgrade_stake_table_v3()`: EOA path: deploy `StakeTableV3` impl, call `proxy.upgradeToAndCall(impl, initData)`,
  verify post-deploy (version == 3, new functions callable, existing V2 state preserved).

**`src/proposals/multisig.rs`:**

- `upgrade_stake_table_v3_multisig_owner()`: multisig path: deploy impl, encode upgrade calldata, output Safe TX Builder
  JSON.

**`src/builder.rs`:**

- Add `--upgrade-stake-table-v3` flag to `DeployerArgs`.
- Route through timelock or multisig based on existing flags.

### Upgrade path per environment

| Environment | Admin       | Upgrade mechanism                                                           | Delay  |
| ----------- | ----------- | --------------------------------------------------------------------------- | ------ |
| Local/CI    | EOA         | Direct `upgradeToAndCall`                                                   | None   |
| Decaf       | OpsTimelock | `timelock.schedule()` then wait then `timelock.execute()`                   | 5 min  |
| Mainnet     | OpsTimelock | `timelock.schedule()` via Safe multisig then wait then `execute()` via Safe | 2 days |

### Storage layout compatibility

`StakeTableV3` adds one new mapping (`x25519Keys`). Run `StorageUpgradeCompatibility.t.sol` with `maxMajorVersion = 3`
against decaf and mainnet to verify no storage conflicts.

### Deployment-info

After deploying V3, the deployment-info tool will automatically detect the version change via `getVersion()` and update
`deployments/decaf.toml` and `deployments/mainnet.toml` (StakeTable version `2.0.0` to `3.0.0`).

### Rollout sequence

1. Release new sequencer binary with `RegisterV3` and `NetworkConfigUpdate` event support (dormant, no contract emits
   them yet).
2. All validators upgrade to new binary.
3. Verify all (or almost all) validators are running the new binary.
4. Deploy `StakeTableV3` implementation contract.
5. Schedule upgrade via OpsTimelock.
6. Execute upgrade after delay expires.
7. Validators call `setNetworkConfig` to register their x25519 key and p2p address.
8. Wait for epoch activation (2-3 epochs) for values to take effect.
9. Activate fast finality feature (separate step, not part of this change).

Steps 7-8 can happen well before step 9. The gap between contract upgrade and fast finality activation gives validators
time to register and for values to propagate through epoch transitions.

## 6. Forward Compatibility

### Interaction with re-delegation (V3 on other branch)

If re-delegation lands first as V3, this feature becomes V4 (or later). The changes are orthogonal: re-delegation adds a
`redelegate` function and `Redelegated` event. This feature adds network config functions and events. No conflicts in
storage, events, or logic.

If both land simultaneously, they can share V3 with combined `initializeV3()`.

### Interaction with slashing

Network config is independent of slashing. x25519 keys and p2p addresses are not involved in slashing logic. No forward
compatibility concerns.

## 7. Test Plan

### Requirements

| ID                         | Description                                                      |
| -------------------------- | ---------------------------------------------------------------- |
| REQ:register-v3            | New registration includes x25519 key and p2p addr                |
| REQ:set-network-config     | Active validator can set x25519 key and p2p addr                 |
| REQ:update-p2p-addr        | Active validator can update p2p addr independently               |
| REQ:x25519-uniqueness      | x25519 keys cannot be reused across validators                   |
| REQ:x25519-nonzero         | x25519 key cannot be bytes32(0)                                  |
| REQ:p2p-nonempty           | p2p addr cannot be empty string                                  |
| REQ:p2p-maxlength          | p2p addr cannot exceed MAX_P2P_ADDR_LENGTH                       |
| REQ:event-register-v3      | `ValidatorRegisteredV3` emitted with correct fields              |
| REQ:event-network-config   | `NetworkConfigUpdated` emitted with correct fields               |
| REQ:rust-register-v3       | Sequencer processes `ValidatorRegisteredV3` and sets x25519/p2p  |
| REQ:rust-network-config    | Sequencer processes `NetworkConfigUpdated` and updates validator |
| REQ:upgrade-v2-to-v3       | V2 to V3 upgrade preserves all existing state                    |
| REQ:upgrade-storage-compat | V3 storage layout compatible with deployed V2                    |
| REQ:cli-set-network-config | staking-cli can call `setNetworkConfig`                          |
| REQ:cli-update-p2p-addr    | staking-cli can call `updateP2pAddr`                             |

### Requirement Tests

| Test                           | Requirement                |
| ------------------------------ | -------------------------- |
| TEST:register-v3-ok            | REQ:register-v3            |
| TEST:set-network-config-ok     | REQ:set-network-config     |
| TEST:update-p2p-addr-ok        | REQ:update-p2p-addr        |
| TEST:x25519-uniqueness-ok      | REQ:x25519-uniqueness      |
| TEST:x25519-nonzero-ok         | REQ:x25519-nonzero         |
| TEST:p2p-nonempty-ok           | REQ:p2p-nonempty           |
| TEST:p2p-maxlength-ok          | REQ:p2p-maxlength          |
| TEST:event-register-v3-ok      | REQ:event-register-v3      |
| TEST:event-network-config-ok   | REQ:event-network-config   |
| TEST:rust-register-v3-ok       | REQ:rust-register-v3       |
| TEST:rust-network-config-ok    | REQ:rust-network-config    |
| TEST:upgrade-v2-to-v3-ok       | REQ:upgrade-v2-to-v3       |
| TEST:upgrade-storage-compat-ok | REQ:upgrade-storage-compat |
| TEST:cli-set-network-config-ok | REQ:cli-set-network-config |
| TEST:cli-update-p2p-addr-ok    | REQ:cli-update-p2p-addr    |

### Contract Edge Cases

| ID                                | Description                                                                                                                                                           |
| --------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| EDGE:register-v3-zero-x25519      | Register with `bytes32(0)` x25519 key. Must revert.                                                                                                                   |
| EDGE:register-v3-empty-p2p        | Register with empty p2p addr. Must revert.                                                                                                                            |
| EDGE:register-v3-long-p2p         | Register with p2p addr exceeding max length. Must revert.                                                                                                             |
| EDGE:register-v3-duplicate-x25519 | Register with x25519 key already used by another validator. Must revert.                                                                                              |
| EDGE:register-v2-deprecated       | `registerValidatorV2` reverts `DeprecatedFunction` after V3 upgrade.                                                                                                  |
| EDGE:set-config-inactive          | Call `setNetworkConfig` on inactive (unregistered) validator. Must revert.                                                                                            |
| EDGE:set-config-exited            | Call `setNetworkConfig` on exited validator. Must revert.                                                                                                             |
| EDGE:set-config-zero-x25519       | Call `setNetworkConfig` with `bytes32(0)`. Must revert.                                                                                                               |
| EDGE:set-config-empty-p2p         | Call `setNetworkConfig` with empty p2p addr. Must revert.                                                                                                             |
| EDGE:set-config-duplicate-x25519  | Call `setNetworkConfig` with used x25519 key. Must revert.                                                                                                            |
| EDGE:set-config-repeated          | Call `setNetworkConfig` twice with different keys. Both succeed. Second key marked as used.                                                                           |
| EDGE:set-config-paused            | Call `setNetworkConfig` while contract paused. Must revert.                                                                                                           |
| EDGE:update-p2p-inactive          | Call `updateP2pAddr` on inactive validator. Must revert.                                                                                                              |
| EDGE:update-p2p-exited            | Call `updateP2pAddr` on exited validator. Must revert.                                                                                                                |
| EDGE:update-p2p-empty             | Call `updateP2pAddr` with empty string. Must revert.                                                                                                                  |
| EDGE:update-p2p-long              | Call `updateP2pAddr` with string exceeding max length. Must revert.                                                                                                   |
| EDGE:update-p2p-paused            | Call `updateP2pAddr` while contract paused. Must revert.                                                                                                              |
| EDGE:update-p2p-repeated          | Call `updateP2pAddr` twice with different addresses. Both succeed.                                                                                                    |
| EDGE:register-v3-boundary-p2p     | Register with p2p addr exactly `MAX_P2P_ADDR_LENGTH` bytes. Must succeed.                                                                                             |
| EDGE:set-config-own-x25519        | Register via V3 with key K, then call `setNetworkConfig` with same key K. Must revert `X25519KeyAlreadyUsed`. Validator must use `updateP2pAddr` to change only addr. |
| EDGE:set-config-unregistered      | Call `setNetworkConfig` from address that never registered. Must revert `ValidatorInactive`.                                                                          |
| EDGE:p2p-no-colon                 | p2p addr with no `:` (e.g. `localhost`). Must revert.                                                                                                                 |
| EDGE:p2p-no-host                  | p2p addr with empty host (e.g. `:8080`). Must revert.                                                                                                                 |
| EDGE:p2p-no-port                  | p2p addr with empty port (e.g. `host:`). Must revert.                                                                                                                 |
| EDGE:p2p-port-zero                | p2p addr with port 0 (e.g. `host:0`). Must revert.                                                                                                                    |
| EDGE:p2p-port-overflow            | p2p addr with port > 65535 (e.g. `host:70000`). Must revert.                                                                                                          |
| EDGE:p2p-port-non-numeric         | p2p addr with non-numeric port (e.g. `host:abc`). Must revert.                                                                                                        |
| EDGE:p2p-port-leading-zero        | p2p addr with leading zero port (e.g. `host:08080`). Accepted (parses to 8080).                                                                                       |
| EDGE:p2p-valid-ipv4               | p2p addr with IPv4 host (e.g. `192.168.1.1:8080`). Must succeed.                                                                                                      |
| EDGE:p2p-valid-ipv6               | p2p addr with IPv6 host (e.g. `::1:8080`). Must succeed. Last `:` separates port.                                                                                     |
| EDGE:p2p-valid-hostname           | p2p addr with hostname (e.g. `node.example.com:8080`). Must succeed.                                                                                                  |
| EDGE:p2p-multiaddr                | p2p addr in multiaddr format (e.g. `/ip4/1.2.3.4/tcp/4001`). Must revert (no valid `:port` suffix).                                                                   |

### Contract Edge Case Tests

| Test                                    | Edge Case                         |
| --------------------------------------- | --------------------------------- |
| TEST:register-v3-zero-x25519-fails      | EDGE:register-v3-zero-x25519      |
| TEST:register-v3-empty-p2p-fails        | EDGE:register-v3-empty-p2p        |
| TEST:register-v3-long-p2p-fails         | EDGE:register-v3-long-p2p         |
| TEST:register-v3-duplicate-x25519-fails | EDGE:register-v3-duplicate-x25519 |
| TEST:register-v2-deprecated-fails       | EDGE:register-v2-deprecated       |
| TEST:set-config-inactive-fails          | EDGE:set-config-inactive          |
| TEST:set-config-exited-fails            | EDGE:set-config-exited            |
| TEST:set-config-zero-x25519-fails       | EDGE:set-config-zero-x25519       |
| TEST:set-config-empty-p2p-fails         | EDGE:set-config-empty-p2p         |
| TEST:set-config-duplicate-x25519-fails  | EDGE:set-config-duplicate-x25519  |
| TEST:set-config-repeated-ok             | EDGE:set-config-repeated          |
| TEST:set-config-paused-fails            | EDGE:set-config-paused            |
| TEST:update-p2p-inactive-fails          | EDGE:update-p2p-inactive          |
| TEST:update-p2p-exited-fails            | EDGE:update-p2p-exited            |
| TEST:update-p2p-empty-fails             | EDGE:update-p2p-empty             |
| TEST:update-p2p-long-fails              | EDGE:update-p2p-long              |
| TEST:update-p2p-paused-fails            | EDGE:update-p2p-paused            |
| TEST:update-p2p-repeated-ok             | EDGE:update-p2p-repeated          |
| TEST:register-v3-boundary-p2p-ok        | EDGE:register-v3-boundary-p2p     |
| TEST:set-config-own-x25519-fails        | EDGE:set-config-own-x25519        |
| TEST:set-config-unregistered-fails      | EDGE:set-config-unregistered      |
| TEST:p2p-no-colon-fails                 | EDGE:p2p-no-colon                 |
| TEST:p2p-no-host-fails                  | EDGE:p2p-no-host                  |
| TEST:p2p-no-port-fails                  | EDGE:p2p-no-port                  |
| TEST:p2p-port-zero-fails                | EDGE:p2p-port-zero                |
| TEST:p2p-port-overflow-fails            | EDGE:p2p-port-overflow            |
| TEST:p2p-port-non-numeric-fails         | EDGE:p2p-port-non-numeric         |
| TEST:p2p-port-leading-zero-ok           | EDGE:p2p-port-leading-zero        |
| TEST:p2p-valid-ipv4-ok                  | EDGE:p2p-valid-ipv4               |
| TEST:p2p-valid-ipv6-ok                  | EDGE:p2p-valid-ipv6               |
| TEST:p2p-valid-hostname-ok              | EDGE:p2p-valid-hostname           |
| TEST:p2p-multiaddr-fails                | EDGE:p2p-multiaddr                |

### Rust Edge Cases

| ID                                 | Description                                                                                          |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------- |
| EDGE:rust-register-v3-invalid-sig  | `ValidatorRegisteredV3` with invalid BLS/Schnorr signature. Validator registered as unauthenticated. |
| EDGE:rust-register-v3-bad-p2p      | `ValidatorRegisteredV3` with unparsable p2p addr. Log warning, set `p2p_addr = None`.                |
| EDGE:rust-config-unknown-validator | `NetworkConfigUpdated` for unknown validator. Error.                                                 |
| EDGE:rust-config-zero-x25519-skip  | `NetworkConfigUpdated` with `bytes32(0)` x25519. Skip key update, only update p2p.                   |
| EDGE:rust-config-duplicate-x25519  | `NetworkConfigUpdated` with already-used x25519 key. Error.                                          |
| EDGE:rust-config-bad-p2p           | `NetworkConfigUpdated` with unparsable p2p addr. Log warning, set `p2p_addr = None`.                 |

### Rust Edge Case Tests

| Test                                     | Edge Case                          |
| ---------------------------------------- | ---------------------------------- |
| TEST:rust-register-v3-invalid-sig-ok     | EDGE:rust-register-v3-invalid-sig  |
| TEST:rust-register-v3-bad-p2p-ok         | EDGE:rust-register-v3-bad-p2p      |
| TEST:rust-config-unknown-validator-fails | EDGE:rust-config-unknown-validator |
| TEST:rust-config-zero-x25519-skip-ok     | EDGE:rust-config-zero-x25519-skip  |
| TEST:rust-config-duplicate-x25519-fails  | EDGE:rust-config-duplicate-x25519  |
| TEST:rust-config-bad-p2p-ok              | EDGE:rust-config-bad-p2p           |

### Upgrade Edge Cases

| ID                                          | Description                                                                                         |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| EDGE:upgrade-reinitialize-v3                | Calling `initializeV3()` twice reverts `InvalidInitialization`.                                     |
| EDGE:upgrade-unauthorized                   | Non-admin caller cannot trigger upgrade.                                                            |
| EDGE:upgrade-v2-ops-after-v3                | V2 operations (delegate, undelegate, claimWithdrawal, updateConsensusKeysV2) work after V3 upgrade. |
| EDGE:upgrade-pending-undelegation-preserved | Pending undelegation from V2 claimable after V3 upgrade.                                            |
| EDGE:upgrade-exited-validator-preserved     | Exited validator delegations claimable after V3 upgrade.                                            |

### Upgrade Edge Case Tests

| Test                                           | Edge Case                                   |
| ---------------------------------------------- | ------------------------------------------- |
| TEST:upgrade-reinitialize-v3-fails             | EDGE:upgrade-reinitialize-v3                |
| TEST:upgrade-unauthorized-fails                | EDGE:upgrade-unauthorized                   |
| TEST:upgrade-v2-ops-after-v3-ok                | EDGE:upgrade-v2-ops-after-v3                |
| TEST:upgrade-pending-undelegation-preserved-ok | EDGE:upgrade-pending-undelegation-preserved |
| TEST:upgrade-exited-validator-preserved-ok     | EDGE:upgrade-exited-validator-preserved     |

### Invariant Tests

Add `setNetworkConfig` and `updateP2pAddr` as fuzzing targets to `StakeTableV2PropTestBase`:

- `setNetworkConfigOk(actorIndex, x25519Key, p2pAddr)`: pick active validator, bound x25519 to unused key, valid p2p
  addr.
- `updateP2pAddrOk(actorIndex, p2pAddr)`: pick active validator, valid p2p addr.
- `setNetworkConfigAny(actorIndex, x25519Key, p2pAddr)`: raw fuzz input, expect reverts.
- `updateP2pAddrAny(actorIndex, p2pAddr)`: raw fuzz input, expect reverts.

Existing invariants cover the new functions because network config changes do not affect delegation balances,
`activeStake`, or `totalPendingWithdrawal`.

### Integration Tests

| Test                             | Description                                                                                                                          |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| TEST:e2e-register-v3-pipeline    | V3 registration event emitted on L1, fetched by sequencer, applied to stake table with x25519 key and p2p addr set on the validator. |
| TEST:e2e-network-config-pipeline | NetworkConfigUpdated event emitted on L1, fetched by sequencer, validator's x25519 key and p2p addr updated in stake table state.    |
| TEST:e2e-epoch-activation        | Validator sets network config, values appear in active validator set after epoch transition (2-3 epochs).                            |
