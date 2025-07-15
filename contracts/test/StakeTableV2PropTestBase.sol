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
}

contract StakeTableV2PropTestBase is FunctionCallTracking {
    using EnumerableSet for EnumerableSet.AddressSet;
    using EnumerableSet for EnumerableSet.Bytes32Set;

    struct ActorFunds {
        uint256 delegations;
        uint256 undelegations;
    }

    struct PendingWithdrawal {
        address actor;
        address validator;
    }

    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    IVM public ivm = IVM(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    // State tracking
    uint256 public trackedTotalSupply;
    uint256 public totalActiveDelegations;
    uint256 public totalActiveUndelegations;

    EnumerableSet.AddressSet internal actors;
    EnumerableSet.AddressSet internal allValidators;
    EnumerableSet.AddressSet internal activeValidators;
    EnumerableSet.AddressSet internal exitedValidators;
    EnumerableSet.Bytes32Set internal pendingWithdrawalKeys;
    EnumerableSet.AddressSet internal validatorsWithDelegations;

    mapping(address account => uint256 balance) public initialBalances;
    mapping(bytes32 withdrawalKey => PendingWithdrawal) public pendingWithdrawals;
    mapping(address => EnumerableSet.AddressSet) internal validatorDelegators;
    mapping(address => EnumerableSet.AddressSet) internal exitedValidatorDelegators;
    mapping(address actor => ActorFunds funds) public trackedActorFunds;

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
        if (allValidators.length() == 0) {
            createValidator(validatorIndex);
        }
        validator = allValidators.at(validatorIndex % allValidators.length());
        _;
    }

    modifier withActiveValidator(uint256 validatorIndex) virtual {
        if (activeValidators.length() == 0) {
            createValidator(validatorIndex);
        }
        validator = activeValidators.at(validatorIndex % activeValidators.length());
        _;
    }

    modifier useActor(uint256 actorIndex) virtual {
        if (actors.length() == 0) {
            createActor(actorIndex);
        }
        actor = actors.at(actorIndex % actors.length());
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
        for (uint256 i = 0; i < actors.length(); i++) {
            total += token.balanceOf(actors.at(i));
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
            anyFunctionStats.registerValidatorAny.successes++;
        } catch {
            // Registration failed - this is acceptable for the Any function
            anyFunctionStats.registerValidatorAny.reverts++;
        }
    }

    function _newAddress(uint256 seed) internal view returns (address) {
        address candidate = address(uint160(uint256(keccak256(abi.encode(seed)))));

        // If address is already an actor, increment until we find an available one
        while (actors.contains(candidate)) {
            candidate = address(uint160(candidate) + 1);
        }

        return candidate;
    }

    function _isValidator(address candidate) internal view returns (bool) {
        (, StakeTable.ValidatorStatus status) = stakeTable.validators(candidate);
        return status == StakeTable.ValidatorStatus.Active;
    }

    function _addValidator(address validatorAddress) internal {
        allValidators.add(validatorAddress);
        activeValidators.add(validatorAddress);
    }

    function _removeActiveValidator(address validatorAddress) internal {
        activeValidators.remove(validatorAddress);
    }

    function deregisterValidatorOk(uint256 validatorIndex) public {
        if (activeValidators.length() == 0) {
            return;
        }
        address validatorAddress = activeValidators.at(validatorIndex % activeValidators.length());

        ivm.prank(validatorAddress);
        stakeTable.deregisterValidator();
        _removeActiveValidator(validatorAddress);
        _addExitedValidator(validatorAddress);
        _removeValidatorFromDelegations(validatorAddress);
        okFunctionStats.deregisterValidatorOk.successes++;
    }

    function deregisterValidatorAny(uint256 validatorIndex) public {
        if (allValidators.length() == 0) {
            return;
        }
        address validatorAddress = allValidators.at(validatorIndex % allValidators.length());

        ivm.prank(validatorAddress);
        try stakeTable.deregisterValidator() {
            _removeActiveValidator(validatorAddress);
            _addExitedValidator(validatorAddress);
            _removeValidatorFromDelegations(validatorAddress);
            anyFunctionStats.deregisterValidatorAny.successes++;
        } catch {
            anyFunctionStats.deregisterValidatorAny.reverts++;
        }
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
        actors.add(actorAddress);
        okFunctionStats.createActor.successes++;

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
        okFunctionStats.createValidator.successes++;

        return validatorAddress;
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
        totalActiveDelegations += amount;
        trackedActorFunds[actor].delegations += amount;
        _addValidatorDelegator(validator, actor);
        okFunctionStats.delegateOk.successes++;
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
            _addValidatorDelegator(validator, actor);
            anyFunctionStats.delegateAny.successes++;
        } catch {
            // Delegation failed - this is acceptable for the Any function
            anyFunctionStats.delegateAny.reverts++;
        }
    }

    function undelegateOk(uint256 actorIndex, uint256 validatorIndex, uint256 amount) public {
        // Use validators with delegations for higher success rate
        if (validatorsWithDelegations.length() == 0) return;

        validator =
            validatorsWithDelegations.at(validatorIndex % validatorsWithDelegations.length());

        // Pick a delegator from this validator's delegators
        EnumerableSet.AddressSet storage delegators = validatorDelegators[validator];
        if (delegators.length() == 0) return;

        actor = delegators.at(actorIndex % delegators.length());

        // Only one undelegation is allowed at a time
        (uint256 existingUndelegation,) = stakeTable.undelegations(validator, actor);
        if (existingUndelegation > 0) return;

        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        amount = boundRange(amount, 1, delegatedAmount);

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

        okFunctionStats.undelegateOk.successes++;
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

            anyFunctionStats.undelegateAny.successes++;
        } catch {
            // Undelegation failed - this is acceptable for the Any function
            anyFunctionStats.undelegateAny.reverts++;
        }
    }

    function _getWithdrawalKey(address _actor, address _validator)
        internal
        pure
        returns (bytes32)
    {
        return keccak256(abi.encodePacked(_actor, _validator));
    }

    function _removeValidatorFromDelegations(address _validator) internal {
        validatorsWithDelegations.remove(_validator);
    }

    function _addValidatorDelegator(address _validator, address _delegator) internal {
        validatorsWithDelegations.add(_validator);
        validatorDelegators[_validator].add(_delegator);
    }

    function _removeValidatorDelegator(address _validator, address _delegator) internal {
        validatorDelegators[_validator].remove(_delegator);
        if (validatorDelegators[_validator].length() == 0) {
            validatorsWithDelegations.remove(_validator);
        }
    }

    function _addExitedValidator(address _validator) internal {
        if (exitedValidators.contains(_validator)) return; // Already exists

        // Copy current delegators to exit tracking before clearing
        EnumerableSet.AddressSet storage delegators = validatorDelegators[_validator];
        for (uint256 i = 0; i < delegators.length(); i++) {
            _addExitedValidatorDelegator(_validator, delegators.at(i));
        }

        exitedValidators.add(_validator);
    }

    function _addExitedValidatorDelegator(address _validator, address _delegator) internal {
        exitedValidatorDelegators[_validator].add(_delegator);
    }

    function _removeExitedValidatorDelegator(address _validator, address _delegator) internal {
        exitedValidatorDelegators[_validator].remove(_delegator);
    }

    function _addPendingWithdrawal(address _actor, address _validator) internal {
        bytes32 key = _getWithdrawalKey(_actor, _validator);
        if (pendingWithdrawalKeys.contains(key)) return; // Already exists

        pendingWithdrawalKeys.add(key);
        pendingWithdrawals[key] = PendingWithdrawal(_actor, _validator);
    }

    function _removePendingWithdrawal(address _actor, address _validator) internal {
        bytes32 key = _getWithdrawalKey(_actor, _validator);
        if (!pendingWithdrawalKeys.contains(key)) return; // Doesn't exist

        pendingWithdrawalKeys.remove(key);
        delete pendingWithdrawals[key];
    }

    function claimWithdrawalOk(uint256 withdrawalIndex) public {
        if (pendingWithdrawalKeys.length() == 0) return;

        bytes32 withdrawalKey =
            pendingWithdrawalKeys.at(withdrawalIndex % pendingWithdrawalKeys.length());
        PendingWithdrawal memory withdrawal = pendingWithdrawals[withdrawalKey];

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
        okFunctionStats.claimWithdrawalOk.successes++;
    }

    function getNumActors() external view returns (uint256) {
        return actors.length();
    }

    function getNumAllValidators() external view returns (uint256) {
        return allValidators.length();
    }

    function getNumActiveValidators() external view returns (uint256) {
        return activeValidators.length();
    }

    function getNumPendingWithdrawals() external view returns (uint256) {
        return pendingWithdrawalKeys.length();
    }

    function getPendingWithdrawalAtIndex(uint256 index) external view returns (address, address) {
        require(index < pendingWithdrawalKeys.length(), "Index out of bounds");
        bytes32 key = pendingWithdrawalKeys.at(index);
        PendingWithdrawal memory withdrawal = pendingWithdrawals[key];
        return (withdrawal.actor, withdrawal.validator);
    }

    function getNumValidatorsWithDelegations() external view returns (uint256) {
        return validatorsWithDelegations.length();
    }

    function getNumValidatorDelegators(address _validator) external view returns (uint256) {
        return validatorDelegators[_validator].length();
    }

    function getNumExitedValidators() external view returns (uint256) {
        return exitedValidators.length();
    }

    function getNumExitedValidatorDelegators(address _validator) external view returns (uint256) {
        return exitedValidatorDelegators[_validator].length();
    }

    function getActorAtIndex(uint256 index) external view returns (address) {
        return actors.at(index);
    }

    function getValidatorWithDelegationsAtIndex(uint256 index)
        external
        view
        returns (address, uint256)
    {
        address _validator = validatorsWithDelegations.at(index);
        return (_validator, validatorDelegators[_validator].length());
    }

    function getExitedValidatorAtIndex(uint256 index) external view returns (address) {
        return exitedValidators.at(index);
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

    function getOkStats() external view returns (OkFunctionStats memory) {
        return okFunctionStats;
    }

    function getAnyStats() external view returns (AnyFunctionStats memory) {
        return anyFunctionStats;
    }

    function advanceTime(uint256 seed) public {
        // Advance time by a random amount up to the escrow period
        uint256 timeAdvance = boundRange(seed, 1, EXIT_ESCROW_PERIOD);
        ivm.warp(block.timestamp + timeAdvance);
        okFunctionStats.advanceTime.successes++;
    }

    function claimValidatorExitOk(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (exitedValidators.length() == 0) return;

        validator = exitedValidators.at(validatorIndex % exitedValidators.length());

        // Pick a delegator from this exited validator's delegators
        EnumerableSet.AddressSet storage delegators = exitedValidatorDelegators[validator];
        if (delegators.length() == 0) return;

        actor = delegators.at(delegatorIndex % delegators.length());

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
        okFunctionStats.claimValidatorExitOk.successes++;
    }

    function claimValidatorExitAny(uint256 validatorIndex, uint256 delegatorIndex) public {
        if (exitedValidators.length() == 0) return;

        validator = exitedValidators.at(validatorIndex % exitedValidators.length());

        // Pick a delegator from this exited validator's delegators
        EnumerableSet.AddressSet storage delegators = exitedValidatorDelegators[validator];
        if (delegators.length() == 0) return;

        actor = delegators.at(delegatorIndex % delegators.length());

        // Read delegation amount BEFORE claiming (claimValidatorExit clears it)
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        ivm.prank(actor);
        try stakeTable.claimValidatorExit(validator) {
            // Update tracking on success using pre-read amount
            totalActiveDelegations -= delegatedAmount;
            trackedActorFunds[actor].delegations -= delegatedAmount;
            _removeExitedValidatorDelegator(validator, actor);
            anyFunctionStats.claimValidatorExitAny.successes++;
        } catch {
            // Claim failed - this is acceptable for the Any function
            anyFunctionStats.claimValidatorExitAny.reverts++;
        }
    }
}
