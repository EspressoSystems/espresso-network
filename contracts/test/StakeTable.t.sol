// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// Libraries
import "forge-std/Test.sol";
// import {console} from "forge-std/console.sol";

using stdStorage for StdStorage;

import { ERC20 } from "solmate/utils/SafeTransferLib.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "../src/libraries/BLSSig.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { AbstractStakeTable } from "../src/interfaces/AbstractStakeTable.sol";
import { LightClient } from "../src/LightClient.sol";
import { LightClientMock } from "../test/mocks/LightClientMock.sol";

// Token contract
import { ExampleToken } from "../src/ExampleToken.sol";

// Target contract
import { StakeTable as S } from "../src/StakeTable.sol";

contract StakeTable_register_Test is Test {
    event Registered(bytes32, uint64, uint256);

    S public stakeTable;
    ExampleToken public token;
    LightClientMock public lcMock;
    uint256 public constant INITIAL_BALANCE = 10 ether;
    address public exampleTokenCreator;

    function genClientWallet(address sender)
        private
        returns (BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory)
    {
        // Generate a BLS signature and other values using rust code
        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "gen-client-wallet";
        cmds[2] = vm.toString(sender);
        cmds[3] = "123";

        bytes memory result = vm.ffi(cmds);
        (
            BN254.G1Point memory blsSig,
            BN254.G2Point memory blsVK,
            uint256 schnorrVKx,
            uint256 schnorrVKy,
        ) = abi.decode(result, (BN254.G1Point, BN254.G2Point, uint256, uint256, address));

        return (
            blsVK, // blsVK
            EdOnBN254.EdOnBN254Point(schnorrVKx, schnorrVKy), // schnorrVK
            blsSig // sig
        );
    }

    function setUp() public {
        exampleTokenCreator = makeAddr("tokenCreator");
        vm.prank(exampleTokenCreator);
        token = new ExampleToken(INITIAL_BALANCE);

        string[] memory cmds = new string[](3);
        cmds[0] = "diff-test";
        cmds[1] = "mock-genesis";
        cmds[2] = "5";

        bytes memory result = vm.ffi(cmds);
        (
            LightClientMock.LightClientState memory state,
            LightClientMock.StakeTableState memory stakeState
        ) = abi.decode(result, (LightClient.LightClientState, LightClient.StakeTableState));
        LightClientMock.LightClientState memory genesis = state;
        LightClientMock.StakeTableState memory genesisStakeTableState = stakeState;

        lcMock = new LightClientMock(genesis, genesisStakeTableState, 864000);
        address lightClientAddress = address(lcMock);
        stakeTable = new S(address(token), lightClientAddress, 10);
    }

    function testFuzz_RevertWhen_InvalidBLSSig(uint256 scalar) external {
        uint64 depositAmount = 10 ether;
        uint64 validUntilEpoch = 5;

        (BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK,) =
            genClientWallet(exampleTokenCreator);

        // Prepare for the token transfer
        vm.startPrank(exampleTokenCreator);
        token.approve(address(stakeTable), depositAmount);

        // Ensure the scalar is valid
        // Note: Apparently BN254.scalarMul is not well defined when the scalar is 0
        scalar = bound(scalar, 1, BN254.R_MOD - 1);
        BN254.validateScalarField(BN254.ScalarField.wrap(scalar));
        BN254.G1Point memory badSig = BN254.scalarMul(BN254.P1(), BN254.ScalarField.wrap(scalar));
        BN254.validateG1Point(badSig);

        // Failed signature verification
        vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
        stakeTable.register(blsVK, schnorrVK, depositAmount, badSig, validUntilEpoch);
        vm.stopPrank();
    }

    // commenting out epoch related tests for now
    // function testFuzz_RevertWhen_InvalidNextRegistrationEpoch(uint64 rand) external {
    //     LCMock.setCurrentEpoch(3);
    //     uint64 currentEpoch = stakeTable.currentEpoch();

    //     uint64 depositAmount = 10 ether;
    //     vm.prank(exampleTokenCreator);
    //     token.approve(address(stakeTable), depositAmount);

    //     (
    //         BN254.G2Point memory blsVK,
    //         EdOnBN254.EdOnBN254Point memory schnorrVK,
    //         BN254.G1Point memory sig
    //     ) = genClientWallet(exampleTokenCreator);

    //     // Invalid next registration epoch
    //     uint64 validUntilEpoch = uint64(bound(rand, 0, currentEpoch - 1));
    //     vm.prank(exampleTokenCreator);
    //     vm.expectRevert(
    //         abi.encodeWithSelector(
    //             S.InvalidNextRegistrationEpoch.selector, currentEpoch + 1, validUntilEpoch
    //         )
    //     );
    //     stakeTable.register(
    //         blsVK,
    //         schnorrVK,
    //         depositAmount,
    //         sig,
    //         validUntilEpoch
    //     );

    //     // Valid next registration epoch
    //     validUntilEpoch = uint64(bound(rand, currentEpoch + 1, type(uint64).max));
    //     vm.prank(exampleTokenCreator);
    //     stakeTable.register(
    //         blsVK,
    //         schnorrVK,
    //         depositAmount,
    //         sig,
    //         validUntilEpoch
    //     );
    // }

    function test_RevertWhen_NodeAlreadyRegistered() external {
        uint64 depositAmount = 10 ether;
        uint64 validUntilEpoch = 5;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(exampleTokenCreator);

        // Prepare for the token transfer
        vm.prank(exampleTokenCreator);
        token.approve(address(stakeTable), depositAmount);

        // Successful call to register
        vm.prank(exampleTokenCreator);
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

        // The node is already registered
        vm.prank(exampleTokenCreator);
        vm.expectRevert(S.NodeAlreadyRegistered.selector);
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
    }

    function test_RevertWhen_NoTokenAllowanceOrBalance() external {
        uint64 depositAmount = 10 ether;
        uint64 validUntilEpoch = 10;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(exampleTokenCreator);

        assertEq(ERC20(token).balanceOf(exampleTokenCreator), INITIAL_BALANCE);
        vm.prank(exampleTokenCreator);
        // The call to register is expected to fail because the depositAmount has not been approved
        // and thus the stake table contract cannot lock the stake.
        vm.expectRevert(abi.encodeWithSelector(S.InsufficientAllowance.selector, 0, depositAmount));
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

        // A user with 0 balance cannot register either
        address newUser = makeAddr("New user with zero balance");
        (blsVK, schnorrVK, sig) = genClientWallet(newUser);

        vm.startPrank(newUser);
        // Prepare for the token transfer by giving the StakeTable contract the required allowance
        token.approve(address(stakeTable), depositAmount);
        vm.expectRevert(abi.encodeWithSelector(S.InsufficientBalance.selector, 0));
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
        vm.stopPrank();
    }

    function test_RevertWhen_WrongStakeAmount() external {
        uint64 depositAmount = 5 ether;
        uint64 validUntilEpoch = 10;

        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(exampleTokenCreator);

        assertEq(ERC20(token).balanceOf(exampleTokenCreator), INITIAL_BALANCE);
        vm.prank(exampleTokenCreator);
        // The call to register is expected to fail because the depositAmount has not been approved
        // and thus the stake table contract cannot lock the stake.
        vm.expectRevert(abi.encodeWithSelector(S.InsufficientStakeAmount.selector, depositAmount));
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
    }

    /// @dev Tests a correct registration
    function test_Registration_succeeds() external {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory sig
        ) = genClientWallet(exampleTokenCreator);

        uint64 depositAmount = 10 ether;
        uint64 validUntilEpoch = 5;

        // Prepare for the token transfer
        vm.prank(exampleTokenCreator);
        token.approve(address(stakeTable), depositAmount);

        // Balances before registration
        assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

        uint256 totalStakeAmount;
        totalStakeAmount = stakeTable.totalStake();
        assertEq(totalStakeAmount, 0);

        AbstractStakeTable.Node memory node;
        node.account = exampleTokenCreator;
        node.balance = depositAmount;
        node.schnorrVK = schnorrVK;
        node.registerEpoch = 1;

        // Check event is emitted after calling successfully `register`
        vm.expectEmit(false, false, false, true, address(stakeTable));
        emit Registered(stakeTable._hashBlsKey(blsVK), node.registerEpoch, node.balance);
        vm.prank(exampleTokenCreator);
        stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

        // Balance after registration
        assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE - depositAmount);
        totalStakeAmount = stakeTable.totalStake();
        assertEq(totalStakeAmount, depositAmount);
    }
}
