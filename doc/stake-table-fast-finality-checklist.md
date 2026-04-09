# Implementation Checklist: X25519 Key and P2P Address Registration

Design doc: [doc/stake-table-fast-finality.md](stake-table-fast-finality.md)

## Phase 1: StakeTableV3.sol contract

- [x] `StakeTableV3 is StakeTableV2` -- [`contracts/src/StakeTableV3.sol`](../contracts/src/StakeTableV3.sol)
- [x] `initializeV3()` with `reinitializer(3)`
- [x] `getVersion()` returns `(3, 0, 0)`
- [x] `x25519Keys` mapping (`bytes32 => bool`)
- [x] `MAX_P2P_ADDR_LENGTH` constant (512)
- [x] `validateP2pAddr` (public pure): colon check, port parse, host non-empty
- [x] New errors: `InvalidX25519Key`, `X25519KeyAlreadyUsed`, `InvalidP2pAddr`
- [x] New events: `ValidatorRegisteredV3`, `NetworkConfigUpdated`
- [x] `registerValidatorV3`: full registration with x25519 + p2p
- [x] Override `registerValidatorV2` to revert `DeprecatedFunction()`
- [x] `setNetworkConfig`: set/rotate x25519 key + p2p addr
- [x] `updateP2pAddr`: update p2p addr only, emit `NetworkConfigUpdated` with `bytes32(0)` x25519

## Phase 2: Contract tests

### 2a: validateP2pAddr unit tests -- [`contracts/test/StakeTableV3.t.sol`](../contracts/test/StakeTableV3.t.sol)

- [x] Valid: IPv4, IPv6, hostname
- [x] Invalid: no colon, empty host, empty port, port zero, port > 65535
- [x] Invalid: non-numeric port, multiaddr format
- [x] Boundary: exactly `MAX_P2P_ADDR_LENGTH`, leading zero port
- [x] Empty string, exceeds max length

### 2b: StakeTableV3 requirement and edge case tests -- [`contracts/test/StakeTableV3.t.sol`](../contracts/test/StakeTableV3.t.sol)

- [x] `registerValidatorV3` happy path + event check
- [x] `registerValidatorV3` edge cases: zero x25519, empty p2p, long p2p, duplicate x25519
- [x] `registerValidatorV2` deprecated revert after V3
- [x] `setNetworkConfig` happy path + event check
- [x] `setNetworkConfig` edge cases: inactive, exited, zero x25519, empty p2p, duplicate x25519
- [x] `setNetworkConfig` repeated with different keys (both succeed)
- [x] `setNetworkConfig` with own registered x25519 key (reverts)
- [x] `setNetworkConfig` paused (reverts)
- [x] `updateP2pAddr` happy path + event check
- [x] `updateP2pAddr` edge cases: inactive, exited, empty, long, paused
- [x] `updateP2pAddr` repeated with different addresses (both succeed)

### 2c: Upgrade tests -- [`contracts/test/StakeTableUpgradeToV3.t.sol`](../contracts/test/StakeTableUpgradeToV3.t.sol)

- [x] V1 -> V2 -> V3 upgrade preserves state
- [x] V2 operations still work after V3 upgrade
- [x] `initializeV3()` twice reverts
- [x] Unauthorized upgrade reverts
- [x] Pending undelegation from V2 claimable after V3
- [x] Exited validator delegations claimable after V3
- [ ] Invariant targets: `setNetworkConfigOk/Any`, `updateP2pAddrOk/Any`
- [ ] Storage compatibility: `StorageUpgradeCompatibility.t.sol` with `maxMajorVersion = 3`

## Phase 3: Rust bindings and event handling

- [x] Generated bindings --
      [`contracts/rust/adapter/src/bindings/stake_table_v3.rs`](../contracts/rust/adapter/src/bindings/stake_table_v3.rs)
- [x] `RegisterV3`, `NetworkConfigUpdate` variants --
      [`crates/espresso/types/src/v0/v0_3/stake_table.rs`](../crates/espresso/types/src/v0/v0_3/stake_table.rs)
- [x] `TryFrom<StakeTableV3Events>` impl --
      [`crates/espresso/types/src/v0/impls/stake_table.rs`](../crates/espresso/types/src/v0/impls/stake_table.rs)
- [x] `apply_event` handlers for RegisterV3 and NetworkConfigUpdate
- [x] `used_x25519_keys: HashSet` in `StakeTableState`
- [x] Event filter with V3 signatures
- [x] V3 authentication -- [`contracts/rust/adapter/src/stake_table.rs`](../contracts/rust/adapter/src/stake_table.rs)

## Phase 4: Rust unit tests -- [`crates/espresso/types/src/v0/impls/stake_table.rs`](../crates/espresso/types/src/v0/impls/stake_table.rs)

- [x] `test_register_v3_sets_x25519_and_p2p`
- [x] `test_register_v3_invalid_sig`
- [x] `test_register_v3_empty_p2p_sets_none`
- [x] `test_network_config_update_sets_values`
- [x] `test_network_config_update_unknown_validator`
- [x] `test_network_config_update_zero_x25519_skips_key`
- [x] `test_network_config_update_duplicate_x25519`
- [x] `test_network_config_update_hostname_p2p`

## Phase 5: Staking CLI -- [`staking-cli/src/`](../staking-cli/src/)

- [x] `set-network-config` command -- `cli.rs`, `transaction.rs`
- [x] `update-p2p-addr` command -- `cli.rs`, `transaction.rs`
- [x] V3 registration with x25519_key + p2p_addr -- `transaction.rs`
- [x] Integration test: `test_set_network_config` -- `registration.rs`
- [x] Integration test: `test_update_p2p_addr` -- `registration.rs`

## Phase 6: Deployer -- [`contracts/rust/deployer/src/lib.rs`](../contracts/rust/deployer/src/lib.rs)

- [x] `upgrade_stake_table_v3()` with EOA admin
- [x] `StakeTableContractVersion::V3` variant
- [x] `Contract::StakeTableV3` enum variant
- [x] V3 deploy path in `deploy_to_rpc`
- [x] Multisig/timelock path -- [`proposals/multisig.rs`](../contracts/rust/deployer/src/proposals/multisig.rs)
- [x] Builder routing with multisig/EOA -- [`builder.rs`](../contracts/rust/deployer/src/builder.rs)
- [x] V3 included in `deploy_all()`
- [x] V3 is the default `StakeTableContractVersion`

## Phase 7: Invariant and compatibility tests

- [x] Invariant fuzz targets -- [`StakeTableV2PropTestBase.sol`](../contracts/test/StakeTableV2PropTestBase.sol)
- [x] MockStakeTableV3 -- [`MockStakeTableV3.sol`](../contracts/test/MockStakeTableV3.sol)
- [x] Storage compatibility maxVersion=3 --
      [`StorageUpgradeCompatibility.t.sol`](../contracts/test/StorageUpgradeCompatibility.t.sol)
- [x] V3 default in all e2e and integration tests
- [x] V3 added to parametrized tests (delegation, registration, persistence, event processing)
