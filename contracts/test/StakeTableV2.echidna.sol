// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { MockStakeTableV2 } from "./MockStakeTableV2.sol";
import { StakeTable } from "../src/StakeTable.sol";
import { ERC20 } from "solmate/tokens/ERC20.sol";
import { BN254 } from "bn254/BN254.sol";
import { EdOnBN254 } from "../src/libraries/EdOnBn254.sol";
import { ILightClient } from "../src/interfaces/ILightClient.sol";
import { ERC1967Proxy } from "@openzeppelin/contracts/proxy/ERC1967/ERC1967Proxy.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

interface IHevm {
    function prank(address) external;
    function startPrank(address) external;
    function stopPrank() external;
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

contract StakeTableV2EchidnaTest {
    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    IHevm public constant vm = IHevm(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    address public constant VALIDATOR1 = address(0x1000);
    address public constant VALIDATOR2 = address(0x2000);
    address public constant DELEGATOR1 = address(0x3000);
    address public constant DELEGATOR2 = address(0x4000);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address => uint256) public initialBalances;

    constructor() {
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

        token.mint(VALIDATOR1, INITIAL_BALANCE);
        token.mint(VALIDATOR2, INITIAL_BALANCE);
        token.mint(DELEGATOR1, INITIAL_BALANCE);
        token.mint(DELEGATOR2, INITIAL_BALANCE);

        initialBalances[VALIDATOR1] = INITIAL_BALANCE;
        initialBalances[VALIDATOR2] = INITIAL_BALANCE;
        initialBalances[DELEGATOR1] = INITIAL_BALANCE;
        initialBalances[DELEGATOR2] = INITIAL_BALANCE;

        vm.prank(VALIDATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(VALIDATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR2);
        token.approve(address(stakeTable), type(uint256).max);
    }

    function registerValidator(address validator) public {
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        (uint256 delegatedAmount, StakeTable.ValidatorStatus status) =
            stakeTable.validators(validator);

        BN254.G2Point memory blsVK = BN254.G2Point({
            x0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "x0")))),
            x1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "x1")))),
            y0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "y0")))),
            y1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "y1"))))
        });

        EdOnBN254.EdOnBN254Point memory schnorrVK = EdOnBN254.EdOnBN254Point({
            x: uint256(keccak256(abi.encode(validator, "schnorr_x"))),
            y: uint256(keccak256(abi.encode(validator, "schnorr_y")))
        });

        BN254.G1Point memory blsSig = BN254.G1Point({
            x: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "sig_x")))),
            y: BN254.BaseField.wrap(uint256(keccak256(abi.encode(validator, "sig_y"))))
        });

        bytes memory schnorrSig = abi.encode(keccak256(abi.encode(validator, "schnorr_sig")));

        vm.prank(validator);
        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) { } catch { }
    }

    function delegate_Any(address delegator, address validator, uint256 amount) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) { } catch { }
    }

    // Functions ensures we are doing a reasonable amount of successful delegations
    function delegate_Ok(address delegator, address validator, uint256 amount) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        amount = amount % (token.balanceOf(delegator) + 1);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) { } catch { }
    }

    function undelegate_Any(address delegator, address validator, uint256 amount) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) { } catch { }
    }

    // Functions ensures we are doing a reasonable amount of successful undelegations
    function undelegate_Ok(address delegator, address validator, uint256 amount) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        amount = amount % (stakeTable.delegations(validator, delegator) + 1);
        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) { } catch { }
    }

    function claimWithdrawal(address delegator, address validator) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        vm.prank(delegator);
        try stakeTable.claimWithdrawal(validator) { } catch { }
    }

    function deregisterValidator(address validator) public {
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        vm.prank(validator);
        try stakeTable.deregisterValidator() { } catch { }
    }

    function claimValidatorExit(address delegator, address validator) public {
        require(delegator == DELEGATOR1 || delegator == DELEGATOR2, "Invalid delegator");
        require(validator == VALIDATOR1 || validator == VALIDATOR2, "Invalid validator");

        vm.prank(delegator);
        try stakeTable.claimValidatorExit(validator) { } catch { }
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

    function echidna_balance_invariant_validator1() public view returns (bool) {
        return getTotalBalance(VALIDATOR1) == initialBalances[VALIDATOR1];
    }

    function echidna_balance_invariant_validator2() public view returns (bool) {
        return getTotalBalance(VALIDATOR2) == initialBalances[VALIDATOR2];
    }

    function echidna_balance_invariant_delegator1() public view returns (bool) {
        return getTotalBalance(DELEGATOR1) == initialBalances[DELEGATOR1];
    }

    function echidna_balance_invariant_delegator2() public view returns (bool) {
        return getTotalBalance(DELEGATOR2) == initialBalances[DELEGATOR2];
    }

    function echidna_total_supply_invariant() public view returns (bool) {
        uint256 totalInContract = token.balanceOf(address(stakeTable));
        uint256 totalInWallets = token.balanceOf(VALIDATOR1) + token.balanceOf(VALIDATOR2)
            + token.balanceOf(DELEGATOR1) + token.balanceOf(DELEGATOR2);
        uint256 totalSupply = totalInContract + totalInWallets;
        uint256 expectedSupply = INITIAL_BALANCE * 4;

        return totalSupply == expectedSupply;
    }
}
