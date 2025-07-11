// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import { StakeTableV2PropTestBase } from "./StakeTableV2PropTestBase.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";

interface IHevm {
    function prank(address) external;
    function startPrank(address) external;
    function stopPrank() external;
}

contract StakeTableV2EchidnaTest is StakeTableV2PropTestBase {
    IHevm public constant VM = IHevm(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    constructor() {
        _deployStakeTable();
        _mintAndApprove();
    }

    function registerValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

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

        VM.prank(validator);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
    }

    function delegateAny(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        VM.prank(delegator);
        stakeTable.delegate(validator, amount);
    }

    // Functions ensures we are doing a reasonable amount of successful delegations
    function delegateOk(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        amount = amount % (token.balanceOf(delegator) + 1);

        VM.prank(delegator);
        stakeTable.delegate(validator, amount);
    }

    function undelegateAny(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        VM.prank(delegator);
        stakeTable.undelegate(validator, amount);
    }

    // Functions ensures we are doing a reasonable amount of successful undelegations
    function undelegateOk(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        amount = amount % (stakeTable.delegations(validator, delegator) + 1);
        VM.prank(delegator);
        stakeTable.undelegate(validator, amount);
    }

    function claimWithdrawal(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        VM.prank(delegator);
        stakeTable.claimWithdrawal(validator);
    }

    function deregisterValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        VM.prank(validator);
        stakeTable.deregisterValidator();
    }

    function claimValidatorExit(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        VM.prank(delegator);
        stakeTable.claimValidatorExit(validator);
    }

    function echidna_BalanceInvariantValidator1() public view returns (bool) {
        return totalOwnedAmount(VALIDATOR1) == initialBalances[VALIDATOR1];
    }

    function echidna_BalanceInvariantValidator2() public view returns (bool) {
        return totalOwnedAmount(VALIDATOR2) == initialBalances[VALIDATOR2];
    }

    function echidna_BalanceInvariantDelegator1() public view returns (bool) {
        return totalOwnedAmount(DELEGATOR1) == initialBalances[DELEGATOR1];
    }

    function echidna_BalanceInvariantDelegator2() public view returns (bool) {
        return totalOwnedAmount(DELEGATOR2) == initialBalances[DELEGATOR2];
    }

    function echidna_TotalSupplyInvariant() public view returns (bool) {
        return _getTotalSupply() == INITIAL_BALANCE * 4;
    }

    function echidna_ContractBalanceMatchesDelegations() public view returns (bool) {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalTracked = _getTotalTrackedFunds();
        return contractBalance == totalTracked;
    }
}
