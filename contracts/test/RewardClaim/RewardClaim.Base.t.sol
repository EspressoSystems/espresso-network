// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.28;

import "forge-std/Test.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import "../../src/RewardClaim.sol";
import "../../src/EspTokenV2.sol";
import "../mocks/LightClientV3Mock.sol";
import "../../src/interfaces/IRewardClaim.sol";

// Conventions:
// - Always use `validateTestCase()` to validate successful claims before fuzzing without changing
//   the state of the test contracts.
contract RewardClaimTestBase is Test {
    RewardClaim public rewardClaim;
    LightClientV3Mock public lightClient;
    EspTokenV2 public espToken;
    address public owner;
    address public pauser;

    struct RewardClaimTestCase {
        address account;
        uint256 lifetimeRewards;
        bytes authData;
    }

    struct RewardFixtureParams {
        uint256 numAccounts;
        uint64 seed;
    }

    function setUp() public virtual {
        owner = makeAddr("owner");
        pauser = makeAddr("pauser");

        EspTokenV2 espTokenImpl = new EspTokenV2();
        bytes memory espTokenInitData = abi.encodeWithSignature(
            "initialize(address,address,uint256,string,string)",
            owner,
            owner,
            100_000_000 ether,
            "Espresso",
            "ESP"
        );
        ERC1967Proxy espTokenProxy = new ERC1967Proxy(address(espTokenImpl), espTokenInitData);
        espToken = EspTokenV2(payable(address(espTokenProxy)));

        lightClient = new LightClientV3Mock();

        RewardClaim rewardClaimImpl = new RewardClaim();
        bytes memory rewardClaimInitData = abi.encodeWithSignature(
            "initialize(address,address,address,address)",
            owner,
            address(espToken),
            address(lightClient),
            pauser
        );
        ERC1967Proxy rewardClaimProxy =
            new ERC1967Proxy(address(rewardClaimImpl), rewardClaimInitData);
        rewardClaim = RewardClaim(payable(address(rewardClaimProxy)));

        vm.prank(owner);
        espToken.initializeV2(address(rewardClaim));
    }

    // Valid claims should be validated before fuzzing to ensure the test setup is correct.
    function validateTestCase(RewardClaimTestCase memory testCase, uint256 authRoot) internal {
        uint256 snapshot = vm.snapshotState();

        lightClient.setAuthRoot(authRoot);
        vm.prank(testCase.account);
        rewardClaim.claimRewards(testCase.lifetimeRewards, testCase.authData);
        assertEq(espToken.balanceOf(testCase.account), testCase.lifetimeRewards);

        vm.revertToState(snapshot);
    }

    function getRewardFixture(RewardFixtureParams memory params)
        internal
        returns (uint256 authRoot, RewardClaimTestCase[] memory fixtures)
    {
        string[] memory cmds = new string[](4);
        cmds[0] = "diff-test";
        cmds[1] = "gen-reward-fixture";
        cmds[2] = vm.toString(params.numAccounts);
        cmds[3] = vm.toString(params.seed);
        bytes memory result = vm.ffi(cmds);
        (authRoot, fixtures) = abi.decode(result, (uint256, RewardClaimTestCase[]));
    }

    function getFixture(uint64 seed)
        internal
        returns (uint256 authRoot, RewardClaimTestCase memory fixture)
    {
        RewardClaimTestCase[] memory fixtures;
        (authRoot, fixtures) = getRewardFixture(RewardFixtureParams({ numAccounts: 1, seed: seed }));
        fixture = fixtures[0];
    }

    function getFixtures(uint256 numAccounts)
        internal
        returns (uint256 authRoot, RewardClaimTestCase[] memory fixtures)
    {
        return getRewardFixture(RewardFixtureParams({ numAccounts: numAccounts, seed: 0 }));
    }

    function getFixturesWithSeed(uint256 numAccounts, uint64 seed)
        internal
        returns (uint256 authRoot, RewardClaimTestCase[] memory fixtures)
    {
        return getRewardFixture(RewardFixtureParams({ numAccounts: numAccounts, seed: seed }));
    }
}
