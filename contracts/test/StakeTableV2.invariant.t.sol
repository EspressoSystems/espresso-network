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

    function _logStat(string memory name, StakeTableV2PropTestBase.FunctionStats memory stat)
        internal
        view
    {
        console2.log(
            string.concat(
                _formatString(name, 29),
                _formatNumber(stat.successes, 9),
                " ",
                _formatNumber(stat.reverts, 7)
            )
        );
    }

    function _formatString(string memory str, uint256 width)
        internal
        pure
        returns (string memory)
    {
        bytes memory strBytes = bytes(str);

        if (strBytes.length >= width) {
            return str;
        }

        bytes memory result = new bytes(width);

        // Copy string
        for (uint256 i = 0; i < strBytes.length; i++) {
            result[i] = strBytes[i];
        }

        // Pad with spaces
        for (uint256 i = strBytes.length; i < width; i++) {
            result[i] = " ";
        }

        return string(result);
    }

    function _logFunctionStats() internal view {
        console2.log("\n=== Function Call Statistics ===");
        console2.log("Function                     Successes  Reverts");
        console2.log("-----------------------------------------------");

        // Ok functions - access via getter function
        StakeTableV2PropTestBase.OkFunctionStats memory okStats = handler.getOkStats();

        _logStat("advanceTime", okStats.advanceTime);
        _logStat("claimValidatorExitOk", okStats.claimValidatorExitOk);
        _logStat("claimWithdrawalOk", okStats.claimWithdrawalOk);
        _logStat("createActor", okStats.createActor);
        _logStat("createValidator", okStats.createValidator);
        _logStat("delegateOk", okStats.delegateOk);
        _logStat("deregisterValidatorOk", okStats.deregisterValidatorOk);
        _logStat("undelegateOk", okStats.undelegateOk);

        console2.log("-----------------------------------------------");

        // Any functions - access via getter function
        StakeTableV2PropTestBase.AnyFunctionStats memory anyStats = handler.getAnyStats();

        _logStat("claimValidatorExitAny", anyStats.claimValidatorExitAny);
        _logStat("delegateAny", anyStats.delegateAny);
        _logStat("deregisterValidatorAny", anyStats.deregisterValidatorAny);
        _logStat("registerValidatorAny", anyStats.registerValidatorAny);
        _logStat("undelegateAny", anyStats.undelegateAny);

        console2.log("-----------------------------------------------");
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
            (address validator, uint256 numDelegators) =
                handler.getValidatorWithDelegationsAtIndex(i);
            totalValidatorDelegatorPairs += numDelegators;
        }
        console2.log("Total validator-delegator pairs:", totalValidatorDelegatorPairs);

        console2.log("Num exited validators:", handler.getNumExitedValidators());

        // Count total exited validator-delegator pairs
        uint256 totalExitedValidatorDelegatorPairs = 0;
        for (uint256 i = 0; i < handler.getNumExitedValidators(); i++) {
            address validator = handler.getExitedValidatorAtIndex(i);
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
        for (uint256 i = 0; i < handler.getNumActors(); i++) {
            address actor = handler.getActorAtIndex(i);
            assertEq(
                handler.totalOwnedAmount(actor),
                handler.initialBalances(actor),
                "Actor balance invariant violated"
            );
        }
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_ContractBalanceMatchesTrackedDelegations() public view {
        uint256 contractBalance = handler.token().balanceOf(address(handler.stakeTable()));
        uint256 totalTracked = handler.totalActiveDelegations() + handler.totalActiveUndelegations();
        assertEq(
            contractBalance,
            totalTracked,
            "Contract balance should equal active delegations + pending undelegations"
        );
    }

    /// @dev Total supply must remain constant
    function invariant_TotalSupply() public view {
        assertEq(
            handler.getTotalSupply(),
            handler.trackedTotalSupply(),
            "Total supply invariant violated"
        );
    }
}
