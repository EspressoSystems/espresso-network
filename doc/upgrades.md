# Upgrades

Hotshot protocol supports upgrades through an Upgrade proposal mechanism. The Upgrade proposal is broadcast separately
from the `QuorumProposal`, typically several views before the upgrade is attempted. The goal is to ensure ample time for
nodes to receive and prepare for the upgrade process.

After enough votes have been collected on the `UpgradeProposal`, an `UpgradeCertificate` is formed. This is attached to
the next `QuorumProposal`, and any node that receives an `UpgradeCertificate` in this way re-attaches it to its own
`QuorumProposal` until the network has upgraded, or (in rare cases) we failed to reach consensus on the
`UpgradeCertificate`.

An upgrade consists of two parts:

- **Version Bump:** The version bump initiates a change in the protocol, which can involve logic updates and type
  changes. Logic updates typically involve adding or modifying the criteria or consequences of block execution. This new
  behavior will be enabled at runtime if sequencer version is greater than or equal to the version behind which they are
  gated. In addition, an upgrade may change the shape of a type. A field of `BlockHeader` might become a `u64` where it
  was before a `u8`. A field may be added to `ChainConfig`. In such cases a new version of these types is added and a
  version of the sequencer designated to incorporate them.
- **Migration:** Migration involves updating existing data to align with the new version, such as updating `ChainConfig`
  values. Since these values are immutable in normal operation, an upgrade is required to modify them. Note that the
  only currently supported upgrade of this kind is the migration of `ChainConfig`.

## Enabling an Upgrade

To enable an upgrade in Hotshot protocol, it is essential to define the base version, the upgrade version, and an
upgrade hash:

- **Base Version:** Represents the current version of the protocol (`0.1` in this example).
- **Upgrade Version:** Specifies the version to which the protocol will upgrade once the process is successful (`0.2` in
  this example).
- **Upgrade Hash:** Acts as a unique identifier for the specific upgrade nodes are voting on. It distinguishes between
  different proposals of the same version upgrade, ensuring nodes vote and execute the correct one. It consists of a
  sequence of 32 bytes.

These are defined in [Versions implementation](../types/src/v0/mod.rs) for the Type (`SequencerVersions` in our case).

```rust
impl Versions for SequencerVersions {
	type Base = StaticVersion<0, 1>;
	type Upgrade = StaticVersion<0, 2>;
	const UPGRADE_HASH: [u8; 32] = [
		1, 0, 1, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0,
	],
	..
}
```

Hotshot provides two modes for upgrades: view-based and time-based.

View-based parameters allow nodes to vote and propose an upgrade based on the current node's view, while time-based
parameters use Unix timestamps for the same purpose.

To simplify configuration, these parameters are fetched from the genesis TOML file and set in the Hotshot config. The
TOML file can include either view-based parameters or time-based parameters, but not both. Furthermore, the start and
stop voting parameters for both time-based and view-based upgrades are optional. If omitted, start parameter will be set
to 0 so that voting begins as soon as node is started while the stop parameter is set to a maximum value so that the
nodes keep voting until enough votes are collected.

View based:

- **start_voting_view:** view at which voting for the upgrade proposal starts.
- **stop_voting_view:** view at which voting for the upgrade proposal stops. To disable an upgrade, set this parameter
  to 0 or ensure `stop_voting_view` is less than `start_voting_view`.
- **start_proposing_view:** the earliest view in which the node can propose an upgrade. This should be set to when an
  upgrade is intended. If the current view > `start_proposing_view`, the node will send out an `UpgradeProposal`.
- **stop_proposing_view:** view after which the node stops proposing an upgrade. If the upgrade proposal fails and the
  current view > stop_proposing_view then the upgrade is never proposed again.

Time based:

- **start_voting_time:** UNIX timestamp at which voting for the upgrade proposal starts.
- **stop_voting_time:** UNIX timestamp at which voting for the upgrade proposal stops.
- **start_proposing_time:** the earliest UNIX timestamp in which the node can propose an upgrade.
- **stop_proposing_time:** UNIX timestamp after which the node stops proposing an upgrade.

The window between `start_proposing_view/time` and `stop_proposing_view/time` should provide sufficient time for nodes
to continue proposing the upgrade until successful.

Ensure that the `ESPRESSO_SEQUENCER_GENESIS_FILE` environment variable is defined to point to the path of the genesis
TOML file. For an example with upgrades enabled, refer to [`data/genesis/demo.toml`](../data/genesis/demo.toml).

### Example TOML Configuration

```toml
[[upgrade]]
version = "0.2"
start_proposing_view = 5
stop_proposing_view = 15

[upgrade.fee]

[upgrade.fee.chain_config]
chain_id = 999999999
base_fee = '1 wei'
max_block_size = '1mb'
fee_recipient = '0x0000000000000000000000000000000000000000'
fee_contract = '0xa15bb66138824a1c7167f5e85b957d04dd34e468'

[[upgrade]]
version = "0.3"
start_proposing_view = 5
stop_proposing_view = 15

[upgrade.marketplace]
```

In the TOML configuration example above, the `upgrade` section defines an array of tables, each specifying upgrade
parameters

- **Version:** the new version after an upgrade is successful.
- **start_proposing_view:** Represents the `start_proposing_view` value at which the upgrade is proposed.
- **stop_proposing_view:** Refers to the view view after which the node stops proposing an upgrade.

The `upgrade.fee.chain_config` table contains the complete set of chain config parameters, which can be used, for
example, to enable protocol fees or modify other parameters.

## Fee upgrade

A successful Hotshot upgrade results in a new version, which allows us to update the `ChainConfig` and execute the
upgrade if there exists any. `Chainconfig` includes the fee parameters. The sequencer node has two states: `NodeState`
and `ValidatedState`. `NodeState` is an immutable state that contains `ResolvableChainConfig` (Enum of `ChainConfig`'s
commitment and full `ChainConfig`), whereas `ValidatedState` is a mutable state. To make updates to the chain config
post-upgrade possible, `ResolvableChainConfig` is also added to `ValidatedState`.

`NodeState` also includes two additional fields: `upgrades` and `current_version`. Functions like `Header::new()` and
`ValidatedState::apply_header()` include a version parameter, which is used to apply upgrades by checking if this
version is greater than `current_version` in NodeState and fetching the upgrade, if available, from the upgrades
BTreeMap in NodeState.

In scenarios where nodes join the network or restart, missing the upgrade window may result in their ValidatedState
having only a chain config commitment. In such cases, nodes need to catch up from their peers to get the full chain
config for this chain config commitment.

Note: For the fee upgrade to work, the builder must have sufficient funds to cover the fees. The Espresso bridge can be
used to fund the builder.
