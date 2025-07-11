// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
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
    // Override the modifiers to use foundry's vm and bound function
    modifier useValidator(uint256 validatorIndex) override {
        validator = actors[bound(validatorIndex, 0, actors.length - 1)];
        _;
    }

    modifier useActor(uint256 actorIndex) override {
        actor = actors[bound(actorIndex, 0, actors.length - 1)];
        vm.startPrank(actor);
        _;
        vm.stopPrank();
    }

    constructor(MockStakeTableV2 _stakeTable, MockERC20 _token) {
        stakeTable = _stakeTable;
        token = _token;
        for (uint256 i = 0; i < actors.length; i++) {
            initialBalances[actors[i]] = INITIAL_BALANCE;
        }
    }
}

contract StakeTableV2InvariantTest is StdInvariant, Test, StakeTableV2PropTestBase {
    StakeTableV2Handler public handler;

    function setUp() public {
        _deployStakeTable();
        _mintAndApprove();

        // Configure contract under test
        handler = new StakeTableV2Handler(stakeTable, token);
        targetContract(address(handler));
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
        assertEq(_getTotalSupply(), INITIAL_BALANCE * 4, "Total supply invariant violated");
    }
}
