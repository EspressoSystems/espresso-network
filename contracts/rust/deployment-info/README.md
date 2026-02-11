# Deployment Info Tool

Tool to collect and output Layer 1 deployment information for Espresso Network contracts.

## Usage

```bash
# Process all networks (decaf, hoodi, mainnet)
cargo run -p deployment-info

# Process a specific network
cargo run -p deployment-info -- --network <mainnet|decaf|hoodi>

# Print to stdout instead of file
cargo run -p deployment-info -- --network decaf --stdout

# Custom RPC URL
cargo run -p deployment-info -- --network mainnet --rpc-url https://eth.llamarpc.com
```

## Directory Structure

- **Input**: [addresses/](addresses/) - contract addresses per network
- **Output**: [deployments/](deployments/) - deployment info per network (TOML)

## Contract Information Collected

For each contract:

- **Address**: From environment variable
- **Owner**: Queried on-chain via AccessControl (`DEFAULT_ADMIN_ROLE`) for StakeTable/RewardClaim, `Ownable.owner()` for
  others
- **Owner name**: Resolved from known addresses (multisigs + timelocks in the .env)
- **Version**: Queried via `IVersioned.getVersion()`
- **Pauser** (StakeTable, RewardClaim): Who holds `PAUSER_ROLE`, resolved to a name

All role holders must be known addresses from the .env config; the tool errors if an unknown address holds a role.

For Safe multisigs:

- **Version**: Queried via `VERSION()`
- **Owners**: Queried via `getOwners()`
- **Threshold**: Queried via `getThreshold()`

For timelocks:

- **Min delay**: Queried via `getMinDelay()`, displayed in human-readable format (e.g. `7days`)

## Deployments

<!-- DEPLOYMENT_TABLE_START -->
<!-- prettier-ignore-start -->
### mainnet

| Contract | Address | Version | Owner | Pauser |
|----------|---------|---------|-------|--------|
| EspToken | [`0x031De51F3E8016514Bd0963d0B2AB825A591Db9A`](https://etherscan.io/address/0x031De51F3E8016514Bd0963d0B2AB825A591Db9A) | 2.0.0 | safe_exit_timelock | - |
| FeeContract | [`0x9fcE21c3F7600Aa63392A5F5713986b39bB98884`](https://etherscan.io/address/0x9fcE21c3F7600Aa63392A5F5713986b39bB98884) | 1.0.0 | espresso_labs | - |
| LightClient | [`0x95Ca91Cea73239b15E5D2e5A74d02d6b5E0ae458`](https://etherscan.io/address/0x95Ca91Cea73239b15E5D2e5A74d02d6b5E0ae458) | 3.0.0 | espresso_labs | - |
| RewardClaim | [`0x67c966a0ecdd5c33608bE7810414e5b54DA878D8`](https://etherscan.io/address/0x67c966a0ecdd5c33608bE7810414e5b54DA878D8) | 1.0.0 | safe_exit_timelock | serviceco |
| StakeTable | [`0xCeF474D372B5b09dEfe2aF187bf17338Dc704451`](https://etherscan.io/address/0xCeF474D372B5b09dEfe2aF187bf17338Dc704451) | 2.0.0 | ops_timelock | serviceco |

| Multisig | Address | Version | Threshold |
|----------|---------|---------|----------|
| espresso_labs | [`0x34F5af5158171Ffd2475d21dB5fc3B311F221982`](https://etherscan.io/address/0x34F5af5158171Ffd2475d21dB5fc3B311F221982) | 1.4.1 | 3 |
| serviceco | [`0x5e37B8038615EF3D75cf28b5982C4CBF065401fB`](https://etherscan.io/address/0x5e37B8038615EF3D75cf28b5982C4CBF065401fB) | 1.4.1 | 3 |

| Timelock | Address | Min Delay |
|---------|---------|----------|
| ops_timelock | [`0x67861f1eF4Db9BCADdD8c5E86dB92386Dd4EC700`](https://etherscan.io/address/0x67861f1eF4Db9BCADdD8c5E86dB92386Dd4EC700) | 2days |
| safe_exit_timelock | [`0x6E7941fE8F9C751363b5c156419a0C8912dEA6b2`](https://etherscan.io/address/0x6E7941fE8F9C751363b5c156419a0C8912dEA6b2) | 14days |

### decaf

| Contract | Address | Version | Owner | Pauser |
|----------|---------|---------|-------|--------|
| EspToken | [`0xb3e655a030e2e34a18b72757b40be086a8F43f3b`](https://sepolia.etherscan.io/address/0xb3e655a030e2e34a18b72757b40be086a8F43f3b) | 2.0.0 | espresso_labs | - |
| FeeContract | [`0x42835083fD1d3FC5d799B5f6815AE4BF2623E6D0`](https://sepolia.etherscan.io/address/0x42835083fD1d3FC5d799B5f6815AE4BF2623E6D0) | 1.0.0 | espresso_labs | - |
| LightClient | [`0x303872BB82a191771321d4828888920100d0b3e4`](https://sepolia.etherscan.io/address/0x303872BB82a191771321d4828888920100d0b3e4) | 3.0.0 | espresso_labs | - |
| RewardClaim | [`0xe81908E34dBb4BA01f27F8769264199727Be50c8`](https://sepolia.etherscan.io/address/0xe81908E34dBb4BA01f27F8769264199727Be50c8) | 1.0.0 | safe_exit_timelock | espresso_labs |
| StakeTable | [`0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037`](https://sepolia.etherscan.io/address/0x40304FbE94D5E7D1492Dd90c53a2D63E8506a037) | 2.0.0 | espresso_labs | espresso_labs |

| Multisig | Address | Version | Threshold |
|----------|---------|---------|----------|
| espresso_labs | [`0xB76834E371B666feEe48e5d7d9A97CA08b5a0620`](https://sepolia.etherscan.io/address/0xB76834E371B666feEe48e5d7d9A97CA08b5a0620) | 1.4.1 | 2 |

| Timelock | Address | Min Delay |
|---------|---------|----------|
| ops_timelock | [`0x8e3b6563D683b87964104A2c3A4bf542bb70767F`](https://sepolia.etherscan.io/address/0x8e3b6563D683b87964104A2c3A4bf542bb70767F) | 5m |
| safe_exit_timelock | [`0x0eB0Ef3B5a46a444C38dA452055bddb273550d5c`](https://sepolia.etherscan.io/address/0x0eB0Ef3B5a46a444C38dA452055bddb273550d5c) | 10m |

### hoodi

| Contract | Address | Version | Owner | Pauser |
|----------|---------|---------|-------|--------|
| EspToken | [`0x7397063418dF1eE7a6c49b5bbc664f8f2c82283a`](https://hoodi.etherscan.io/address/0x7397063418dF1eE7a6c49b5bbc664f8f2c82283a) | 2.0.0 | safe_exit_timelock | - |
| FeeContract | [`0xE6548625c4D5441872820E5af3e809F5B839D914`](https://hoodi.etherscan.io/address/0xE6548625c4D5441872820E5af3e809F5B839D914) | 1.0.0 | ops_timelock | - |
| LightClient | [`0x578767719287386043D3Aaf2F5A68993C3a42CD5`](https://hoodi.etherscan.io/address/0x578767719287386043D3Aaf2F5A68993C3a42CD5) | 3.0.0 | ops_timelock | - |
| RewardClaim | [`0xf6FCFd30F1b22BF6E78AdE09c54566594c703183`](https://hoodi.etherscan.io/address/0xf6FCFd30F1b22BF6E78AdE09c54566594c703183) | 1.0.0 | safe_exit_timelock | espresso_labs |
| StakeTable | [`0x6e9c2DCe0Cb780a06c811Ca245b8d497eF6E96C5`](https://hoodi.etherscan.io/address/0x6e9c2DCe0Cb780a06c811Ca245b8d497eF6E96C5) | 2.0.0 | ops_timelock | espresso_labs |

| Multisig | Address | Version | Threshold |
|----------|---------|---------|----------|
| espresso_labs | [`0x26bF8656f1654A14570Af587aDc8cac68fDa6Fcf`](https://hoodi.etherscan.io/address/0x26bF8656f1654A14570Af587aDc8cac68fDa6Fcf) | 1.4.1 | 2 |

| Timelock | Address | Min Delay |
|---------|---------|----------|
| ops_timelock | [`0xf04E9344F0F28AA5ef0f321a9Cfc00680BC53118`](https://hoodi.etherscan.io/address/0xf04E9344F0F28AA5ef0f321a9Cfc00680BC53118) | 5m |
| safe_exit_timelock | [`0xE34cDDdF2271492809728C83eB347b36Ba001438`](https://hoodi.etherscan.io/address/0xE34cDDdF2271492809728C83eB347b36Ba001438) | 10m |
<!-- prettier-ignore-end -->
<!-- DEPLOYMENT_TABLE_END -->
