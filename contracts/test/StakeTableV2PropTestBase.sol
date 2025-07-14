// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { console2 } from "forge-std/console2.sol";

// Minimal VM interface that works with foundry and echidna
interface IVM {
    function prank(address) external;
    function startPrank(address) external;
    function stopPrank() external;
    function warp(uint256) external;
}

contract MockERC20 is ERC20 {
    constructor() ERC20("MockToken", "MTK", 18) { }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract MockLightClient is ILightClient {
    function blocksPerEpoch() external pure returns (uint64) {
        return 100;
    }
}

contract StakeTableV2PropTestBase {
    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    IVM public ivm = IVM(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    address[] public actors;
    address[] public allValidators;
    address[] public activeValidators;

    mapping(address validator => uint256 index) public activeValidatorIndex;
    mapping(address validator => bool exists) public activeValidatorMap;

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public trackedTotalSupply;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address account => uint256 balance) public initialBalances;
    mapping(address account => bool exists) public actorMap;

    uint256 public totalActiveDelegations;
    uint256 public totalActiveUndelegations;

    struct ActorFunds {
        uint256 delegations;
        uint256 undelegations;
    }

    mapping(address actor => ActorFunds funds) public trackedActorFunds;

    // Track pending withdrawals for efficient claiming
    struct PendingWithdrawal {
        address actor;
        address validator;
    }

    PendingWithdrawal[] public pendingWithdrawals;
    mapping(bytes32 withdrawalKey => uint256 index) public pendingWithdrawalIndex;
    mapping(bytes32 withdrawalKey => bool exists) public pendingWithdrawalMap;

    // Track validators with delegations for efficient undelegation
    address[] public validatorsWithDelegations;
    mapping(address validator => uint256 index) public validatorsWithDelegationsIndex;
    mapping(address validator => bool exists) public validatorsWithDelegationsMap;

    // Track delegators per validator for efficient undelegation
    mapping(address validator => address[] delegators) public validatorDelegators;
    mapping(address validator => mapping(address delegator => uint256 index)) public
        validatorDelegatorIndex;
    mapping(address validator => mapping(address delegator => bool exists)) public
        validatorDelegatorMap;

    // Track validators that have exited for claim processing
    address[] public exitedValidators;
    mapping(address validator => uint256 index) public exitedValidatorIndex;
    mapping(address validator => bool exists) public exitedValidatorMap;

    // Track delegators for each exited validator
    mapping(address validator => address[] delegators) public exitedValidatorDelegators;
    mapping(address validator => mapping(address delegator => uint256 index)) public
        exitedValidatorDelegatorIndex;
    mapping(address validator => mapping(address delegator => bool exists)) public
        exitedValidatorDelegatorMap;

    // Transaction success counters
    uint256 public countOk_registerValidator;
    uint256 public countOk_deregisterValidator;
    uint256 public countOk_delegate;
    uint256 public countOk_undelegate;
    uint256 public countOk_claimWithdrawal;
    uint256 public countOk_createActor;
    uint256 public countOk_createValidator;
    uint256 public countOk_advanceTime;
    uint256 public countOk_claimValidatorExit;

    address internal validator;
    address internal actor;

    modifier withValidator(uint256 validatorIndex) virtual {
        if (allValidators.length == 0) {
            createValidator(validatorIndex);
        }
        validator = allValidators[validatorIndex % allValidators.length];
        _;
    }

    modifier withActiveValidator(uint256 validatorIndex) virtual {
        if (activeValidators.length == 0) {
            createValidator(validatorIndex);
        }
        validator = activeValidators[validatorIndex % activeValidators.length];
        _;
    }

    modifier useActor(uint256 actorIndex) virtual {
        if (actors.length == 0) {
            createActor(actorIndex);
        }
        actor = actors[actorIndex % actors.length];
        ivm.startPrank(actor);
        _;
        ivm.stopPrank();
    }

    constructor() {
        _deployStakeTable();
        trackedTotalSupply = token.totalSupply();
    }

    function _deployStakeTable() internal {
        address admin = address(this);

        token = new MockERC20();
        lightClient = new MockLightClient();

        // Deploy V1 implementation contract
        StakeTable stakeTableV1Impl = new StakeTable();

        // Encode initialization data for V1
        bytes memory initData = abi.encodeWithSignature(
            "initialize(address,address,uint256,address)",
            address(token),
            address(lightClient),
            EXIT_ESCROW_PERIOD,
            admin
        );

        // Deploy proxy with V1 implementation
        ERC1967Proxy proxy = new ERC1967Proxy(address(stakeTableV1Impl), initData);

        // Deploy V2 implementation contract
        MockStakeTableV2 stakeTableV2Impl = new MockStakeTableV2();

        // Upgrade to V2
        StakeTable(payable(address(proxy))).upgradeToAndCall(
            address(stakeTableV2Impl),
            abi.encodeWithSignature("initializeV2(address,address)", admin, admin)
        );

        // Cast to V2 interface
        stakeTable = MockStakeTableV2(payable(address(proxy)));
    }

    function _genDummyValidatorKeys(address _validator)
        internal
        pure
        returns (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        )
    {
        blsVK = BN254.G2Point({
            x0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "x0")))),
            x1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "x1")))),
            y0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "y0")))),
            y1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "y1"))))
        });

        schnorrVK = EdOnBN254.EdOnBN254Point({
            x: uint256(keccak256(abi.encode(_validator, "schnorr_x"))),
            y: uint256(keccak256(abi.encode(_validator, "schnorr_y")))
        });

        blsSig = BN254.G1Point({
            x: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "sig_x")))),
            y: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "sig_y"))))
        });

        schnorrSig = abi.encode(keccak256(abi.encode(_validator, "schnorr_sig")));
    }

    function totalOwnedAmount(address account) public view returns (uint256) {
        uint256 walletBalance = token.balanceOf(account);
        ActorFunds memory funds = trackedActorFunds[account];
        return walletBalance + funds.delegations + funds.undelegations;
    }

    function _getTotalSupply() internal view returns (uint256 total) {
        total += token.balanceOf(address(stakeTable));
        for (uint256 i = 0; i < actors.length; i++) {
            total += token.balanceOf(actors[i]);
        }
    }

    function _getTotalTrackedFunds() internal view returns (uint256 total) {
        return totalActiveDelegations + totalActiveUndelegations;
    }

    // NOTE: The create validator function is used to generate a new validators successfully.

    function registerValidatorAny(uint256 actorIndex) public useActor(actorIndex) {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(actor);

        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) {
            _addValidator(actor);
            countOk_registerValidator++;
        } catch {
            // Registration failed - this is acceptable for the Any function
        }
    }

    function _newAddress(uint256 seed) internal view returns (address) {
        address candidate = address(uint160(uint256(keccak256(abi.encode(seed)))));

        // If address is already an actor, increment until we find an available one
        while (_isActor(candidate)) {
            candidate = address(uint160(candidate) + 1);
        }

        return candidate;
    }

    function _isActor(address candidate) internal view returns (bool) {
        return actorMap[candidate];
    }

    function _isValidator(address candidate) internal view returns (bool) {
        (, StakeTable.ValidatorStatus status) = stakeTable.validators(candidate);
        return status == StakeTable.ValidatorStatus.Active;
    }

    function _addValidator(address validatorAddress) internal {
        allValidators.push(validatorAddress);

        uint256 newIndex = activeValidators.length;
        activeValidators.push(validatorAddress);
        activeValidatorIndex[validatorAddress] = newIndex;
        activeValidatorMap[validatorAddress] = true;
    }

    function _removeActiveValidator(address validatorAddress) internal {
        if (!activeValidatorMap[validatorAddress]) {
            return; // Validator not active
        }

        uint256 indexToRemove = activeValidatorIndex[validatorAddress];
        uint256 lastIndex = activeValidators.length - 1;

        if (indexToRemove != lastIndex) {
            // Move last element to the position being removed
            address lastValidator = activeValidators[lastIndex];
            activeValidators[indexToRemove] = lastValidator;
            activeValidatorIndex[lastValidator] = indexToRemove;
        }

        // Remove the last element
        activeValidators.pop();
        delete activeValidatorIndex[validatorAddress];
        activeValidatorMap[validatorAddress] = false;
    }

    function deregisterValidatorOk(uint256 validatorIndex) public {
        if (activeValidators.length == 0) {
            return;
        }
        address validatorAddress = activeValidators[validatorIndex % activeValidators.length];

        ivm.prank(validatorAddress);
        stakeTable.deregisterValidator();
        _removeActiveValidator(validatorAddress);
        _addExitedValidator(validatorAddress);
        _removeValidatorFromDelegations(validatorAddress);
        countOk_deregisterValidator++;
    }

    function deregisterValidatorAny(uint256 validatorIndex) public {
        if (allValidators.length == 0) {
            return;
        }
        address validatorAddress = allValidators[validatorIndex % allValidators.length];

        ivm.prank(validatorAddress);
        try stakeTable.deregisterValidator() {
            _removeActiveValidator(validatorAddress);
            _addExitedValidator(validatorAddress);
            _removeValidatorFromDelegations(validatorAddress);
            countOk_deregisterValidator++;
        } catch { }
    }

    function createActor(uint256 seed) public returns (address) {
        address actorAddress = _newAddress(seed);

        // Fund the actor with tokens
        token.mint(actorAddress, INITIAL_BALANCE);
        initialBalances[actorAddress] = INITIAL_BALANCE;
        trackedTotalSupply += INITIAL_BALANCE;

        // Approve stake table to spend tokens
        ivm.prank(actorAddress);
        token.approve(address(stakeTable), type(uint256).max);

        // Add to actors array and map
        actors.push(actorAddress);
        actorMap[actorAddress] = true;
        countOk_createActor++;

        return actorAddress;
    }

    function createValidator(uint256 seed) public returns (address) {
        address validatorAddress = createActor(seed);

        // Register as validator in stake table
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(validatorAddress);

        ivm.prank(validatorAddress);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
        _addValidator(validatorAddress);
        countOk_createValidator++;

        return validatorAddress;
    }

    function delegateOk(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        uint256 balance = token.balanceOf(actor);
        amount = amount % (balance + 1);

        if (amount == 0) return;

        stakeTable.delegate(validator, amount);

        // Update tracking
        totalActiveDelegations += amount;
        trackedActorFunds[actor].delegations += amount;
        _addValidatorWithDelegations(validator);
        _addValidatorDelegator(validator, actor);
        countOk_delegate++;
    }

    function delegateAny(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        try stakeTable.delegate(validator, amount) {
            // Update tracking on success
            totalActiveDelegations += amount;
            trackedActorFunds[actor].delegations += amount;
            _addValidatorWithDelegations(validator);
            _addValidatorDelegator(validator, actor);
            countOk_delegate++;
        } catch {
            // Delegation failed - this is acceptable for the Any function
        }
    }

    function undelegateOk(uint256 actorIndex, uint256 validatorIndex, uint256 amount) public {
        // Use validators with delegations for higher success rate
        if (validatorsWithDelegations.length == 0) return;

        validator = validatorsWithDelegations[validatorIndex % validatorsWithDelegations.length];

        // Pick a delegator from this validator's delegators
        address[] memory delegators = validatorDelegators[validator];
        if (delegators.length == 0) return;

        actor = delegators[actorIndex % delegators.length];

        // Only one undelegation is allowed at a time
        (uint256 existingUndelegation,) = stakeTable.undelegations(validator, actor);
        if (existingUndelegation > 0) return;

        uint256 delegatedAmount = stakeTable.delegations(validator, actor);
        if (delegatedAmount == 0) return;

        amount = amount % (delegatedAmount + 1);

        if (amount == 0) return;

        ivm.prank(actor);
        stakeTable.undelegate(validator, amount);

        // Update tracking
        totalActiveDelegations -= amount;
        totalActiveUndelegations += amount;
        trackedActorFunds[actor].delegations -= amount;
        trackedActorFunds[actor].undelegations += amount;
        _addPendingWithdrawal(actor, validator);

        // Remove delegator from tracking if delegation amount reaches 0
        if (stakeTable.delegations(validator, actor) == 0) {
            _removeValidatorDelegator(validator, actor);
        }

        countOk_undelegate++;
    }

    function undelegateAny(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        try stakeTable.undelegate(validator, amount) {
            // Update tracking on success
            totalActiveDelegations -= amount;
            totalActiveUndelegations += amount;
            trackedActorFunds[actor].delegations -= amount;
            trackedActorFunds[actor].undelegations += amount;
            _addPendingWithdrawal(actor, validator);

            // Remove delegator from tracking if delegation amount reaches 0
            if (stakeTable.delegations(validator, actor) == 0) {
                _removeValidatorDelegator(validator, actor);
            }

            countOk_undelegate++;
        } catch {
            // Undelegation failed - this is acceptable for the Any function
        }
    }

    function _getWithdrawalKey(address actor, address validator) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(actor, validator));
    }

    function _addValidatorWithDelegations(address validator) internal {
        if (validatorsWithDelegationsMap[validator]) return; // Already exists

        uint256 newIndex = validatorsWithDelegations.length;
        validatorsWithDelegations.push(validator);
        validatorsWithDelegationsIndex[validator] = newIndex;
        validatorsWithDelegationsMap[validator] = true;
    }

    function _removeValidatorFromDelegations(address validator) internal {
        if (!validatorsWithDelegationsMap[validator]) return; // Doesn't exist

        uint256 indexToRemove = validatorsWithDelegationsIndex[validator];
        uint256 lastIndex = validatorsWithDelegations.length - 1;

        if (indexToRemove != lastIndex) {
            address lastValidator = validatorsWithDelegations[lastIndex];
            validatorsWithDelegations[indexToRemove] = lastValidator;
            validatorsWithDelegationsIndex[lastValidator] = indexToRemove;
        }

        validatorsWithDelegations.pop();
        delete validatorsWithDelegationsIndex[validator];
        validatorsWithDelegationsMap[validator] = false;

        // Clear all delegators for this validator
        delete validatorDelegators[validator];
    }

    function _addValidatorDelegator(address validator, address delegator) internal {
        if (validatorDelegatorMap[validator][delegator]) return; // Already exists

        uint256 newIndex = validatorDelegators[validator].length;
        validatorDelegators[validator].push(delegator);
        validatorDelegatorIndex[validator][delegator] = newIndex;
        validatorDelegatorMap[validator][delegator] = true;
    }

    function _removeValidatorDelegator(address validator, address delegator) internal {
        if (!validatorDelegatorMap[validator][delegator]) return; // Doesn't exist

        uint256 indexToRemove = validatorDelegatorIndex[validator][delegator];
        uint256 lastIndex = validatorDelegators[validator].length - 1;

        if (indexToRemove != lastIndex) {
            address lastDelegator = validatorDelegators[validator][lastIndex];
            validatorDelegators[validator][indexToRemove] = lastDelegator;
            validatorDelegatorIndex[validator][lastDelegator] = indexToRemove;
        }

        validatorDelegators[validator].pop();
        delete validatorDelegatorIndex[validator][delegator];
        validatorDelegatorMap[validator][delegator] = false;
    }

    function _addExitedValidator(address validator) internal {
        if (exitedValidatorMap[validator]) return; // Already exists

        // Copy current delegators to exit tracking before clearing
        address[] memory delegators = validatorDelegators[validator];
        for (uint256 i = 0; i < delegators.length; i++) {
            _addExitedValidatorDelegator(validator, delegators[i]);
        }

        uint256 newIndex = exitedValidators.length;
        exitedValidators.push(validator);
        exitedValidatorIndex[validator] = newIndex;
        exitedValidatorMap[validator] = true;
    }

    function _addExitedValidatorDelegator(address validator, address delegator) internal {
        if (exitedValidatorDelegatorMap[validator][delegator]) return; // Already exists

        uint256 newIndex = exitedValidatorDelegators[validator].length;
        exitedValidatorDelegators[validator].push(delegator);
        exitedValidatorDelegatorIndex[validator][delegator] = newIndex;
        exitedValidatorDelegatorMap[validator][delegator] = true;
    }

    function _removeExitedValidatorDelegator(address validator, address delegator) internal {
        if (!exitedValidatorDelegatorMap[validator][delegator]) return; // Doesn't exist

        uint256 indexToRemove = exitedValidatorDelegatorIndex[validator][delegator];
        uint256 lastIndex = exitedValidatorDelegators[validator].length - 1;

        if (indexToRemove != lastIndex) {
            address lastDelegator = exitedValidatorDelegators[validator][lastIndex];
            exitedValidatorDelegators[validator][indexToRemove] = lastDelegator;
            exitedValidatorDelegatorIndex[validator][lastDelegator] = indexToRemove;
        }

        exitedValidatorDelegators[validator].pop();
        delete exitedValidatorDelegatorIndex[validator][delegator];
        exitedValidatorDelegatorMap[validator][delegator] = false;
    }

    function _addPendingWithdrawal(address actor, address validator) internal {
        bytes32 key = _getWithdrawalKey(actor, validator);
        if (pendingWithdrawalMap[key]) return; // Already exists

        uint256 newIndex = pendingWithdrawals.length;
        pendingWithdrawals.push(PendingWithdrawal(actor, validator));
        pendingWithdrawalIndex[key] = newIndex;
        pendingWithdrawalMap[key] = true;
    }

    function _removePendingWithdrawal(address actor, address validator) internal {
        bytes32 key = _getWithdrawalKey(actor, validator);
        if (!pendingWithdrawalMap[key]) return; // Doesn't exist

        uint256 indexToRemove = pendingWithdrawalIndex[key];
        uint256 lastIndex = pendingWithdrawals.length - 1;

        if (indexToRemove != lastIndex) {
            // Move last element to the position being removed
            PendingWithdrawal memory lastWithdrawal = pendingWithdrawals[lastIndex];
            pendingWithdrawals[indexToRemove] = lastWithdrawal;
            bytes32 lastKey = _getWithdrawalKey(lastWithdrawal.actor, lastWithdrawal.validator);
            pendingWithdrawalIndex[lastKey] = indexToRemove;
        }

        // Remove the last element
        pendingWithdrawals.pop();
        delete pendingWithdrawalIndex[key];
        pendingWithdrawalMap[key] = false;
    }

    function claimWithdrawalOk(uint256 withdrawalIndex) public {
        if (pendingWithdrawals.length == 0) return;

        PendingWithdrawal memory withdrawal =
            pendingWithdrawals[withdrawalIndex % pendingWithdrawals.length];

        (uint256 undelegationAmount, uint256 unlocksAt) =
            stakeTable.undelegations(withdrawal.validator, withdrawal.actor);

        if (undelegationAmount == 0) return;
        if (block.timestamp < unlocksAt) {
            // Advance time by escrow period to enable withdrawal
            ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);
        }

        ivm.prank(withdrawal.actor);
        stakeTable.claimWithdrawal(withdrawal.validator);

        // Update tracking
        totalActiveUndelegations -= undelegationAmount;
        trackedActorFunds[withdrawal.actor].undelegations -= undelegationAmount;
        _removePendingWithdrawal(withdrawal.actor, withdrawal.validator);
        countOk_claimWithdrawal++;
    }

    function getNumActors() external view returns (uint256) {
        return actors.length;
    }

    function getNumAllValidators() external view returns (uint256) {
        return allValidators.length;
    }

    function getNumActiveValidators() external view returns (uint256) {
        return activeValidators.length;
    }

    function getNumPendingWithdrawals() external view returns (uint256) {
        return pendingWithdrawals.length;
    }

    function getNumValidatorsWithDelegations() external view returns (uint256) {
        return validatorsWithDelegations.length;
    }

    function getNumValidatorDelegators(address validator) external view returns (uint256) {
        return validatorDelegators[validator].length;
    }

    function getNumExitedValidators() external view returns (uint256) {
        return exitedValidators.length;
    }

    function getNumExitedValidatorDelegators(address validator) external view returns (uint256) {
        return exitedValidatorDelegators[validator].length;
    }

    function advanceTime(uint256 seed) public {
        // Advance time by a random amount up to the escrow period
        uint256 timeAdvance = seed % (EXIT_ESCROW_PERIOD + 1);
        if (timeAdvance > 0) {
            ivm.warp(block.timestamp + timeAdvance);
            countOk_advanceTime++;
        }
    }

    function claimValidatorExitOk(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (exitedValidators.length == 0) return;

        validator = exitedValidators[validatorIndex % exitedValidators.length];

        // Pick a delegator from this exited validator's delegators
        address[] memory delegators = exitedValidatorDelegators[validator];
        if (delegators.length == 0) return;

        actor = delegators[delegatorIndex % delegators.length];

        // Check if there's actually a delegation to claim
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);
        if (delegatedAmount == 0) return;

        // Check if validator has actually exited
        uint256 unlocksAt = stakeTable.validatorExits(validator);
        if (unlocksAt == 0) return;

        // Advance time if needed
        if (block.timestamp < unlocksAt) {
            ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);
        }

        ivm.prank(actor);
        stakeTable.claimValidatorExit(validator);

        // Update tracking
        totalActiveDelegations -= delegatedAmount;
        trackedActorFunds[actor].delegations -= delegatedAmount;
        _removeExitedValidatorDelegator(validator, actor);
        countOk_claimValidatorExit++;
    }

    function claimValidatorExitAny(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (exitedValidators.length == 0) return;

        validator = exitedValidators[validatorIndex % exitedValidators.length];

        // Pick a delegator from this exited validator's delegators
        address[] memory delegators = exitedValidatorDelegators[validator];
        if (delegators.length == 0) return;

        actor = delegators[delegatorIndex % delegators.length];

        // Read delegation amount BEFORE claiming (claimValidatorExit clears it)
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        ivm.prank(actor);
        try stakeTable.claimValidatorExit(validator) {
            // Update tracking on success using pre-read amount
            totalActiveDelegations -= delegatedAmount;
            trackedActorFunds[actor].delegations -= delegatedAmount;
            _removeExitedValidatorDelegator(validator, actor);
            countOk_claimValidatorExit++;
        } catch {
            // Claim failed - this is acceptable for the Any function
        }
    }
}
