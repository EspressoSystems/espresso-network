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
                2649802374932624387153994594179954797052236298577734721225954650141338250062
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                17210221239101525331757559361866348524796108551760509636510899672662099501094
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                10156859211144359036114525116260456253301293271453341931052459546571032235389
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                3588426657979850263477383565628106099842580325059193277737999534832397193841
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                13099195391305126530064017786280921726830538171940519895428174464459909160235
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                11186779979015306736518853773328969989770529983902793278531992239693262715662
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                16184770821581062206275358659062387411512173074710117500424581508294211648035
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                7338896258191827772063582726737841574230231225182408604464064387449794779791
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                8353305629271564755372891227994980005356273802053596630850467234465200531652
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                10291910591033477522289177924291634183567498130167897221173568338384944711535
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                21115635314879257216952235197061731849004573663113391542890668953309640350382
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                21459556843410804642858281916701244142970818195904423338403446595763450659162
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                1457586411286048310642899773271645171747475578696941725071000861737492973494
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                15999793161329455798988643322363609910239570831798512801467828402601532789355
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                1220062464643365751169310431752108036257083801298649770117998168580299373341
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                15499832095771728208152863263416188807464762609853875757942638678781918925171
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                3859924827028400287953527522084267486784699826832280650062897126258411861851
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                7243895711860093613798273400903286766211348150801308126132381766964700510596
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                11863869406504771646943026853490315405646576413078778421791393241410822123556
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                13699437437452297282999029635498914916096827160865140038067800708724455962367
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                3650158951707090841892099108901263275784123604443401901800220787721094136852
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                12280950923742320439860628962263671284501675523219838199294810543205095431700
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                974230902198704771912480461927194718745891523412622287424686568242249650002
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                13947058962659555517484657565021178567981250185899971603326042813336855857912
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                9058359460298716541311536784656975775509594222412129948863552869086531898750
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                8109219459436865100440659342788838316185863077065325229765947365595838475456
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                11928934583634792194891279984777281565212861880931258770565693795203008007139
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                17288743683484699598240568465690145511708559919945621135647046311758753443717
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                18575902763196189310005427151636257842973194449923169978405905895806161723689
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                11931847403437449060961588057723770658007606197850490315621483279346195057931
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                13210790024890410267394972371444290859420741922045063922980495258995719574756
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                21237519961195698492780302753686775893551118054680967604961140867093897898257
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                5347437635665221427376743017571210679661913419143170243117757460258661360010
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                12502271215853446893016871908406611509147509913559334162346115742776179805080
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                6169021557265838939247420085088936623651587033676752096909406618529115741913
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                4936284658503230425926326952136513549882873492424638503934216662374910419726
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
