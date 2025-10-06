// SPDX-License-Identifier: UNLICENSED
/* solhint-disable func-name-mixedcase, no-console */
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
import { console } from "forge-std/console.sol";
import {
    StakeTableV2PropTestBase, MockStakeTableV2, MockERC20
} from "./StakeTableV2PropTestBase.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { InvariantStats } from "./utils/InvariantStats.sol";

// Contract containing the important test logic (invariants and test setup)
contract StakeTableV2InvariantTest is StdInvariant, Test {
    StakeTableV2PropTestBase public handler;
    InvariantStats public stats;

    function setUp() public {
        // Configure contract under test
        handler = new StakeTableV2PropTestBase();
        stats = new InvariantStats(handler);
        targetContract(address(handler));
        bytes4[] memory selectors = new bytes4[](1);
        selectors[0] = StakeTableV2PropTestBase.withdrawAllFunds.selector;
        excludeSelector(FuzzSelector(address(handler), selectors));
    }

    function afterInvariant() external {
        console.log("\n=== Call stats for last invariant run ===");
        stats.logFunctionStats();

        console.log("\n=== State before withdrawal ===");
        stats.logCurrentState();

        // Run the slow delegated amount check only once per invariant run
        handler.assertValidatorDelegatedAmountSum();

        // Ensure all participants can withdraw all their funds
        handler.withdrawAllFunds();

        console.log("\n=== State after withdrawal ===");
        stats.logCurrentState();

        handler.verifyFinalState();

        // verify the invariants
        invariant_ContractBalanceMatchesTrackedDelegations();
        invariant_TotalSupply();

        // verify the total stake invariants
        invariant_activeStakeMatchesTracked();
        invariant_ValidatorStakeNotExceedsContractBalance();

        // additionally check the actor balances
        assertActorsRecoveredFunds();
    }

    /// @dev The total amount of tokens owned by an actor does not change
    function assertActorsRecoveredFunds() public view {
        // Slow: O(n) in contrast to all others that may run during each step.
        for (uint256 i = 0; i < handler.getNumActors(); i++) {
            address actor = handler.getActorAtIndex(i);
            assertEq(
                handler.totalOwnedAmount(actor),
                handler.getInitialBalance(actor),
                "Actor balance not conserved in tracking"
            );
            assertEq(
                handler.token().balanceOf(actor),
                handler.getInitialBalance(actor),
                "Actor balance not conserved after withdrawal"
            );
        }
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_ContractBalanceMatchesTrackedDelegations() public view {
        StakeTableV2PropTestBase.TestState memory state = handler.getTestState();

        uint256 contractBalance = handler.token().balanceOf(address(handler.stakeTable()));
        uint256 totalTracked = state.totalDelegated + state.totalPendingWithdrawal;
        assertEq(
            contractBalance,
            totalTracked,
            "Contract balance should equal active delegations + pending withdrawals"
        );
    }

    /// @dev Total supply must remain constant
    function invariant_TotalSupply() public view {
        assertEq(
            handler.token().totalSupply(),
            handler.getTestState().trackedTotalSupply,
            "Total supply invariant violated"
        );
    }

    /// @dev Contract's activeStake should match tracked activeStake
    function invariant_activeStakeMatchesTracked() public view {
        assertEq(
            handler.stakeTable().activeStake(),
            handler.getactiveStake(),
            "Contract activeStake does not match tracked activeStake"
        );
    }

    /// @dev Total validator stake should not exceed contract balance
    function invariant_ValidatorStakeNotExceedsContractBalance() public view {
        assertLe(
            handler.stakeTable().activeStake(),
            handler.stakeTable().token().balanceOf(address(handler.stakeTable())),
            "Total validator stake should not exceed contract balance"
        );
    }
}
