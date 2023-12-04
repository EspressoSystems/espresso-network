pragma solidity ^0.8.0;

import { SafeTransferLib, ERC20 } from "solmate/utils/SafeTransferLib.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";
import { AbstractStakeTable } from "./interfaces/AbstractStakeTable.sol";
import { ExampleToken } from "../src/ExampleToken.sol";
import { LightClient } from "../src/LightClient.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";

/// @title Implementation of the Stake Table interface
contract StakeTable is AbstractStakeTable {
    /// Error to notify restaking is not implemented yet.
    error RestakingNotImplemented();

    /// Error raised when the registration is aborted because it happens after the user specified
    /// deadline. The first field is the next registration epoch and the second is the last epoch
    /// the user is willing to wait for the registration to happen.
    error InvalidNextRegistrationEpoch(uint64, uint64);

    /// Error raised when a user tries to register another set of keys from the same ethereum
    /// account.
    error NodeAlreadyRegistered();

    /// Mapping from a hash of a BLS key to a node struct defined in the abstract contract.
    mapping(bytes32 keyHash => Node node) public nodes;

    /// Total native stake locked for the latest stake table (HEAD).
    uint256 public totalNativeStake;

    /// Total restaked stake locked for the latest stake table (HEAD).
    uint256 public totalRestakedStake;

    /// Size of the registration queue for the next available epoch.
    uint64 public numRegistrations;

    /// Size of the exit queue for the next available epoch.
    uint64 public numPendingExits;

    /// Address of the native token contract.
    address public tokenAddress;

    /// Reference to the light client contract.
    LightClient public lightClient;

    constructor(address _tokenAddress, address _lightClientAddress) {
        tokenAddress = _tokenAddress;
        lightClient = LightClient(_lightClientAddress);
    }

    /// @dev Computes a hash value of some G2 point.
    /// @param blsVK BLS verification key in G2
    /// @return keccak256(blsVK)
    function _hashBlsKey(BN254.G2Point memory blsVK) public pure returns (bytes32) {
        return keccak256(abi.encode(blsVK.x0, blsVK.x1, blsVK.y0, blsVK.y1));
    }

    /// @dev Fetches the current epoch from the light client contract.
    /// @return current epoch (computed from the current block)
    function currentEpoch() public view returns (uint64) {
        return lightClient.currentEpoch();
    }

    /// @notice Total stakes of the registered keys in the latest stake table (Head).
    /// @dev Given that the current implementation does not support restaking, the second value of
    /// the output is set to 0.
    /// @return The total stake for native token and restaked token respectively.
    function totalStake() external view override returns (uint256, uint256) {
        return (totalNativeStake, totalRestakedStake);
    }

    /// @notice Look up the balance of `blsVK`
    /// @param blsVK BLS public key controlled by the user.
    /// @return Current balance owned by the user.
    function lookupStake(BN254.G2Point memory blsVK) external view override returns (uint64) {
        Node memory node = this.lookupNode(blsVK);
        return node.balance;
    }

    /// @notice Look up the full `Node` state associated with `blsVK`
    /// @dev The lookup is achieved by hashing first the four field elements of blsVK using
    /// keccak256.
    /// @return Node indexed by blsVK
    function lookupNode(BN254.G2Point memory blsVK) external view override returns (Node memory) {
        return nodes[_hashBlsKey(blsVK)];
    }

    /// @notice Get the next available epoch for new registration.
    /// @return Number of the epoch when the user can register.
    function nextRegistrationEpoch() external view override returns (uint64) {
        // TODO implement queue logic
        return currentEpoch() + 1;
    }

    /// @notice Get the number of pending registration requests in the waiting queue
    function numPendingRegistrations() external view override returns (uint64) {
        return numRegistrations;
    }

    /// @notice Get the next available epoch for exit
    function nextExitEpoch() external view override returns (uint64) {
        if (numPendingExits == 0) {
            return 0;
        } else {
            return 1;
        }
    }

    /// @notice Get the number of pending exit requests in the waiting queue
    function numPendingExit() external view override returns (uint64) {
        return numPendingExits;
    }

    /// @notice Register a validator in the stake table, transfer of tokens incurred!
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key (as the auxiliary info)
    /// @param amount The amount to register
    /// @param stakeType The type of staking (native or restaking)
    /// @param blsSig The BLS signature that authenticates the ethereum account this function is
    /// called from
    /// @param validUntilEpoch The maximum epoch the sender is willing to wait to be included
    /// (cannot be smaller than the current epoch)
    ///
    /// @return success status
    ///
    /// @dev No validity check on `schnorrVK`, as it's assumed to be sender's responsibility,
    /// the contract only treat it as auxiliary info submitted by `blsVK`.
    /// @dev `blsSig` field is necessary to prevent "rogue public-key attack".
    /// The signature is over the caller address of the function to ensure that each message is
    /// unique.
    function register(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        uint64 amount,
        StakeType stakeType,
        BN254.G1Point memory blsSig,
        uint64 validUntilEpoch
    ) external override returns (bool) {
        if (stakeType != StakeType.Native) {
            revert RestakingNotImplemented();
        }

        bytes memory message = abi.encode(msg.sender);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        // Find the earliest epoch at which this node can register. Usually, this will be
        // currentEpoch() + 1 (the start of the next full epoch), but in periods of high churn the
        // queue may fill up and it may be later. If the queue is so full that the wait time exceeds
        // the caller's desired maximum wait, abort.
        uint64 registerEpoch = this.nextRegistrationEpoch();
        if (registerEpoch > validUntilEpoch) {
            revert InvalidNextRegistrationEpoch(registerEpoch, validUntilEpoch);
        }

        bytes32 key = _hashBlsKey(blsVK);
        Node memory node = nodes[key];

        // The node must not already be registered.
        if (node.account != address(0x0)) {
            revert NodeAlreadyRegistered();
        }

        // Create an entry for the node.
        node.account = msg.sender;
        node.balance = amount;
        node.stakeType = stakeType;
        node.schnorrVK = schnorrVK;
        node.registerEpoch = registerEpoch;

        nodes[key] = node;

        // Lock the deposited tokens in this contract.
        if (stakeType == StakeType.Native) {
            totalNativeStake += amount;
            SafeTransferLib.safeTransferFrom(ERC20(tokenAddress), msg.sender, address(this), amount);
        } // Other case will be implemented when we support restaking

        emit Registered(key, registerEpoch, stakeType, amount);

        return true;
    }

    /// @notice Deposit more stakes to registered keys
    ///
    /// @param blsVK The BLS verification key
    /// @param amount The amount to deposit
    /// @return (newBalance, effectiveEpoch) the new balance effective at a future epoch
    function deposit(BN254.G2Point memory blsVK, uint64 amount)
        external
        override
        returns (uint64, uint64)
    {
        bytes32 hash = _hashBlsKey(blsVK);
        nodes[hash].balance += amount;
        return (0, 0);
    }

    /// @notice Request to exit from the stake table, not immediately withdrawable!
    ///
    /// @param blsVK The BLS verification key to exit
    /// @return success status
    function requestExit(BN254.G2Point memory blsVK) external override returns (bool) {
        bytes32 hash = _hashBlsKey(blsVK);
        nodes[hash].exitEpoch = 0;
        return true;
    }

    /// @notice Withdraw from the staking pool. Transfers occur! Only successfully exited keys can
    /// withdraw past their `exitEpoch`.
    ///
    /// @param blsVK The BLS verification key to withdraw
    /// @return The total amount withdrawn, equal to `Node.balance` associated with `blsVK`
    function withdrawFunds(BN254.G2Point memory blsVK) external override returns (uint64) {
        bytes32 hash = _hashBlsKey(blsVK);
        nodes[hash].balance = 0;
        return 0;
    }
}
