# Implementation Checklist: X25519 Key and P2P Address Registration

Design doc: [doc/stake-table-fast-finality.md](stake-table-fast-finality.md)

After each phase stabilizes, update the design doc with links to test files and implementations.

## Phase 1: StakeTableV3.sol contract

- [ ] `StakeTableV3 is StakeTableV2`
- [ ] `initializeV3()` with `reinitializer(3)`
- [ ] `getVersion()` returns `(3, 0, 0)`
- [ ] `x25519Keys` mapping (`bytes32 => bool`)
- [ ] `MAX_P2P_ADDR_LENGTH` constant (512)
- [ ] `validateP2pAddr` (public pure): colon check, port parse, host non-empty
- [ ] New errors: `InvalidX25519Key`, `X25519KeyAlreadyUsed`, `InvalidP2pAddr`
- [ ] New events: `ValidatorRegisteredV3`, `NetworkConfigUpdated`
- [ ] `registerValidatorV3`: full registration with x25519 + p2p
- [ ] Override `registerValidatorV2` to revert `DeprecatedFunction()`
- [ ] `setNetworkConfig`: set/rotate x25519 key + p2p addr
- [ ] `updateP2pAddr`: update p2p addr only, emit `NetworkConfigUpdated` with `bytes32(0)` x25519

## Phase 2: Contract tests

### 2a: validateP2pAddr unit tests

- [ ] Valid: IPv4 (`192.168.1.1:8080`), IPv6 (`::1:8080`), hostname (`node.example.com:8080`)
- [ ] Invalid: no colon, empty host (`:8080`), empty port (`host:`), port zero, port > 65535
- [ ] Invalid: non-numeric port, multiaddr format (`/ip4/1.2.3.4/tcp/4001`)
- [ ] Boundary: exactly `MAX_P2P_ADDR_LENGTH`, leading zero port (`host:08080`)
- [ ] Empty string, exceeds max length

### 2b: StakeTableV3 requirement and edge case tests

- [ ] `registerValidatorV3` happy path + event check
- [ ] `registerValidatorV3` edge cases: zero x25519, empty p2p, long p2p, duplicate x25519
- [ ] `registerValidatorV2` deprecated revert after V3
- [ ] `setNetworkConfig` happy path + event check
- [ ] `setNetworkConfig` edge cases: inactive, exited, zero x25519, empty p2p, duplicate x25519
- [ ] `setNetworkConfig` repeated with different keys (both succeed)
- [ ] `setNetworkConfig` with own registered x25519 key (reverts)
- [ ] `setNetworkConfig` from unregistered address (reverts)
- [ ] `setNetworkConfig` paused (reverts)
- [ ] `updateP2pAddr` happy path + event check
- [ ] `updateP2pAddr` edge cases: inactive, exited, empty, long, paused
- [ ] `updateP2pAddr` repeated with different addresses (both succeed)
- [ ] Boundary: p2p addr exactly `MAX_P2P_ADDR_LENGTH` (succeeds)

### 2c: Upgrade and invariant tests

- [ ] `StakeTableUpgradeToV3.t.sol`: V1 -> V2 -> V3 upgrade preserves state
- [ ] V2 operations still work after V3 upgrade (delegate, undelegate, claimWithdrawal, updateConsensusKeysV2)
- [ ] `initializeV3()` twice reverts
- [ ] Unauthorized upgrade reverts
- [ ] Pending undelegation from V2 claimable after V3
- [ ] Exited validator delegations claimable after V3
- [ ] Invariant targets: `setNetworkConfigOk/Any`, `updateP2pAddrOk/Any`
- [ ] Storage compatibility: `StorageUpgradeCompatibility.t.sol` with `maxMajorVersion = 3`

## Phase 3: Rust bindings and event handling

- [ ] `just gen-bindings` (regenerate with V3 ABI)
- [ ] Add `RegisterV3`, `NetworkConfigUpdate` variants to `StakeTableEvent`
- [ ] `TryFrom<StakeTableV3Events>` impl
- [ ] `apply_event` handler for `RegisterV3` (same as V2 but sets x25519_key + p2p_addr)
- [ ] `apply_event` handler for `NetworkConfigUpdate` (x25519 if non-zero, p2p_addr always)
- [ ] `used_x25519_keys: HashSet<x25519::PublicKey>` in `StakeTableState`
- [ ] Event filter: add `ValidatorRegisteredV3::SIGNATURE`, `NetworkConfigUpdated::SIGNATURE`
- [ ] `contracts/rust/adapter/src/stake_table.rs`: authentication for V3 registration

## Phase 4: Rust unit tests

- [ ] `RegisterV3` with invalid BLS/Schnorr sig (registered as unauthenticated)
- [ ] `RegisterV3` with unparsable p2p addr (warning, p2p_addr = None)
- [ ] `NetworkConfigUpdate` for unknown validator (error)
- [ ] `NetworkConfigUpdate` with `bytes32(0)` x25519 (skip key update, only update p2p)
- [ ] `NetworkConfigUpdate` with duplicate x25519 key (error)
- [ ] `NetworkConfigUpdate` with unparsable p2p addr (warning, p2p_addr = None)

## Phase 5: Staking CLI

- [ ] `set-network-config --x25519-key <KEY> --p2p-addr <ADDR>` command
- [ ] `update-p2p-addr --p2p-addr <ADDR>` command
- [ ] Update register command with `--x25519-key` and `--p2p-addr` flags
- [ ] Event display formatting for `ValidatorRegisteredV3` and `NetworkConfigUpdated`

## Phase 6: Deployer

- [ ] `prepare_stake_table_v3_upgrade()` / `upgrade_stake_table_v3()`
- [ ] Multisig/timelock path (`upgrade_stake_table_v3_multisig_owner()`)
- [ ] `--upgrade-stake-table-v3` CLI flag
