// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import { PermissionedStakeTable } from "../src/PermissionedStakeTable.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";

contract PermissionedStakeTableTest is Test {
    PermissionedStakeTable stakeTable;
    address owner = address(1);

    function setUp() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory initialStakers = nodes(0, 1);
        stakeTable = new PermissionedStakeTable(initialStakers);
    }

    // Create `numNodes` node IDs from `start` for testing.
    function nodes(uint64 start, uint64 numNodes)
        private
        returns (PermissionedStakeTable.NodeInfo[] memory)
    {
        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "gen-random-g2-point";

        PermissionedStakeTable.NodeInfo[] memory ps =
            new PermissionedStakeTable.NodeInfo[](numNodes);

        for (uint64 i = 0; i < numNodes; i++) {
            cmds[2] = vm.toString(start + 1 + i);
            bytes memory result = vm.ffi(cmds);
            BN254.G2Point memory bls = abi.decode(result, (BN254.G2Point));
            ps[i] = PermissionedStakeTable.NodeInfo(bls, EdOnBN254.EdOnBN254Point(0, 1), true);
        }
        return ps;
    }

    function testInsert() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(empty, stakers);

        stakeTable.update(empty, stakers);

        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertMany() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 10);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(empty, stakers);

        stakeTable.update(empty, stakers);

        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertRevertsIfStakerExists() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);
        stakeTable.update(empty, stakers);

        // Try adding the same staker again
        vm.expectRevert(
            abi.encodeWithSelector(
                PermissionedStakeTable.StakerAlreadyExists.selector, stakers[0].blsVK
            )
        );
        vm.prank(owner);
        stakeTable.update(empty, stakers);
    }

    function testRemove() public {
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);
        vm.prank(owner);
        stakeTable.update(empty, stakers);

        vm.prank(owner);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(stakers, empty);

        stakeTable.update(stakers, empty);

        assertFalse(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testRemoveRevertsIfStakerNotFound() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);
        vm.expectRevert(
            abi.encodeWithSelector(PermissionedStakeTable.StakerNotFound.selector, stakers[0].blsVK)
        );
        // Attempt to remove a non-existent staker
        stakeTable.update(stakers, empty);
    }

    function testNonOwnerCannotInsert() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);
        stakeTable.update(empty, stakers);
    }

    function testNonOwnerCannotRemove() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        PermissionedStakeTable.NodeInfo[] memory stakers = nodes(1, 1);
        PermissionedStakeTable.NodeInfo[] memory empty = nodes(1, 0);
        stakeTable.update(stakers, empty);
    }
}
