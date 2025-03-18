// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import { PermissionedStakeTable } from "../src/PermissionedStakeTable.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { InitializedAt } from "../src/InitializedAt.sol";

contract PermissionedStakeTableTest is Test {
    PermissionedStakeTable stakeTable;
    address owner = address(1);

    function setUp() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory initialStakers = createNodes(0, 1);
        stakeTable = new PermissionedStakeTable(initialStakers);
    }

    // Create `numNodes` node IDs from `start` for testing.
    function createNodes(uint64 start, uint64 numNodes)
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

    // Convert NodeInfo array to BLS keys array
    function toBls(PermissionedStakeTable.NodeInfo[] memory nodes)
        private
        pure
        returns (BN254.G2Point[] memory)
    {
        BN254.G2Point[] memory bls = new BN254.G2Point[](nodes.length);
        for (uint64 i = 0; i < nodes.length; i++) {
            bls[i] = nodes[i].blsVK;
        }
        return bls;
    }

    // Empty array of NodeInfo
    // solhint-disable-next-line no-empty-blocks
    function emptyNodes() private pure returns (PermissionedStakeTable.NodeInfo[] memory nodes) { }

    // Empty array of BLS keys
    // solhint-disable-next-line no-empty-blocks
    function emptyKeys() private pure returns (BN254.G2Point[] memory keys) { }

    function testDeploymentStoresBlockNumber() public {
        setUp();
        assertEq(stakeTable.initializedAtBlock(), block.number);
    }

    function testInsert() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = createNodes(1, 1);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(emptyKeys(), stakers);

        stakeTable.update(emptyKeys(), stakers);

        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertMany() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = createNodes(1, 10);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(emptyKeys(), stakers);

        stakeTable.update(emptyKeys(), stakers);

        assertTrue(stakeTable.isStaker(stakers[0].blsVK));
    }

    function testInsertRevertsIfStakerExists() public {
        vm.prank(owner);
        PermissionedStakeTable.NodeInfo[] memory stakers = createNodes(1, 1);
        stakeTable.update(emptyKeys(), stakers);

        // Try adding the same staker again
        vm.expectRevert(
            abi.encodeWithSelector(
                PermissionedStakeTable.StakerAlreadyExists.selector, stakers[0].blsVK
            )
        );
        vm.prank(owner);
        stakeTable.update(emptyKeys(), stakers);
    }

    function testRemove() public {
        PermissionedStakeTable.NodeInfo[] memory stakersToInsert = createNodes(1, 1);
        BN254.G2Point[] memory keysToRemove = toBls(stakersToInsert);
        vm.prank(owner);

        // Insert the stakers we want to remove later.
        stakeTable.update(emptyKeys(), stakersToInsert);

        vm.prank(owner);

        vm.expectEmit();
        emit PermissionedStakeTable.StakersUpdated(keysToRemove, emptyNodes());

        stakeTable.update(keysToRemove, emptyNodes());

        assertFalse(stakeTable.isStaker(keysToRemove[0]));
    }

    function testRemoveRevertsIfStakerNotFound() public {
        vm.prank(owner);
        BN254.G2Point[] memory keysToRemove = toBls(createNodes(1, 1));
        vm.expectRevert(
            abi.encodeWithSelector(PermissionedStakeTable.StakerNotFound.selector, keysToRemove[0])
        );
        // Attempt to remove a non-existent staker
        stakeTable.update(keysToRemove, emptyNodes());
    }

    function testNonOwnerCannotInsert() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        PermissionedStakeTable.NodeInfo[] memory stakers = createNodes(1, 1);
        stakeTable.update(emptyKeys(), stakers);
    }

    function testNonOwnerCannotRemove() public {
        vm.prank(address(2));
        vm.expectRevert(
            abi.encodeWithSelector(Ownable.OwnableUnauthorizedAccount.selector, address(2))
        );
        BN254.G2Point[] memory keys = toBls(createNodes(1, 1));
        stakeTable.update(keys, emptyNodes());
    }
}
