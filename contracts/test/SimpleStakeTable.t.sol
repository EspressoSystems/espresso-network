// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import { SimpleStakeTable } from "../src/SimpleStakeTable.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

contract SimpleStakeTableTest is Test {
    SimpleStakeTable stakeTable;
    address owner = address(1); // mock owner address

    function setUp() public {
        vm.prank(owner); // impersonate the owner
        stakeTable = new SimpleStakeTable(owner);
    }

    function nodes(uint64 num) private returns (SimpleStakeTable.NodeInfo[] memory) {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "gen-random-g2-point";

        SimpleStakeTable.NodeInfo[] memory ps = new SimpleStakeTable.NodeInfo[](num);

        for (uint64 i = 0; i < num; i++) {
            cmds[2] = vm.toString(i + 1);
            bytes memory result = vm.ffi(cmds);
            BN254.G2Point memory bls = abi.decode(result, (BN254.G2Point));
            ps[i] = SimpleStakeTable.NodeInfo(bls, EdOnBN254.EdOnBN254Point(0, 0));
        }
        return ps;
    }

    function testInsertAndIsStaker() public {
        vm.prank(owner);
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        stakeTable.insert(stakers);
        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertAndIsStakerMany() public {
        vm.prank(owner);
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(10);
        stakeTable.insert(stakers);
        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertRevertsIfStakerExists() public {
        vm.prank(owner);
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        stakeTable.insert(stakers);

        // Try adding the same staker again
        vm.expectRevert(
            abi.encodeWithSelector(SimpleStakeTable.StakerAlreadyExists.selector, stakers[0].blsVK)
        );
        vm.prank(owner);
        stakeTable.insert(stakers);
    }

    function testRemoveAndIsNotStaker() public {
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        vm.prank(owner);
        stakeTable.insert(stakers);

        vm.prank(owner);
        stakeTable.remove(stakers);

        assertFalse(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testRemoveRevertsIfStakerNotFound() public {
        vm.prank(owner);
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        vm.expectRevert(
            abi.encodeWithSelector(SimpleStakeTable.StakerNotFound.selector, stakers[0].blsVK)
        );
        // Attempt to remove a non-existent staker
        stakeTable.remove(stakers);
    }

    function testNonOwnerCannotInsert() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        stakeTable.insert(stakers);
    }

    function testNonOwnerCannotRemove() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        SimpleStakeTable.NodeInfo[] memory stakers = nodes(1);
        stakeTable.remove(stakers);
    }
}
