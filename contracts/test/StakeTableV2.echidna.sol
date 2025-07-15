// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import { StakeTableV2PropTestBase } from "./StakeTableV2PropTestBase.sol";
import { EnumerableSet } from "@openzeppelin/contracts/utils/structs/EnumerableSet.sol";

contract StakeTableV2EchidnaTest is StakeTableV2PropTestBase {
    using EnumerableSet for EnumerableSet.AddressSet;

    constructor() { }

    /// @dev The total amount of tokens owned by an actor does not change
    function echidna_actorOwnedAmounts() public view returns (bool) {
        for (uint256 i = 0; i < actors.length(); i++) {
            if (totalOwnedAmount(actors.at(i)) != initialBalances[actors.at(i)]) {
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
        return _getTotalSupply() == trackedTotalSupply;
    }
}
