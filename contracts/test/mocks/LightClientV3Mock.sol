// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { LightClient as LC } from "../../src/LightClient.sol";
import { LightClientV3 as LCV3 } from "../../src/LightClientV3.sol";
import { IPlonkVerifier } from "../../src/interfaces/IPlonkVerifier.sol";
import { PlonkVerifierV3 as PV } from "../../src/libraries/PlonkVerifierV3.sol";

/// @notice The only differences with LightClientV2Mock are the `_getVk()` and inheritance from LCV3
/// instead of LCV2.
contract LightClientV3Mock is LCV3 {
    bool internal hotShotDown;
    uint256 internal frozenL1Height;

    /// copy from LightClientMock.sol
    function setHotShotDownSince(uint256 l1Height) public {
        hotShotDown = true;
        frozenL1Height = l1Height;
    }
    /// copy from LightClientMock.sol

    function setHotShotUp() public {
        hotShotDown = false;
    }

    /// @dev override the production-implementation with frozen data
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 threshold)
        public
        view
        override
        returns (bool)
    {
        return hotShotDown
            ? blockNumber - frozenL1Height > threshold
            : super.lagOverEscapeHatchThreshold(blockNumber, threshold);
    }

    /// @dev Directly mutate finalizedState variable for test
    function setFinalizedState(LC.LightClientState memory state) public {
        finalizedState = state;
        updateStateHistory(uint64(block.number), uint64(block.timestamp), state);
    }

    /// @dev Directly mutate votingStakeTableState variable for test
    function setVotingStakeTableState(LC.StakeTableState memory stake) public {
        votingStakeTableState = stake;
    }

    /// @dev same as LCV1Mock
    function setStateHistory(StateHistoryCommitment[] memory _stateHistoryCommitments) public {
        // delete the previous stateHistoryCommitments
        delete stateHistoryCommitments;

        // Set the stateHistoryCommitments to the new values
        for (uint256 i = 0; i < _stateHistoryCommitments.length; i++) {
            stateHistoryCommitments.push(_stateHistoryCommitments[i]);
        }
    }

    function setBlocksPerEpoch(uint64 newBlocksPerEpoch) public {
        blocksPerEpoch = newBlocksPerEpoch;
    }

    // generated and copied from `cargo run --bin gen-vk-contract --release -- --mock`
    function _getVk() public pure override returns (IPlonkVerifier.VerifyingKey memory vk) {
        assembly {
            // domain size
            mstore(vk, 65536)
            // num of public inputs
            mstore(add(vk, 0x20), 5)

            // sigma0
            mstore(
                mload(add(vk, 0x40)),
                705833326226136011613973268744241337385248070294834671175011446752249752757
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                5532176181504301342400052516202631677965449190925412493847562736709226200234
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                9099686211961018792809235930812170135492087630134427525908890558996754794153
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                19570924693133768520868810101007969745847050516817323637918187106502005035298
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                11026226343434330782792705383760239404675735069053341585452110116462174746099
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                7982955670391258494122674236772681615694623604270955684104734375461071887933
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                21557654978558647209056189784427921970975037795147313595418436511489588399365
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                5195538490346942719135303116229159473722335454407496045708985620692394281802
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                19394790675468168483677357494957280912861874379776709509823052949744022607936
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                5552259934472006538113971199912369919430253902310388010754466153465911997154
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                1820498731744016826870662019917539993301771640710341211179691422973647664331
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                6253285704114952797993126337259335488224404618461084154587538677017834566384
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                6028350708767171878209540712452773552095132435407754671629147884447427770808
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                5445997894917639096688762751152579046809436460629999520344645243674344803436
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                2369571646585119690723110623179408275843782873937993627691746593662833875762
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                2176611442449594644923503498273103653028357007010192595482190906072406820536
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                7800906905490010801009021062614722556604062824615215112092303976330330852280
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                2628478188225066178751254191341674895059831453549271995875437681597538402091
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                4800664830661829904845818792763957041235891502718479149323211875027258950430
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                645526482286771336893024822595549124630263248468775315759678754861872244922
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                8250507429404186899916188256132800731630148711721282640636553567085662115116
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                4025373137671989730677906743876605182781693794957191549061607076241888058818
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                11012870724819742288816849889479816048166053622567455652643878415854231518082
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                4854942088598758975530324490292406548401171290797860533969986879330652606915
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                17291240031988789923730980556099797877577791306295398033735278743444404336809
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                14038324264906088100578751917963891300025518948183951359747517141310674646086
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                21479268503279800727286782230641392323579891088774828631022559970305213962846
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                9787009121801224106747203570920094844135252743511206811556279949206372862775
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                10482997848711105668219269591746944989639095509685250007935429187779072996894
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                8909534723210990277634000435741529268240605588724314657454195929201332741609
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                5406586919454289790496244239526321542379166703709057392203377170837328621027
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                9799690090922242414356823622591935958993846082390627304215218024535785874019
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                4461400223619786239875799787252722740651756526644996430334537323489864281754
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                21201652053468047659550599944924817606397753628779457222087132408363047893707
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                1943417439988505551227782709961658217286217229339882663003143584563355477830
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                7359298723482050623495023291008711429740005981006345479094239313225186929215
            )
            // g2LSB
            mstore(
                add(vk, 0x280), 0xb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c41801
            )
            // g2MSB
            mstore(
                add(vk, 0x2A0), 0xc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e26
            )
        }
    }

    function firstEpoch() public view returns (uint64) {
        return _firstEpoch;
    }
}
