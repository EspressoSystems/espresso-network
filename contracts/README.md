<!-- This file serves as the landing page for the published smart contract docs.
     Links point to .sol files for GitHub. During doc generation (just doc-contracts),
     they are transformed to point to the generated documentation. -->

# Espresso Network Smart Contracts

Smart contracts for the Espresso Network.

Upgradeable contracts use the frozen inheritance pattern: each new version (e.g. `StakeTableV2`) inherits from the
previous version, preserving storage layout and ensuring upgrade safety.

### Contracts

- [EspToken](src/EspToken.sol)
- [EspTokenV2](src/EspTokenV2.sol)
- [FeeContract](src/FeeContract.sol)
- [InitializedAt](src/InitializedAt.sol)
- [LightClient](src/LightClient.sol)
- [LightClientArbitrum](src/LightClientArbitrum.sol)
- [LightClientArbitrumV2](src/LightClientArbitrumV2.sol)
- [LightClientArbitrumV3](src/LightClientArbitrumV3.sol)
- [LightClientV2](src/LightClientV2.sol)
- [LightClientV3](src/LightClientV3.sol)
- [OpsTimelock](src/OpsTimelock.sol)
- [RewardClaim](src/RewardClaim.sol)
- [SafeExitTimelock](src/SafeExitTimelock.sol)
- [StakeTable](src/StakeTable.sol)
- [StakeTableV2](src/StakeTableV2.sol)

### Interfaces

- [ILightClient](src/interfaces/ILightClient.sol)
- [IPlonkVerifier](src/interfaces/IPlonkVerifier.sol)
- [IRewardClaim](src/interfaces/IRewardClaim.sol)

### Libraries

- [BLSSig](src/libraries/BLSSig.sol)
- [EdOnBN254](src/libraries/EdOnBn254.sol)
- [PlonkVerifier](src/libraries/PlonkVerifier.sol)
- [RewardMerkleTreeVerifier](src/libraries/RewardMerkleTreeVerifier.sol)
