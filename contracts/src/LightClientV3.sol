// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.28;

import { BN254 } from "bn254/BN254.sol";
import { LightClientV2 } from "./LightClientV2.sol";
import { IPlonkVerifier } from "./interfaces/IPlonkVerifier.sol";
import { PlonkVerifierV3 } from "./libraries/PlonkVerifierV3.sol";
import { LightClientStateUpdateVKV3 as VkLib } from "./libraries/LightClientStateUpdateVKV3.sol";

/// @title LightClientV3
/// @notice LightClientV2 with an additional root for gas-efficient state authentication and
/// @notice improved public input derivation for efficiency and future-proof.
contract LightClientV3 is LightClientV2 {
    /// @notice a state value signed by validators as part of the extended light client state
    uint256 public authRoot;
    /// @notice Unfortunate re-declaration since V2 mark firstEpoch as private
    uint64 internal _firstEpoch;

    function initializeV3() public reinitializer(3) {
        _firstEpoch = epochFromBlockNumber(epochStartBlock, blocksPerEpoch);
    }

    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (3, 0, 0);
    }

    /// @dev override the V2's to disable calling it
    function newFinalizedState(
        LightClientState memory,
        StakeTableState memory,
        IPlonkVerifier.PlonkProof memory
    ) external pure override {
        revert DeprecatedApi();
    }

    /// @dev See detailed doc in `LightClient.sol` and `LightClientV2.sol`
    /// @param newAuthRoot is the authentication root corresponding to newState
    /// @dev more detailed inline code comments, see `LightClientV2.sol`
    /// @dev diff w/ V2 is marked with "DIFF:" in comment
    function newFinalizedState(
        LightClientState memory newState,
        StakeTableState memory nextStakeTable,
        uint256 newAuthRoot,
        IPlonkVerifier.PlonkProof memory proof
    ) external virtual {
        if (isPermissionedProverEnabled() && msg.sender != permissionedProver) {
            revert ProverNotPermissioned();
        }

        if (
            newState.viewNum <= finalizedState.viewNum
                || newState.blockHeight <= finalizedState.blockHeight
        ) {
            revert OutdatedState();
        }
        BN254.validateScalarField(newState.blockCommRoot);
        BN254.validateScalarField(nextStakeTable.blsKeyComm);
        BN254.validateScalarField(nextStakeTable.schnorrKeyComm);
        BN254.validateScalarField(nextStakeTable.amountComm);
        // DIFF: also validate the new public input
        BN254.validateScalarField(BN254.ScalarField.wrap(newAuthRoot));

        // epoch-related checks
        uint64 lastUpdateEpoch = currentEpoch();
        uint64 newStateEpoch = epochFromBlockNumber(newState.blockHeight, blocksPerEpoch);
        if (newStateEpoch >= _firstEpoch) {
            require(!isGtEpochRoot(newState.blockHeight), MissingEpochRootUpdate());
        }
        if (newStateEpoch > _firstEpoch) {
            require(newStateEpoch - lastUpdateEpoch < 2, MissingEpochRootUpdate());
            if (newStateEpoch == lastUpdateEpoch + 1 && !isEpochRoot(finalizedState.blockHeight)) {
                revert MissingEpochRootUpdate();
            }
        }

        // DIFF: pass in additional authRoot param
        verifyProof(newState, nextStakeTable, newAuthRoot, proof);
        finalizedState = newState;
        // DIFF: update authRoot upon successful SNARK verification
        authRoot = newAuthRoot;

        // during epoch change, also update to the new stake table
        if (newStateEpoch >= _firstEpoch && isEpochRoot(newState.blockHeight)) {
            votingStakeTableState = nextStakeTable;
            emit NewEpoch(newStateEpoch + 1);
        }

        updateStateHistory(uint64(currentBlockNumber()), uint64(block.timestamp), newState);

        emit NewState(newState.viewNum, newState.blockHeight, newState.blockCommRoot);
    }

    function _getVk()
        public
        pure
        virtual
        override
        returns (IPlonkVerifier.VerifyingKey memory vk)
    {
        vk = VkLib.getVk();
    }

    /// @dev compare to V2, we change public input length from 11 to 5:
    /// @dev 4 from votingStakeTableState, 1 from msg_signed := H(authenticated states)
    function verifyProof(
        LightClientState memory state,
        StakeTableState memory nextStakeTable,
        uint256 newAuthRoot,
        IPlonkVerifier.PlonkProof memory proof
    ) internal virtual {
        IPlonkVerifier.VerifyingKey memory vk = _getVk();

        // DIFF: a redesign of public input from V2, reduced number of public inputs while
        // being more future-proof:
        //
        // In V2, everything state-signer sign over (incl. LightClientState, votingStake, nextStake)
        // are all being part of the public inputs, such state with correct quorum signature
        // certifies the state and can be used for future authentication against.
        //
        // However, the V2 design suffers from "one more state to sign requires circuit update".
        //
        // In V3, we streamline the actual message signed (thus checked in circuit) to be
        //   msg_signed := keccak256(all states to certify) mod p
        // so that circuit will verify signature over msg_signed, and here in verification contract
        // we will enforce the keccak256 relationship between the msg_preimage (i.e. the actual
        // certified states) and msg_signed which is cheap.
        //
        // If we change the "states to certify" definition in the future, we don't have to update
        // the circuit anymore, which also means we don't have to update Plonk Verification Key.
        // But we do need to upgrade this `verifyProof()` function to update the input to keccak256.
        uint256[5] memory publicInput;
        // these fields are still public input, because their computation is verified in circuit
        publicInput[0] = BN254.ScalarField.unwrap(votingStakeTableState.blsKeyComm);
        publicInput[1] = BN254.ScalarField.unwrap(votingStakeTableState.schnorrKeyComm);
        publicInput[2] = BN254.ScalarField.unwrap(votingStakeTableState.amountComm);
        publicInput[3] = votingStakeTableState.threshold;

        // this enforce the preimage relation of the actual message being signed,
        // the signature is being verified in circuit, but these preimage values are unknown
        // (to the circuit).
        bytes memory encodedNextStakeTable;
        if (state.blockHeight >= epochStartBlock && isEpochRoot(state.blockHeight)) {
            encodedNextStakeTable = abi.encode(nextStakeTable);
        } else {
            encodedNextStakeTable = abi.encode(votingStakeTableState);
        }

        bytes32 msgSigned =
            keccak256(abi.encodePacked(abi.encode(state), encodedNextStakeTable, newAuthRoot));
        publicInput[4] = uint256(msgSigned) % BN254.R_MOD;

        if (!PlonkVerifierV3.verify(vk, publicInput, proof)) {
            revert InvalidProof();
        }
    }
}
