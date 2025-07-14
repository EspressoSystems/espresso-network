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

    function _formatNumber(uint256 num, uint256 width) internal pure returns (string memory) {
        string memory numStr = _uintToString(num);
        bytes memory numBytes = bytes(numStr);

        if (numBytes.length >= width) {
            return numStr;
        }

        bytes memory result = new bytes(width);
        uint256 padding = width - numBytes.length;

        // Add leading spaces for right alignment
        for (uint256 i = 0; i < padding; i++) {
            result[i] = " ";
        }

        // Copy number string
        for (uint256 i = 0; i < numBytes.length; i++) {
            result[padding + i] = numBytes[i];
        }

        return string(result);
    }

    function _uintToString(uint256 value) internal pure returns (string memory) {
        if (value == 0) {
            return "0";
        }

        uint256 temp = value;
        uint256 digits;
        while (temp != 0) {
            digits++;
            temp /= 10;
        }

        bytes memory buffer = new bytes(digits);
        while (value != 0) {
            digits -= 1;
            buffer[digits] = bytes1(uint8(48 + uint256(value % 10)));
            value /= 10;
        }

        return string(buffer);
    }

    function _logFunctionStats() internal view {
        console2.log("\n=== Function Call Statistics ===");
        console2.log("Function                     Successes  Reverts");
        console2.log("---------------------------------------------");

        // Ok functions - access via getter function
        StakeTableV2PropTestBase.OkFunctionStats memory okStats = handler.getOkStats();

        console2.log(
            string.concat(
                "advanceTime                  ",
                _formatNumber(okStats.advanceTime.successes, 9),
                " ",
                _formatNumber(okStats.advanceTime.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "claimValidatorExitOk         ",
                _formatNumber(okStats.claimValidatorExitOk.successes, 9),
                " ",
                _formatNumber(okStats.claimValidatorExitOk.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "claimWithdrawalOk            ",
                _formatNumber(okStats.claimWithdrawalOk.successes, 9),
                " ",
                _formatNumber(okStats.claimWithdrawalOk.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "createActor                  ",
                _formatNumber(okStats.createActor.successes, 9),
                " ",
                _formatNumber(okStats.createActor.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "createValidator              ",
                _formatNumber(okStats.createValidator.successes, 9),
                " ",
                _formatNumber(okStats.createValidator.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "delegateOk                   ",
                _formatNumber(okStats.delegateOk.successes, 9),
                " ",
                _formatNumber(okStats.delegateOk.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "deregisterValidatorOk        ",
                _formatNumber(okStats.deregisterValidatorOk.successes, 9),
                " ",
                _formatNumber(okStats.deregisterValidatorOk.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "undelegateOk                 ",
                _formatNumber(okStats.undelegateOk.successes, 9),
                " ",
                _formatNumber(okStats.undelegateOk.reverts, 7)
            )
        );

        // Any functions - access via getter function
        StakeTableV2PropTestBase.AnyFunctionStats memory anyStats = handler.getAnyStats();

        console2.log(
            string.concat(
                "claimValidatorExitAny        ",
                _formatNumber(anyStats.claimValidatorExitAny.successes, 9),
                " ",
                _formatNumber(anyStats.claimValidatorExitAny.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "delegateAny                  ",
                _formatNumber(anyStats.delegateAny.successes, 9),
                " ",
                _formatNumber(anyStats.delegateAny.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "deregisterValidatorAny       ",
                _formatNumber(anyStats.deregisterValidatorAny.successes, 9),
                " ",
                _formatNumber(anyStats.deregisterValidatorAny.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "registerValidatorAny         ",
                _formatNumber(anyStats.registerValidatorAny.successes, 9),
                " ",
                _formatNumber(anyStats.registerValidatorAny.reverts, 7)
            )
        );
        console2.log(
            string.concat(
                "undelegateAny                ",
                _formatNumber(anyStats.undelegateAny.successes, 9),
                " ",
                _formatNumber(anyStats.undelegateAny.reverts, 7)
            )
        );

        console2.log("---------------------------------------------");
        console2.log(
            string.concat(
                "Total                        ",
                _formatNumber(handler.getTotalSuccesses(), 9),
                " ",
                _formatNumber(handler.getTotalReverts(), 7)
            )
        );
    }

    function _logCurrentState() internal view {
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

        console2.log("Num exited validators:", handler.getNumExitedValidators());

        // Count total exited validator-delegator pairs
        uint256 totalExitedValidatorDelegatorPairs = 0;
        for (uint256 i = 0; i < handler.getNumExitedValidators(); i++) {
            address validator = handler.exitedValidators(i);
            totalExitedValidatorDelegatorPairs += handler.getNumExitedValidatorDelegators(validator);
        }
        console2.log("Total exited validator-delegator pairs:", totalExitedValidatorDelegatorPairs);

        console2.log("Total active delegations:", handler.totalActiveDelegations());
        console2.log("Total active undelegations:", handler.totalActiveUndelegations());
        console2.log("Tracked total supply:", handler.trackedTotalSupply());
    }

    function afterInvariant() external view {
        _logFunctionStats();
        _logCurrentState();
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
