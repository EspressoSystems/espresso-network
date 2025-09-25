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

    /// @dev Directly set the authRoot for testing reward claim verification
    function setAuthRoot(uint256 newAuthRoot) public {
        authRoot = newAuthRoot;
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
                15146603847186693773577935701787262941275498929212647783265220150932461611445
            )
            mstore(
                add(mload(add(vk, 0x40)), 0x20),
                19042267120441091594604177777517111637634637930570733778838761287345595126332
            )
            // sigma1
            mstore(
                mload(add(vk, 0x60)),
                15613309569820500918613539617704885761150311657508563645891129097503121295058
            )
            mstore(
                add(mload(add(vk, 0x60)), 0x20),
                4091760237063681549138145749759982760948235560533199154930100388417687350323
            )
            // sigma2
            mstore(
                mload(add(vk, 0x80)),
                3296590657690444436868898536967896063317911154459189757671345388245416394097
            )
            mstore(
                add(mload(add(vk, 0x80)), 0x20),
                12234899211262279532538821695672494171104275154886460309227230140755391923030
            )
            // sigma3
            mstore(
                mload(add(vk, 0xa0)),
                15962460021671355008462811586481833376207692713306402085784087042867688819139
            )
            mstore(
                add(mload(add(vk, 0xa0)), 0x20),
                20900698990414929572130742746229278316148692126988739925679764132066872520799
            )
            // sigma4
            mstore(
                mload(add(vk, 0xc0)),
                4111158936602121870776615997113609582306054725368807395267543775384900020561
            )
            mstore(
                add(mload(add(vk, 0xc0)), 0x20),
                8742869355040258296485514189437071635818933232986275383691190673458807595841
            )

            // q1
            mstore(
                mload(add(vk, 0xe0)),
                924515542943465612007867490306107667211703406123900862085303799481172389281
            )
            mstore(
                add(mload(add(vk, 0xe0)), 0x20),
                18817313267202318027588410797197636491471123129236972541468869318378358114102
            )
            // q2
            mstore(
                mload(add(vk, 0x100)),
                5364005451731671845260834992079507745584177110322986270530554359480401493394
            )
            mstore(
                add(mload(add(vk, 0x100)), 0x20),
                634836242355230920694918978273357517883491173736486103023384027483272775333
            )
            // q3
            mstore(
                mload(add(vk, 0x120)),
                4029575009912891937184303329451605884551074886735722755018056037925232420794
            )
            mstore(
                add(mload(add(vk, 0x120)), 0x20),
                9172174224883992140586489766626652964639826699624048979032058681981263481620
            )
            // q4
            mstore(
                mload(add(vk, 0x140)),
                14886709647556991099557916466306001120348418681020725615993869787601424793651
            )
            mstore(
                add(mload(add(vk, 0x140)), 0x20),
                4488694189686124303083253648661106949485366992049104741634078198390371554775
            )

            // qM12
            mstore(
                mload(add(vk, 0x160)),
                17589544374554413188752731910975452224748812296552099023195268459191699194017
            )
            mstore(
                add(mload(add(vk, 0x160)), 0x20),
                2252481852864123621213063324496633247991306212353025419515607521390999179293
            )
            // qM34
            mstore(
                mload(add(vk, 0x180)),
                7550907146897358712460949091143560301131704190210042189403637764286827198910
            )
            mstore(
                add(mload(add(vk, 0x180)), 0x20),
                20891635970543137508729284633399444398583596513804270342478143154345406525201
            )

            // qO
            mstore(
                mload(add(vk, 0x1a0)),
                15410336158571431438744593579908799866992332595723537921479853455047441099770
            )
            mstore(
                add(mload(add(vk, 0x1a0)), 0x20),
                11247661482853475700896893258733718038392625968536230619826539436179267312293
            )
            // qC
            mstore(
                mload(add(vk, 0x1c0)),
                1661753165041925236131923031658483189302434852122037359802226770913115834058
            )
            mstore(
                add(mload(add(vk, 0x1c0)), 0x20),
                884834834157904188113807962163497020866496529280705908369177108759154702548
            )
            // qH1
            mstore(
                mload(add(vk, 0x1e0)),
                4974052326085684961504324563490961685442241521161198100153693572047876343314
            )
            mstore(
                add(mload(add(vk, 0x1e0)), 0x20),
                18475256192052874015909037890216992614076410350344870119754855777629295042317
            )
            // qH2
            mstore(
                mload(add(vk, 0x200)),
                6128989572649042827216267447040446234675161863507699053631515921363460050966
            )
            mstore(
                add(mload(add(vk, 0x200)), 0x20),
                12267504148217862385476631159176383273726002054346915098626572901999844034378
            )
            // qH3
            mstore(
                mload(add(vk, 0x220)),
                9297769356112970525918539799333818641464923259100804666212086031927604258861
            )
            mstore(
                add(mload(add(vk, 0x220)), 0x20),
                4574474577694292976800882742699555303549132192653529405506660802895532018198
            )
            // qH4
            mstore(
                mload(add(vk, 0x240)),
                15430295613149471126046964100059046203108105988581948935345487622297713354985
            )
            mstore(
                add(mload(add(vk, 0x240)), 0x20),
                17950781335819529613587253399325748287374503415063610520452140436464400837169
            )
            // qEcc
            mstore(
                mload(add(vk, 0x260)),
                6635226542788845655587663904746059195726400540797851250036233614492434175128
            )
            mstore(
                add(mload(add(vk, 0x260)), 0x20),
                17390235494985803067664813889347848645385565351735685490384698703005000705977
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
