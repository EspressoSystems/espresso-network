// SPDX-License-Identifier: MIT
/* solhint-disable func-name-mixedcase, one-contract-per-file */
pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

// Minimal VM interface that works with foundry and echidna
interface IVM {
    function prank(address) external;
}

contract MockERC20 is ERC20 {
    constructor() ERC20("MockToken", "MTK", 18) { }

    function mint(address to, uint256 amount) external {
        _mint(to, amount);
    }
}

contract MockLightClient is ILightClient {
    function blocksPerEpoch() external pure returns (uint64) {
        return 100;
    }
}

contract StakeTableV2PropTestBase {
    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    IVM public constant ivm = IVM(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    address public constant ACTOR1 = address(0x1000);
    address public constant ACTOR2 = address(0x2000);
    address public constant ACTOR3 = address(0x3000);
    address public constant ACTOR4 = address(0x4000);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address account => uint256 balance) public initialBalances;

    // All actors can be both validators and delegators
    address[4] public actors = [ACTOR1, ACTOR2, ACTOR3, ACTOR4];

    function _deployStakeTable() internal {
        address admin = address(this);

        token = new MockERC20();
        lightClient = new MockLightClient();

        // Deploy V1 implementation contract
        StakeTable stakeTableV1Impl = new StakeTable();

        // Encode initialization data for V1
        bytes memory initData = abi.encodeWithSignature(
            "initialize(address,address,uint256,address)",
            address(token),
            address(lightClient),
            EXIT_ESCROW_PERIOD,
            admin
        );

        // Deploy proxy with V1 implementation
        ERC1967Proxy proxy = new ERC1967Proxy(address(stakeTableV1Impl), initData);

        // Deploy V2 implementation contract
        MockStakeTableV2 stakeTableV2Impl = new MockStakeTableV2();

        // Upgrade to V2
        StakeTable(payable(address(proxy))).upgradeToAndCall(
            address(stakeTableV2Impl),
            abi.encodeWithSignature("initializeV2(address,address)", admin, admin)
        );

        // Cast to V2 interface
        stakeTable = MockStakeTableV2(payable(address(proxy)));
    }

    function _mintAndApprove() internal {
        // Mint tokens to accounts and approve stake table
        for (uint256 i = 0; i < actors.length; i++) {
            token.mint(actors[i], INITIAL_BALANCE);
            initialBalances[actors[i]] = INITIAL_BALANCE;

            ivm.prank(actors[i]);
            token.approve(address(stakeTable), type(uint256).max);
        }
    }

    function _genDummyValidatorKeys(address validator)
        internal
        pure
        returns (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        )
    {
        blsVK = BN254.G2Point({
            x0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "x0")))),
            x1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "x1")))),
            y0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "y0")))),
            y1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "y1"))))
        });

        schnorrVK = EdOnBN254.EdOnBN254Point({
            x: uint256(keccak256(abi.encode(validator, "schnorr_x"))),
            y: uint256(keccak256(abi.encode(validator, "schnorr_y")))
        });

        blsSig = BN254.G1Point({
            x: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "sig_x")))),
            y: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "sig_y"))))
        });

        schnorrSig = abi.encode(keccak256(abi.encode(validator, "schnorr_sig")));
    }

    function totalOwnedAmount(address account) public view returns (uint256) {
        uint256 walletBalance = token.balanceOf(account);
        uint256 stakedBalance = 0;
        uint256 pendingWithdrawal = 0;

        // Check delegations from all actors
        for (uint256 i = 0; i < actors.length; i++) {
            stakedBalance += stakeTable.delegations(actors[i], account);

            (uint256 undelegationAmount,) = stakeTable.undelegations(actors[i], account);
            pendingWithdrawal += undelegationAmount;
        }

        return walletBalance + stakedBalance + pendingWithdrawal;
    }

    function _getTotalSupply() internal view returns (uint256 total) {
        total += token.balanceOf(address(stakeTable));
        for (uint256 i = 0; i < actors.length; i++) {
            total += token.balanceOf(actors[i]);
        }
    }

    function _getTotalTrackedFunds() internal view returns (uint256 total) {
        // Sum all active delegations and pending undelegations between all actors
        for (uint256 val = 0; val < actors.length; val++) {
            for (uint256 del = 0; del < actors.length; del++) {
                total += stakeTable.delegations(actors[val], actors[del]);
                (uint256 amount,) = stakeTable.undelegations(actors[val], actors[del]);
                total += amount;
            }
        }
    }
}
