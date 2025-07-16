// SPDX-License-Identifier: UNLICENSED
/* solhint-disable func-name-mixedcase */
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
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
    }

    function afterInvariant() external view {
        stats.logFunctionStats();
        stats.logCurrentState();
    }

    /// @dev The total amount of tokens owned by an actor does not change
    function invariant_actorOwnedAmounts() public view {
        for (uint256 i = 0; i < handler.getNumActors(); i++) {
            address actor = handler.getActorAtIndex(i);
            assertEq(
                handler.totalOwnedAmount(actor),
                handler.getInitialBalance(actor),
                "Actor balance invariant violated"
            );
        }
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_ContractBalanceMatchesTrackedDelegations() public view {
        uint256 contractBalance = handler.token().balanceOf(address(handler.stakeTable()));
        uint256 totalTracked =
            handler.getTestState().totalDelegated + handler.getTestState().totalPendingWithdrawals;
        assertEq(
            contractBalance,
            totalTracked,
            "Contract balance should equal active delegations + pending withdrawals"
        );
    }

    /// @dev Total supply must remain constant
    function invariant_TotalSupply() public view {
        assertEq(
            handler.getTotalSupply(),
            handler.getTestState().trackedTotalSupply,
            "Total supply invariant violated"
        );
    }
}
