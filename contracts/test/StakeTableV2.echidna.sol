// SPDX-License-Identifier: Unlicensed
/* solhint-disable func-name-mixedcase */
pragma solidity ^0.8.0;

import { StakeTableV2PropTestBase } from "./StakeTableV2PropTestBase.sol";
import { EnumerableSet } from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";

contract StakeTableV2EchidnaTest is StakeTableV2PropTestBase {
    using EnumerableSet for EnumerableSet.AddressSet;

    /// @dev The total amount of tokens owned by an actor does not change
    function echidna_actorOwnedAmounts() public view returns (bool) {
        for (uint256 i = 0; i < actors.all.length(); i++) {
            if (totalOwnedAmount(actors.all.at(i)) != actors.initialBalances[actors.all.at(i)]) {
                return false;
            }
        }
        return true;
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function echidna_ContractBalanceMatchesTrackedDelegations() public view returns (bool) {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalTracked = testState.totalDelegated + testState.totalPendingWithdrawal;
        return contractBalance == totalTracked;
    }

    /// @dev Total supply must remain constant
    function echidna_TotalSupply() public view returns (bool) {
        return this.getTotalSupply() == this.getTestState().trackedTotalSupply;
    }

    // Note: Unlike Foundry invariant tests, Echidna doesn't support post-test cleanup.
    // The withdrawAllFunds() verification is only available in the Foundry test suite.
}
