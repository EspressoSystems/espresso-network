// SPDX-License-Identifier: UNLICENSED
/* solhint-disable no-console */
pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { EnumerableSet } from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";
import { EnumerableMap } from "@openzeppelin/contracts/utils/structs/EnumerableMap.sol";
import { console } from "forge-std/console.sol";

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
    struct FuncStats {
        uint256 ok;
        uint256 reverts;
    }

    // Split into smaller structs to avoid stack too deep
    struct CallStatsOk {
        FuncStats delegate;
        FuncStats undelegate;
        FuncStats deregisterValidator;
        FuncStats claimWithdrawal;
        FuncStats claimValidatorExit;
        FuncStats createActor;
        FuncStats createValidator;
        FuncStats advanceTime;
    }

    struct CallStatsAny {
        FuncStats registerValidator;
        FuncStats delegate;
        FuncStats undelegate;
        FuncStats deregisterValidator;
        FuncStats claimValidatorExit;
    }

    struct CallStats {
        CallStatsOk ok;
        CallStatsAny any;
    }

    CallStats public stats;

    function getCallStats() external view returns (CallStats memory) {
        return stats;
    }
}

contract StakeTableV2PropTestBase is FunctionCallTracking {
    using EnumerableSet for EnumerableSet.AddressSet;

    struct TestState {
        uint256 trackedTotalSupply;
        uint256 totalDelegated;
        uint256 totalPendingWithdrawal;
        uint256 numPendingWithdrawals;
        uint256 numActiveDelegations;
        uint256 totalStake;
        uint256 activeStake;
    }

    // Actors can be validators and/or delegators
    struct Actors {
        EnumerableSet.AddressSet all;
        mapping(address actor => uint256 balance) initialBalances;
        mapping(address actor => ActorFunds funds) trackedFunds;
    }

    struct ActorFunds {
        uint256 delegated;
        uint256 pendingWithdrawal;
    }

    // All validators are in `all`, other sets are used to track validators in
    // specific states.
    struct Validators {
        EnumerableSet.AddressSet all;
        EnumerableSet.AddressSet active;
        EnumerableSet.AddressSet exited;
        EnumerableSet.AddressSet staked;
        EnumerableSet.AddressSet withPendingWithdrawals;
    }

    struct Delegators {
        mapping(address validator => EnumerableSet.AddressSet delegators) delegators;
        mapping(address validator => EnumerableSet.AddressSet actors) pendingWithdrawals;
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

    modifier withValidator(uint256 valIndex) virtual {
        if (validators.all.length() == 0) {
            createValidator(valIndex);
        }
        validator = validators.all.at(valIndex % validators.all.length());
        _;
    }

    modifier withActiveValidator(uint256 valIndex) virtual {
        if (validators.active.length() == 0) {
            createValidator(valIndex);
        }
        validator = validators.active.at(valIndex % validators.active.length());
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
        deployStakeTable();
        testState.trackedTotalSupply = token.totalSupply();
    }

    function deployStakeTable() internal {
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
        MockStakeTableV2.InitialCommission[] memory emptyCommissions;
        StakeTable(payable(address(proxy))).upgradeToAndCall(
            address(stakeTableV2Impl),
            abi.encodeWithSignature(
                "initializeV2(address,address,(address,uint16)[])", admin, admin, emptyCommissions
            )
        );

        // Cast to V2 interface
        stakeTable = MockStakeTableV2(payable(address(proxy)));
    }

    function genDummyValidatorKeys(address _validator)
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
        return walletBalance + funds.delegated + funds.pendingWithdrawal;
    }

    // NOTE: The create validator function is used to generate a new validators
    // successfully. Therefore there is no `registerValidatorOk` function. That
    // function is not only a fuzzing target but is also used to create
    // validators if needed for other functions to execute. Hence it doesn't
    // follow the `Ok`/`Any` pattern.

    function registerValidatorAny(uint256 actorIndex) public useActor(actorIndex) {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = genDummyValidatorKeys(actor);

        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) {
            trackRegisterValidator(actor);
            stats.any.registerValidator.ok++;
        } catch {
            // Registration failed - this is acceptable for the Any function
            stats.any.registerValidator.reverts++;
        }
    }

    function newAddress(uint256 seed) internal view returns (address) {
        address candidate = address(uint160(uint256(keccak256(abi.encode(seed)))));

        // If address is already an actor, increment until we find an available one
        while (actors.all.contains(candidate)) {
            candidate = address(uint160(candidate) + 1);
        }

        return candidate;
    }

    function deregisterValidatorOk(uint256 valIndex) public {
        if (validators.active.length() == 0) {
            return;
        }
        address val = validators.active.at(valIndex % validators.active.length());

        ivm.prank(val);
        stakeTable.deregisterValidator();
        trackDeregisterValidator(val);
        stats.ok.deregisterValidator.ok++;
    }

    function deregisterValidatorAny(uint256 valIndex) public {
        if (validators.all.length() == 0) {
            return;
        }
        address val = validators.all.at(valIndex % validators.all.length());

        ivm.prank(val);
        try stakeTable.deregisterValidator() {
            trackDeregisterValidator(val);
            stats.any.deregisterValidator.ok++;
        } catch {
            stats.any.deregisterValidator.reverts++;
        }
    }

    function createActor(uint256 seed) public returns (address) {
        address newActor = newAddress(seed);

        // Fund the actor with tokens
        token.mint(newActor, INITIAL_BALANCE);
        actors.initialBalances[newActor] = INITIAL_BALANCE;
        testState.trackedTotalSupply += INITIAL_BALANCE;

        // Approve stake table to spend tokens
        ivm.prank(newActor);
        token.approve(address(stakeTable), type(uint256).max);

        // Add to actors array and map
        actors.all.add(newActor);
        stats.ok.createActor.ok++;

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
        ) = genDummyValidatorKeys(val);

        ivm.prank(val);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
        trackRegisterValidator(val);
        stats.ok.createValidator.ok++;

        return val;
    }

    function delegateOk(uint256 actorIndex, uint256 valIndex, uint256 amount)
        public
        withActiveValidator(valIndex)
        useActor(actorIndex)
    {
        uint256 balance = token.balanceOf(actor);
        if (balance == 0) return;

        amount = boundRange(amount, 1, balance);

        stakeTable.delegate(validator, amount);
        trackDelegate(actor, validator, amount);
        stats.ok.delegate.ok++;
    }

    function delegateAny(uint256 actorIndex, uint256 valIndex, uint256 amount)
        public
        withActiveValidator(valIndex)
        useActor(actorIndex)
    {
        try stakeTable.delegate(validator, amount) {
            trackDelegate(actor, validator, amount);
            stats.any.delegate.ok++;
        } catch {
            stats.any.delegate.reverts++;
        }
    }

    function undelegateOk(uint256 actorIndex, uint256 valIndex, uint256 amount) public {
        // Use validators with delegations for higher success rate
        if (validators.staked.length() == 0) return;

        validator = validators.staked.at(valIndex % validators.staked.length());

        EnumerableSet.AddressSet storage validatorDelegators = delegators.delegators[validator];

        actor = validatorDelegators.at(actorIndex % validatorDelegators.length());

        // only one pending withdrawal is allowed at a time
        (uint256 existingUndelegation,) = stakeTable.undelegations(validator, actor);
        if (existingUndelegation > 0) return;

        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        if (delegatedAmount == 0) {
            revert("Tracking inconsistency: delegatedAmount=0 but actor still has delegations");
        }

        amount = boundRange(amount, 1, delegatedAmount);

        ivm.prank(actor);
        stakeTable.undelegate(validator, amount);
        trackUndelegate(actor, validator, amount);
        stats.ok.undelegate.ok++;
    }

    function undelegateAny(uint256 actorIndex, uint256 valIndex, uint256 amount)
        public
        withActiveValidator(valIndex)
        useActor(actorIndex)
    {
        try stakeTable.undelegate(validator, amount) {
            trackUndelegate(actor, validator, amount);
            stats.any.undelegate.ok++;
        } catch {
            stats.any.undelegate.reverts++;
        }
    }

    function trackDelegate(address actorAddr, address val, uint256 amount) internal {
        testState.totalDelegated += amount;
        testState.totalStake += amount; // Add this
        testState.activeStake += amount; // Add this
        actors.trackedFunds[actorAddr].delegated += amount;
        validators.staked.add(val);

        // Only increment counter if this is a new delegation (actor wasn't already delegating to
        // this validator)
        if (!delegators.delegators[val].contains(actorAddr)) {
            testState.numActiveDelegations++;
        }
        delegators.delegators[val].add(actorAddr);
    }

    function trackUndelegate(address actorAddr, address val, uint256 amount) internal {
        testState.totalDelegated -= amount;
        testState.totalPendingWithdrawal += amount;
        testState.activeStake -= amount;
        actors.trackedFunds[actorAddr].delegated -= amount;
        actors.trackedFunds[actorAddr].pendingWithdrawal += amount;
        addPendingWithdrawal(actorAddr, val);

        // Remove delegator from tracking if delegation amount reaches 0
        if (stakeTable.delegations(val, actorAddr) == 0) {
            trackRemoveDelegation(val, actorAddr);
        }
    }

    function trackRemoveDelegation(address val, address del) internal {
        if (delegators.delegators[val].contains(del)) {
            delegators.delegators[val].remove(del);
            testState.numActiveDelegations--;
            // Remove from staked validators if delegation amount reaches 0
            if (delegators.delegators[val].length() == 0) {
                validators.staked.remove(val);
            }
        }
    }

    function trackRegisterValidator(address val) internal {
        validators.all.add(val);
        validators.active.add(val);
    }

    function trackDeregisterValidator(address val) internal {
        validators.active.remove(val);
        validators.exited.add(val);
        validators.staked.remove(val);

        (uint256 validatorDelegatedAmount,) = stakeTable.validators(val);
        testState.activeStake -= validatorDelegatedAmount;
    }

    function trackClaimWithdrawal(address actorAddr, address val, uint256 undelegationAmount)
        internal
    {
        testState.totalPendingWithdrawal -= undelegationAmount;
        testState.totalStake -= undelegationAmount;
        actors.trackedFunds[actorAddr].pendingWithdrawal -= undelegationAmount;
        delegators.pendingWithdrawals[val].remove(actorAddr);
        if (delegators.pendingWithdrawals[val].length() == 0) {
            validators.withPendingWithdrawals.remove(val);
        }
        testState.numPendingWithdrawals--;
    }

    function trackClaimValidatorExit(address actorAddr, address val, uint256 delegatedAmount)
        internal
    {
        testState.totalDelegated -= delegatedAmount;
        testState.totalStake -= delegatedAmount;
        actors.trackedFunds[actorAddr].delegated -= delegatedAmount;
        trackRemoveDelegation(val, actorAddr);
    }

    function addPendingWithdrawal(address actorAddr, address val) internal {
        if (delegators.pendingWithdrawals[val].contains(actorAddr)) return; // Already exists

        delegators.pendingWithdrawals[val].add(actorAddr);
        validators.withPendingWithdrawals.add(val);
        testState.numPendingWithdrawals++;
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
        trackClaimWithdrawal(pendingActor, val, undelegationAmount);
        stats.ok.claimWithdrawal.ok++;
    }

    // Getter functions for external contract usage (invariant tests, stats logging)
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
        return testState.numPendingWithdrawals;
    }

    function getNumValidatorsWithDelegations() external view returns (uint256) {
        return validators.staked.length();
    }

    function getNumExitedValidators() external view returns (uint256) {
        return validators.exited.length();
    }

    function getActorAtIndex(uint256 index) external view returns (address) {
        return actors.all.at(index);
    }

    function getInitialBalance(address actorAddr) external view returns (uint256) {
        return actors.initialBalances[actorAddr];
    }

    function getTestState() external view returns (TestState memory) {
        return testState;
    }

    function getTotalStake() external view returns (uint256) {
        return testState.totalStake;
    }

    function getactiveStake() external view returns (uint256) {
        return testState.activeStake;
    }

    function getTotalSuccesses() external view returns (uint256) {
        uint256 total = 0;
        // Ok functions
        total += stats.ok.delegate.ok;
        total += stats.ok.undelegate.ok;
        total += stats.ok.deregisterValidator.ok;
        total += stats.ok.claimWithdrawal.ok;
        total += stats.ok.claimValidatorExit.ok;
        total += stats.ok.createActor.ok;
        total += stats.ok.createValidator.ok;
        total += stats.ok.advanceTime.ok;
        // Any functions
        total += stats.any.registerValidator.ok;
        total += stats.any.delegate.ok;
        total += stats.any.undelegate.ok;
        total += stats.any.deregisterValidator.ok;
        total += stats.any.claimValidatorExit.ok;
        return total;
    }

    function getTotalReverts() external view returns (uint256) {
        uint256 total = 0;
        total += stats.any.registerValidator.reverts;
        total += stats.any.delegate.reverts;
        total += stats.any.undelegate.reverts;
        total += stats.any.deregisterValidator.reverts;
        total += stats.any.claimValidatorExit.reverts;
        return total;
    }

    function getTotalCalls() external view returns (uint256) {
        return this.getTotalSuccesses() + this.getTotalReverts();
    }

    function advanceTime(uint256 seed) public {
        // Advance time by a random amount up to the escrow period
        uint256 timeAdvance = boundRange(seed, 1, EXIT_ESCROW_PERIOD);
        ivm.warp(block.timestamp + timeAdvance);
        stats.ok.advanceTime.ok++;
    }

    function claimValidatorExitOk(uint256 valIndex, uint256 delegatorIndex) public {
        if (validators.exited.length() == 0) return;

        validator = validators.exited.at(valIndex % validators.exited.length());

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
        trackClaimValidatorExit(actor, validator, delegatedAmount);
        stats.ok.claimValidatorExit.ok++;
    }

    function claimValidatorExitAny(uint256 valIndex, uint256 delegatorIndex) public {
        if (validators.exited.length() == 0) return;

        validator = validators.exited.at(valIndex % validators.exited.length());

        // Use actors set to pick a delegator
        if (actors.all.length() == 0) return;

        actor = actors.all.at(delegatorIndex % actors.all.length());

        // Read delegation amount BEFORE claiming (claimValidatorExit clears it)
        uint256 delegatedAmount = stakeTable.delegations(validator, actor);

        ivm.prank(actor);
        try stakeTable.claimValidatorExit(validator) {
            trackClaimValidatorExit(actor, validator, delegatedAmount);
            stats.any.claimValidatorExit.ok++;
        } catch {
            stats.any.claimValidatorExit.reverts++;
        }
    }

    // NOTE: intended to be run at the end of the test
    //
    // We used to have a bug where the contract lost track of pending
    // withdrawals by overwriting them. Draining all tracked funds from the
    // contract will surface such inconsistencies.
    //
    // TODO: this function is missing tracking. Refactor out helper functions to
    // perform actions and update tracking. Use them here and in the Ok()
    // functions.
    //
    // solhint-disable code-complexity
    function withdrawAllFunds() public {
        // unlock
        ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);

        for (uint256 i = 0; i < actors.all.length(); i++) {
            address del = actors.all.at(i);
            for (uint256 j = 0; j < validators.all.length(); j++) {
                address val = validators.all.at(j);

                // claim undelegation
                (uint256 undelegationAmount,) = stakeTable.undelegations(val, del);

                if (undelegationAmount > 0) {
                    ivm.prank(del);
                    stakeTable.claimWithdrawal(val);
                    trackClaimWithdrawal(del, val, undelegationAmount);
                }

                uint256 delegatedAmount = stakeTable.delegations(val, del);

                if (delegatedAmount > 0) {
                    //  if validator exited, claim exit
                    uint256 validatorExitTime = stakeTable.validatorExits(val);
                    if (validatorExitTime > 0) {
                        ivm.prank(del);
                        stakeTable.claimValidatorExit(val);
                        trackClaimValidatorExit(del, val, delegatedAmount);
                    } else {
                        // undelegate remaining delegated amount
                        ivm.prank(del);
                        stakeTable.undelegate(val, delegatedAmount);
                        trackUndelegate(del, val, delegatedAmount);
                    }
                }
            }
        }

        // unlock
        ivm.warp(block.timestamp + EXIT_ESCROW_PERIOD);

        // Finally, claim the new withdrawals
        for (uint256 i = 0; i < actors.all.length(); i++) {
            address del = actors.all.at(i);
            for (uint256 j = 0; j < validators.all.length(); j++) {
                address val = validators.all.at(j);
                (uint256 undelegationAmount,) = stakeTable.undelegations(val, del);

                if (undelegationAmount > 0) {
                    ivm.prank(del);
                    stakeTable.claimWithdrawal(val);
                    trackClaimWithdrawal(del, val, undelegationAmount);
                }
            }
        }
    }

    function verifyFinalState() external view {
        // Verify StakeTable has zero balance
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        require(contractBalance == 0, "StakeTable should have zero balance after full withdrawal");

        // Verify each actor has their original balance back
        for (uint256 i = 0; i < actors.all.length(); i++) {
            address actor_ = actors.all.at(i);
            uint256 currentBalance = token.balanceOf(actor_);
            uint256 originalBalance = actors.initialBalances[actor_];
            require(currentBalance == originalBalance, "Actor should have original balance back");
        }

        // Verify no pending withdrawals remain
        require(testState.totalPendingWithdrawal == 0, "No pending withdrawals should remain");
        require(testState.totalDelegated == 0, "No delegations should remain");

        require(testState.totalStake == 0, "Total stake should be zero after full withdrawal");
        require(
            testState.activeStake == 0, "Total validator stake should be zero after full withdrawal"
        );
    }

    /// @dev Assert sum of delegated amounts to a validator equals validator.delegatedAmount
    ///
    /// Iterates over all validators and their delegators, and is therefore very slow.
    function assertValidatorDelegatedAmountSum() public view {
        for (uint256 i = 0; i < validators.all.length(); i++) {
            address val = validators.all.at(i);

            // Get the validator's delegatedAmount from the contract
            (uint256 validatorDelegatedAmount,) = stakeTable.validators(val);

            // Calculate sum of all individual delegations to this validator
            uint256 sumOfDelegations = 0;
            EnumerableSet.AddressSet storage validatorDelegators = delegators.delegators[val];
            for (uint256 j = 0; j < validatorDelegators.length(); j++) {
                address delegator = validatorDelegators.at(j);
                sumOfDelegations += stakeTable.delegations(val, delegator);
            }

            require(
                validatorDelegatedAmount == sumOfDelegations, "Validator delegatedAmount mismatch"
            );
        }
    }

    /// @dev Assert that contract's totalStake and activeStake match our tracking
    function assertTotalStakeInvariants() public view {
        require(
            stakeTable.totalStake() == testState.totalStake,
            "Contract totalStake does not match tracked totalStake"
        );

        require(
            stakeTable.activeStake() == testState.activeStake,
            "Contract activeStake does not match tracked activeStake"
        );
    }
}
