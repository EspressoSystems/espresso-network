// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import { Test } from "forge-std/Test.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

// Target contract
import { FeeContract } from "../src/FeeContract.sol";
import { DeployFeeContractScript } from "./script/Fee.s.sol";

/// @title FeeContract Test
contract FeeContractUpgradabilityTest is Test {
    address payable public proxy;
    address public admin;
    FeeContract public feeContractProxy;

    function setUp() public {
        DeployFeeContractScript deployer = new DeployFeeContractScript();
        (proxy, admin) = deployer.run();
        feeContractProxy = FeeContract(proxy);
    }

    //test deposits work with a proxy
    function testFuzz_deposit(address user, uint256 amount) public payable {
        vm.assume(user != address(0));
        amount =
            bound(amount, feeContractProxy.minDepositAmount(), feeContractProxy.maxDepositAmount());

        uint256 balanceBefore = feeContractProxy.balances(user);

        //check that the depositEvent is emitted
        vm.expectEmit(true, false, false, true);
        // We emit the event we expect to see.
        emit FeeContract.Deposit(user, amount);

        //deposit for the user
        feeContractProxy.deposit{ value: amount }(user);

        //get the balance for that user after the deposit
        uint256 balanceAfter = feeContractProxy.balances(user);
        assertEq(balanceAfter, balanceBefore + amount);
    }

    // test depositing for many users
    function test_depositForManyDifferentUsers() public payable {
        for (uint256 i = 0; i < 10; i++) {
            address user = makeAddr(string(abi.encode(i)));
            uint256 amount = i + feeContractProxy.minDepositAmount();

            //fund this account
            vm.deal(user, amount);

            //check the balance before
            uint256 balanceBefore = feeContractProxy.balances(user);

            //prank as if the deposit is made by the user
            vm.prank(user);

            //deposit for the user
            feeContractProxy.deposit{ value: amount }(user);

            //get the balance for that user after the deposit
            uint256 balanceAfter = feeContractProxy.balances(user);
            assertEq(balanceAfter, balanceBefore + amount);
        }
    }

    // test depositing for the same user many times
    function test_depositManyTimesForTheSameUser() public payable {
        address user = makeAddr("user");

        //fund this account
        vm.deal(user, 1 ether);

        uint256 totalAmountDeposited = 0;

        for (uint256 i = 0; i < 10; i++) {
            uint256 amount = i + feeContractProxy.minDepositAmount();

            //check the balance before
            uint256 balanceBefore = feeContractProxy.balances(user);

            //prank as if the deposit is made by the user
            vm.prank(user);

            //deposit for the user
            feeContractProxy.deposit{ value: amount }(user);

            //get the balance for that user after the deposit
            uint256 balanceAfter = feeContractProxy.balances(user);
            assertEq(balanceAfter, balanceBefore + amount);
            totalAmountDeposited += amount;
        }

        //affirm that the totalAmountDeposited is the user's current balance
        assertEq(feeContractProxy.balances(user), totalAmountDeposited);
    }

    // test calling no function with a payable amount is not successful
    function testFuzz_noFunction(uint256 amount) public payable {
        address fcAddress = address(feeContractProxy);
        (bool success,) = fcAddress.call{ value: amount }("");

        //assert that the transaction was not successful
        assertFalse(success);

        //assert that the balance of the fee contract is still zero
        assertEq(address(feeContractProxy).balance, 0);
    }

    // test calling a function that does not exist  is not successful
    function testFuzz_nonExistentFunction(uint256 amount) public payable {
        address fcAddress = address(feeContractProxy);
        (bool success,) =
            fcAddress.call{ value: amount }(abi.encodeWithSignature("withdraw(address)", "0x"));

        //assert that the transaction was not successful
        assertFalse(success);

        //assert that the balance of the fee contract is still zero
        assertEq(address(feeContractProxy).balance, 0);
    }

    // test upgrading with admin account succeeds
    function testUpgradeTo() public {
        FeeContractV2Test feeContractV2 = new FeeContractV2Test();

        vm.prank(admin);
        vm.expectEmit(false, false, false, true);
        // We emit the event we expect to see.
        emit FeeContract.Upgrade(address(feeContractV2));

        feeContractProxy.upgradeToAndCall(address(feeContractV2), "");
    }

    // test upgrading with wrong user account does not succeed
    function testUpgradeToWithWrongAdmin() public {
        FeeContractV2Test feeContractV2 = new FeeContractV2Test();

        //start the upgrade with another address which isn't the admin
        address otherUser = makeAddr("otherUser");
        vm.prank(otherUser);
        vm.expectRevert(
            abi.encodeWithSelector(
                OwnableUpgradeable.OwnableUnauthorizedAccount.selector, otherUser
            )
        );
        feeContractProxy.upgradeToAndCall(address(feeContractV2), "");
    }

    //test deposits with a large amount reverts
    function test_depositMaxAmount() public {
        address user = makeAddr("user");
        uint256 amount = feeContractProxy.maxDepositAmount() + 1;

        vm.expectRevert(FeeContract.DepositTooLarge.selector);

        //deposit for the user
        feeContractProxy.deposit{ value: amount }(user);
    }

    //test deposits with a less than the min amount reverts
    function test_depositMinAmount() public {
        address user = makeAddr("user");
        uint256 amount = feeContractProxy.minDepositAmount() - 0.0001 ether;

        vm.expectRevert(FeeContract.DepositTooSmall.selector);

        //deposit for the user
        feeContractProxy.deposit{ value: amount }(user);
    }

    //test deposits with invalid user address reverts
    function test_invalidUserAddress() public {
        address user = address(0);
        uint256 amount = 0.5 ether;

        vm.expectRevert(FeeContract.InvalidUserAddress.selector);

        //deposit for the user
        feeContractProxy.deposit{ value: amount }(user);
    }

    // test that new users have a zero balance
    function testFuzz_newUserHasZeroBalance(address user) public view {
        vm.assume(user != address(0));

        uint256 balance = feeContractProxy.balances(user);

        assertEq(balance, 0);
    }

    // test renounceOwnership reverts
    function test_renounceOwnership_Reverts() public {
        vm.prank(admin);
        vm.expectRevert(FeeContract.OwnershipCannotBeRenounced.selector);
        feeContractProxy.renounceOwnership();
    }

    // test renounceOwnership by non-owner reverts
    function test_renounceOwnership_ByNonOwnerReverts() public {
        address otherUser = makeAddr("otherUser");
        vm.prank(otherUser);
        vm.expectRevert(
            abi.encodeWithSelector(
                OwnableUpgradeable.OwnableUnauthorizedAccount.selector, otherUser
            )
        );
        feeContractProxy.renounceOwnership();
    }
}

contract FeeContractV2Test is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);

    /// @notice since the constructor initializes storage on this contract we disable it
    /// @dev storage is on the proxy contract since it calls this contract via delegatecall
    constructor() {
        _disableInitializers();
    }

    /// @notice This contract is called by the proxy when you deploy this contract
    function initialize() public initializer {
        __Ownable_init(msg.sender); //sets owner to msg.sender
        __UUPSUpgradeable_init();
    }

    /// @notice only the owner can authorize an upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        emit Upgrade(newImplementation);
    }
}
