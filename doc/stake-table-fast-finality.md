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

#### 2.2 `updateNetworkConfig`

```solidity
function updateNetworkConfig(
    bytes32 x25519Key,
    string memory p2pAddr
) external virtual whenNotPaused
```

Sets or updates the x25519 key and p2p address for a validator.

Primary intent: initial configuration for validators registered before the V3 upgrade.

Also usable for x25519 key rotation independent of consensus keys. A validator who only wants to change their p2p
address (without rotating x25519) should use `updateP2pAddr` instead, since `updateNetworkConfig` requires a new, unused
x25519 key.

**Checks:**

- `ensureValidatorActive(msg.sender)`
- `x25519Key != bytes32(0)`
- `!x25519Keys[x25519Key]` (must be a key never used before)
- `bytes(p2pAddr).length > 0`
- `bytes(p2pAddr).length <= MAX_P2P_ADDR_LENGTH`

**State changes:**

- `x25519Keys[x25519Key] = true`

**Emits:** `X25519KeyUpdated(msg.sender, x25519Key)` and `P2pAddrUpdated(msg.sender, p2pAddr)`

**Why not one-time only?** Without per-validator storage of the current x25519 key, we cannot enforce one-time. More
importantly, making it repeatable avoids locking out a validator who makes an error. x25519 key uniqueness still
prevents accidental reuse.

#### 2.3 `updateX25519Key`

```solidity
function updateX25519Key(
    bytes32 x25519Key
) external virtual whenNotPaused
```

Updates only the x25519 key. Intent: key rotation without changing network address.

**Checks:**

- `ensureValidatorActive(msg.sender)`
- `x25519Key != bytes32(0)`
- `!x25519Keys[x25519Key]` (must be a key never used before)

**State changes:**

- `x25519Keys[x25519Key] = true`

**Emits:** `X25519KeyUpdated(msg.sender, x25519Key)`

#### 2.4 `updateP2pAddr`

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

**Emits:** `P2pAddrUpdated(msg.sender, p2pAddr)`

### Why four functions instead of one

A single `updateNetworkConfig(x25519Key, p2pAddr)` with sentinel values (zero = keep current) creates ambiguity: the
contract cannot distinguish "I want to update both" from "I only want to update one". With separate functions, intent is
explicit:

| Intent                           | Function              | Contract validates                      |
| -------------------------------- | --------------------- | --------------------------------------- |
| New registration (post-upgrade)  | `registerValidatorV3` | Both required, full registration checks |
| Set/rotate x25519 key + p2p addr | `updateNetworkConfig` | Both required, x25519 uniqueness        |
| Rotate x25519 key only           | `updateX25519Key`     | x25519 uniqueness                       |
| Change p2p addr only             | `updateP2pAddr`       | p2p non-empty only                      |

This also lets us add different access control or rate limiting per intent in the future without refactoring.

### Why 3 new event types

One event per field, no sentinel values. Each event carries exactly one piece of data, making the Rust handler
straightforward: no need to check for `bytes32(0)` sentinels or conditional logic per field.

| Event                   | Emitted by                               | New?                                |
| ----------------------- | ---------------------------------------- | ----------------------------------- |
| `ValidatorRegisteredV3` | `registerValidatorV3`                    | Yes (replaces V2 in same code path) |
| `X25519KeyUpdated`      | `updateNetworkConfig`, `updateX25519Key` | Yes                                 |
| `P2pAddrUpdated`        | `updateNetworkConfig`, `updateP2pAddr`   | Yes                                 |

`updateNetworkConfig` emits both `X25519KeyUpdated` and `P2pAddrUpdated`. The individual setter functions emit only
their respective event. This removes sentinel logic from both the contract and Rust sides.

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

event X25519KeyUpdated(
    address indexed validator,
    bytes32 x25519Key
);

event P2pAddrUpdated(
    address indexed validator,
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

Add three variants to `StakeTableEvent`:

```rust
pub enum StakeTableEvent {
    // ... existing variants ...
    RegisterV3(ValidatorRegisteredV3),
    X25519KeyUpdate(X25519KeyUpdated),
    P2pAddrUpdate(P2pAddrUpdated),
}
```

**`crates/espresso/types/src/v0/impls/stake_table.rs`**

- Regenerate Rust bindings with V3 ABI (`just gen-bindings`). This produces `StakeTableV3Events` which is a superset of
  V2 events. Replace `StakeTableV2Events` with `StakeTableV3Events` everywhere.

- Update `TryFrom<StakeTableV3Events>` impl:
  - Map `ValidatorRegisteredV3` to `StakeTableEvent::RegisterV3`
  - Map `X25519KeyUpdated` to `StakeTableEvent::X25519KeyUpdate`
  - Map `P2pAddrUpdated` to `StakeTableEvent::P2pAddrUpdate`

- `apply_event` handler for `RegisterV3`: same as `RegisterV2` handler but sets `x25519_key` and `p2p_addr` on the
  `RegisteredValidator`. Parse x25519 key from `bytes32`, parse p2p addr from string via `NetAddr::from_str`.

- `apply_event` handler for `X25519KeyUpdate`:
  - Look up validator, error if not found.
  - Validate uniqueness against `used_x25519_keys`, parse and set `x25519_key` on validator, add to used set.

- `apply_event` handler for `P2pAddrUpdate`:
  - Look up validator, error if not found.
  - Parse and set `p2p_addr` from the string field.

- Add `used_x25519_keys: HashSet<x25519::PublicKey>` to `StakeTableState`. This mirrors the contract's uniqueness check.
  The contract enforces uniqueness on L1 but the Rust side replays events from L1 and must independently validate them
  (same as BLS and Schnorr key tracking).

- Event filter: add `ValidatorRegisteredV3::SIGNATURE`, `X25519KeyUpdated::SIGNATURE`, and `P2pAddrUpdated::SIGNATURE`
  to the topic filter list.

**`contracts/rust/adapter/src/stake_table.rs`**

- Add authentication for `ValidatorRegisteredV3`: same BLS + Schnorr verification as V2, x25519 and p2p addr are not
  authenticated via signatures (x25519 has no signature scheme, p2p addr is operational data).

### Commitment considerations

The `Committable` impl for `RegisteredValidator` conditionally includes x25519_key and p2p_addr only when they are
`Some`. This is backward compatible: these fields are `None` until the StakeTableV3 contract is deployed and a validator
explicitly sets them via `registerValidatorV3`, `updateNetworkConfig`, `updateX25519Key`, or `updateP2pAddr`. Pre-V3
validators keep the same commitment as before.

This approach was chosen over keeping them out of the commitment entirely because including them ensures all nodes agree
on the network config for each validator, which matters for fast finality peer discovery.

### Epoch activation delay

Stake table changes take 2-3 epochs to become active in consensus. This applies to x25519 and p2p addr updates as well
since they are fields on `RegisteredValidator` which is part of the epoch stake table.

This is acceptable. Validators will register their network config well before fast finality is activated. For ongoing
updates (e.g. IP change), the epoch delay means there is a window where the old address is still in use. This is the
same situation as key rotation today.

## 4. Staking CLI Changes

**File:** `staking-cli/src/lib.rs`

Add three new commands:

```
update-network-config --x25519-key <KEY> --p2p-addr <ADDR>
update-x25519-key --x25519-key <KEY>
update-p2p-addr --p2p-addr <ADDR>
```

- `update-network-config`: calls `stake_table.updateNetworkConfig(x25519Key, p2pAddr)`
- `update-x25519-key`: calls `stake_table.updateX25519Key(x25519Key)`
- `update-p2p-addr`: calls `stake_table.updateP2pAddr(p2pAddr)`
- Update `registerValidatorV3` command (or extend existing register command) to include `--x25519-key` and `--p2p-addr`
  flags.
- Add event display formatting for `ValidatorRegisteredV3`, `X25519KeyUpdated`, and `P2pAddrUpdated`.

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

1. Release new sequencer binary with `RegisterV3`, `X25519KeyUpdate`, and `P2pAddrUpdate` event support (dormant, no
   contract emits them yet).
2. All validators upgrade to new binary.
3. Verify all (or almost all) validators are running the new binary.
4. Deploy `StakeTableV3` implementation contract.
5. Schedule upgrade via OpsTimelock.
6. Execute upgrade after delay expires.
7. Validators call `updateNetworkConfig` to register their x25519 key and p2p address.
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

| ID                            | Description                                                     |
| ----------------------------- | --------------------------------------------------------------- |
| REQ:register-v3               | New registration includes x25519 key and p2p addr               |
| REQ:update-network-config     | Active validator can set x25519 key and p2p addr                |
| REQ:update-x25519-key         | Active validator can update x25519 key independently            |
| REQ:update-p2p-addr           | Active validator can update p2p addr independently              |
| REQ:x25519-uniqueness         | x25519 keys cannot be reused across validators                  |
| REQ:x25519-nonzero            | x25519 key cannot be bytes32(0)                                 |
| REQ:p2p-nonempty              | p2p addr cannot be empty string                                 |
| REQ:p2p-maxlength             | p2p addr cannot exceed MAX_P2P_ADDR_LENGTH                      |
| REQ:event-register-v3         | `ValidatorRegisteredV3` emitted with correct fields             |
| REQ:event-x25519-key          | `X25519KeyUpdated` emitted with correct fields                  |
| REQ:event-p2p-addr            | `P2pAddrUpdated` emitted with correct fields                    |
| REQ:rust-register-v3          | Sequencer processes `ValidatorRegisteredV3` and sets x25519/p2p |
| REQ:rust-x25519-key           | Sequencer processes `X25519KeyUpdated` and updates validator    |
| REQ:rust-p2p-addr             | Sequencer processes `P2pAddrUpdated` and updates validator      |
| REQ:upgrade-v2-to-v3          | V2 to V3 upgrade preserves all existing state                   |
| REQ:upgrade-storage-compat    | V3 storage layout compatible with deployed V2                   |
| REQ:cli-update-network-config | staking-cli can call `updateNetworkConfig`                      |
| REQ:cli-update-x25519-key     | staking-cli can call `updateX25519Key`                          |
| REQ:cli-update-p2p-addr       | staking-cli can call `updateP2pAddr`                            |

### Requirement Tests

| Test                              | Requirement                   | Implementation                                                                                 |
| --------------------------------- | ----------------------------- | ---------------------------------------------------------------------------------------------- |
| TEST:register-v3-ok               | REQ:register-v3               | [`test_RegisterValidatorV3_Success`](../contracts/test/StakeTableV3.t.sol)                     |
| TEST:update-network-config-ok     | REQ:update-network-config     | [`test_UpdateNetworkConfig_Success`](../contracts/test/StakeTableV3.t.sol)                     |
| TEST:update-x25519-key-ok         | REQ:update-x25519-key         | [`test_UpdateX25519Key_Success`](../contracts/test/StakeTableV3.t.sol)                         |
| TEST:update-p2p-addr-ok           | REQ:update-p2p-addr           | [`test_UpdateP2pAddr_Success`](../contracts/test/StakeTableV3.t.sol)                           |
| TEST:x25519-uniqueness-ok         | REQ:x25519-uniqueness         | [`test_RegisterValidatorV3_DuplicateX25519_Reverts`](../contracts/test/StakeTableV3.t.sol)     |
| TEST:x25519-nonzero-ok            | REQ:x25519-nonzero            | [`test_RegisterValidatorV3_ZeroX25519_Reverts`](../contracts/test/StakeTableV3.t.sol)          |
| TEST:p2p-nonempty-ok              | REQ:p2p-nonempty              | [`test_ValidateP2pAddr_Empty`](../contracts/test/StakeTableV3.t.sol)                           |
| TEST:p2p-maxlength-ok             | REQ:p2p-maxlength             | [`test_ValidateP2pAddr_TooLong`](../contracts/test/StakeTableV3.t.sol)                         |
| TEST:event-register-v3-ok         | REQ:event-register-v3         | [`test_RegisterValidatorV3_Success`](../contracts/test/StakeTableV3.t.sol)                     |
| TEST:event-x25519-key-ok          | REQ:event-x25519-key          | [`test_UpdateX25519Key_Success`](../contracts/test/StakeTableV3.t.sol)                         |
| TEST:event-p2p-addr-ok            | REQ:event-p2p-addr            | [`test_UpdateP2pAddr_Success`](../contracts/test/StakeTableV3.t.sol)                           |
| TEST:rust-register-v3-ok          | REQ:rust-register-v3          | [`test_register_v3_sets_x25519_and_p2p`](../crates/espresso/types/src/v0/impls/stake_table.rs) |
| TEST:rust-x25519-key-ok           | REQ:rust-x25519-key           | [`test_x25519_key_update_sets_value`](../crates/espresso/types/src/v0/impls/stake_table.rs)    |
| TEST:rust-p2p-addr-ok             | REQ:rust-p2p-addr             | [`test_p2p_addr_update_sets_value`](../crates/espresso/types/src/v0/impls/stake_table.rs)      |
| TEST:upgrade-v2-to-v3-ok          | REQ:upgrade-v2-to-v3          | [`test_UpgradeV2ToV3_PreservesState`](../contracts/test/StakeTableUpgradeToV3.t.sol)           |
| TEST:upgrade-storage-compat-ok    | REQ:upgrade-storage-compat    | not yet implemented                                                                            |
| TEST:cli-update-network-config-ok | REQ:cli-update-network-config | [`test_update_network_config`](../staking-cli/src/registration.rs)                             |
| TEST:cli-update-x25519-key-ok     | REQ:cli-update-x25519-key     | [`test_update_x25519_key`](../staking-cli/src/registration.rs)                                 |
| TEST:cli-update-p2p-addr-ok       | REQ:cli-update-p2p-addr       | [`test_update_p2p_addr`](../staking-cli/src/registration.rs)                                   |

### Contract Edge Cases

| ID                                | Description                                                                                                                                                              |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| EDGE:register-v3-zero-x25519      | Register with `bytes32(0)` x25519 key. Must revert.                                                                                                                      |
| EDGE:register-v3-empty-p2p        | Register with empty p2p addr. Must revert.                                                                                                                               |
| EDGE:register-v3-long-p2p         | Register with p2p addr exceeding max length. Must revert.                                                                                                                |
| EDGE:register-v3-duplicate-x25519 | Register with x25519 key already used by another validator. Must revert.                                                                                                 |
| EDGE:register-v2-deprecated       | `registerValidatorV2` reverts `DeprecatedFunction` after V3 upgrade.                                                                                                     |
| EDGE:set-config-inactive          | Call `updateNetworkConfig` on inactive (unregistered) validator. Must revert.                                                                                            |
| EDGE:set-config-exited            | Call `updateNetworkConfig` on exited validator. Must revert.                                                                                                             |
| EDGE:set-config-zero-x25519       | Call `updateNetworkConfig` with `bytes32(0)`. Must revert.                                                                                                               |
| EDGE:set-config-empty-p2p         | Call `updateNetworkConfig` with empty p2p addr. Must revert.                                                                                                             |
| EDGE:set-config-duplicate-x25519  | Call `updateNetworkConfig` with used x25519 key. Must revert.                                                                                                            |
| EDGE:set-config-repeated          | Call `updateNetworkConfig` twice with different keys. Both succeed. Second key marked as used.                                                                           |
| EDGE:set-config-paused            | Call `updateNetworkConfig` while contract paused. Must revert.                                                                                                           |
| EDGE:set-x25519-inactive          | Call `updateX25519Key` on inactive validator. Must revert.                                                                                                               |
| EDGE:set-x25519-exited            | Call `updateX25519Key` on exited validator. Must revert.                                                                                                                 |
| EDGE:set-x25519-zero              | Call `updateX25519Key` with `bytes32(0)`. Must revert.                                                                                                                   |
| EDGE:set-x25519-duplicate         | Call `updateX25519Key` with used x25519 key. Must revert.                                                                                                                |
| EDGE:set-x25519-repeated          | Call `updateX25519Key` twice with different keys. Both succeed. Both keys marked as used.                                                                                |
| EDGE:set-x25519-paused            | Call `updateX25519Key` while contract paused. Must revert.                                                                                                               |
| EDGE:set-p2p-inactive             | Call `updateP2pAddr` on inactive validator. Must revert.                                                                                                                 |
| EDGE:set-p2p-exited               | Call `updateP2pAddr` on exited validator. Must revert.                                                                                                                   |
| EDGE:set-p2p-empty                | Call `updateP2pAddr` with empty string. Must revert.                                                                                                                     |
| EDGE:set-p2p-long                 | Call `updateP2pAddr` with string exceeding max length. Must revert.                                                                                                      |
| EDGE:set-p2p-paused               | Call `updateP2pAddr` while contract paused. Must revert.                                                                                                                 |
| EDGE:set-p2p-repeated             | Call `updateP2pAddr` twice with different addresses. Both succeed.                                                                                                       |
| EDGE:register-v3-boundary-p2p     | Register with p2p addr exactly `MAX_P2P_ADDR_LENGTH` bytes. Must succeed.                                                                                                |
| EDGE:set-config-own-x25519        | Register via V3 with key K, then call `updateNetworkConfig` with same key K. Must revert `X25519KeyAlreadyUsed`. Validator must use `updateP2pAddr` to change only addr. |
| EDGE:set-config-unregistered      | Call `updateNetworkConfig` from address that never registered. Must revert `ValidatorInactive`.                                                                          |
| EDGE:p2p-no-colon                 | p2p addr with no `:` (e.g. `localhost`). Must revert.                                                                                                                    |
| EDGE:p2p-no-host                  | p2p addr with empty host (e.g. `:8080`). Must revert.                                                                                                                    |
| EDGE:p2p-no-port                  | p2p addr with empty port (e.g. `host:`). Must revert.                                                                                                                    |
| EDGE:p2p-port-zero                | p2p addr with port 0 (e.g. `host:0`). Must revert.                                                                                                                       |
| EDGE:p2p-port-overflow            | p2p addr with port > 65535 (e.g. `host:70000`). Must revert.                                                                                                             |
| EDGE:p2p-port-non-numeric         | p2p addr with non-numeric port (e.g. `host:abc`). Must revert.                                                                                                           |
| EDGE:p2p-port-leading-zero        | p2p addr with leading zero port (e.g. `host:08080`). Accepted (parses to 8080).                                                                                          |
| EDGE:p2p-valid-ipv4               | p2p addr with IPv4 host (e.g. `192.168.1.1:8080`). Must succeed.                                                                                                         |
| EDGE:p2p-valid-ipv6               | p2p addr with IPv6 host (e.g. `::1:8080`). Must succeed. Last `:` separates port.                                                                                        |
| EDGE:p2p-valid-hostname           | p2p addr with hostname (e.g. `node.example.com:8080`). Must succeed.                                                                                                     |
| EDGE:p2p-multiaddr                | p2p addr in multiaddr format (e.g. `/ip4/1.2.3.4/tcp/4001`). Must revert (no valid `:port` suffix).                                                                      |

### Contract Edge Case Tests

All contract edge case tests are in [`contracts/test/StakeTableV3.t.sol`](../contracts/test/StakeTableV3.t.sol):

| Test                                                      | Edge Case                         |
| --------------------------------------------------------- | --------------------------------- |
| [`test_RegisterValidatorV3_ZeroX25519_Reverts`][v3t]      | EDGE:register-v3-zero-x25519      |
| [`test_RegisterValidatorV3_EmptyP2p_Reverts`][v3t]        | EDGE:register-v3-empty-p2p        |
| [`test_RegisterValidatorV3_LongP2p_Reverts`][v3t]         | EDGE:register-v3-long-p2p         |
| [`test_RegisterValidatorV3_DuplicateX25519_Reverts`][v3t] | EDGE:register-v3-duplicate-x25519 |
| [`test_RegisterValidatorV2_Deprecated_Reverts`][v3t]      | EDGE:register-v2-deprecated       |
| [`test_UpdateNetworkConfig_Inactive_Reverts`][v3t]        | EDGE:set-config-inactive          |
| [`test_UpdateNetworkConfig_Exited_Reverts`][v3t]          | EDGE:set-config-exited            |
| [`test_UpdateNetworkConfig_ZeroX25519_Reverts`][v3t]      | EDGE:set-config-zero-x25519       |
| [`test_UpdateNetworkConfig_EmptyP2p_Reverts`][v3t]        | EDGE:set-config-empty-p2p         |
| [`test_UpdateNetworkConfig_DuplicateX25519_Reverts`][v3t] | EDGE:set-config-duplicate-x25519  |
| [`test_UpdateNetworkConfig_Repeated_Success`][v3t]        | EDGE:set-config-repeated          |
| [`test_UpdateNetworkConfig_Paused_Reverts`][v3t]          | EDGE:set-config-paused            |
| [`test_UpdateX25519Key_Inactive_Reverts`][v3t]            | EDGE:set-x25519-inactive          |
| [`test_UpdateX25519Key_Exited_Reverts`][v3t]              | EDGE:set-x25519-exited            |
| [`test_UpdateX25519Key_Zero_Reverts`][v3t]                | EDGE:set-x25519-zero              |
| [`test_UpdateX25519Key_Duplicate_Reverts`][v3t]           | EDGE:set-x25519-duplicate         |
| [`test_UpdateX25519Key_Repeated_Success`][v3t]            | EDGE:set-x25519-repeated          |
| [`test_UpdateX25519Key_Paused_Reverts`][v3t]              | EDGE:set-x25519-paused            |
| [`test_UpdateP2pAddr_Inactive_Reverts`][v3t]              | EDGE:set-p2p-inactive             |
| [`test_UpdateP2pAddr_Exited_Reverts`][v3t]                | EDGE:set-p2p-exited               |
| [`test_UpdateP2pAddr_Empty_Reverts`][v3t]                 | EDGE:set-p2p-empty                |
| [`test_UpdateP2pAddr_Long_Reverts`][v3t]                  | EDGE:set-p2p-long                 |
| [`test_UpdateP2pAddr_Paused_Reverts`][v3t]                | EDGE:set-p2p-paused               |
| [`test_UpdateP2pAddr_Repeated_Success`][v3t]              | EDGE:set-p2p-repeated             |
| [`test_ValidateP2pAddr_ExactMaxLength`][v3t]              | EDGE:register-v3-boundary-p2p     |
| [`test_UpdateNetworkConfig_OwnX25519_Reverts`][v3t]       | EDGE:set-config-own-x25519        |
| [`test_UpdateNetworkConfig_Inactive_Reverts`][v3t]        | EDGE:set-config-unregistered      |
| [`test_ValidateP2pAddr_NoColon`][v3t]                     | EDGE:p2p-no-colon                 |
| [`test_ValidateP2pAddr_EmptyHost`][v3t]                   | EDGE:p2p-no-host                  |
| [`test_ValidateP2pAddr_EmptyPort`][v3t]                   | EDGE:p2p-no-port                  |
| [`test_ValidateP2pAddr_PortZero`][v3t]                    | EDGE:p2p-port-zero                |
| [`test_ValidateP2pAddr_PortOverflow`][v3t]                | EDGE:p2p-port-overflow            |
| [`test_ValidateP2pAddr_PortNonNumeric`][v3t]              | EDGE:p2p-port-non-numeric         |
| [`test_ValidateP2pAddr_LeadingZeroPort`][v3t]             | EDGE:p2p-port-leading-zero        |
| [`test_ValidateP2pAddr_ValidIpv4`][v3t]                   | EDGE:p2p-valid-ipv4               |
| [`test_ValidateP2pAddr_ValidIpv6`][v3t]                   | EDGE:p2p-valid-ipv6               |
| [`test_ValidateP2pAddr_ValidHostname`][v3t]               | EDGE:p2p-valid-hostname           |
| [`test_ValidateP2pAddr_Multiaddr`][v3t]                   | EDGE:p2p-multiaddr                |

[v3t]: ../contracts/test/StakeTableV3.t.sol

### Rust Edge Cases

| ID                                   | Description                                                                                          |
| ------------------------------------ | ---------------------------------------------------------------------------------------------------- |
| EDGE:rust-register-v3-invalid-sig    | `ValidatorRegisteredV3` with invalid BLS/Schnorr signature. Validator registered as unauthenticated. |
| EDGE:rust-register-v3-bad-p2p        | `ValidatorRegisteredV3` with unparsable p2p addr. Log warning, set `p2p_addr = None`.                |
| EDGE:rust-x25519-unknown-validator   | `X25519KeyUpdated` for unknown validator. Error.                                                     |
| EDGE:rust-x25519-duplicate           | `X25519KeyUpdated` with already-used x25519 key. Error.                                              |
| EDGE:rust-p2p-addr-unknown-validator | `P2pAddrUpdated` for unknown validator. Error.                                                       |
| EDGE:rust-p2p-addr-bad-p2p           | `P2pAddrUpdated` with unparsable p2p addr. Log warning, set `p2p_addr = None`.                       |

### Rust Edge Case Tests

All Rust edge case tests are in [`crates/espresso/types/src/v0/impls/stake_table.rs`][rst]:

| Test                                              | Edge Case                            |
| ------------------------------------------------- | ------------------------------------ |
| [`test_register_v3_invalid_sig`][rst]             | EDGE:rust-register-v3-invalid-sig    |
| [`test_register_v3_empty_p2p_sets_none`][rst]     | EDGE:rust-register-v3-bad-p2p        |
| [`test_x25519_key_update_unknown_validator`][rst] | EDGE:rust-x25519-unknown-validator   |
| [`test_x25519_key_update_duplicate`][rst]         | EDGE:rust-x25519-duplicate           |
| [`test_p2p_addr_update_unknown_validator`][rst]   | EDGE:rust-p2p-addr-unknown-validator |
| [`test_p2p_addr_update_bad_p2p`][rst]             | EDGE:rust-p2p-addr-bad-p2p           |

[rst]: ../crates/espresso/types/src/v0/impls/stake_table.rs

### Upgrade Edge Cases

| ID                                          | Description                                                                                         |
| ------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| EDGE:upgrade-reinitialize-v3                | Calling `initializeV3()` twice reverts `InvalidInitialization`.                                     |
| EDGE:upgrade-unauthorized                   | Non-admin caller cannot trigger upgrade.                                                            |
| EDGE:upgrade-v2-ops-after-v3                | V2 operations (delegate, undelegate, claimWithdrawal, updateConsensusKeysV2) work after V3 upgrade. |
| EDGE:upgrade-pending-undelegation-preserved | Pending undelegation from V2 claimable after V3 upgrade.                                            |
| EDGE:upgrade-exited-validator-preserved     | Exited validator delegations claimable after V3 upgrade.                                            |

### Upgrade Edge Case Tests

All upgrade tests are in [`contracts/test/StakeTableUpgradeToV3.t.sol`][upt]:

| Test                                                     | Edge Case                                   |
| -------------------------------------------------------- | ------------------------------------------- |
| [`test_UpgradeV2ToV3_ReinitializeReverts`][upt]          | EDGE:upgrade-reinitialize-v3                |
| [`test_UpgradeV2ToV3_UnauthorizedReverts`][upt]          | EDGE:upgrade-unauthorized                   |
| [`test_UpgradeV2ToV3_V2OpsAfterUpgrade`][upt]            | EDGE:upgrade-v2-ops-after-v3                |
| [`test_UpgradeV2ToV3_PendingUndelegationPreserved`][upt] | EDGE:upgrade-pending-undelegation-preserved |
| [`test_UpgradeV2ToV3_ExitedValidatorPreserved`][upt]     | EDGE:upgrade-exited-validator-preserved     |

[upt]: ../contracts/test/StakeTableUpgradeToV3.t.sol

### Invariant Tests

Add `updateNetworkConfig`, `updateX25519Key`, and `updateP2pAddr` as fuzzing targets to `StakeTableV2PropTestBase`:

- `updateNetworkConfigOk(actorIndex, x25519Key, p2pAddr)`: pick active validator, bound x25519 to unused key, valid p2p
  addr.
- `updateX25519KeyOk(actorIndex, x25519Key)`: pick active validator, bound x25519 to unused key.
- `updateP2pAddrOk(actorIndex, p2pAddr)`: pick active validator, valid p2p addr.
- `updateNetworkConfigAny(actorIndex, x25519Key, p2pAddr)`: raw fuzz input, expect reverts.
- `updateX25519KeyAny(actorIndex, x25519Key)`: raw fuzz input, expect reverts.
- `updateP2pAddrAny(actorIndex, p2pAddr)`: raw fuzz input, expect reverts.

Existing invariants cover the new functions because network config changes do not affect delegation balances,
`activeStake`, or `totalPendingWithdrawal`.

### Integration Tests

CLI integration tests are in [`staking-cli/src/registration.rs`](../staking-cli/src/registration.rs):

| Test                                                               | Description                                                                                                 |
| ------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------- |
| [`test_update_network_config`](../staking-cli/src/registration.rs) | Deploy V3, register validator, call updateNetworkConfig, verify X25519KeyUpdated and P2pAddrUpdated events. |
| [`test_update_x25519_key`](../staking-cli/src/registration.rs)     | Deploy V3, register validator, call updateX25519Key, verify X25519KeyUpdated event.                         |
| [`test_update_p2p_addr`](../staking-cli/src/registration.rs)       | Deploy V3, register validator, call updateP2pAddr, verify P2pAddrUpdated event.                             |
| TEST:e2e-register-v3-pipeline                                      | not yet implemented                                                                                         |
| TEST:e2e-network-config-pipeline                                   | not yet implemented                                                                                         |
| TEST:e2e-epoch-activation                                          | not yet implemented                                                                                         |
