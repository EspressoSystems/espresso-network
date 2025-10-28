// SPDX-License-Identifier: UNLICENSED

/* solhint-disable func-name-mixedcase */

pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "../mocks/MockRewardClaim.sol";
import "../../src/RewardClaim.sol";

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
        impl = address(new MockRewardClaim());
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

    function test_Initialize_RevertsZeroOwner() public {
        (bytes memory initData) = prepare(supply, address(0), lc, pauser);

        vm.expectRevert(abi.encodeWithSignature("OwnableInvalidOwner(address)", address(0)));
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
        MockRewardClaim rewardClaim = MockRewardClaim(payable(address(proxy)));

        assertEq(rewardClaim.dailyLimit(), 1);
    }
}
