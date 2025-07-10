// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";

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

contract StakeTableV2TestBase {
    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;

    address public constant VALIDATOR1 = address(0x1000);
    address public constant VALIDATOR2 = address(0x2000);
    address public constant DELEGATOR1 = address(0x3000);
    address public constant DELEGATOR2 = address(0x4000);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address => uint256) public initialBalances;

    // Arrays for efficient address lookup
    address[2] public validators = [VALIDATOR1, VALIDATOR2];
    address[2] public delegators = [DELEGATOR1, DELEGATOR2];

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
        // Mint tokens to accounts
        token.mint(VALIDATOR1, INITIAL_BALANCE);
        token.mint(VALIDATOR2, INITIAL_BALANCE);
        token.mint(DELEGATOR1, INITIAL_BALANCE);
        token.mint(DELEGATOR2, INITIAL_BALANCE);

        // Store initial balances
        initialBalances[VALIDATOR1] = INITIAL_BALANCE;
        initialBalances[VALIDATOR2] = INITIAL_BALANCE;
        initialBalances[DELEGATOR1] = INITIAL_BALANCE;
        initialBalances[DELEGATOR2] = INITIAL_BALANCE;
    }

    function _generateValidatorKeys(address validator)
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

    function getTotalBalance(address account) public view returns (uint256) {
        uint256 walletBalance = token.balanceOf(account);
        uint256 stakedBalance = 0;
        uint256 pendingWithdrawal = 0;

        stakedBalance += stakeTable.delegations(VALIDATOR1, account);
        stakedBalance += stakeTable.delegations(VALIDATOR2, account);

        (uint256 undelegation1Amount,) = stakeTable.undelegations(VALIDATOR1, account);
        (uint256 undelegation2Amount,) = stakeTable.undelegations(VALIDATOR2, account);

        pendingWithdrawal += undelegation1Amount;
        pendingWithdrawal += undelegation2Amount;

        return walletBalance + stakedBalance + pendingWithdrawal;
    }

    function _getTotalSupply() internal view returns (uint256) {
        uint256 totalInContract = token.balanceOf(address(stakeTable));
        uint256 totalInWallets = token.balanceOf(VALIDATOR1) + token.balanceOf(VALIDATOR2)
            + token.balanceOf(DELEGATOR1) + token.balanceOf(DELEGATOR2);
        return totalInContract + totalInWallets;
    }

    function _getContractBalanceVsDelegations()
        internal
        view
        returns (uint256 contractBalance, uint256 totalTracked)
    {
        contractBalance = token.balanceOf(address(stakeTable));
        uint256 totalDelegated = 0;

        // Sum all active delegations
        totalDelegated += stakeTable.delegations(VALIDATOR1, VALIDATOR1);
        totalDelegated += stakeTable.delegations(VALIDATOR1, VALIDATOR2);
        totalDelegated += stakeTable.delegations(VALIDATOR1, DELEGATOR1);
        totalDelegated += stakeTable.delegations(VALIDATOR1, DELEGATOR2);

        totalDelegated += stakeTable.delegations(VALIDATOR2, VALIDATOR1);
        totalDelegated += stakeTable.delegations(VALIDATOR2, VALIDATOR2);
        totalDelegated += stakeTable.delegations(VALIDATOR2, DELEGATOR1);
        totalDelegated += stakeTable.delegations(VALIDATOR2, DELEGATOR2);

        // Sum all pending undelegations
        (uint256 v1v1Amount,) = stakeTable.undelegations(VALIDATOR1, VALIDATOR1);
        (uint256 v1v2Amount,) = stakeTable.undelegations(VALIDATOR1, VALIDATOR2);
        (uint256 v1d1Amount,) = stakeTable.undelegations(VALIDATOR1, DELEGATOR1);
        (uint256 v1d2Amount,) = stakeTable.undelegations(VALIDATOR1, DELEGATOR2);

        (uint256 v2v1Amount,) = stakeTable.undelegations(VALIDATOR2, VALIDATOR1);
        (uint256 v2v2Amount,) = stakeTable.undelegations(VALIDATOR2, VALIDATOR2);
        (uint256 v2d1Amount,) = stakeTable.undelegations(VALIDATOR2, DELEGATOR1);
        (uint256 v2d2Amount,) = stakeTable.undelegations(VALIDATOR2, DELEGATOR2);

        uint256 totalPendingUndelegations = v1v1Amount + v1v2Amount + v1d1Amount + v1d2Amount
            + v2v1Amount + v2v2Amount + v2d1Amount + v2d2Amount;

        totalTracked = totalDelegated + totalPendingUndelegations;
    }
}
