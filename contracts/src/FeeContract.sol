// SPDX-License-Identifier: UNLICENSED

pragma solidity ^0.8.0;

import { OwnableUpgradeable } from
    "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import { Initializable } from "@openzeppelin/contracts-upgradeable/proxy/utils/Initializable.sol";
import { UUPSUpgradeable } from
    "@openzeppelin/contracts-upgradeable/proxy/utils/UUPSUpgradeable.sol";

contract FeeContract is Initializable, OwnableUpgradeable, UUPSUpgradeable {
    // === Events ===
    //
    /// @notice Notify a new deposit
    event Deposit(address indexed user, uint256 amount);
    event Log(string func, uint256 gas);

    /// @notice upgrade event when the proxy updates the implementation it's pointing to
    event Upgrade(address implementation);
    // === Constants ===
    //
    /// @notice max amount allowed to be deposited to prevent fat finger errors
    // @TODO confirm this amount with product

    uint256 public immutable MAX_DEPOSIT_AMOUNT = 1 ether;
    uint256 public immutable MIN_DEPOSIT_AMOUNT = 0.001 ether;

    // === Errors ===
    //
    /// @notice error types
    error InvalidUserAddress();
    error DepositTooLarge();
    error DepositTooSmall();
    error FunctionDoesNotExist();
    error NoFunctionCalled();

    /// @notice store user balances in a mapping
    mapping(address user => uint256 amount) public balances;

    /// @notice since the constuctor initializes storage on this contract we disable it
    /// @dev storage is on the proxy contract since it calls this contract via delegatecall
    constructor() {
        _disableInitializers();
    }

    /// @notice This contract is called by the proxy when you deploy this contract
    function initialize() public initializer {
        __Ownable_init(msg.sender); //sets owner to msg.sender
        __UUPSUpgradeable_init();
    }

    /// @notice Revert if a method name does not exist
    fallback() external payable {
        // send / transfer (forwards 2300 gas to this fallback function)
        // call (forwards all of the gas)
        revert FunctionDoesNotExist();
    }

    /// @notice Revert if no method name was called
    receive() external payable {
        revert NoFunctionCalled();
    }

    /// @notice Allows anyone to deposit an ETH balance for any user
    /// @dev the deposit amount is less than a specified threshold to prevent fat finger errors
    function deposit(address user) public payable {
        if (msg.value < MIN_DEPOSIT_AMOUNT) {
            revert DepositTooSmall();
        }
        if (msg.value > MAX_DEPOSIT_AMOUNT) {
            revert DepositTooLarge();
        }
        if (user == address(0)) {
            revert InvalidUserAddress();
        }
        balances[user] += msg.value;
        emit Deposit(user, msg.value);
    }

    /// @notice only the owner can authorize an upgrade
    function _authorizeUpgrade(address newImplementation) internal override onlyOwner {
        emit Upgrade(newImplementation);
    }
}
