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
                2473251910047105925820190625566233929163397980670710751116438490774391132831
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                4069073876072076484413626486440080220445693755180304507094552606143111101216
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                6925622402093475269813626175992055043289506738085206035399514128542394488810
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                9357370651059177782162578502915716854423969285384006481863401827366114288627
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                4659017600175874888967778877363890940700763849640208485298745625622004570907
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                4563691808889717670720986992268220018506975189725708373172424199939557819302
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                12714893726160898072094187735306844134158167163198214435624672278060294667851
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                19900418844238818352106006463253623908070266266092538667044512460355887814890
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                20443945200347699969232720048201189196187031802209232181658313379911174345490
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                17214591818366530606491948698379258925966585226989621915858578113996136551214
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                17096079989278420336516227148528091222685739749162250314657538420387914165425
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                21498024520654416969997692468966834066434134430233009559598147975058817495183
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                3269861915757092720363076665842495073606362075873859405974314493684774188896
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                17067155362846309752355652047858683149822417820194330056035012900327122521111
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                9072946306354301137913491413888250657006164729714431776587916120295427097180
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                6524306749513374305450210356958936978024936599056918193682797696249975449984
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                5619910551133069453307758831477325045161487372592470966519858919820142372961
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                6775563302262661088511668403710482438083641672766732232528534358296407245616
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                18819330992994080783903675967079211574175561449103717866457423783581731057852
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                8028049609434882331783583467776037403466038316715769021013114563095043997149
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                20983834618358614994031982320368278190368406963857365933788345924373828486274
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                419798266856441105607391713516862280955176529292473175851391914742573211871
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                7638823469040831765194872625499365246824116428927479261661583317333796371115
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                1280988271581830744459440363242349959971484468845466593084477463895302318733
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                10348548460579628986681835480309765171726378542027544012271013514133779664637
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                12648503636623206754088047353990763088726206358936745525873572726938675805656
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                20703176183031164065073448964370154871896514799543772149896252982203187088842
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                5910601253563779709218322764960158451530909579248771502278666427412372968321
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                20580105217192332043529728108686246057566444996388420759502866152490205590145
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                11244310534145933022523297921993196983707218459975774626091859185922571673206
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                12489577706283508651617369622739673656850060612525218982966653622465274019228
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                13531248973119612071022583200864210066878004911787717798960851056310870267978
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                519614558731089159041150736049232468826309619979774870516511194783499357554
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                6736132186538169525020641480142497890562466302350427120556120675843313071223
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                8751908345348294132005636809396198237167001162745650194099554228516291870141
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                11389402492968699553684680014932644408802742849376790779663996784193166667833
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
