// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "../../src/RewardClaim.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";

contract MockERC20 is ERC20 {
    constructor(uint256 supply) ERC20("Test", "TEST") {
        _mint(msg.sender, supply);
    }
}

contract RewardClaimInitializeTest is Test {
    address owner;
    address pauser;
    address lc;
    uint256 supply;
    address impl;

    function setUp() public {
        owner = makeAddr("owner");
        pauser = makeAddr("pauser");
        lc = makeAddr("lc");
        supply = 100;
        impl = address(new RewardClaim());
    }

    function prepare(uint256 tokenSupply, address _owner, address _lightClient, address _pauser)
        internal
        returns (bytes memory initData)
    {
        address token = address(new MockERC20(tokenSupply));
        initData = abi.encodeWithSignature(
            "initialize(address,address,address,address)", _owner, token, _lightClient, _pauser
        );
    }

    function test_Initialize_RevertsZeroSupply() public {
        (bytes memory initData) = prepare(0, owner, lc, pauser);

        vm.expectRevert(RewardClaim.ZeroTotalSupply.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_RevertsLowTotalSupply() public {
        (bytes memory initData) = prepare(99, owner, lc, pauser);

        vm.expectRevert(RewardClaim.ZeroDailyLimit.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_RevertsZeroAdmin() public {
        (bytes memory initData) = prepare(supply, address(0), lc, pauser);

        vm.expectRevert(RewardClaim.ZeroAdminAddress.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_RevertsZeroToken() public {
        bytes memory initData = abi.encodeWithSignature(
            "initialize(address,address,address,address)", owner, address(0), lc, pauser
        );

        vm.expectRevert(RewardClaim.ZeroTokenAddress.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_RevertsZeroLightClient() public {
        (bytes memory initData) = prepare(supply, owner, address(0), pauser);

        vm.expectRevert(RewardClaim.ZeroLightClientAddress.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_RevertsZeroPauser() public {
        (bytes memory initData) = prepare(supply, owner, lc, address(0));

        vm.expectRevert(RewardClaim.ZeroPauserAddress.selector);
        new ERC1967Proxy(impl, initData);
    }

    function test_Initialize_SucceedsMinimumSupply() public {
        (bytes memory initData) = prepare(supply, owner, lc, pauser);

        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        assertEq(rewardClaim.dailyLimitWei(), 1);
        assertEq(rewardClaim.currentAdmin(), owner);
    }

    function test_SetDailyLimit_RevertsZeroComputedLimit() public {
        bytes memory initData = prepare(supply, owner, lc, pauser);
        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        vm.prank(owner);
        vm.expectRevert(RewardClaim.ZeroDailyLimit.selector);
        rewardClaim.setDailyLimit(1);
    }

    function test_Initialize_TotalClaimedIsZero() public {
        (bytes memory initData) = prepare(supply, owner, lc, pauser);

        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        assertEq(rewardClaim.totalClaimed(), 0);
    }

    // REQ:rc-constructor-disable
    function test_Constructor_DisablesInitializers() public {
        RewardClaim rcImpl = new RewardClaim();
        address token = address(new MockERC20(supply));
        // The implementation contract itself should reject initialize
        vm.expectRevert(Initializable.InvalidInitialization.selector);
        rcImpl.initialize(owner, token, lc, pauser);
    }

    // REQ:rc-init-oz-calls - Pausable initialized
    function test_Initialize_PausableWorks() public {
        bytes memory initData = prepare(supply, owner, lc, pauser);
        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        assertFalse(rewardClaim.paused());
        vm.prank(pauser);
        rewardClaim.pause();
        assertTrue(rewardClaim.paused());
        vm.prank(pauser);
        rewardClaim.unpause();
        assertFalse(rewardClaim.paused());
    }

    // REQ:rc-init-oz-calls - AccessControl initialized
    function test_Initialize_AccessControlWorks() public {
        bytes memory initData = prepare(supply, owner, lc, pauser);
        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        assertTrue(rewardClaim.hasRole(rewardClaim.DEFAULT_ADMIN_ROLE(), owner));
        assertTrue(rewardClaim.hasRole(rewardClaim.PAUSER_ROLE(), pauser));
    }

    // REQ:rc-grant-role-first-admin - grantRole handles oldAdmin == address(0)
    function test_Initialize_GrantRoleFirstAdmin() public {
        bytes memory initData = prepare(supply, owner, lc, pauser);
        ERC1967Proxy proxy = new ERC1967Proxy(impl, initData);
        RewardClaim rewardClaim = RewardClaim(payable(address(proxy)));

        // After initialize, currentAdmin should be set
        assertEq(rewardClaim.currentAdmin(), owner);
        bytes32 adminRole = rewardClaim.DEFAULT_ADMIN_ROLE();
        assertTrue(rewardClaim.hasRole(adminRole, owner));

        // Transfer admin to a new address (tests grantRole with non-zero oldAdmin)
        address newAdmin = makeAddr("newAdmin");
        vm.prank(owner);
        rewardClaim.grantRole(adminRole, newAdmin);
        assertEq(rewardClaim.currentAdmin(), newAdmin);
        assertTrue(rewardClaim.hasRole(adminRole, newAdmin));
        assertFalse(rewardClaim.hasRole(adminRole, owner));
    }
}
