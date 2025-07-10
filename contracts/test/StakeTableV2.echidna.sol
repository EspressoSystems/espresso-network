// SPDX-License-Identifier: MIT
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
    IHevm public constant vm = IHevm(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    constructor() {
        _deployStakeTable();
        _mintAndApprove();

        vm.prank(VALIDATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(VALIDATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR2);
        token.approve(address(stakeTable), type(uint256).max);
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
        ) = _generateValidatorKeys(validator);

        vm.prank(validator);
        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) { } catch { }
    }

    function delegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) { } catch { }
    }

    // Functions ensures we are doing a reasonable amount of successful delegations
    function delegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        amount = amount % (token.balanceOf(delegator) + 1);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) { } catch { }
    }

    function undelegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount)
        public
    {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) { } catch { }
    }

    // Functions ensures we are doing a reasonable amount of successful undelegations
    function undelegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        amount = amount % (stakeTable.delegations(validator, delegator) + 1);
        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) { } catch { }
    }

    function claimWithdrawal(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.claimWithdrawal(validator) { } catch { }
    }

    function deregisterValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        vm.prank(validator);
        try stakeTable.deregisterValidator() { } catch { }
    }

    function claimValidatorExit(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.claimValidatorExit(validator) { } catch { }
    }

    function echidna_balance_invariant_validator1() public view returns (bool) {
        return getTotalBalance(VALIDATOR1) == initialBalances[VALIDATOR1];
    }

    function echidna_balance_invariant_validator2() public view returns (bool) {
        return getTotalBalance(VALIDATOR2) == initialBalances[VALIDATOR2];
    }

    function echidna_balance_invariant_delegator1() public view returns (bool) {
        return getTotalBalance(DELEGATOR1) == initialBalances[DELEGATOR1];
    }

    function echidna_balance_invariant_delegator2() public view returns (bool) {
        return getTotalBalance(DELEGATOR2) == initialBalances[DELEGATOR2];
    }

    function echidna_total_supply_invariant() public view returns (bool) {
        return _getTotalSupply() == INITIAL_BALANCE * 4;
    }

    function echidna_contract_balance_matches_delegations() public view returns (bool) {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalTracked = _getTotalTrackedFunds();
        return contractBalance == totalTracked;
    }
}
