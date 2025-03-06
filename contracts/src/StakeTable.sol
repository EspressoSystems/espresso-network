pragma solidity ^0.8.0;

import { SafeTransferLib, ERC20 } from "solmate/utils/SafeTransferLib.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";
import { AbstractStakeTable } from "./interfaces/AbstractStakeTable.sol";
import { LightClient } from "../src/LightClient.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { InitializedAt } from "./InitializedAt.sol";

using EdOnBN254 for EdOnBN254.EdOnBN254Point;

/// @title Implementation of the Stake Table interface
contract StakeTable is AbstractStakeTable, Ownable, InitializedAt {
    /// Error raised when a user tries to register a validator with the same address
    error ValidatorAlreadyRegistered();

    /// Error raised when a user tries to interact with a validator that isn't registered
    error UnknownValidator();

    /// Error raised when a validator has already exited.
    error ValidatorAlreadyExited();

    /// Error raised when a validator has not exited yet.
    error ValidatorNotExited();

    // Error raised when a user tries to withdraw funds before the exit escrow period is over.
    error PrematureWithdrawal();

    // Error raised when this contract does not have the sufficient allowance on the stake ERC20
    // token
    error InsufficientAllowance(uint256, uint256);

    // Error raised when the staker does not have the sufficient balance on the stake ERC20 token
    error InsufficientBalance(uint256);

    // Error raised when the staker does not have the sufficient stake balance to withdraw
    error NothingToWithdraw();

    // Error raised when the staker provides a zero SchnorrVK
    error InvalidSchnorrVK();

    /// The commission is invalid
    error InvalidCommission();

    struct Validator {
        bool isRegistered;
        ValidatorStatus status;
        uint256 delegatedAmount;
    }

    struct Undelegation {
        uint256 amount;
        uint256 unlocksAt;
    }

    enum ValidatorStatus {
        Active,
        Exited
    }

    /// Currently active validators
    mapping(address validator => Validator) public validators;

    /// Validators that have exited
    mapping(address validator => uint256 unlocksAt) public validatorExits;

    /// Currently active delegations
    mapping(address validator => mapping(address delegator => uint256 amount)) delegations;

    /// Currently exiting delegations
    //
    // @dev these are stored indexed by validator so we can keep track of them
    // for slashing later
    mapping(address validator => mapping(address delegator => Undelegation)) undelegations;

    /// Address of the native token contract.
    address public tokenAddress;

    /// The time the contract will hold funds after undelegations are requested.
    ///
    /// Must allow ample time for node to exit active validator set and slashing
    /// evidence to be submitted.
    uint256 public exitEscrowPeriod;

    address public admin;

    /// TODO change constructor to initialize function when we make the contract upgradeable
    constructor(address _tokenAddress, uint256 _exitEscrowPeriod, address _initialOwner)
        Ownable(_initialOwner)
        InitializedAt()
    {
        tokenAddress = _tokenAddress;
        exitEscrowPeriod = _exitEscrowPeriod;
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

    function ensureValidatorRegistered(address validator) internal view {
        if (!validators[validator].isRegistered) {
            revert UnknownValidator();
        }
    }

    function ensureValidatorNotRegistered(address validator) internal view {
        if (validators[validator].isRegistered) {
            revert ValidatorAlreadyRegistered();
        }
    }

    function ensureValidatorNotExited(address validator) internal view {
        if (validatorExits[validator] != 0) {
            revert ValidatorAlreadyExited();
        }
    }

    // @dev We don't check the validity of the schnorr verifying key but providing a zero key is
    // definitely a mistake by the caller, therefore we revert.
    function ensureNonZeroSchnorrKey(EdOnBN254.EdOnBN254Point memory schnorrVK) internal pure {
        EdOnBN254.EdOnBN254Point memory zeroSchnorrKey = EdOnBN254.EdOnBN254Point(0, 0);

        if (schnorrVK.isEqual(zeroSchnorrKey)) {
            revert InvalidSchnorrVK();
        }
    }

    /// @notice Register a validator in the stake table
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key (as the auxiliary info)
    /// @param blsSig The BLS signature that authenticates the ethereum account this function is
    ///        called from
    ///
    /// @dev The function will revert if
    ///      - if the validator is already registered
    ///      - any of the keys are zero
    ///      - if the bls signature verification fails (this prevents rogue public-key attacks)
    ///
    /// @dev No validity check on `schnorrVK`, as it's assumed to be sender's responsibility,
    function registerValidator(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        uint16 commission
    ) external virtual override {
        ensureValidatorNotRegistered(msg.sender);
        ensureNonZeroSchnorrKey(schnorrVK);

        // Verify that the validator can sign for that blsVK.
        // This prevents rogue public-key attacks.
        bytes memory message = abi.encode(msg.sender);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        // commission is in percent with 2 decimals: from 0 for 0.00% to 10_000 for 100%
        if (commission > 10000) {
            revert InvalidCommission();
        }

        validators[msg.sender] =
            Validator({ isRegistered: true, status: ValidatorStatus.Active, delegatedAmount: 0 });

        emit ValidatorRegistered(msg.sender, blsVK, schnorrVK, commission);
    }

    /// @notice Delegate to a validator
    /// @param validator The validator to delegate to
    /// @param amount The amount to delegate
    function delegate(address validator, uint256 amount) external virtual override {
        ensureValidatorRegistered(validator);
        ensureValidatorNotExited(validator);

        uint256 allowance = ERC20(tokenAddress).allowance(msg.sender, address(this));
        if (allowance < amount) {
            revert InsufficientAllowance(allowance, amount);
        }

        validators[validator].delegatedAmount += amount;

        SafeTransferLib.safeTransferFrom(ERC20(tokenAddress), msg.sender, address(this), amount);

        emit Delegated(msg.sender, validator, amount);
    }

    function undelegate(address validator, uint256 amount) external virtual override {
        ensureValidatorRegistered(validator);
        ensureValidatorNotExited(validator);

        if (validators[msg.sender].status == ValidatorStatus.Exited) {
            revert ValidatorAlreadyExited();
        }

        uint256 balance = delegations[validator][msg.sender];
        if (balance < amount) {
            revert InsufficientBalance(amount);
        }

        delegations[validator][msg.sender] -= amount;
        undelegations[validator][msg.sender] =
            Undelegation({ amount: amount, unlocksAt: block.timestamp + exitEscrowPeriod });

        emit Undelegated(msg.sender, validator, amount);
    }

    function deregisterValidator() external virtual override {
        ensureValidatorRegistered(msg.sender);
        ensureValidatorNotExited(msg.sender);

        validators[msg.sender].status = ValidatorStatus.Exited;
        validatorExits[msg.sender] = block.timestamp + exitEscrowPeriod;

        emit ValidatorExit(msg.sender);
    }

    /// @notice Withdraw undelegated funds
    function claimWithdrawal(address validator) external virtual override {
        if (block.timestamp < undelegations[validator][msg.sender].unlocksAt) {
            revert PrematureWithdrawal();
        }

        uint256 amount = undelegations[validator][msg.sender].amount;
        if (amount == 0) {
            revert NothingToWithdraw();
        }

        // Mark funds as spent
        delete undelegations[validator][msg.sender];

        SafeTransferLib.safeTransfer(ERC20(tokenAddress), msg.sender, amount);

        emit Withdrawal(msg.sender, amount);
    }

    /// @notice Withdraw funds after a validator has exited
    function claimValidatorExit(address validator) external virtual override {
        uint256 unlocksAt = validatorExits[msg.sender];
        if (unlocksAt == 0) {
            revert ValidatorNotExited();
        }

        if (block.timestamp < unlocksAt) {
            revert PrematureWithdrawal();
        }

        uint256 amount = delegations[validator][msg.sender];
        if (amount == 0) {
            revert NothingToWithdraw();
        }

        // Mark funds as spent
        delegations[validator][msg.sender] = 0;

        SafeTransferLib.safeTransfer(ERC20(tokenAddress), msg.sender, amount);

        emit Withdrawal(msg.sender, amount);
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
        ensureValidatorRegistered(msg.sender);
        ensureValidatorNotExited(msg.sender);
        ensureNonZeroSchnorrKey(newSchnorrVK);

        // Verify that the validator can sign for that newBlsVK, otherwise it
        // inner reverts with BLSSigVerificationFailed
        bytes memory message = abi.encode(msg.sender);
        BLSSig.verifyBlsSig(message, newBlsSig, newBlsVK);

        emit ConsensusKeysUpdated(msg.sender, newBlsVK, newSchnorrVK);
    }
}
