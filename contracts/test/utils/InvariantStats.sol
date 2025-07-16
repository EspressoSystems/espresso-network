// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase */
pragma solidity ^0.8.0;

import { console2 } from "forge-std/console2.sol";
import { StakeTableV2PropTestBase } from "../StakeTableV2PropTestBase.sol";

// Contract containing helper functions for displaying stats
contract InvariantStats {
    StakeTableV2PropTestBase internal handler;

    constructor(StakeTableV2PropTestBase _handler) {
        handler = _handler;
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

    function _logStat(string memory name, StakeTableV2PropTestBase.FuncStats memory stat)
        internal
        pure
    {
        console2.log(
            string.concat(
                _formatString(name, 29),
                _formatNumber(stat.ok, 9),
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

    function logFunctionStats() public view {
        console2.log("\n=== Call stats for last invariant run ===");
        console2.log("function                     successes  reverts");
        console2.log("-----------------------------------------------");

        // Get call stats via getter function
        StakeTableV2PropTestBase.CallStats memory callStats = handler.getCallStats();

        _logStat("advanceTime", callStats.ok.advanceTime);
        _logStat("claimValidatorExitOk", callStats.ok.claimValidatorExit);
        _logStat("claimWithdrawalOk", callStats.ok.claimWithdrawal);
        _logStat("createActor", callStats.ok.createActor);
        _logStat("createValidator", callStats.ok.createValidator);
        _logStat("delegateOk", callStats.ok.delegate);
        _logStat("deregisterValidatorOk", callStats.ok.deregisterValidator);
        _logStat("undelegateOk", callStats.ok.undelegate);

        console2.log("-----------------------------------------------");

        _logStat("claimValidatorExitAny", callStats.any.claimValidatorExit);
        _logStat("delegateAny", callStats.any.delegate);
        _logStat("deregisterValidatorAny", callStats.any.deregisterValidator);
        _logStat("registerValidatorAny", callStats.any.registerValidator);
        _logStat("undelegateAny", callStats.any.undelegate);

        console2.log("-----------------------------------------------");
        console2.log(
            string.concat(
                "total                        ",
                _formatNumber(handler.getTotalSuccesses(), 9),
                " ",
                _formatNumber(handler.getTotalReverts(), 7)
            )
        );
    }

    function logCurrentState() public view {
        console2.log("\n=== Current State ===");
        console2.log("Num actors:", handler.getNumActors());
        console2.log("Num all validators:", handler.getNumAllValidators());
        console2.log("Num active validators:", handler.getNumActiveValidators());
        console2.log("Num pending withdrawals:", handler.getNumPendingWithdrawals());
        console2.log("Num validators with delegations:", handler.getNumValidatorsWithDelegations());

        // Count total validator-delegator pairs
        uint256 totalValidatorDelegatorPairs = 0;
        for (uint256 i = 0; i < handler.getNumValidatorsWithDelegations(); i++) {
            (, uint256 numDelegators) = handler.getValidatorWithDelegationsAtIndex(i);
            totalValidatorDelegatorPairs += numDelegators;
        }
        console2.log("Total validator-delegator pairs:", totalValidatorDelegatorPairs);
        console2.log("Num exited validators:", handler.getNumExitedValidators());
        console2.log("Total active delegations:", handler.getTestState().totalDelegated);
        console2.log("Total pending withdrawals:", handler.getTestState().totalPendingWithdrawals);
        console2.log("Tracked total supply:", handler.getTestState().trackedTotalSupply);
    }
}
