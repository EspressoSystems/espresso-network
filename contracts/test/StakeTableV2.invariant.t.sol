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
    constructor(MockStakeTableV2 _stakeTable, MockERC20 _token) {
        stakeTable = _stakeTable;
        token = _token;

        // Set initial balances for all actors
        for (uint256 i = 0; i < actors.length; i++) {
            initialBalances[actors[i]] = INITIAL_BALANCE;
        }
    }

    function registerValidator(uint256 actorIndex) public {
        address validator = actors[actorIndex % 4];

        (, StakeTable.ValidatorStatus status) = stakeTable.validators(validator);
        if (status != StakeTable.ValidatorStatus.Unknown) {
            return;
        }

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(validator);

        vm.prank(validator);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
    }

    function delegateAny(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        vm.prank(delegator);
        stakeTable.delegate(validator, amount);
    }

    function delegateOk(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        uint256 balance = token.balanceOf(delegator);
        if (balance == 0) return;

        amount = bound(amount, 1, balance);

        vm.prank(delegator);
        stakeTable.delegate(validator, amount);
    }

    function undelegateAny(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        vm.prank(delegator);
        stakeTable.undelegate(validator, amount);
    }

    function undelegateOk(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        uint256 delegatedAmount = stakeTable.delegations(validator, delegator);
        if (delegatedAmount == 0) return;

        amount = bound(amount, 1, delegatedAmount);

        vm.prank(delegator);
        stakeTable.undelegate(validator, amount);
    }

    function claimWithdrawal(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        vm.prank(delegator);
        stakeTable.claimWithdrawal(validator);
    }

    function deregisterValidator(uint256 validatorIndex) public {
        address validator = actors[validatorIndex % 4];

        vm.prank(validator);
        stakeTable.deregisterValidator();
    }

    function claimValidatorExit(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = actors[delegatorIndex % 4];
        address validator = actors[validatorIndex % 4];

        vm.prank(delegator);
        stakeTable.claimValidatorExit(validator);
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
