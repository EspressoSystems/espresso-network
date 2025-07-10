// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import "forge-std/Test.sol";
import "forge-std/StdInvariant.sol";
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

contract StakeTableV2Handler is Test {
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

    // Ghost variables for tracking
    uint256 public ghost_totalDelegated;
    uint256 public ghost_totalUndelegated;

    constructor(MockStakeTableV2 _stakeTable, MockERC20 _token) {
        stakeTable = _stakeTable;
        token = _token;

        // Set initial balances
        initialBalances[VALIDATOR1] = INITIAL_BALANCE;
        initialBalances[VALIDATOR2] = INITIAL_BALANCE;
        initialBalances[DELEGATOR1] = INITIAL_BALANCE;
        initialBalances[DELEGATOR2] = INITIAL_BALANCE;
    }

    function registerValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        (, StakeTable.ValidatorStatus status) = stakeTable.validators(validator);
        if (status != StakeTable.ValidatorStatus.Unknown) {
            return;
        }

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

    function delegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 balanceBefore = token.balanceOf(delegator);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) {
            // Track successful delegation
            uint256 balanceAfter = token.balanceOf(delegator);
            uint256 actualAmount = balanceBefore - balanceAfter;
            ghost_totalDelegated += actualAmount;
        } catch { }
    }

    function delegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 balance = token.balanceOf(delegator);
        if (balance == 0) return;

        amount = bound(amount, 1, balance);

        uint256 balanceBefore = token.balanceOf(delegator);

        vm.prank(delegator);
        try stakeTable.delegate(validator, amount) {
            // Track successful delegation
            uint256 balanceAfter = token.balanceOf(delegator);
            uint256 actualAmount = balanceBefore - balanceAfter;
            ghost_totalDelegated += actualAmount;
        } catch { }
    }

    function undelegate_Any(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount)
        public
    {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) {
            ghost_totalUndelegated += amount;
        } catch { }
    }

    function undelegate_Ok(uint256 delegatorIndex, uint256 validatorIndex, uint256 amount) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        uint256 delegatedAmount = stakeTable.delegations(validator, delegator);
        if (delegatedAmount == 0) return;

        amount = bound(amount, 1, delegatedAmount);

        vm.prank(delegator);
        try stakeTable.undelegate(validator, amount) {
            ghost_totalUndelegated += amount;
        } catch { }
    }

    function claimWithdrawal(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

        vm.prank(delegator);
        try stakeTable.claimWithdrawal(validator) { } catch { }
    }

    function deregisterValidator(uint256 validatorIndex) public {
        address validator = validators[validatorIndex % 2];

        vm.prank(validator);
        try stakeTable.deregisterValidator() { } catch { }
    }

    function claimValidatorExit(uint256 delegatorIndex, uint256 validatorIndex) public {
        address delegator = delegators[delegatorIndex % 2];
        address validator = validators[validatorIndex % 2];

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
}

contract StakeTableV2InvariantTest is StdInvariant, Test {
    MockStakeTableV2 public stakeTable;
    MockERC20 public token;
    MockLightClient public lightClient;
    StakeTableV2Handler public handler;

    address public constant VALIDATOR1 = address(0x1000);
    address public constant VALIDATOR2 = address(0x2000);
    address public constant DELEGATOR1 = address(0x3000);
    address public constant DELEGATOR2 = address(0x4000);

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address => uint256) public initialBalances;

    function setUp() public {
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

        // Set up approvals
        vm.prank(VALIDATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(VALIDATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR1);
        token.approve(address(stakeTable), type(uint256).max);

        vm.prank(DELEGATOR2);
        token.approve(address(stakeTable), type(uint256).max);

        // Create handler
        handler = new StakeTableV2Handler(stakeTable, token);

        // Target the handler for invariant testing
        targetContract(address(handler));

        // Configure the number of runs for invariant testing
        vm.deal(address(handler), 100 ether);
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

    /// @dev Balance invariant: wallet + staked + pending withdrawals should equal initial balance
    function invariant_balanceInvariantValidator1() public view {
        assertEq(
            getTotalBalance(VALIDATOR1),
            initialBalances[VALIDATOR1],
            "Validator1 balance invariant violated"
        );
    }

    function invariant_balanceInvariantValidator2() public view {
        assertEq(
            getTotalBalance(VALIDATOR2),
            initialBalances[VALIDATOR2],
            "Validator2 balance invariant violated"
        );
    }

    function invariant_balanceInvariantDelegator1() public view {
        assertEq(
            getTotalBalance(DELEGATOR1),
            initialBalances[DELEGATOR1],
            "Delegator1 balance invariant violated"
        );
    }

    function invariant_balanceInvariantDelegator2() public view {
        assertEq(
            getTotalBalance(DELEGATOR2),
            initialBalances[DELEGATOR2],
            "Delegator2 balance invariant violated"
        );
    }

    /// @dev Total supply should remain constant
    function invariant_totalSupplyInvariant() public view {
        uint256 totalInContract = token.balanceOf(address(stakeTable));
        uint256 totalInWallets = token.balanceOf(VALIDATOR1) + token.balanceOf(VALIDATOR2)
            + token.balanceOf(DELEGATOR1) + token.balanceOf(DELEGATOR2);
        uint256 totalSupply = totalInContract + totalInWallets;
        uint256 expectedSupply = INITIAL_BALANCE * 4;

        assertEq(totalSupply, expectedSupply, "Total supply invariant violated");
    }

    /// @dev Contract balance should equal sum of all delegated amounts
    function invariant_contractBalanceMatchesDelegations() public view {
        uint256 contractBalance = token.balanceOf(address(stakeTable));
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

        assertEq(
            contractBalance,
            totalDelegated + totalPendingUndelegations,
            "Contract balance should equal active delegations + pending undelegations"
        );
    }
}
