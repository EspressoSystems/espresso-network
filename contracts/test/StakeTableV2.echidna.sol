// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import { StakeTableV2PropTestBase } from "./StakeTableV2PropTestBase.sol";

contract StakeTableV2EchidnaTest is StakeTableV2PropTestBase {
    constructor() {
        _deployStakeTable();
        _mintAndApprove();
    }

    /// @dev The total amount of tokens owned by an actor does not change
    function echidna_actorOwnedAmounts() public view returns (bool) {
        for (uint256 i = 0; i < actors.length; i++) {
            if (totalOwnedAmount(actors[i]) != initialBalances[actors[i]]) {
                return false;
            }
        }
        return true;
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function echidna_ContractBalanceMatchesTrackedDelegations() public view returns (bool) {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalTracked = _getTotalTrackedFunds();
        return contractBalance == totalTracked;
    }

    /// @dev Total supply must remain constant
    function echidna_TotalSupply() public view returns (bool) {
        return _getTotalSupply() == INITIAL_BALANCE * 4;
    }
}
