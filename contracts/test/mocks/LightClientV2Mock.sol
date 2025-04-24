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
                21568523741873700538599156769763171811590077655784233382517856207887270391828
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                11016167378463266194221450403115511925903320597244053618871959577766534234199
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                6242200717789942119584373442067409085263503996185564664035012246390810049374
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                20327639590645078692272366774822595936792010953533322308923376384770099482978
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                3998473635078201473455318350368891770180585995756646799455275690643804207333
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                435348305493647541672866434425509790332355174224577939762604621477279457345
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                3841269352791570102400474603697560574291681874429561132362648675857373121779
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                840123309558033808766090647441772828266647129325327385937568594840403800402
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                11501871940539373682638729008612832388120417878548445683921885518295556769975
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                10644525444639852934520182216704987991622496935544069265579329239645763191366
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                16543641133393102728401391455145096135635252965596694307893459430506296209868
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                10202059712985677895857664453148396350957313499144632112048293576752099978030
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                12369401916991282261299727764167587198575783747286042606526892074312643775722
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                15236855189172128334513080725077144519425107247511363386735279405441654072100
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                7364193263380897455766225430107140335836933309064165348628087738763726403821
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                10520374722381796716143593580980527346574381665802339103600108246431183811624
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                18479733680309955613654283517406453256528875728320997185501564901051632514401
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                1973557680863202626378704642021529608879486716425499648594259196327586461602
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                11475995542650898524884773749776290586194546575252790479732326812771179079557
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                9567823197346095437367808396726144362887555632002875507300058078126865694257
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                7591864667761867251944520292986946642574877428359456709480720079508065575858
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                15581624153275255992703995999352372096312627850607230823477988991136629591839
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                9875671957483633815289504312106088033899947632392864245450141494609573181309
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                5131506440874409933428082397836510549082004432365899248394379477946846192863
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                2285301167483849620357003031221096419897308399537526250455000751545653588401
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                21710186780187534453410858467085268089583772741717001493902994347048055130211
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                8510055999174987348763232246028091899956646187718150438418873158988286282738
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                11160028652877244584572153327058644848925661982628619657928186658900102698674
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                6812248435914231697797863290383502968775638568963173888613217370691598179790
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                10288031744943320195220546183832895314194617344766586077138716301051404942700
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                7073441101526441130326675672872251782623383962596021001856074883490444635729
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                10912992294712044685870339796081929127791537178880469654916734246832522853441
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                19759277998863810664024933554928916584313156629896742577450508109085736606052
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                18978171626604150823002716117679602054445996916974746838136235438198499993631
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                20666573922540010635242270071163126450457243909857540968737935382270244366724
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                12429234064418095971915118978396765378803191095410026012713099669250449537359
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
