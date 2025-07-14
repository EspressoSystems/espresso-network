// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
import { console2 } from "forge-std/console2.sol";
import {
    StakeTableV2PropTestBase, MockStakeTableV2, MockERC20
} from "./StakeTableV2PropTestBase.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";

contract StakeTableV2InvariantTest is StdInvariant, Test, StakeTableV2PropTestBase {
    StakeTableV2PropTestBase public handler;

    function setUp() public {
        // Configure contract under test
        handler = new StakeTableV2PropTestBase();
        targetContract(address(handler));
    }

    function afterInvariant() external view {
        console2.log("\n=== Transaction Success Counters ===");
        console2.log("countOk_createActor:", handler.countOk_createActor());
        console2.log("countOk_createValidator:", handler.countOk_createValidator());
        console2.log("countOk_registerValidator:", handler.countOk_registerValidator());
        console2.log("countOk_deregisterValidator:", handler.countOk_deregisterValidator());
        console2.log("countOk_delegate:", handler.countOk_delegate());
        console2.log("countOk_undelegate:", handler.countOk_undelegate());
        console2.log("countOk_claimWithdrawal:", handler.countOk_claimWithdrawal());
        console2.log("countOk_advanceTime:", handler.countOk_advanceTime());

        uint256 totalSuccessful = handler.countOk_createActor() + handler.countOk_createValidator()
            + handler.countOk_registerValidator() + handler.countOk_deregisterValidator()
            + handler.countOk_delegate() + handler.countOk_undelegate()
            + handler.countOk_claimWithdrawal() + handler.countOk_advanceTime();
        console2.log("Total successful transactions:", totalSuccessful);

        console2.log("\n=== Current State ===");
        console2.log("Num actors:", handler.getNumActors());
        console2.log("Num all validators:", handler.getNumAllValidators());
        console2.log("Num active validators:", handler.getNumActiveValidators());
        console2.log("Num pending withdrawals:", handler.getNumPendingWithdrawals());
        console2.log("Num validators with delegations:", handler.getNumValidatorsWithDelegations());

        // Count total validator-delegator pairs
        uint256 totalValidatorDelegatorPairs = 0;
        for (uint256 i = 0; i < handler.getNumValidatorsWithDelegations(); i++) {
            address validator = handler.validatorsWithDelegations(i);
            totalValidatorDelegatorPairs += handler.getNumValidatorDelegators(validator);
        }
        console2.log("Total validator-delegator pairs:", totalValidatorDelegatorPairs);

        console2.log("Total active delegations:", handler.totalActiveDelegations());
        console2.log("Total active undelegations:", handler.totalActiveUndelegations());
        console2.log("Tracked total supply:", handler.trackedTotalSupply());
    }

    /// @dev The total amount of tokens owned by an actor does not change
    function invariant_actorOwnedAmounts() public view {
        for (uint256 i = 0; i < actors.length; i++) {
            assertEq(
                totalOwnedAmount(actors[i]),
                initialBalances[actors[i]],
                "Actor balance invariant violated"
            );
        }
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_ContractBalanceMatchesTrackedDelegations() public view {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalTracked = _getTotalTrackedFunds();
        assertEq(
            contractBalance,
            totalTracked,
            "Contract balance should equal active delegations + pending undelegations"
        );
    }

    /// @dev Total supply must remain constant
    function invariant_TotalSupply() public view {
        assertEq(_getTotalSupply(), trackedTotalSupply, "Total supply invariant violated");
    }
}
