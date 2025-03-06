// SPDX-License-Identifier: Unlicensed

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../libraries/EdOnBn254.sol";

/// @title Interface for stake table that keep track of validators' external key and staked amount.
/// @dev Stake delegation happens in a separate DelegationPool contract, and specific to
/// instantiation thus not part of this interface
/// @dev Stake table contract should store a reference to the `LightClient.sol` to query
/// "epoch-related" info
abstract contract AbstractStakeTable {
    // === Events ===

    /// @notice Signals a registration of a new validator.
    ///
    /// Signals to the confirmation layer that a new validator is available for
    /// delegations. The confirmation layer needs to to use this event to keep
    /// track of the validator's keys for the stake table.
    event ValidatorRegistered(
        address account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission
    );

    /// @notice Validator initiated exit from stake table
    ///
    /// All funds delegated to this validator will be undelegated and the
    /// validator will be removed from the active stake table.
    event ValidatorExit(address validator);

    /// @notice Signals a new delegation to a validator
    ///
    /// A delegator delegates funds to a validator. The confirmation layer needs
    /// to update the stake table and adjust the weight for this validator and
    /// the delegators delegation associated with it.
    event Delegated(address delegator, address validator, uint256 amount);

    /// @notice Signals an undelegation to a validator
    ///
    /// A delegator undelegates funds from a validator. The confirmation layer
    /// needs to update the stake table and adjust the weight for this validator
    /// and the delegators delegation associated with it.
    event Undelegated(address delegator, address validator, uint256 amount);

    /// @notice Signals a consensus key update for a validator
    ///
    /// @param account the address of the validator
    /// @param blsVK the new BLS verification key
    /// @param schnorrVK the new Schnorr verification key
    ///
    /// The confirmation layer needs to update the stake table with the new keys.
    event ConsensusKeysUpdated(
        address account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK
    );

    /// @notice Unlocked funds were claimed.
    ///
    /// @dev This event is not relevant for the confirmation layer because the
    /// events that remove stake from the stake table are `Undelegated` and
    /// `ValidatorExit`.
    event Withdrawal(address account, uint256 amount);

    // === Structs ===

    /// @dev (sadly, Solidity doesn't support type alias on non-primitive types)
    // We avoid declaring another struct even if the type info helps with readability,
    // extra layer of struct introduces overhead and more gas cost.
    //
    /// @dev BLS verification key (used for Consensus voting in HotShot) is `BN254.G2Point`.
    // `type BlsVerKey is BN254.G2Point;`
    //
    /// @dev Verification key of a Schnorr signature is just a `EdOnBN254.EdOnBN254Point`
    // `type SchnorrVerKey is EdOnBN254.EdOnBN254Point;`

    /// @notice Represents a HotShot validator node
    /// In the dual-staking model, a HotShot validator could have multiple `Node` entries.
    /// @param account The Ethereum account of the validator.
    /// @param balance The amount of token staked.
    /// @param registerEpoch The starting epoch for the validator.
    /// @param exitEpoch The ending epoch for the validator.
    /// @param schnorrVK The Schnorr verification key associated. Used for signing the light client
    /// state.
    /// @param blsVK The BLS verification key associated. Used for consensus voting.
    // struct Node {
    //     address account;
    //     uint256 balance;
    //     EdOnBN254.EdOnBN254Point schnorrVK;
    //     BN254.G2Point blsVK;
    // }

    /// These are just examples for now
    // enum SlashableOffense {
    //     DoubleSigning,
    //     InvalidSignature
    // }

    // === Table State & Stats ===

    // /// @notice Look up the balance of `account`
    // function lookupStake(address account) external view virtual returns (uint256);

    // /// @notice Look up the full `Node` state associated with `account`
    // function lookupNode(address account) external view virtual returns (Node memory);

    // === Write APIs ===

    /// @notice Register a validator in the stake table, transfer of tokens incurred!
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key (as the auxiliary info)
    /// @param blsSig The BLS signature that the caller owns the `blsVK`
    /// @dev No validity check on `schnorrVK`, as it's assumed to be sender's responsibility,
    /// the contract only treat it as auxiliary info submitted by `blsVK`.
    /// @dev `blsSig` field is necessary to prevent "rogue public-key attack".
    /// The signature is over the caller address of the function to ensure that each message is
    /// unique.
    function registerValidator(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        uint16 commission
    ) external virtual;

    /// @notice The validator and all their delegation will exit the stake table.
    function deregisterValidator() external virtual;

    /// @notice Delegate
    ///
    /// @param amount The amount to deposit
    function delegate(address validator, uint256 amount) external virtual;

    /// @notice initiate withdrawal
    function undelegate(address validator, uint256 amount) external virtual;

    /// @notice Withdraw an undelegation
    function claimWithdrawal(address validator) external virtual;

    /// @notice Withdraw after a validator has exited
    function claimValidatorExit(address validator) external virtual;

    /// @notice Update the consensus keys for a validator
    /// @dev This function is used to update the consensus keys for a validator
    /// @dev This function can only be called by the validator itself when it's not in the exit
    /// queue
    /// @dev The validator will need to give up either its old BLS key and/or old Schnorr key
    /// @dev The validator will need to provide a BLS signature over the new BLS key
    /// @param newBlsVK The new BLS verification key
    /// @param newSchnorrVK The new Schnorr verification key
    /// @param newBlsSig The BLS signature that the account owns the new BLS key
    function updateConsensusKeys(
        BN254.G2Point memory newBlsVK,
        EdOnBN254.EdOnBN254Point memory newSchnorrVK,
        BN254.G1Point memory newBlsSig
    ) external virtual;
}
