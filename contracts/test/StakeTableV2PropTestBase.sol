pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { console2 } from "forge-std/console2.sol";
import { EnumerableSet } from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import { EnumerableMap } from "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";

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

contract FunctionCallTracking {
    // Structured function call tracking
    struct FunctionStats {
        uint256 successes;
        uint256 reverts;
    }

    // Split into smaller structs to avoid stack too deep
    struct OkFunctionStats {
        FunctionStats delegateOk;
        FunctionStats undelegateOk;
        FunctionStats deregisterValidatorOk;
        FunctionStats claimWithdrawalOk;
        FunctionStats claimValidatorExitOk;
        FunctionStats createActor;
        FunctionStats createValidator;
        FunctionStats advanceTime;
    }

    struct AnyFunctionStats {
        FunctionStats registerValidatorAny;
        FunctionStats delegateAny;
        FunctionStats undelegateAny;
        FunctionStats deregisterValidatorAny;
        FunctionStats claimValidatorExitAny;
    }

    OkFunctionStats public okFunctionStats;
    AnyFunctionStats public anyFunctionStats;

    function getOkStats() external view returns (OkFunctionStats memory) {
        return okFunctionStats;
    }

    function getAnyStats() external view returns (AnyFunctionStats memory) {
        return anyFunctionStats;
    }
}

contract StakeTableV2PropTestBase is FunctionCallTracking {
    using EnumerableSet for EnumerableSet.AddressSet;
    using EnumerableSet for EnumerableSet.Bytes32Set;

    struct ActorFunds {
        uint256 delegated;
        uint256 undelegated;
    }

    struct Validators {
        EnumerableSet.AddressSet all; // validators
        EnumerableSet.AddressSet active; // validatorsActive
        EnumerableSet.AddressSet exited; // validatorsExited
        EnumerableSet.AddressSet staked; // validatorsStaked (has delegations)
        EnumerableSet.AddressSet withPendingWithdrawals; // validatorsWithPendingWithdrawals
    }

    struct Delegators {
        mapping(address validator => EnumerableSet.AddressSet delegators) delegators;
        mapping(address validator => EnumerableSet.AddressSet actors) pendingWithdrawals;
    }

    struct TestState {
        uint256 trackedTotalSupply;
        uint256 totalDelegated;
        uint256 totalUndelegated;
    }

    struct Actors {
        EnumerableSet.AddressSet all; // actors
        mapping(address actor => uint256 balance) initialBalances;
        mapping(address actor => ActorFunds funds) trackedFunds;
    }

    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    IVM public ivm = IVM(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    // Organized state tracking
    Validators internal validators;
    Delegators internal delegators;
    TestState public testState;
    Actors internal actors;

    // For current validator and actor modifiers
    address internal validator;
    address internal actor;

    // Like foundry's `bound`, but usable from echidna and forge
    function boundRange(uint256 x, uint256 min, uint256 max) public pure returns (uint256 result) {
        require(min <= max, "boundRange: min > max");
        if (max == min) return min;

        // If x is already in bounds, return it
        if (x >= min && x <= max) return x;

        // Otherwise, bound it within the range
        uint256 range = max - min + 1;
        return min + (x % range);
    }

    modifier withValidator(uint256 validatorIndex) virtual {
        if (validators.all.length() == 0) {
            createValidator(validatorIndex);
        }
        validator = validators.all.at(validatorIndex % validators.all.length());
        _;
    }

    modifier withActiveValidator(uint256 validatorIndex) virtual {
        if (validators.active.length() == 0) {
            createValidator(validatorIndex);
        }
        validator = validators.active.at(validatorIndex % validators.active.length());
        _;
    }

    modifier useActor(uint256 actorIndex) virtual {
        if (actors.all.length() == 0) {
            createActor(actorIndex);
        }
        actor = actors.all.at(actorIndex % actors.all.length());
        ivm.startPrank(actor);
        _;
        ivm.stopPrank();
    }

    constructor() {
        _deployStakeTable();
        testState.trackedTotalSupply = token.totalSupply();
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
        ActorFunds memory funds = actors.trackedFunds[account];
        return walletBalance + funds.delegated + funds.undelegated;
    }

    function _getTotalSupply() internal view returns (uint256 total) {
        total += token.balanceOf(address(stakeTable));
        for (uint256 i = 0; i < actors.all.length(); i++) {
            total += token.balanceOf(actors.all.at(i));
        }
    }

    function _getTotalTrackedFunds() internal view returns (uint256 total) {
        return testState.totalDelegated + testState.totalUndelegated;
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
            anyFunctionStats.registerValidatorAny.successes++;
        } catch {
            // Registration failed - this is acceptable for the Any function
            anyFunctionStats.registerValidatorAny.reverts++;
        }
    }

    function _newAddress(uint256 seed) internal view returns (address) {
        address candidate = address(uint160(uint256(keccak256(abi.encode(seed)))));

        // If address is already an actor, increment until we find an available one
        while (actors.all.contains(candidate)) {
            candidate = address(uint160(candidate) + 1);
        }

        return candidate;
    }

    function _isValidator(address candidate) internal view returns (bool) {
        (, StakeTable.ValidatorStatus status) = stakeTable.validators(candidate);
        return status == StakeTable.ValidatorStatus.Active;
    }

    function _addValidator(address val) internal {
        validators.all.add(val);
        validators.active.add(val);
    }

    function _removeActiveValidator(address val) internal {
        validators.active.remove(val);
    }

    function deregisterValidatorOk(uint256 validatorIndex) public {
        if (validators.active.length() == 0) {
            return;
        }
        address val = validators.active.at(validatorIndex % validators.active.length());

        ivm.prank(val);
        stakeTable.deregisterValidator();
        _deregisterValidator(val);
        okFunctionStats.deregisterValidatorOk.successes++;
    }

    function deregisterValidatorAny(uint256 validatorIndex) public {
        if (validators.all.length() == 0) {
            return;
        }
        address val = validators.all.at(validatorIndex % validators.all.length());

        ivm.prank(val);
        try stakeTable.deregisterValidator() {
            _deregisterValidator(val);
            anyFunctionStats.deregisterValidatorAny.successes++;
        } catch {
            anyFunctionStats.deregisterValidatorAny.reverts++;
        }
    }

    function createActor(uint256 seed) public returns (address) {
        address newActor = _newAddress(seed);

        // Fund the actor with tokens
        token.mint(newActor, INITIAL_BALANCE);
        actors.initialBalances[newActor] = INITIAL_BALANCE;
        testState.trackedTotalSupply += INITIAL_BALANCE;

        // Approve stake table to spend tokens
        ivm.prank(newActor);
        token.approve(address(stakeTable), type(uint256).max);

        // Add to actors array and map
        actors.all.add(newActor);
        okFunctionStats.createActor.successes++;

        return newActor;
    }

    function createValidator(uint256 seed) public returns (address) {
        address val = createActor(seed);

        // Register as validator in stake table
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(val);

        ivm.prank(val);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
        _addValidator(val);
        okFunctionStats.createValidator.successes++;

        return val;
    }

    function delegateOk(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        uint256 balance = token.balanceOf(actor);
        if (balance == 0) return;

        amount = boundRange(amount, 1, balance);

        stakeTable.delegate(validator, amount);

        // Update tracking
        _trackDelegation(actor, validator, amount);
        okFunctionStats.delegateOk.successes++;
    }

    function delegateAny(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        try stakeTable.delegate(validator, amount) {
            // Update tracking on success
            _trackDelegation(actor, validator, amount);
            anyFunctionStats.delegateAny.successes++;
        } catch {
            // Delegation failed - this is acceptable for the Any function
            anyFunctionStats.delegateAny.reverts++;
        }
    }

    function undelegateOk(uint256 actorIndex, uint256 validatorIndex, uint256 amount) public {
        // Use validators with delegations for higher success rate
        if (validators.staked.length() == 0) return;

        validator = validators.staked.at(validatorIndex % validators.staked.length());

        // Pick a delegator from this validator's delegators
        EnumerableSet.AddressSet storage validatorDelegators = delegators.delegators[validator];
        if (validatorDelegators.length() == 0) return;

        actor = validatorDelegators.at(actorIndex % validatorDelegators.length());

        // Only one undelegation is allowed at a time
        (uint256 existingUndelegation,) = stakeTable.undelegations(validator, actor);
        if (existingUndelegation > 0) return;

        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        amount = boundRange(amount, 1, delegatedAmount);

        ivm.prank(actor);
        stakeTable.undelegate(validator, amount);

        // Update tracking
        _trackUndelegation(actor, validator, amount);
        okFunctionStats.undelegateOk.successes++;
    }

    function undelegateAny(uint256 actorIndex, uint256 validatorIndex, uint256 amount)
        public
        withActiveValidator(validatorIndex)
        useActor(actorIndex)
    {
        try stakeTable.undelegate(validator, amount) {
            // Update tracking on success
            _trackUndelegation(actor, validator, amount);
            anyFunctionStats.undelegateAny.successes++;
        } catch {
            // Undelegation failed - this is acceptable for the Any function
            anyFunctionStats.undelegateAny.reverts++;
        }
    }

    function _deregisterValidator(address val) internal {
        validators.active.remove(val);
        validators.exited.add(val);
        validators.staked.remove(val);
    }

    function _trackDelegation(address actorAddr, address val, uint256 amount) internal {
        testState.totalDelegated += amount;
        actors.trackedFunds[actorAddr].delegated += amount;
        validators.staked.add(val);
        delegators.delegators[val].add(actorAddr);
    }

    function _trackUndelegation(address actorAddr, address val, uint256 amount) internal {
        testState.totalDelegated -= amount;
        testState.totalUndelegated += amount;
        actors.trackedFunds[actorAddr].delegated -= amount;
        actors.trackedFunds[actorAddr].undelegated += amount;
        _addPendingWithdrawal(actorAddr, val);

        // Remove delegator from tracking if delegation amount reaches 0
        if (stakeTable.delegations(val, actorAddr) == 0) {
            _removeValidatorDelegator(val, actorAddr);
        }
    }

    function _addValidatorDelegator(address val, address del) internal {
        validators.staked.add(val);
        delegators.delegators[val].add(del);
    }

    function _removeValidatorDelegator(address val, address del) internal {
        delegators.delegators[val].remove(del);
        if (delegators.delegators[val].length() == 0) {
            validators.staked.remove(val);
        }
    }

    function _addExitedValidator(address val) internal {
        if (validators.exited.contains(val)) return; // Already exists
        validators.exited.add(val);
    }

    function _addPendingWithdrawal(address actorAddr, address val) internal {
        if (delegators.pendingWithdrawals[val].contains(actorAddr)) return; // Already exists

        delegators.pendingWithdrawals[val].add(actorAddr);
        validators.withPendingWithdrawals.add(val);
    }

    function _removePendingWithdrawal(address actorAddr, address val) internal {
        if (!delegators.pendingWithdrawals[val].contains(actorAddr)) return; // Doesn't exist

        delegators.pendingWithdrawals[val].remove(actorAddr);
        if (delegators.pendingWithdrawals[val].length() == 0) {
            validators.withPendingWithdrawals.remove(val);
        }
    }

    function claimWithdrawalOk(uint256 withdrawalIndex) public {
        if (validators.withPendingWithdrawals.length() == 0) return;

        // Pick a validator with pending withdrawals
        address val = validators.withPendingWithdrawals.at(
            withdrawalIndex % validators.withPendingWithdrawals.length()
        );

        // Pick an actor with pending withdrawal for this validator
        EnumerableSet.AddressSet storage pendingActors = delegators.pendingWithdrawals[val];
        if (pendingActors.length() == 0) return;

        address pendingActor = pendingActors.at(withdrawalIndex % pendingActors.length());

        (uint256 undelegationAmount, uint256 unlocksAt) =
            stakeTable.undelegations(val, pendingActor);

        if (undelegationAmount == 0) return;
        if (block.timestamp < unlocksAt) {
            // Advance time by escrow period to enable withdrawal
            ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);
        }

        ivm.prank(pendingActor);
        stakeTable.claimWithdrawal(val);

        // Update tracking
        testState.totalUndelegated -= undelegationAmount;
        actors.trackedFunds[pendingActor].undelegated -= undelegationAmount;
        _removePendingWithdrawal(pendingActor, val);
        okFunctionStats.claimWithdrawalOk.successes++;
    }

    function getNumActors() external view returns (uint256) {
        return actors.all.length();
    }

    function getNumAllValidators() external view returns (uint256) {
        return validators.all.length();
    }

    function getNumActiveValidators() external view returns (uint256) {
        return validators.active.length();
    }

    function getNumPendingWithdrawals() external view returns (uint256) {
        uint256 total = 0;
        for (uint256 i = 0; i < validators.withPendingWithdrawals.length(); i++) {
            total += delegators.pendingWithdrawals[validators.withPendingWithdrawals.at(i)].length();
        }
        return total;
    }

    function getPendingWithdrawalAtIndex(uint256 index) external view returns (address, address) {
        uint256 totalPendingWithdrawals = this.getNumPendingWithdrawals();
        require(index < totalPendingWithdrawals, "Index out of bounds");

        uint256 currentIndex = 0;
        for (uint256 i = 0; i < validators.withPendingWithdrawals.length(); i++) {
            address val = validators.withPendingWithdrawals.at(i);
            EnumerableSet.AddressSet storage pendingActors = delegators.pendingWithdrawals[val];

            if (currentIndex + pendingActors.length() > index) {
                // The target index is within this validator's actors
                address pendingActor = pendingActors.at(index - currentIndex);
                return (pendingActor, val);
            }
            currentIndex += pendingActors.length();
        }

        revert("Index out of bounds");
    }

    function getNumValidatorsWithDelegations() external view returns (uint256) {
        return validators.staked.length();
    }

    function getNumValidatorDelegators(address val) external view returns (uint256) {
        return delegators.delegators[val].length();
    }

    function getNumExitedValidators() external view returns (uint256) {
        return validators.exited.length();
    }

    function getNumExitedValidatorDelegators(address val) external view returns (uint256) {
        // Since we no longer track exitedValidatorDelegators separately,
        // we'll return 0 to indicate no specific tracking
        return 0;
    }

    function getActorAtIndex(uint256 index) external view returns (address) {
        return actors.all.at(index);
    }

    function getValidatorWithDelegationsAtIndex(uint256 index)
        external
        view
        returns (address, uint256)
    {
        address val = validators.staked.at(index);
        return (val, delegators.delegators[val].length());
    }

    function getExitedValidatorAtIndex(uint256 index) external view returns (address) {
        return validators.exited.at(index);
    }

    function getInitialBalance(address actorAddr) external view returns (uint256) {
        return actors.initialBalances[actorAddr];
    }

    function getTestState() external view returns (TestState memory) {
        return testState;
    }

    function getTotalSupply() external view returns (uint256) {
        return _getTotalSupply();
    }

    function getTotalSuccesses() external view returns (uint256) {
        uint256 total = 0;
        // Ok functions
        total += okFunctionStats.delegateOk.successes;
        total += okFunctionStats.undelegateOk.successes;
        total += okFunctionStats.deregisterValidatorOk.successes;
        total += okFunctionStats.claimWithdrawalOk.successes;
        total += okFunctionStats.claimValidatorExitOk.successes;
        total += okFunctionStats.createActor.successes;
        total += okFunctionStats.createValidator.successes;
        total += okFunctionStats.advanceTime.successes;
        // Any functions
        total += anyFunctionStats.registerValidatorAny.successes;
        total += anyFunctionStats.delegateAny.successes;
        total += anyFunctionStats.undelegateAny.successes;
        total += anyFunctionStats.deregisterValidatorAny.successes;
        total += anyFunctionStats.claimValidatorExitAny.successes;
        return total;
    }

    function getTotalReverts() external view returns (uint256) {
        uint256 total = 0;
        total += anyFunctionStats.registerValidatorAny.reverts;
        total += anyFunctionStats.delegateAny.reverts;
        total += anyFunctionStats.undelegateAny.reverts;
        total += anyFunctionStats.deregisterValidatorAny.reverts;
        total += anyFunctionStats.claimValidatorExitAny.reverts;
        return total;
    }

    function getTotalCalls() external view returns (uint256) {
        return this.getTotalSuccesses() + this.getTotalReverts();
    }

    function advanceTime(uint256 seed) public {
        // Advance time by a random amount up to the escrow period
        uint256 timeAdvance = boundRange(seed, 1, EXIT_ESCROW_PERIOD);
        ivm.warp(block.timestamp + timeAdvance);
        okFunctionStats.advanceTime.successes++;
    }

    function claimValidatorExitOk(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (validators.exited.length() == 0) return;

        validator = validators.exited.at(validatorIndex % validators.exited.length());

        // Check if validator has actually exited
        uint256 unlocksAt = stakeTable.validatorExits(validator);
        if (unlocksAt == 0) return;

        // Use actors set to pick a delegator - we'll try to find one with a delegation
        if (actors.all.length() == 0) return;

        actor = actors.all.at(delegatorIndex % actors.all.length());

        // Check if there's actually a delegation to claim
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);
        if (delegatedAmount == 0) return;

        // Advance time if needed
        if (block.timestamp < unlocksAt) {
            ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);
        }

        ivm.prank(actor);
        stakeTable.claimValidatorExit(validator);

        // Update tracking
        testState.totalDelegated -= delegatedAmount;
        actors.trackedFunds[actor].delegated -= delegatedAmount;
        okFunctionStats.claimValidatorExitOk.successes++;
    }

    function claimValidatorExitAny(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (validators.exited.length() == 0) return;

        validator = validators.exited.at(validatorIndex % validators.exited.length());

        // Use actors set to pick a delegator
        if (actors.all.length() == 0) return;

        actor = actors.all.at(delegatorIndex % actors.all.length());

        // Read delegation amount BEFORE claiming (claimValidatorExit clears it)
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        ivm.prank(actor);
        try stakeTable.claimValidatorExit(validator) {
            // Update tracking on success using pre-read amount
            testState.totalDelegated -= delegatedAmount;
            actors.trackedFunds[actor].delegated -= delegatedAmount;
            anyFunctionStats.claimValidatorExitAny.successes++;
        } catch {
            // Claim failed - this is acceptable for the Any function
            anyFunctionStats.claimValidatorExitAny.reverts++;
        }
    }
}
