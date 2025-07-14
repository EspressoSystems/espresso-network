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
    function startPrank(address) external;
    function stopPrank() external;
    function warp(uint256) external;
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
    IVM public ivm = IVM(0x7109709ECfa91a80626fF3989D68f67F5b1DD12D);

    address[] public actors;
    address[] public allValidators;
    address[] public activeValidators;

    mapping(address validator => uint256 index) public activeValidatorIndex;
    mapping(address validator => bool exists) public activeValidatorMap;

    uint256 public constant INITIAL_BALANCE = 1000000000e18;
    uint256 public trackedTotalSupply;
    uint256 public constant EXIT_ESCROW_PERIOD = 7 days;

    mapping(address account => uint256 balance) public initialBalances;
    mapping(address account => bool exists) public actorMap;

    uint256 public totalActiveDelegations;
    uint256 public totalActiveUndelegations;

    struct ActorFunds {
        uint256 delegations;
        uint256 undelegations;
    }

    mapping(address actor => ActorFunds funds) public trackedActorFunds;

    address internal validator;
    address internal actor;

    modifier useValidator(uint256 validatorIndex) virtual {
        if (allValidators.length == 0) {
            createValidator(validatorIndex);
        }
        validator = allValidators[validatorIndex % allValidators.length];
        _;
    }

    modifier useActiveValidator(uint256 validatorIndex) virtual {
        if (activeValidators.length == 0) {
            createValidator(validatorIndex);
        }
        validator = activeValidators[validatorIndex % activeValidators.length];
        _;
    }

    modifier useActor(uint256 actorIndex) virtual {
        if (actors.length == 0) {
            createActor(actorIndex);
        }
        actor = actors[actorIndex % actors.length];
        ivm.startPrank(actor);
        _;
        ivm.stopPrank();
    }

    constructor() {
        _deployStakeTable();
        trackedTotalSupply = token.totalSupply();
    }

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

    function _genDummyValidatorKeys(address _validator)
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
            x0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "x0")))),
            x1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "x1")))),
            y0: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "y0")))),
            y1: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "y1"))))
        });

        schnorrVK = EdOnBN254.EdOnBN254Point({
            x: uint256(keccak256(abi.encode(_validator, "schnorr_x"))),
            y: uint256(keccak256(abi.encode(_validator, "schnorr_y")))
        });

        blsSig = BN254.G1Point({
            x: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "sig_x")))),
            y: BN254.BaseField.wrap(uint256(keccak256(abi.encode(_validator, "sig_y"))))
        });

        schnorrSig = abi.encode(keccak256(abi.encode(_validator, "schnorr_sig")));
    }

    function totalOwnedAmount(address account) public view returns (uint256) {
        uint256 walletBalance = token.balanceOf(account);
        ActorFunds memory funds = trackedActorFunds[account];
        return walletBalance + funds.delegations + funds.undelegations;
    }

    function _getTotalSupply() internal view returns (uint256 total) {
        total += token.balanceOf(address(stakeTable));
        for (uint256 i = 0; i < actors.length; i++) {
            total += token.balanceOf(actors[i]);
        }
    }

    function _getTotalTrackedFunds() internal view returns (uint256 total) {
        return totalActiveDelegations + totalActiveUndelegations;
    }

    // NOTE: The create validator function is used to generate a new validators successfully.

    function registerValidatorAny(uint256 actorIndex) public useActor(actorIndex) {
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(actor);

        try stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000) {
            _addValidator(actor);
        } catch {
            // Registration failed - this is acceptable for the Any function
        }
    }

    function _newAddress(uint256 seed) internal view returns (address) {
        address candidate = address(uint160(uint256(keccak256(abi.encode(seed)))));

        // If address is already an actor, increment until we find an available one
        while (_isActor(candidate)) {
            candidate = address(uint160(candidate) + 1);
        }

        return candidate;
    }

    function _isActor(address candidate) internal view returns (bool) {
        return actorMap[candidate];
    }

    function _isValidator(address candidate) internal view returns (bool) {
        (, StakeTable.ValidatorStatus status) = stakeTable.validators(candidate);
        return status == StakeTable.ValidatorStatus.Active;
    }

    function _addValidator(address validatorAddress) internal {
        allValidators.push(validatorAddress);

        uint256 newIndex = activeValidators.length;
        activeValidators.push(validatorAddress);
        activeValidatorIndex[validatorAddress] = newIndex;
        activeValidatorMap[validatorAddress] = true;
    }

    function _removeActiveValidator(address validatorAddress) internal {
        if (!activeValidatorMap[validatorAddress]) {
            return; // Validator not active
        }

        uint256 indexToRemove = activeValidatorIndex[validatorAddress];
        uint256 lastIndex = activeValidators.length - 1;

        if (indexToRemove != lastIndex) {
            // Move last element to the position being removed
            address lastValidator = activeValidators[lastIndex];
            activeValidators[indexToRemove] = lastValidator;
            activeValidatorIndex[lastValidator] = indexToRemove;
        }

        // Remove the last element
        activeValidators.pop();
        delete activeValidatorIndex[validatorAddress];
        activeValidatorMap[validatorAddress] = false;
    }

    function deregisterValidatorOk(uint256 validatorIndex) public {
        if (activeValidators.length == 0) {
            return;
        }
        address validatorAddress = activeValidators[validatorIndex % activeValidators.length];

        ivm.prank(validatorAddress);
        stakeTable.deregisterValidator();
        _removeActiveValidator(validatorAddress);
    }

    function deregisterValidatorAny(uint256 validatorIndex) public {
        if (allValidators.length == 0) {
            return;
        }
        address validatorAddress = allValidators[validatorIndex % allValidators.length];

        ivm.prank(validatorAddress);
        try stakeTable.deregisterValidator() {
            _removeActiveValidator(validatorAddress);
        } catch { }
    }

    function createActor(uint256 seed) public returns (address) {
        address actorAddress = _newAddress(seed);

        // Fund the actor with tokens
        token.mint(actorAddress, INITIAL_BALANCE);
        initialBalances[actorAddress] = INITIAL_BALANCE;
        trackedTotalSupply += INITIAL_BALANCE;

        // Approve stake table to spend tokens
        ivm.prank(actorAddress);
        token.approve(address(stakeTable), type(uint256).max);

        // Add to actors array and map
        actors.push(actorAddress);
        actorMap[actorAddress] = true;

        return actorAddress;
    }

    function createValidator(uint256 seed) public returns (address) {
        address validatorAddress = createActor(seed);

        // Register as validator in stake table
        (
            BN254.G2Point memory blsVK,
            EdOnBN254.EdOnBN254Point memory schnorrVK,
            BN254.G1Point memory blsSig,
            bytes memory schnorrSig
        ) = _genDummyValidatorKeys(validatorAddress);

        ivm.prank(validatorAddress);
        stakeTable.registerValidatorV2(blsVK, schnorrVK, blsSig, schnorrSig, 1000);
        _addValidator(validatorAddress);

        return validatorAddress;
    }
}
