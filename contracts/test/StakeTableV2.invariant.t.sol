// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
import {
    StakeTableV2PropTestBase, MockStakeTableV2, MockERC20
} from "./StakeTableV2PropTestBase.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";

contract StakeTableV2Handler is Test, StakeTableV2PropTestBase {
    // Ghost variables for tracking
    uint256 public ghost_totalDelegated;
    uint256 public ghost_totalUndelegated;

    constructor(MockStakeTableV2 _stakeTable, MockERC20 _token) {
        stakeTable = _stakeTable;
        token = _token;

        // Set initial balances
        initialBalances[VALIDATOR1] = INITIAL_BALANCE;
        initialBalances[VALIDATOR2] = INITIAL_BALANCE;
        initialBalances[DELEGATOR1] = INITIAL_BALANCE;
        initialBalances[DELEGATOR2] = INITIAL_BALANCE;
    }

    function registerValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        (, StakeTable.ValidatorStatus status) = stakeTable.validators(validator);
        if (status != StakeTable.ValidatorStatus.Unknown) {
            return;
        }

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _generateValidatorKeys(validator);

        vm.prank(validator);
        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) { } catch { }
    }

    function delegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 balanceBefore = token.balanceOf(delegator);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) {
            // Track successful delegation
            uint256 balanceAfter = token.balanceOf(delegator);
            uint256 actualAmount = balanceBefore - balanceAfter;
            ghost_totalDelegated += actualAmount;
        } catch { }
    }

    function delegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 balance = token.balanceOf(delegator);
        if (balance == 0) return;

        amount = bound(amount, 1, balance);

        uint256 balanceBefore = token.balanceOf(delegator);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) {
            // Track successful delegation
            uint256 balanceAfter = token.balanceOf(delegator);
            uint256 actualAmount = balanceBefore - balanceAfter;
            ghost_totalDelegated += actualAmount;
        } catch { }
    }

    function undelegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount)
        public
    {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) {
            ghost_totalUndelegated += amount;
        } catch { }
    }

    function undelegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 delegatedAmount = stakeTable.delegations(validator, delegator);
        if (delegatedAmount == 0) return;

        amount = bound(amount, 1, delegatedAmount);

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) {
            ghost_totalUndelegated += amount;
        } catch { }
    }

    function claimWithdrawal(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.claimWithdrawal(validator) { } catch { }
    }

    function deregisterValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        vm.prank(validator);
        try stakeTable.deregisterValidator() { } catch { }
    }

    function claimValidatorExit(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.claimValidatorExit(validator) { } catch { }
    }
}

contract StakeTableV2InvariantTest is StdInvariant, Test, StakeTableV2PropTestBase {
    StakeTableV2Handler public handler;

    function setUp() public {
        _deployStakeTable();
        _mintAndApprove();

        // Set up approvals
        vm.prank(VALIDATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(VALIDATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        // Create handler
        handler = new StakeTableV2Handler(stakeTable, token);

        // Target the handler for invariant testing
        targetContract(address(handler));

        // Configure the number of runs for invariant testing
        vm.deal(address(handler), 100 ether);
    }

    /// @dev Balance invariant: wallet + staked + pending withdrawals should equal initial balance
    function invariant_balanceInvariantValidator1() public view {
        assertEq(
            getTotalBalance(VALIDATOR1),
            initialBalances[VALIDATOR1],
            "Validator1 balance invariant violated"
        );
    }

    function invariant_balanceInvariantValidator2() public view {
        assertEq(
            getTotalBalance(VALIDATOR2),
            initialBalances[VALIDATOR2],
            "Validator2 balance invariant violated"
        );
    }

    function invariant_balanceInvariantDelegator1() public view {
        assertEq(
            getTotalBalance(DELEGATOR1),
            initialBalances[DELEGATOR1],
            "Delegator1 balance invariant violated"
        );
    }

    function invariant_balanceInvariantDelegator2() public view {
        assertEq(
            getTotalBalance(DELEGATOR2),
            initialBalances[DELEGATOR2],
            "Delegator2 balance invariant violated"
        );
    }

    /// @dev Total supply should remain constant
    function invariant_totalSupplyInvariant() public view {
        assertEq(_getTotalSupply(), INITIAL_BALANCE * 4, "Total supply invariant violated");
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_contractBalanceMatchesDelegations() public view {
        (uint256 contractBalance, uint256 totalTracked) = _getContractBalanceVsDelegations();
        assertEq(
            contractBalance,
            totalTracked,
            "Contract balance should equal active delegations + pending undelegations"
        );
    }
}
