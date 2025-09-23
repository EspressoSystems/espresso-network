// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { BN254 } from "bn254/BN254.sol";
import { LightClient as LC } from "../../src/LightClient.sol";
import { LightClientV2 as LCV2 } from "../../src/LightClientV2.sol";
import { IPlonkVerifier } from "../../src/interfaces/IPlonkVerifier.sol";
import { PlonkVerifierV2 as PV } from "../../src/libraries/PlonkVerifierV2.sol";

contract LightClientV2Mock is LCV2 {
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
            mstore(add(vk, 0x20), 11)

            // sigma0
            mstore(
                mload(add(vk, 0x40)),
                7516094699371900076654618062289447453916839050780620325716956832656984860660
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                20806112153066331416832484866930710640147884494141638060080934601569355756448
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                9317717291706745111276019053203642470886570481016578175682927350152876450888
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                18781724586650726267645468577119848013127512516513886867960294068863197700288
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                14081427263673252657775936000561823294112601964070048844845352940476092452455
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                13876052175476266886103151901895245325298942861232556397206635469948361100184
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                8382815868076011633843894606922241535576166975891196181243354437187663633154
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                5168254766187915143712017615324420436736654822510808010556364714020714944790
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                5484236835825854247997292431744487766497831897197445585455398010168055498969
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                15435247211104967722842153867319657548029887758796573588901910170935965608013
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                4626547019366514791940071689436055319735636258313650407188460941005075613924
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                16098533624240821315220948337841320095681147136851367101342233261169932775967
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                10823958560114947324943775442005386726799499866066776680128823719134967015177
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                7651863518089257675898083696044743753670867518104935100401274301172319144463
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                7071336756326622455773427057971174702476739324762108801104174700460089749081
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                11542108630217642762097488778115906748638570853625016161735668620763851192059
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                10934835698201396955522061804380375216316559903990861893964267614281377526893
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                6112350972741517370384700070977966726397474419257626553848393098914859166769
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                20183417107240469996355324332906758480135635906743446200251617123140840624090
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                2366624215904973163634335191812961627814331144711898529373204621895164963305
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                19921857303273963232388958871645740453392504705795644703537290564371727617593
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                8639194776836613500521567020197460206340767771564383103366563631319881637324
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                18734710688035913939248182083502159224727541618267766804813152387506239442264
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                4482880759475417150174513935905815582401860335121758263225289423051294332083
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                15768109710657932023693059366981979141281424875535828344938957029654767446547
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                20967055200235574017197758674341917448933918519424732403233845286302845027543
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                6835155554463820210670029963668090884234422743466064676129829904636715332863
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                7265950792597860686986555189492187107227937886733186783620241492028455705502
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                21096302024058252198426625291598927067457236974741065294778386524911366336195
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                16417880285575887510192948449431666399230017926632621847000994736735996856660
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                10844745343337868800123134691482643317512186765223111942538583160118671498153
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                689972845596702443165423616308501615120804536161194282271935519670611993755
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                15216853194496336267525850920539029760147396734248612489099379553532424783157
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                10435258148697174847453396238287596554539629103388727255087914127285991293175
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                4904749785738579414544663771697114595511150894364914262728157604938592520003
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                10770767966539817028865683120079112263403194783178461636435981761384359423751
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

    function getFirstEpoch() public view returns (uint64) {
        return epochFromBlockNumber(epochStartBlock, blocksPerEpoch);
    }
}
