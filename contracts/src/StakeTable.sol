pragma solidity ^0.8.0;

import { SafeTransferLib, ERC20 } from "solmate/utils/SafeTransferLib.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";
import { AbstractStakeTable } from "./interfaces/AbstractStakeTable.sol";
import { LightClientV2 as LightClient } from "../src/LightClientV2.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { InitializedAt } from "./InitializedAt.sol";

using EdOnBN254 for EdOnBN254.EdOnBN254Point;

/// @title Implementation of the Stake Table interface
contract StakeTable is AbstractStakeTable, Ownable, InitializedAt {
    /// Error to notify restaking is not implemented yet.
    error RestakingNotImplemented();

    /// Error raised when the registration is aborted because it happens after the user specified
    /// deadline. The first field is the next registration epoch and the second is the last epoch
    /// the user is willing to wait for the registration to happen.
    error InvalidNextRegistrationEpoch(uint64, uint64);

    /// Error raised when a user tries to register another set of keys from the same ethereum
    /// account.
    error NodeAlreadyRegistered();

    /// Error raised when a user tries to withdraw funds from a node that is not registered.
    error NodeNotRegistered();

    /// Error raised when a user tries to make a deposit or request an exit but does not control the
    /// node public key.
    error Unauthenticated();

    /// Error raised when a user tries to deposit before the registration is complete.
    error PrematureDeposit();

    /// Error raised when a user tries to exit before the registration is complete.
    error PrematureExit();

    /// Error raised when a user tries to deposit while an exit request is in progress.
    error ExitRequestInProgress();

    // Error raised when a user tries to withdraw funds before the exit escrow period is over.
    error PrematureWithdrawal();

    // Error raised when this contract does not have the sufficient allowance on the stake ERC20
    // token
    error InsufficientAllowance(uint256, uint256);

    // Error raised when the staker does not have the sufficient balance on the stake ERC20 token
    error InsufficientBalance(uint256);

    // Error raised when the staker does not have the sufficient stake balance to withdraw
    error InsufficientStakeBalance(uint256);

    // Error raised when the staker does not register with the correct stakeAmount
    error InsufficientStakeAmount(uint256);

    // Error raised when the staker does not provide a new schnorrVK
    error InvalidSchnorrVK();

    // Error raised when the staker does not provide a new blsVK
    error InvalidBlsVK();

    // Error raised when zero point keys are provided
    error NoKeyChange();

    /// Error raised when the caller is not the owner
    error Unauthorized();

    /// Error raised when the light client address is invalid
    error InvalidAddress();

    /// Error raised when the value is invalid
    error InvalidValue();

    /// Mapping from a hash of a BLS key to a node struct defined in the abstract contract.
    mapping(address account => Node node) public nodes;

    /// Total stake locked;
    uint256 public totalStake;

    /// Address of the native token contract.
    address public tokenAddress;

    /// Reference to the light client contract.
    LightClient public lightClient;

    /// @notice the first available epoch for registration
    uint64 public registrationEpoch;
    /// @notice number of pending registrations in the `registrationEpoch` (not the
    /// total pending queue size!)
    uint64 public override numPendingRegistrationsInEpoch;

    /// @notice the first available epoch for exit
    uint64 public exitEpoch;
    /// @notice number of pending exits in the `exitEpoch` (not the total pending
    /// queue size!)
    uint64 public override numPendingExitsInEpoch;

    /// @notice The number of validators that can register per epoch as well as
    /// the number of validators that can exit per epoch.
    uint64 public maxNumChurnPerEpoch;

    address public admin;

    uint256 public minStakeAmount;

    /// TODO change constructor to initialize function when we make the contract upgradeable
    constructor(
        address _tokenAddress,
        address _lightClientAddress,
        uint64 _maxNumChurnPerEpoch,
        uint256 _minStakeAmount,
        address _initialOwner
    ) Ownable(_initialOwner) InitializedAt() {
        tokenAddress = _tokenAddress;
        lightClient = LightClient(_lightClientAddress);
        maxNumChurnPerEpoch = _maxNumChurnPerEpoch;

        // A set of hardcoded stakers is defined for the first epoch.
        registrationEpoch = 1;
        numPendingRegistrationsInEpoch = 0;

        // It is not possible to exit during the first epoch.
        exitEpoch = 1;
        numPendingExitsInEpoch = 0;

        minStakeAmount = _minStakeAmount;
        admin = msg.sender;
    }

    /// @dev Computes a hash value of some G2 point.
    /// @param blsVK BLS verification key in G2
    /// @return keccak256(blsVK)
    function _hashBlsKey(BN254.G2Point memory blsVK) public pure returns (bytes32) {
        return keccak256(abi.encode(blsVK.x0, blsVK.x1, blsVK.y0, blsVK.y1));
    }

    /// @dev Compares two BLS keys for equality
    /// @param a First BLS key
    /// @param b Second BLS key
    /// @return True if the keys are equal, false otherwise
    function _isEqualBlsKey(BN254.G2Point memory a, BN254.G2Point memory b)
        public
        pure
        returns (bool)
    {
        return BN254.BaseField.unwrap(a.x0) == BN254.BaseField.unwrap(b.x0)
            && BN254.BaseField.unwrap(a.x1) == BN254.BaseField.unwrap(b.x1)
            && BN254.BaseField.unwrap(a.y0) == BN254.BaseField.unwrap(b.y0)
            && BN254.BaseField.unwrap(a.y1) == BN254.BaseField.unwrap(b.y1);
    }

    /// @notice Add a registration
    function pushToRegistrationQueue() internal virtual override {
        // Either we have a need for a new registration epoch and registrations queue for the
        // current epoch is zero or we have a free slot in the current registration epoch so we
        // append to the registration queue, `numPendingRegistrationsInEpoch`.
        // if the current epoch is max uint64, the registration queue will not be updated and this
        // function will revert

        uint64 curEpoch = lightClient.currentEpoch();
        if (registrationEpoch < curEpoch + 1) {
            // The registration epoch is outdated.
            registrationEpoch = curEpoch + 1;
            numPendingRegistrationsInEpoch = 0;
        } else if (numPendingRegistrationsInEpoch >= maxNumChurnPerEpoch) {
            // The queue for in the current registration epoch is full.
            registrationEpoch += 1;
            numPendingRegistrationsInEpoch = 0;
        } else {
            // We got a free slot in the current registration epoch.
            numPendingRegistrationsInEpoch += 1;
        }
    }

    /// @notice Add an exit
    function pushToExitQueue() internal virtual override {
        // Either we have a need for a new exit epoch and exits queue for the
        // current epoch is zero or we have a free slot in the current exit epoch so we
        // append to the exit queue, `numPendingExitsInEpoch`.
        // if the current epoch is max uint64, the exit queue will not be updated and this function
        // will revert

        uint64 curEpoch = lightClient.currentEpoch();
        if (exitEpoch < curEpoch + 1) {
            // The exit epoch is outdated.
            exitEpoch = curEpoch + 1;
            numPendingExitsInEpoch = 0;
        } else if (numPendingExitsInEpoch >= maxNumChurnPerEpoch) {
            // The queue for in the current exit epoch is full.
            exitEpoch += 1;
            numPendingExitsInEpoch = 0;
        } else {
            // We got a free slot in the current exit epoch.
            numPendingExitsInEpoch += 1;
        }
    }

    /// @notice Look up the balance of `account`
    /// @param account account controlled by the user.
    /// @return Current balance owned by the user.
    function lookupStake(address account) external view virtual override returns (uint256) {
        Node memory node = this.lookupNode(account);
        return node.balance;
    }

    /// @notice Look up the full `Node` state associated with `account`
    /// @return Node indexed by account
    function lookupNode(address account) external view virtual override returns (Node memory) {
        return nodes[account];
    }

    /// @notice Defines the exit escrow period for a node.
    /// TODO discuss Alex, Jeb. How much do we want to specify this function? Also marked as public
    /// for easier testing.
    /// @dev To put this function into context let us consider the following workflow: requestExit
    /// --> (queueing) --> Exited --> (escrow) --> Witdrawable. The first phase is about waiting in
    /// queue due to rate-limiting on exit, the wait is dependent on the exit amount and currently
    /// exit traffic. At the point of "Exited", the node is officially off duty, and stops
    /// participating in consensus.
    ///  The second phase is about slashable security, the wait is dependent only on amount, during
    /// which period cryptographic evidence of misbehavior (e.g. double-voting) might still lead to
    /// the forfeit of stakes. From the point of `Withdrawable` onwards, the staker can freely
    /// withdraw.
    /// @param node node which is assigned an exit escrow period.
    /// @return Number of epochs post exit after which funds can be withdrawn.
    /// TODO modify this according to the current spec
    function exitEscrowPeriod(Node memory node) public pure returns (uint64) {
        if (node.balance > 100) {
            return 10;
        } else {
            return 5;
        }
    }

    /// @notice Register a validator in the stake table, transfer of tokens incurred!
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key (as the auxiliary info)
    /// @param amount The amount to register
    /// @param blsSig The BLS signature that authenticates the ethereum account this function is
    /// called from
    /// @param validUntilEpoch The maximum epoch the sender is willing to wait to be included
    /// (cannot be smaller than the current epoch)
    ///
    /// @dev The function will revert if the sender does not have the correct stake amount.
    /// @dev The function will revert if the sender does not have the correct allowance.
    /// @dev The function will revert if the sender does not have the correct balance.
    /// @dev The function will revert if the sender does not have the correct BLS signature.
    /// `blsSig` field is necessary to prevent "rogue public-key attack".
    /// The signature is over the caller address of the function to ensure that each message is
    /// unique.
    /// @dev No validity check on `schnorrVK`, as it's assumed to be sender's responsibility,
    /// the contract only treat it as auxiliary info submitted by `blsVK`.
    /// @dev The function will revert if the sender does not have the correct registration epoch.
    function register(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        uint256 amount,
        BN254.G1Point memory blsSig,
        uint64 validUntilEpoch
    ) external virtual override {
        if (amount < minStakeAmount) {
            revert InsufficientStakeAmount(amount);
        }

        Node memory node = nodes[msg.sender];

        if (node.account != address(0x0)) {
            revert NodeAlreadyRegistered();
        }

        uint256 allowance = ERC20(tokenAddress).allowance(msg.sender, address(this));
        if (allowance < amount) {
            revert InsufficientAllowance(allowance, amount);
        }

        uint256 balance = ERC20(tokenAddress).balanceOf(msg.sender);
        if (balance < amount) {
            revert InsufficientBalance(balance);
        }

        if (
            _isEqualBlsKey(
                blsVK,
                BN254.G2Point(
                    BN254.BaseField.wrap(0),
                    BN254.BaseField.wrap(0),
                    BN254.BaseField.wrap(0),
                    BN254.BaseField.wrap(0)
                )
            )
        ) {
            revert InvalidBlsVK();
        }

        // Verify that the validator can sign for that blsVK by ensuring that the message that has
        // been signed is the sender's address
        // This is to prevent "rogue public-key attack"
        bytes memory message = abi.encode(msg.sender);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        if (schnorrVK.isEqual(EdOnBN254.EdOnBN254Point(0, 0))) {
            revert InvalidSchnorrVK();
        }

        // Find the earliest epoch at which this node can register. Usually, this will be
        // currentEpoch() + 1 (the start of the next full epoch), but in periods of high churn the
        // queue may fill up and it may be later. If the queue is so full that the wait time exceeds
        // the caller's desired maximum wait, abort.
        pushToRegistrationQueue();
        if (registrationEpoch > validUntilEpoch) {
            revert InvalidNextRegistrationEpoch(registrationEpoch, validUntilEpoch);
        }

        SafeTransferLib.safeTransferFrom(ERC20(tokenAddress), msg.sender, address(this), amount);

        totalStake += amount;

        node.account = msg.sender;
        node.balance = amount;
        node.blsVK = blsVK;
        node.schnorrVK = schnorrVK;
        node.registerEpoch = registrationEpoch;

        nodes[msg.sender] = node;

        emit Registered(msg.sender, registrationEpoch, amount);
    }

    /// @notice Deposit more stakes to registered keys
    /// @dev TODO this implementation will be revisited later. See
    /// https://github.com/EspressoSystems/espresso-sequencer/issues/806
    /// @dev TODO modify this according to the current spec
    /// @param amount The amount to deposit
    /// @return (newBalance, effectiveEpoch) the new balance effective at a future epoch
    function deposit(uint256 amount) external virtual override returns (uint256, uint64) {
        Node memory node = nodes[msg.sender];

        if (node.account == address(0)) {
            revert NodeNotRegistered();
        }

        if (node.account != msg.sender) {
            revert Unauthenticated();
        }

        // A node cannot deposit more tokens while it waiting to register.
        uint64 _curEpoch = lightClient.currentEpoch();
        if (_curEpoch <= node.registerEpoch) {
            revert PrematureDeposit();
        }

        // A node cannot deposit more tokens if an exit request is in progress.
        if (node.exitEpoch != 0) {
            revert ExitRequestInProgress();
        }

        nodes[msg.sender].balance += amount;
        SafeTransferLib.safeTransferFrom(ERC20(tokenAddress), msg.sender, address(this), amount);

        emit Deposit(msg.sender, uint256(amount));

        uint64 effectiveEpoch = _curEpoch + 1;

        return (nodes[msg.sender].balance, effectiveEpoch);
    }

    /// @notice Request to exit from the stake table, not immediately withdrawable!
    ///
    /// @dev TODO modify this according to the current spec
    function requestExit() external virtual override {
        Node memory node = nodes[msg.sender];
        uint64 curEpoch = lightClient.currentEpoch();

        if (node.account == address(0)) {
            revert NodeNotRegistered();
        }

        if (node.account != msg.sender) {
            revert Unauthenticated();
        }

        // Cannot request to exit if an exit request is already in progress.
        if (node.exitEpoch != 0) {
            revert ExitRequestInProgress();
        }

        // Cannot exit before becoming an active participant. Activation happens one epoch after the
        // node's registration epoch, due to the consensus-imposed activation waiting period.
        if (curEpoch < node.registerEpoch + 1) {
            revert PrematureExit();
        }

        // Prepare the node to exit.
        pushToExitQueue();
        nodes[msg.sender].exitEpoch = exitEpoch;

        emit Exit(msg.sender, exitEpoch);
    }

    /// @notice Withdraw from the staking pool. Transfers occur! Only successfully exited keys can
    /// withdraw past their `exitEpoch`. Validators have to first call requestExit to be assigned an
    /// exit epoch
    /// @return The total amount withdrawn, equal to `Node.balance` associated with `blsVK`
    /// TODO: add epoch logic so that we can ensure the node has first requested to exit and waiting
    /// for the exit escrow period to be over
    function withdrawFunds() external virtual override returns (uint256) {
        Node memory node = nodes[msg.sender];
        uint64 curEpoch = lightClient.currentEpoch();

        if (node.account == address(0)) {
            revert NodeNotRegistered();
        }

        if (node.account != msg.sender) {
            revert Unauthenticated();
        }

        uint256 balance = node.balance;
        if (balance == 0) {
            // then there's nothing to withdraw but revert so that they're aware that the withdrawal
            // failed
            revert InsufficientStakeBalance(0);
        }

        // Verify that the exit escrow period is over.
        if (curEpoch < node.exitEpoch + exitEscrowPeriod(node)) {
            revert PrematureWithdrawal();
        }
        totalStake -= balance;

        // Delete the node from the stake table.
        delete nodes[msg.sender];

        // Transfer the balance to the node's account.
        SafeTransferLib.safeTransfer(ERC20(tokenAddress), node.account, balance);

        return balance;
    }

    /// @notice Update the consensus keys for a validator
    /// @dev This function is used to update the consensus keys for a validator
    /// @dev This function can only be called by the validator itself when it's not in the exit
    /// queue
    /// @dev The validator will need to give up either its old BLS key and/or old Schnorr key
    /// @dev The validator will need to provide a BLS signature to prove that the account owns the
    /// new BLS key
    /// @param newBlsVK The new BLS verification key
    /// @param newSchnorrVK The new Schnorr verification key
    /// @param newBlsSig The BLS signature that the account owns the new BLS key
    function updateConsensusKeys(
        BN254.G2Point memory newBlsVK,
        EdOnBN254.EdOnBN254Point memory newSchnorrVK,
        BN254.G1Point memory newBlsSig
    ) external virtual override {
        Node memory node = nodes[msg.sender];

        if (node.account == address(0)) revert NodeNotRegistered();

        // Verify that the node is not in the exit queue
        if (node.exitEpoch != 0) revert ExitRequestInProgress();

        if (_isEqualBlsKey(newBlsVK, node.blsVK) && newSchnorrVK.isEqual(node.schnorrVK)) {
            revert NoKeyChange();
        }

        // Zero-point constants for verification
        BN254.G2Point memory zeroBlsKey = BN254.G2Point(
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0),
            BN254.BaseField.wrap(0)
        );
        EdOnBN254.EdOnBN254Point memory zeroSchnorrKey = EdOnBN254.EdOnBN254Point(0, 0);

        if (_isEqualBlsKey(newBlsVK, zeroBlsKey)) revert InvalidBlsVK();

        if (newSchnorrVK.isEqual(zeroSchnorrKey)) revert InvalidSchnorrVK();

        // Verify that the validator can sign for that newBlsVK, otherwise it inner reverts with
        // BLSSigVerificationFailed
        bytes memory message = abi.encode(msg.sender);
        BLSSig.verifyBlsSig(message, newBlsSig, newBlsVK);

        node.blsVK = newBlsVK;

        node.schnorrVK = newSchnorrVK;

        nodes[msg.sender] = node;

        emit UpdatedConsensusKeys(msg.sender, node.blsVK, node.schnorrVK);
    }

    /// @notice Update the min stake amount
    /// @dev The min stake amount cannot be set to zero
    /// @param _minStakeAmount The new min stake amount
    function updateMinStakeAmount(uint256 _minStakeAmount) external onlyOwner {
        if (_minStakeAmount == 0) revert InvalidValue();
        minStakeAmount = _minStakeAmount;
        emit MinStakeAmountUpdated(minStakeAmount);
    }

    /// @notice Update the max churn rate
    /// @dev The max churn rate cannot be set to zero
    /// @param _maxChurnRate The new max churn rate
    function updateMaxChurnRate(uint64 _maxChurnRate) external onlyOwner {
        if (_maxChurnRate == 0) revert InvalidValue();
        maxNumChurnPerEpoch = _maxChurnRate;
        emit MaxChurnRateUpdated(maxNumChurnPerEpoch);
    }

    /// @notice Update the light client address
    /// @dev The light client address cannot be set to the zero address
    /// @param _lightClientAddress The new light client address
    function updateLightClientAddress(address _lightClientAddress) external onlyOwner {
        if (_lightClientAddress == address(0)) revert InvalidAddress();
        lightClient = LightClient(_lightClientAddress);
        emit LightClientAddressUpdated(_lightClientAddress);
    }
}
