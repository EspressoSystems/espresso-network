// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import { Test } from "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// Target contracts
import { EspToken } from "../src/EspToken.sol";
import { EspTokenV2 } from "../src/EspTokenV2.sol";
import { SafeExitTimelock } from "../src/SafeExitTimelock.sol";
import { TimelockController } from "@openzeppelin/contracts/governance/TimelockController.sol";
import { IAccessControl } from "@openzeppelin/contracts/access/IAccessControl.sol";
import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract EspTokenUpgradabilityTest is Test {
    address public admin;
    address tokenGrantRecipient;
    EspToken public token;
    uint256 public initialSupply = 3_590_000_000;
    uint256 public initialSupplyEther = initialSupply * 10 ** 18;
    string public name = "Espresso";
    string public symbol = "ESP";

    function setUp() public {
        tokenGrantRecipient = makeAddr("tokenGrantRecipient");
        admin = makeAddr("admin");

        EspToken tokenImpl = new EspToken();
        bytes memory initData = abi.encodeWithSignature(
            "initialize(address,address,uint256,string,string)",
            admin,
            tokenGrantRecipient,
            initialSupply,
            name,
            symbol
        );
        ERC1967Proxy proxy = new ERC1967Proxy(address(tokenImpl), initData);
        token = EspToken(payable(address(proxy)));
    }

    // For now we just check that the contract is deployed and minted balance is as expected.

    function testDeployment() public payable {
        assertEq(token.name(), name);
        assertEq(token.symbol(), symbol);
        assertEq(token.balanceOf(tokenGrantRecipient), initialSupplyEther);
    }

    function testUpgrade() public {
        EspTokenV2 tokenV2 = new EspTokenV2();
        vm.startPrank(admin);
        token.upgradeToAndCall(address(tokenV2), "");
        vm.stopPrank();
        assertEq(token.name(), name);
        assertEq(token.symbol(), symbol);
        assertEq(token.balanceOf(tokenGrantRecipient), initialSupplyEther);
        (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion) = token.getVersion();
        assertEq(majorVersion, 2);
        assertEq(minorVersion, 0);
        assertEq(patchVersion, 0);
    }

    function test_SafeExitTimelock() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);
        assertEq(timelock.getMinDelay(), minDelaySeconds);
        assertEq(timelock.hasRole(timelock.PROPOSER_ROLE(), proposers[0]), true);
        assertEq(timelock.hasRole(timelock.EXECUTOR_ROLE(), executors[0]), true);
        assertEq(timelock.hasRole(timelock.DEFAULT_ADMIN_ROLE(), admin), true);
    }

    function test_RevertWhenExecuteBeforeDelay() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);
        vm.prank(admin);
        token.transferOwnership(address(timelock));
        assertEq(token.owner(), address(timelock));

        vm.startPrank(proposers[0]);
        bytes memory transferOwnershipData =
            abi.encodeWithSignature("transferOwnership(address)", tokenGrantRecipient);
        timelock.schedule(
            address(token), 0, transferOwnershipData, bytes32(0), bytes32(0), minDelaySeconds
        );
        bytes32 operation =
            timelock.hashOperation(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));
        vm.stopPrank();

        vm.startPrank(executors[0]);
        vm.expectRevert(
            abi.encodeWithSelector(
                TimelockController.TimelockUnexpectedOperationState.selector,
                operation,
                bytes32(1 << uint8(TimelockController.OperationState.Ready))
            )
        );
        timelock.execute(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));

        vm.warp(block.timestamp + minDelaySeconds + 1);
        vm.startPrank(executors[0]);
        timelock.execute(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));
        vm.stopPrank();

        assertEq(token.owner(), tokenGrantRecipient);
    }

    function test_RevertWhen_ExecuteByNonExecutor() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);

        vm.prank(admin);
        token.transferOwnership(address(timelock));

        vm.startPrank(proposers[0]);
        bytes memory transferOwnershipData =
            abi.encodeWithSignature("transferOwnership(address)", tokenGrantRecipient);
        timelock.schedule(
            address(token), 0, transferOwnershipData, bytes32(0), bytes32(0), minDelaySeconds
        );
        vm.stopPrank();

        vm.warp(block.timestamp + minDelaySeconds + 1);

        address nonExecutor = makeAddr("nonExecutor");
        vm.startPrank(nonExecutor);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                nonExecutor,
                timelock.EXECUTOR_ROLE()
            )
        );
        timelock.execute(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));
        vm.stopPrank();
    }

    function test_RevertWhen_ScheduleByNonProposer() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);

        vm.prank(admin);
        token.transferOwnership(address(timelock));

        address nonProposer = makeAddr("nonProposer");
        vm.startPrank(nonProposer);
        bytes memory transferOwnershipData =
            abi.encodeWithSignature("transferOwnership(address)", tokenGrantRecipient);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                nonProposer,
                timelock.PROPOSER_ROLE()
            )
        );
        timelock.schedule(
            address(token), 0, transferOwnershipData, bytes32(0), bytes32(0), minDelaySeconds
        );
        vm.stopPrank();
    }

    function test_UpdateMinDelay() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);

        uint256 newDelay = 20;

        // Schedule the delay update through the timelock itself
        vm.startPrank(proposers[0]);
        bytes memory updateDelayData = abi.encodeWithSignature("updateDelay(uint256)", newDelay);
        timelock.schedule(
            address(timelock), 0, updateDelayData, bytes32(0), bytes32(0), minDelaySeconds
        );
        vm.stopPrank();

        // Wait for the delay period
        vm.warp(block.timestamp + minDelaySeconds + 1);

        // Execute the delay update
        vm.startPrank(executors[0]);
        timelock.execute(address(timelock), 0, updateDelayData, bytes32(0), bytes32(0));
        vm.stopPrank();

        assertEq(timelock.getMinDelay(), newDelay);
    }

    function test_RevertWhen_NonExecutorUpdatesDelay() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);

        uint256 newDelay = 20;

        // Schedule the delay update through the timelock itself
        vm.startPrank(proposers[0]);
        bytes memory updateDelayData = abi.encodeWithSignature("updateDelay(uint256)", newDelay);
        timelock.schedule(
            address(timelock), 0, updateDelayData, bytes32(0), bytes32(0), minDelaySeconds
        );
        vm.stopPrank();

        // Wait for the delay period
        vm.warp(block.timestamp + minDelaySeconds + 1);

        // Try to have a non-executor execute the delay update
        vm.startPrank(admin);
        vm.expectRevert(
            abi.encodeWithSelector(
                IAccessControl.AccessControlUnauthorizedAccount.selector,
                admin,
                timelock.EXECUTOR_ROLE()
            )
        );
        timelock.execute(address(timelock), 0, updateDelayData, bytes32(0), bytes32(0));
        vm.stopPrank();
    }

    function test_RevertWhen_AdminUpdatesDelayDirectly() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);

        // Even admin cannot update delay directly
        vm.startPrank(admin);
        vm.expectRevert(
            abi.encodeWithSelector(TimelockController.TimelockUnauthorizedCaller.selector, admin)
        );
        timelock.updateDelay(20);
        vm.stopPrank();
    }

    function test_RevertWhenInvalidProposal() public {
        uint256 minDelaySeconds = 10;
        address[] memory proposers = new address[](1);
        address[] memory executors = new address[](1);
        proposers[0] = makeAddr("proposer");
        executors[0] = makeAddr("executor");
        SafeExitTimelock timelock =
            new SafeExitTimelock(minDelaySeconds, proposers, executors, admin);
        vm.prank(admin);
        token.transferOwnership(address(timelock));
        assertEq(token.owner(), address(timelock));

        vm.startPrank(proposers[0]);
        address newOwner = address(0);
        bytes memory transferOwnershipData =
            abi.encodeWithSignature("transferOwnership(address)", newOwner);
        timelock.schedule(
            address(token), 0, transferOwnershipData, bytes32(0), bytes32(0), minDelaySeconds
        );
        vm.stopPrank();

        vm.warp(block.timestamp + minDelaySeconds + 100);

        // get the operation state of the scheduled operation
        bytes32 operation =
            timelock.hashOperation(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));
        TimelockController.OperationState operationState = timelock.getOperationState(operation);
        assert(operationState == TimelockController.OperationState.Ready);

        vm.startPrank(executors[0]);
        vm.expectRevert(
            abi.encodeWithSelector(OwnableUpgradeable.OwnableInvalidOwner.selector, newOwner)
        );
        timelock.execute(address(token), 0, transferOwnershipData, bytes32(0), bytes32(0));
        vm.stopPrank();
    }
}
