# Reward claims

Rewards accumulate in `RewardMerkleTreeV2` (`crates/espresso/types/src/v0/reward_mt/mod.rs`), a 160-level binary tree
keyed by Ethereum address. `Header::auth_root()` (`crates/espresso/types/src/v0/impls/header.rs:1626`) keccak-hashes its
root with seven zero placeholders; `LightClientV3.authRoot` stores that value on L1.

1. Query `/v1/reward-state-v2/reward-claim-input/{height}/{address}` for the merkle proof
2. Call `RewardClaim.claimRewards(lifetimeRewards, authData)` on L1
3. `RewardClaim` verifies the proof against `lightClient.authRoot()`
4. Mints `lifetimeRewards - claimedRewards[claimer]` ESP, subject to `dailyLimitWei`
