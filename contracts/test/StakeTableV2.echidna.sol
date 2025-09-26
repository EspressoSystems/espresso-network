// SPDX-License-Identifier: UNLICENSED
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
        return token.totalSupply() == this.getTestState().trackedTotalSupply;
    }

    // Note: Unlike Foundry invariant tests, Echidna doesn't support post-test cleanup.
    // The withdrawAllFunds() verification is only available in the Foundry test suite.

    /// @dev Total stake should equal sum of all delegated amounts
    function echidna_TotalStakeMatchesTracked() public view returns (bool) {
        return stakeTable.totalStake() == testState.totalStake;
    }

    /// @dev Total validator stake should equal sum of all delegated amounts
    function echidna_TotalValidatorStakeMatchesTracked() public view returns (bool) {
        return stakeTable.totalValidatorStake() == testState.totalValidatorStake;
    }

    /// @dev Total stake should equal contract balance
    function echidna_TotalStakeEqualsContractBalance() public view returns (bool) {
        return stakeTable.totalStake() == token.balanceOf(address(stakeTable));
    }

    /// @dev Total validator stake should not exceed total stake
    function echidna_TotalValidatorStakeNotExceedsTotalStake() public view returns (bool) {
        return stakeTable.totalValidatorStake() <= stakeTable.totalStake();
    }
}
