// SPDX-License-Identifier: Unlicensed

/* solhint-disable contract-name-camelcase, func-name-mixedcase, one-contract-per-file */

pragma solidity ^0.8.0;

// // Libraries
// import "forge-std/Test.sol";
// // import {console} from "forge-std/console.sol";

// using stdStorage for StdStorage;

// import { ERC20 } from "solmate/utils/SafeTransferLib.sol";
// import { BN254 } from "bn254/BN254.sol";
// import { BLSSig } from "../src/libraries/BLSSig.sol";
// import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
// import { AbstractStakeTable } from "../src/interfaces/AbstractStakeTable.sol";
// import { LightClient } from "../src/LightClient.sol";
// import { LightClientMock } from "../test/mocks/LightClientMock.sol";
// import { InitializedAt } from "../src/InitializedAt.sol";
// import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
// import { IPlonkVerifier as V } from "../src/interfaces/IPlonkVerifier.sol";

// // Token contract
// import { ExampleToken } from "../src/ExampleToken.sol";

// // Target contract
// import { StakeTable as S } from "../src/StakeTable.sol";
// import { StakeTableMock } from "../test/mocks/StakeTableMock.sol";

// contract StakeTable_register_Test is Test {
//     StakeTableMock public stakeTable;
//     ExampleToken public token;
//     LightClientMock public lcMock;
//     uint256 public constant INITIAL_BALANCE = 10 ether;
//     uint256 public constant MIN_STAKE_AMOUNT = 10 ether;
//     address public exampleTokenCreator;
//     uint64 public churnRate = 10;
//     uint64 public hotShotBlocksPerEpoch = 1;

//     function genClientWallet(address sender, string memory seed)
//         private
//         returns (BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory)
//     {
//         // Generate a BLS signature and other values using rust code
//         string[] memory cmds = new string[](4);
//         cmds[0] = "diff-test";
//         cmds[1] = "gen-client-wallet";
//         cmds[2] = vm.toString(sender);
//         cmds[3] = seed;

//         bytes memory result = vm.ffi(cmds);
//         (
//             BN254.G1Point memory blsSig,
//             BN254.G2Point memory blsVK,
//             uint256 schnorrVKx,
//             uint256 schnorrVKy,
//         ) = abi.decode(result, (BN254.G1Point, BN254.G2Point, uint256, uint256, address));

//         return (
//             blsVK, // blsVK
//             EdOnBN254.EdOnBN254Point(schnorrVKx, schnorrVKy), // schnorrVK
//             blsSig // sig
//         );
//     }

//     function setUpCustom(uint64 _churnRate, uint64 _blocksPerEpoch) public {
//         exampleTokenCreator = makeAddr("tokenCreator");
//         vm.prank(exampleTokenCreator);
//         token = new ExampleToken(INITIAL_BALANCE);

//         string[] memory cmds = new string[](3);
//         cmds[0] = "diff-test";
//         cmds[1] = "mock-genesis";
//         cmds[2] = "5";

//         bytes memory result = vm.ffi(cmds);
//         (
//             LightClientMock.LightClientState memory state,
//             LightClientMock.StakeTableState memory stakeState
//         ) = abi.decode(result, (LightClient.LightClientState, LightClient.StakeTableState));
//         LightClientMock.LightClientState memory genesis = state;
//         LightClientMock.StakeTableState memory genesisStakeTableState = stakeState;

//         lcMock = new LightClientMock(genesis, genesisStakeTableState, 864000);
//         stakeTable = new StakeTableMock(
//             address(token),
//             address(lcMock),
//             _churnRate,
//             _blocksPerEpoch,
//             MIN_STAKE_AMOUNT,
//             exampleTokenCreator
//         );
//     }

//     function setUp() public {
//         setUpCustom(churnRate, hotShotBlocksPerEpoch);
//     }

//     function test_RevertWhen_InvalidHotShotBlocksPerEpoch() external {
//         vm.expectRevert(S.InvalidHotShotBlocksPerEpoch.selector);
//         new StakeTableMock(
//             address(token), address(lcMock), churnRate, 0, MIN_STAKE_AMOUNT, exampleTokenCreator
//         );
//     }

//     function test_Deployment_StoresBlockNumber() public {
//         setUp();
//         assertEq(stakeTable.initializedAtBlock(), block.number);
//     }

//     function testFuzz_RevertWhen_InvalidBLSSig(uint256 scalar) external {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         (BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK,) =
//             genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Ensure the scalar is valid
//         // Note: Apparently BN254.scalarMul is not well defined when the scalar is 0
//         scalar = bound(scalar, 1, BN254.R_MOD - 1);
//         BN254.validateScalarField(BN254.ScalarField.wrap(scalar));
//         BN254.G1Point memory badSig = BN254.scalarMul(BN254.P1(),
// BN254.ScalarField.wrap(scalar));
//         BN254.validateG1Point(badSig);

//         // Failed signature verification
//         vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, badSig, validUntilEpoch);
//         vm.stopPrank();
//     }

//     // commenting out epoch related tests for now
//     // function testFuzz_RevertWhen_InvalidNextRegistrationEpoch(uint64 rand) external {
//     //     LCMock.setCurrentEpoch(3);
//     //     uint64 currentEpoch = stakeTable.currentEpoch();

//     //     uint64 depositAmount = 10 ether;
//     //     vm.prank(exampleTokenCreator);
//     //     token.approve(address(stakeTable), depositAmount);

//     //     (
//     //         BN254.G2Point memory blsVK,
//     //         EdOnBN254.EdOnBN254Point memory schnorrVK,
//     //         BN254.G1Point memory sig
//     //     ) = genClientWallet(exampleTokenCreator);

//     //     // Invalid next registration epoch
//     //     uint64 validUntilEpoch = uint64(bound(rand, 0, currentEpoch - 1));
//     //     vm.prank(exampleTokenCreator);
//     //     vm.expectRevert(
//     //         abi.encodeWithSelector(
//     //             S.InvalidNextRegistrationEpoch.selector, currentEpoch + 1, validUntilEpoch
//     //         )
//     //     );
//     //     stakeTable.register(
//     //         blsVK,
//     //         schnorrVK,
//     //         depositAmount,
//     //         sig,
//     //         validUntilEpoch
//     //     );

//     //     // Valid next registration epoch
//     //     validUntilEpoch = uint64(bound(rand, currentEpoch + 1, type(uint64).max));
//     //     vm.prank(exampleTokenCreator);
//     //     stakeTable.register(
//     //         blsVK,
//     //         schnorrVK,
//     //         depositAmount,
//     //         sig,
//     //         validUntilEpoch
//     //     );
//     // }

//     function test_RevertWhen_NodeAlreadyRegistered() external {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer
//         vm.prank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Successful call to register
//         vm.prank(exampleTokenCreator);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // The node is already registered
//         vm.prank(exampleTokenCreator);
//         vm.expectRevert(S.NodeAlreadyRegistered.selector);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
//     }

//     function test_RevertWhen_NoTokenAllowanceOrBalance() external {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 10;
//         string memory seed = "123";

//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         assertEq(ERC20(token).balanceOf(exampleTokenCreator), INITIAL_BALANCE);
//         vm.prank(exampleTokenCreator);
//         // The call to register is expected to fail because the depositAmount has not been
// approved
//         // and thus the stake table contract cannot lock the stake.
//         vm.expectRevert(abi.encodeWithSelector(S.InsufficientAllowance.selector, 0,
// depositAmount));
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // A user with 0 balance cannot register either
//         address newUser = makeAddr("New user with zero balance");
//         (blsVK, schnorrVK, sig) = genClientWallet(newUser, seed);

//         vm.startPrank(newUser);
//         // Prepare for the token transfer by giving the StakeTable contract the required
// allowance
//         token.approve(address(stakeTable), depositAmount);
//         vm.expectRevert(abi.encodeWithSelector(S.InsufficientBalance.selector, 0));
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
//         vm.stopPrank();
//     }

//     function test_RevertWhen_InsufficientStakeAmount() external {
//         uint64 depositAmount = uint64(stakeTable.minStakeAmount()) - 1;
//         uint64 validUntilEpoch = 10;
//         string memory seed = "123";

//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         assertEq(ERC20(token).balanceOf(exampleTokenCreator), INITIAL_BALANCE);
//         vm.prank(exampleTokenCreator);
//         // The call to register is expected to fail because the correct depositAmount has not
// been
//         // approved/supplied
//         // and thus the stake table contract cannot lock the stake.
//         vm.expectRevert(abi.encodeWithSelector(S.InsufficientStakeAmount.selector,
// depositAmount));
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);
//     }

//     /// @dev Tests a correct registration
//     function test_Registration_succeeds() external {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer
//         vm.prank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         uint256 totalStakeAmount;
//         totalStakeAmount = stakeTable.totalStake();
//         assertEq(totalStakeAmount, 0);

//         // Check event is emitted after calling successfully `register`
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         vm.prank(exampleTokenCreator);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Balance after registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE - depositAmount);
//         totalStakeAmount = stakeTable.totalStake();
//         assertEq(totalStakeAmount, depositAmount);

//         // lookup the node and verify the data
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         assertEq(node.account, exampleTokenCreator);
//         assertEq(node.balance, depositAmount);
//         assertEq(node.registerEpoch, 1);
//         assertTrue(stakeTable._isEqualBlsKey(node.blsVK, blsVK));
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, schnorrVK));

//         // lookup the stake and verify the amount
//         uint256 stakeAmount = stakeTable.lookupStake(exampleTokenCreator);
//         assertEq(stakeAmount, depositAmount);
//     }

//     /// @dev Tests a correct registration
//     function test_RevertWhen_InvalidBlsVK_or_InvalidSchnorrVK_on_Registration() external {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         // generate a valid blsVK and schnorrVK
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // revert when the blsVK is the zero point
//         BN254.G2Point memory zeroBlsVK = BN254.G2Point(
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0)
//         );
//         vm.expectRevert(S.InvalidBlsVK.selector);
//         stakeTable.register(zeroBlsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // revert when the schnorrVK is the zero point
//         EdOnBN254.EdOnBN254Point memory zeroSchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);
//         vm.expectRevert(S.InvalidSchnorrVK.selector);
//         stakeTable.register(blsVK, zeroSchnorrVK, depositAmount, sig, validUntilEpoch);

//         // lookup the node and verify the data but expect the node to be empty
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         assertEq(node.account, address(0));

//         vm.stopPrank();
//     }

//     function test_UpdateConsensusKeys_Succeeds() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         // Check event is emitted after calling successfully `register`
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: generate a new blsVK and schnorrVK
//         seed = "234";
//         (
//             BN254.G2Point memory newBlsVK,
//             EdOnBN254.EdOnBN254Point memory newSchnorrVK,
//             BN254.G1Point memory newBlsSig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // assert that the new blsVK and schnorrVK are not the same as the old ones
//         assertFalse(stakeTable._isEqualBlsKey(newBlsVK, blsVK));
//         assertFalse(EdOnBN254.isEqual(newSchnorrVK, schnorrVK));

//         // Step 3: update the consensus keys
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.UpdatedConsensusKeys(exampleTokenCreator, newBlsVK,
// newSchnorrVK);
//         stakeTable.updateConsensusKeys(newBlsVK, newSchnorrVK, newBlsSig);

//         // Step 4: verify the update
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         assertTrue(stakeTable._isEqualBlsKey(node.blsVK, newBlsVK));
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, newSchnorrVK));
//         assertEq(node.balance, depositAmount);
//         assertEq(node.account, exampleTokenCreator);

//         vm.stopPrank();
//     }

//     function test_RevertWhen_UpdateConsensusKeysWithSameKeys() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: update the consensus keys with the same keys
//         vm.expectRevert(S.NoKeyChange.selector);
//         stakeTable.updateConsensusKeys(blsVK, schnorrVK, sig);

//         vm.stopPrank();
//     }

//     function test_RevertWhen_UpdateConsensusKeysWithEmptyKeys() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // empty keys
//         BN254.G2Point memory emptyBlsVK = BN254.G2Point(
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0)
//         );
//         EdOnBN254.EdOnBN254Point memory emptySchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);

//         // Step 2: attempt to update the consensus keys with the same keys
//         vm.expectRevert(S.InvalidBlsVK.selector);
//         stakeTable.updateConsensusKeys(emptyBlsVK, emptySchnorrVK, sig);

//         vm.stopPrank();
//     }

//     function test_RevertWhen_UpdateConsensusKeysWithInvalidSignature() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         BN254.G1Point memory badSig =
//             BN254.G1Point(BN254.BaseField.wrap(0), BN254.BaseField.wrap(0));

//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: generate a new blsVK and schnorrVK
//         seed = "234";
//         (BN254.G2Point memory newBlsVK, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
//             genClientWallet(exampleTokenCreator, seed);

//         // Step 3: attempt to update the consensus keys with the new keys but invalid signature
//         vm.expectRevert(BLSSig.BLSSigVerificationFailed.selector);
//         stakeTable.updateConsensusKeys(newBlsVK, newSchnorrVK, badSig);

//         vm.stopPrank();
//     }

//     function test_RevertWhen_UpdateConsensusKeysWithZeroBlsKeyButNewSchnorrVK() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: generate an empty and new schnorrVK
//         seed = "234";
//         (, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
//             genClientWallet(exampleTokenCreator, seed);

//         BN254.G2Point memory emptyBlsVK = BN254.G2Point(
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0),
//             BN254.BaseField.wrap(0)
//         );

//         // Step 3: update the consensus keys with the new schnorr Key but zero bls key which
//         // indicates no change in the bls key
//         vm.expectRevert(S.InvalidBlsVK.selector);
//         stakeTable.updateConsensusKeys(emptyBlsVK, newSchnorrVK, sig);

//         vm.stopPrank();
//     }

//     function test_RevertWhen_UpdateConsensusKeysWithZeroSchnorrVKButNewBlsVK() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: generate a new blsVK
//         seed = "234";
//         (BN254.G2Point memory newBlsVK,, BN254.G1Point memory newSig) =
//             genClientWallet(exampleTokenCreator, seed);

//         // Step 3: generate empty schnorrVK
//         EdOnBN254.EdOnBN254Point memory emptySchnorrVK = EdOnBN254.EdOnBN254Point(0, 0);

//         // Step 4: update the consensus keys with the new bls keys but empty schnorrVK
//         vm.expectRevert(S.InvalidSchnorrVK.selector);
//         stakeTable.updateConsensusKeys(newBlsVK, emptySchnorrVK, newSig);

//         vm.stopPrank();
//     }

//     function test_UpdateConsensusKeysWithSameBlsKeyButNewSchnorrVK_Succeeds() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory blsSig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, blsSig, validUntilEpoch);

//         // Step 2: generate a new schnorrVK
//         seed = "234";
//         (, EdOnBN254.EdOnBN254Point memory newSchnorrVK,) =
//             genClientWallet(exampleTokenCreator, seed);

//         // Step 3: update the consensus keys with the new schnorrVK
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.UpdatedConsensusKeys(exampleTokenCreator, blsVK, newSchnorrVK);
//         stakeTable.updateConsensusKeys(blsVK, newSchnorrVK, blsSig);

//         // Step 4: verify the update
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         assertTrue(stakeTable._isEqualBlsKey(node.blsVK, blsVK)); // same as current bls vk
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, newSchnorrVK)); // new schnorr vk
//         assertEq(node.balance, depositAmount); //same balance
//         assertEq(node.account, exampleTokenCreator); //same account

//         vm.stopPrank();
//     }

//     function test_UpdateConsensusKeysWithNewBlsKeyButSameSchnorrVK_Succeeds() public {
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         //Step 1: generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // Step 2: generate an empty and new schnorrVK
//         seed = "234";
//         (BN254.G2Point memory newBlsVK,, BN254.G1Point memory newSig) =
//             genClientWallet(exampleTokenCreator, seed);

//         // Step 3: update the consensus keys with the same bls keys but new schnorrV
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.UpdatedConsensusKeys(exampleTokenCreator, newBlsVK, schnorrVK);
//         stakeTable.updateConsensusKeys(newBlsVK, schnorrVK, newSig);

//         // Step 4: verify the update
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         assertTrue(stakeTable._isEqualBlsKey(node.blsVK, newBlsVK)); // same as current bls vk
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, schnorrVK)); // same as current schnorr vk
//         assertEq(node.balance, depositAmount); //same balance
//         assertEq(node.account, exampleTokenCreator); //same account

//         vm.stopPrank();
//     }

//     function test_lookupNodeAndLookupStake_fails() public {
//         address randomUser = makeAddr("randomUser");

//         // lookup the stake for an address that is not registered and verify the amount is empty
//         uint256 stakeAmount = stakeTable.lookupStake(randomUser);
//         assertEq(stakeAmount, 0);

//         // lookup the node for an address that is not registered and verify the data is empty
//         AbstractStakeTable.Node memory node = stakeTable.lookupNode(randomUser);
//         assertEq(node.account, address(0));
//         assertEq(node.balance, 0);
//         assertEq(node.registerEpoch, 0);
//         assertTrue(
//             stakeTable._isEqualBlsKey(
//                 node.blsVK,
//                 BN254.G2Point(
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0)
//                 )
//             )
//         );
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, EdOnBN254.EdOnBN254Point(0, 0)));

//         // look up the stake for the zero address and verify the amount is empty
//         stakeAmount = stakeTable.lookupStake(address(0));
//         assertEq(stakeAmount, 0);

//         // look up the node for the zero address and verify the data is empty
//         node = stakeTable.lookupNode(address(0));
//         assertEq(node.account, address(0));
//         assertEq(node.balance, 0);
//         assertEq(node.registerEpoch, 0);
//         assertTrue(
//             stakeTable._isEqualBlsKey(
//                 node.blsVK,
//                 BN254.G2Point(
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0),
//                     BN254.BaseField.wrap(0)
//                 )
//             )
//         );
//         assertTrue(EdOnBN254.isEqual(node.schnorrVK, EdOnBN254.EdOnBN254Point(0, 0)));
//     }

//     function test_WithdrawFunds_succeeds() public {
//         // register -> updateEpoch -> requestExit -> updateEpoch -> withdraw
//         // Register the node and set exit epoch
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // Balances before registration
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);

//         // register the node
//         vm.startPrank(exampleTokenCreator);
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         // the node must be registered before you can exit so update the hotshot block number on
// the
//         // light client contract
//         S.Node memory node = stakeTable.lookupNode(exampleTokenCreator);
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, node.registerEpoch + 1, BN254.ScalarField.wrap(0))
//         );
//         assertGe(stakeTable.currentEpoch(), node.registerEpoch + 1);

//         stakeTable.requestExit();

//         // update the hotshot block number on the light client contract to be greater than the
//         // exitEpoch
//         // and the exitEscrowPeriod is over
//         node = stakeTable.lookupNode(exampleTokenCreator);
//         uint64 exitEscrowPeriod = stakeTable.mockExitEscrowPeriod(node);
//         uint64 validWithdrawalEpoch = node.exitEpoch + exitEscrowPeriod + 1;
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, validWithdrawalEpoch, BN254.ScalarField.wrap(0))
//         );
//         assertGe(stakeTable.currentEpoch(), validWithdrawalEpoch);

//         // Withdraw the funds
//         vm.startPrank(exampleTokenCreator);
//         uint256 balance = stakeTable.withdrawFunds();
//         vm.stopPrank();

//         // verify the withdraw
//         assertEq(balance, depositAmount);
//         assertEq(token.balanceOf(exampleTokenCreator), INITIAL_BALANCE);
//         assertEq(stakeTable.totalStake(), 0);
//         assertEq(stakeTable.lookupNode(exampleTokenCreator).balance, 0);
//         assertEq(stakeTable.lookupNode(exampleTokenCreator).account, address(0));

//         // test withdraw fails if the user tries to withdraw again
//         vm.expectRevert(S.NodeNotRegistered.selector);
//         stakeTable.withdrawFunds();

//         vm.stopPrank();
//     }

//     function test_WithdrawFunds_RevertWhen_NodeNotRegistered() public {
//         // Register the node and set exit epoch
//         uint64 depositAmount = 10 ether;
//         uint64 validUntilEpoch = 5;
//         string memory seed = "123";

//         // generate a new blsVK and schnorrVK and register this node
//         (
//             BN254.G2Point memory blsVK,
//             EdOnBN254.EdOnBN254Point memory schnorrVK,
//             BN254.G1Point memory sig
//         ) = genClientWallet(exampleTokenCreator, seed);

//         // Prepare for the token transfer by granting allowance to the contract
//         vm.startPrank(exampleTokenCreator);
//         token.approve(address(stakeTable), depositAmount);

//         // register the node
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.Registered(exampleTokenCreator, 1, depositAmount);
//         stakeTable.register(blsVK, schnorrVK, depositAmount, sig, validUntilEpoch);

//         vm.stopPrank();

//         vm.startPrank(makeAddr("randomUser"));
//         // withdraw the funds
//         vm.expectRevert(S.NodeNotRegistered.selector);
//         stakeTable.withdrawFunds();
//         vm.stopPrank();
//     }

//     // test set admin succeeds
//     function test_setAdmin_succeeds() public {
//         vm.prank(exampleTokenCreator);
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit Ownable.OwnershipTransferred(exampleTokenCreator, makeAddr("admin"));
//         stakeTable.transferOwnership(makeAddr("admin"));
//         assertEq(stakeTable.owner(), makeAddr("admin"));
//     }

//     // test set admin fails if not admin or invalid admin address
//     function test_revertWhen_setAdmin_NotAdminOrInvalidAdminAddress() public {
//         vm.startPrank(makeAddr("randomUser"));
//         vm.expectRevert(
//             abi.encodeWithSelector(
//                 Ownable.OwnableUnauthorizedAccount.selector, makeAddr("randomUser")
//             )
//         );
//         stakeTable.transferOwnership(makeAddr("admin"));
//         vm.stopPrank();

//         vm.prank(exampleTokenCreator);
//         vm.expectRevert(abi.encodeWithSelector(Ownable.OwnableInvalidOwner.selector,
// address(0)));
//         stakeTable.transferOwnership(address(0));
//     }

//     // test update min stake amount succeeds
//     function test_updateMinStakeAmount_succeeds() public {
//         vm.prank(exampleTokenCreator);
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.MinStakeAmountUpdated(10 ether);
//         stakeTable.updateMinStakeAmount(10 ether);
//         assertEq(stakeTable.minStakeAmount(), 10 ether);
//     }

//     // test update min stake amount fails if not admin or invalid stake amount
//     function test_revertWhen_updateMinStakeAmount_NotAdminOrInvalidStakeAmount() public {
//         vm.startPrank(makeAddr("randomUser"));
//         vm.expectRevert(
//             abi.encodeWithSelector(
//                 Ownable.OwnableUnauthorizedAccount.selector, makeAddr("randomUser")
//             )
//         );
//         stakeTable.updateMinStakeAmount(10 ether);
//         vm.stopPrank();

//         vm.prank(exampleTokenCreator);
//         vm.expectRevert(S.InvalidValue.selector);
//         stakeTable.updateMinStakeAmount(0);
//     }

//     // test update max churn rate succeeds
//     function test_updateMaxChurnRate_succeeds() public {
//         vm.prank(exampleTokenCreator);
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.MaxChurnRateUpdated(10);
//         stakeTable.updateMaxChurnRate(10);
//         assertEq(stakeTable.maxNumChurnPerEpoch(), 10);
//     }

//     // test update max churn rate fails if not admin or invalid churn amount
//     function test_revertWhen_updateMaxChurnRate_NotAdminOrInvalidChurnAmount() public {
//         vm.startPrank(makeAddr("randomUser"));
//         vm.expectRevert(
//             abi.encodeWithSelector(
//                 Ownable.OwnableUnauthorizedAccount.selector, makeAddr("randomUser")
//             )
//         );
//         stakeTable.updateMaxChurnRate(10);
//         vm.stopPrank();

//         vm.prank(exampleTokenCreator);
//         vm.expectRevert(S.InvalidValue.selector);
//         stakeTable.updateMaxChurnRate(0);
//     }

//     // test update light client address succeeds
//     function test_updateLightClientAddress_succeeds() public {
//         vm.prank(exampleTokenCreator);
//         vm.expectEmit(false, false, false, true, address(stakeTable));
//         emit AbstractStakeTable.LightClientAddressUpdated(makeAddr("lightClient"));
//         stakeTable.updateLightClientAddress(makeAddr("lightClient"));
//         assertEq(address(stakeTable.lightClient()), makeAddr("lightClient"));
//     }

//     // test update light client address fails if not admin or bad address
//     function test_revertWhen_updateLightClientAddress_NotAdminOrBadAddress() public {
//         vm.startPrank(makeAddr("randomUser"));
//         vm.expectRevert(
//             abi.encodeWithSelector(
//                 Ownable.OwnableUnauthorizedAccount.selector, makeAddr("randomUser")
//             )
//         );
//         stakeTable.updateLightClientAddress(makeAddr("lightClient"));
//         vm.stopPrank();

//         vm.prank(exampleTokenCreator);
//         vm.expectRevert(S.InvalidAddress.selector);
//         stakeTable.updateLightClientAddress(address(0));
//     }

//     // TESTS FOR CURRENT EPOCH
//     function test_initialEpoch_isZero() public view {
//         // assert the current block height is initialBlockHeight
//         uint64 initialBlockHeight = 0;
//         (, uint64 currentBlockHeight,) = lcMock.finalizedState();
//         assertEq(currentBlockHeight, initialBlockHeight);

//         // Calculate the expected epoch
//         uint64 expectedEpoch = 0;

//         // Call the currentEpoch function
//         uint64 currentEpoch = stakeTable.currentEpoch();

//         // Assert that the current epoch is calculated correctly
//         assertEq(currentEpoch, expectedEpoch);
//         assertEq(currentEpoch, 0);
//     }

//     function test_currentEpoch_isUpdated() public {
//         test_initialEpoch_isZero();

//         // set new finalized state on the light client contract
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 10, BN254.ScalarField.wrap(0)));

//         // verify the current epoch is updated and is non-zero
//         assertNotEq(stakeTable.currentEpoch(), 0);

//         // verify the expected epoch
//         uint64 expectedEpoch = 10; // 10 / 1
//         assertEq(stakeTable.currentEpoch(), expectedEpoch);
//     }

//     function test_currentEpoch_blocksPerEpochNotOne() public {
//         setUpCustom(10, /*churn*/ 3 /*blocksPerEpoch*/ );
//         test_initialEpoch_isZero();
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 2, BN254.ScalarField.wrap(0)));
//         assertEq(stakeTable.currentEpoch(), 0);
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 3, BN254.ScalarField.wrap(0)));
//         assertEq(stakeTable.currentEpoch(), 1);
//     }

//     // test various edge cases for the currentEpoch
//     function test_currentEpoch_edgeCases() public {
//         // test edge case when the block height is less than the hotShotBlocksPerEpoch
//         uint64 hotShotBlockHeight = 0;
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
//         );
//         assertEq(stakeTable.currentEpoch(), 0);

//         // test edge case when the block height is exactly divisible by the hotShotBlocksPerEpoch
//         hotShotBlockHeight = 1;
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
//         );
//         assertEq(stakeTable.currentEpoch(), 1);

//         // test edge case when the block height is greater than the hotShotBlocksPerEpoch
//         hotShotBlockHeight = 2;
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
//         );
//         assertEq(stakeTable.currentEpoch(), 2);

//         // test edge case when the block height is very large
//         hotShotBlockHeight = type(uint64).max;
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, hotShotBlockHeight, BN254.ScalarField.wrap(0))
//         );
//         assertEq(stakeTable.currentEpoch(), hotShotBlockHeight / hotShotBlocksPerEpoch);
//     }

//     // TESTS FOR NEXT REGISTRATION EPOCH

//     /// @notice test the next available epoch (registration/exit) when the current epoch is zero
//     function test_nextAvailableEpoch_whenCurrentEpochIsZero() public {
//         // test for registration
//         // check that the current epoch is zero
//         assertEq(stakeTable.currentEpoch(), 0);

//         // check that the first registration epoch is equal to one
//         assertEq(stakeTable.registrationEpoch(), 1);

//         // assert that the next registration epoch is equal to the  registration
//         // epoch
//         stakeTable.mockPushToRegistrationQueue();
//         assertEq(stakeTable.registrationEpoch(), 1);
//         assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

//         // test for exit
//         // assert that the next exit epoch is equal to the  exit epoch
//         assertEq(stakeTable.exitEpoch(), 1);
//         stakeTable.mockPushToExitQueue();
//         assertEq(stakeTable.exitEpoch(), 1); // the epoch is one as you just pushed one exit
//         assertEq(stakeTable.numPendingExitsInEpoch(), 1);
//     }

//     /// @notice test the next available epoch (registration/exit) when the current epoch + 1
//     /// is greater than the  registration/exit epoch
//     function test_nextAvailableEpoch_whenCurrentEpochPlusOneIsGreaterThanregistrationEpoch()
//         public
//     {
//         // test for registration
//         // set the current epoch to 1 by updating the latest hotshot block number on the LC
// contract
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
//         assertEq(stakeTable.currentEpoch(), 1);

//         // assert that the registrationEpoch is 1
//         assertEq(stakeTable.registrationEpoch(), 1);
//         assertGe(stakeTable.currentEpoch() + 1, stakeTable.registrationEpoch());

//         // assert that the next registration epoch is equal to stakeTable.currentEpoch() + 1
//         stakeTable.mockPushToRegistrationQueue();
//         assertEq(stakeTable.registrationEpoch(), stakeTable.currentEpoch() + 1);
//         assertEq(stakeTable.numPendingRegistrationsInEpoch(), 0);

//         // test for exit
//         assertEq(stakeTable.exitEpoch(), 1);

//         // assert that the next exit epoch is equal to 2
//         stakeTable.mockPushToExitQueue();
//         assertEq(stakeTable.exitEpoch(), stakeTable.currentEpoch() + 1);
//         assertEq(stakeTable.numPendingExitsInEpoch(), 0);
//     }

//     /// @notice test nextAvailableEpoch when firstAvailableEpoch (registration/exit) is greater
// than
//     /// currentEpoch + 1
//     function test_nextAvailableEpoch_whenFirstAvailableEpochIsGreaterThanCurrentEpochPlusOne()
//         public
//     {
//         // set the current epoch to 1 by updating the latest hotshot block number on the LC
// contract
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
//         assertEq(stakeTable.currentEpoch(), 1);

//         // set the  registration epoch to 3
//         uint64 registrationEpoch = 3;
//         stakeTable.setRegistrationEpoch(registrationEpoch);

//         // assert that the next registration epoch is greater than the current epoch
//         assertGt(stakeTable.registrationEpoch(), stakeTable.currentEpoch());

//         // assert that the next registration epoch is equal to 3
//         stakeTable.mockPushToRegistrationQueue();
//         assertEq(stakeTable.registrationEpoch(), registrationEpoch);
//         assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

//         // set the  registration epoch to max uint64
//         registrationEpoch = type(uint64).max;
//         stakeTable.setRegistrationEpoch(registrationEpoch);

//         // assert that the next registration epoch is equal to max uint64
//         stakeTable.mockPushToRegistrationQueue();
//         assertEq(stakeTable.registrationEpoch(), registrationEpoch);
//         assertEq(stakeTable.numPendingRegistrationsInEpoch(), 2);

//         // test for exit
//         // set the  exit epoch to 3
//         uint64 exitEpoch = 3;
//         stakeTable.setExitEpoch(exitEpoch);

//         // assert that the next exit epoch is equal to 3
//         stakeTable.mockPushToExitQueue();
//         assertEq(stakeTable.exitEpoch(), exitEpoch);
//         assertEq(stakeTable.numPendingExitsInEpoch(), 1);
//     }

//     /// @notice test registrationEpoch when the current epoch + 1 is equal to the
//     /// registration/exit epoch
//     function test_registrationEpoch_whenCurrentEpochPlusOneIsEqualToregistrationEpoch() public {
//         // test for registration
//         // set the current epoch to 1 by updating the latest hotshot block number on the LC
// contract
//         lcMock.setFinalizedState(LightClient.LightClientState(0, 1, BN254.ScalarField.wrap(0)));
//         assertEq(stakeTable.currentEpoch(), 1);

//         // set the  registration epoch to 2
//         uint64 registrationEpoch = 2;
//         stakeTable.setRegistrationEpoch(registrationEpoch);

//         // assert that the next registration epoch is equal to 2
//         stakeTable.mockPushToRegistrationQueue();
//         assertEq(stakeTable.registrationEpoch(), registrationEpoch);
//         assertEq(stakeTable.numPendingRegistrationsInEpoch(), 1);

//         // test for exit
//         // set the  exit epoch to 2
//         uint64 exitEpoch = 2;
//         stakeTable.setExitEpoch(exitEpoch);

//         // assert that the next exit epoch is equal to 2
//         stakeTable.mockPushToExitQueue();
//         assertEq(stakeTable.exitEpoch(), exitEpoch);
//         assertEq(stakeTable.numPendingExitsInEpoch(), 1);
//     }

//     // test pushToRegistrationQueue reverts when the current epoch is max uint64
//     // note, the current epoch is max uint64 only when the hotshot blocks per epoch is 1
//     // and the hotshot block number is max uint64
//     function test_revertWhen_pushToRegistrationQueue_whenCurrentEpochIsMaxUint64() public {
//         // set the current hotshot block number to max uint64
//         lcMock.setFinalizedState(
//             LightClient.LightClientState(0, type(uint64).max, BN254.ScalarField.wrap(0))
//         );
//         assertEq(stakeTable.currentEpoch(), type(uint64).max);

//         // set the hotshot blocks per epoch to 1
//         vm.prank(exampleTokenCreator);
//         stakeTable.mockUpdateHotShotBlocksPerEpoch(1);
//         assertEq(stakeTable.hotShotBlocksPerEpoch(), 1);

//         // push to registration queue and expect a panic due to arithmetic overflow
//         vm.expectRevert(stdError.arithmeticError);
//         stakeTable.mockPushToRegistrationQueue();
//     }
// }
