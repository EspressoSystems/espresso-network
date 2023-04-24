pragma solidity ^0.8.0;

import {BN254} from "bn254/BN254.sol";
import {BLSSig} from "./libraries/BLSSig.sol";
// import {BN256} from "solidity-BN256G2/BN256G2.sol";

contract HotShot {
    uint256 public constant MAX_BLOCKS = 1000;
    mapping(uint256 => uint256) public commitments;
    uint256 public blockHeight;

    // Stake table related data structures
    mapping(uint256 => uint256) private stakeAmounts;
    BN254.G2Point[] private stakingKeys;

    event NewBlocks(uint256 firstBlockNumber, uint256 numBlocks);

    error WrongNumberOfQCs(uint256 numBlocks, uint256 numQCs);
    error TooManyBlocks(uint256 numBlocks);
    error InvalidQC(uint256 blockNumber);

    function verifyQC(uint256, /*blockNumber*/ uint256, /*commitment*/ bytes calldata /*qc*/ )
        private
        pure
        returns (bool)
    {
        // TODO Check the QC
        // TODO Check the block number
        return true;
    }

    function newBlocks(uint256[] calldata newCommitments, bytes[] calldata qcs) external {
        if (newCommitments.length != qcs.length) {
            revert WrongNumberOfQCs(newCommitments.length, qcs.length);
        }
        if (newCommitments.length > MAX_BLOCKS) {
            revert TooManyBlocks(newCommitments.length);
        }

        uint256 firstBlockNumber = blockHeight;
        for (uint256 i = 0; i < newCommitments.length; ++i) {
            if (!verifyQC(blockHeight, newCommitments[i], qcs[i])) {
                revert InvalidQC(blockHeight);
            }

            commitments[blockHeight] = newCommitments[i];
            blockHeight += 1;
        }

        emit NewBlocks(firstBlockNumber, newCommitments.length);
    }

    // Stake table related functions
    function addNewStakingKey(BN254.G2Point memory staking_key, uint256 amount) public {
        uint256 index = stakingKeys.length;
        stakeAmounts[index] = amount;
        stakingKeys.push(staking_key);
    }

    function getStakingKey(uint256 index) public view returns (BN254.G2Point memory, uint256) {
        return (stakingKeys[index], stakeAmounts[index]);
    }

    // TODO document
    function verify_agg_sig(bytes memory message, BN254.G1Point memory sig, uint256[] memory bitmap) public {
        // Build aggregated public key

        // Loop until we find a one in the bitmap
        uint256 index = 0;
        while (bitmap[index] == 0 && index < bitmap.length) {
            index++;
        }

        // TODO test
        require(index < bitmap.length, "At least one key must be selected.");

        //        BN254.G2Point agg_pk = stakingKeys[index];
        //        for (int i=index;i<bitmap.length;i++){
        //            // Compute the group multiplication of the two keys
        //
        //            if (bitmap[i] == 1) {
        //                BN254.G2Point memory pk = stakingKeys[i];
        //
        //                uint256 p1xx = agg_pk.x0;
        //                uint256 p1xy = agg_pk.x1;
        //                uint256 p1yx = agg_pk.y0;
        //                uint256 p1yy = agg_pk.y1;
        //                uint256 p2xx = pk.x0;
        //                uint256 p2xy = pk.x1;
        //                uint256 p2yx = pk.y0;
        //                uint256 p2yy = pk.y1;
        //
        //                (uint256 p3xx, uint256 p3xy, uint256 p3yx, uint256 p3yy) = ECTwistAdd(p1xx,p1xy,p1yx,p1yy,p2xx, p2xy,p2yy);
        //                agg_pk = BN254.G2Point(p3xx,p3xy, p3yx,p3yy);
        //
        //            }
        //        }
    }
}
