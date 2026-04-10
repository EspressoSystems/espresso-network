// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTableV2 } from "./StakeTableV2.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";

/// @title StakeTableV3 - Adds x25519 key and p2p address to validator registration.
///
/// @dev All functions are marked as virtual so that future upgrades can override them.
///
/// @notice V3 adds:
/// 1. x25519 encryption key tracking with uniqueness enforcement
/// 2. p2p address (host:port) validation and registration
/// 3. `registerValidatorV3` replaces `registerValidatorV2` (which is now deprecated)
/// 4. `setX25519Key`, `setP2pAddr`, and `setNetworkConfig` for updating network configuration
///
/// All new functions are virtual. Deprecated overrides are not virtual (same pattern as V2).
contract StakeTableV3 is StakeTableV2 {
    // === Storage ===

    /// @notice x25519 keys that have been registered
    /// @dev Ensures uniqueness of x25519 keys across validators
    mapping(bytes32 x25519Key => bool used) public x25519Keys;

    // === Constants ===

    /// @notice Maximum length for p2p address strings (in bytes)
    uint256 public constant MAX_P2P_ADDR_LENGTH = 512;

    // === Events ===

    /// @notice A validator is registered with x25519 key and p2p address
    event ValidatorRegisteredV3(
        address indexed account,
        BN254.G2Point blsVK,
        EdOnBN254.EdOnBN254Point schnorrVK,
        uint16 commission,
        BN254.G1Point blsSig,
        bytes schnorrSig,
        string metadataUri,
        bytes32 x25519Key,
        string p2pAddr
    );

    /// @notice A validator updated their x25519 encryption key
    /// @param validator The address of the validator
    /// @param x25519Key The new x25519 key
    event X25519KeyUpdated(address indexed validator, bytes32 x25519Key);

    /// @notice A validator updated their p2p address
    /// @param validator The address of the validator
    /// @param p2pAddr The new p2p address
    event P2pAddrUpdated(address indexed validator, string p2pAddr);

    // === Errors ===

    /// The x25519 key is bytes32(0)
    error InvalidX25519Key();

    /// The x25519 key has been previously registered
    error X25519KeyAlreadyUsed();

    /// The p2p address validation failed
    error InvalidP2pAddr();

    // === Constructor ===

    /// @notice Constructor
    /// @dev Disables initializers to prevent implementation contract from being initialized
    constructor() {
        _disableInitializers();
    }

    // === Initializer ===

    /// @notice Reinitialize the contract for V3
    function initializeV3() public virtual onlyOwner reinitializer(3) { }

    // === Version ===

    /// @notice Get the version of the contract
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (3, 0, 0);
    }

    // === Validation ===

    /// @notice Validate a p2p address in host:port format
    /// @param p2pAddr The p2p address to validate
    /// @dev Host must be non-empty, port must be 1-65535
    function validateP2pAddr(string memory p2pAddr) public pure virtual {
        bytes memory b = bytes(p2pAddr);
        require(b.length > 0 && b.length <= MAX_P2P_ADDR_LENGTH, InvalidP2pAddr());

        // Find last ':' (matches Rust's rsplit_once(':'))
        uint256 colonIdx = type(uint256).max;
        for (uint256 i = b.length; i > 0; i--) {
            if (b[i - 1] == 0x3A) {
                colonIdx = i - 1;
                break;
            }
        }
        // Must have colon with non-empty host before it
        require(colonIdx != type(uint256).max && colonIdx > 0, InvalidP2pAddr());

        // Parse port: digits only, 1-65535
        uint256 portLen = b.length - colonIdx - 1;
        require(portLen > 0 && portLen <= 5, InvalidP2pAddr());
        uint256 port = 0;
        for (uint256 i = colonIdx + 1; i < b.length; i++) {
            uint8 c = uint8(b[i]);
            require(c >= 0x30 && c <= 0x39, InvalidP2pAddr());
            port = port * 10 + (c - 0x30);
        }
        require(port > 0 && port <= 65535, InvalidP2pAddr());
    }

    // === Registration ===

    /// @notice Register a validator with x25519 key and p2p address
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    /// @param metadataUri The metadata URI for the validator
    /// @param x25519Key The x25519 encryption key for the validator
    /// @param p2pAddr The p2p address (host:port) for the validator
    function registerValidatorV3(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission,
        string memory metadataUri,
        bytes32 x25519Key,
        string memory p2pAddr
    ) external virtual whenNotPaused {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        require(schnorrSig.length == 64, InvalidSchnorrSig());
        require(commission <= MAX_COMMISSION_BPS, InvalidCommission());
        validateMetadataUri(metadataUri);

        require(x25519Key != bytes32(0), InvalidX25519Key());
        require(!x25519Keys[x25519Key], X25519KeyAlreadyUsed());
        validateP2pAddr(p2pAddr);

        blsKeys[_hashBlsKey(blsVK)] = true;
        schnorrKeys[_hashSchnorrKey(schnorrVK)] = true;
        validators[validator] = Validator({ status: ValidatorStatus.Active, delegatedAmount: 0 });
        commissionTracking[validator] =
            CommissionTracking({ commission: commission, lastIncreaseTime: 0 });
        x25519Keys[x25519Key] = true;

        emit ValidatorRegisteredV3(
            validator,
            blsVK,
            schnorrVK,
            commission,
            blsSig,
            schnorrSig,
            metadataUri,
            x25519Key,
            p2pAddr
        );
    }

    /// @notice Deprecate registerValidatorV2
    /// @dev Users must call registerValidatorV3 instead
    function registerValidatorV2(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory,
        bytes memory,
        uint16,
        string memory
    ) external pure override {
        revert DeprecatedFunction();
    }

    // === Network Config ===

    /// @notice Set or rotate the x25519 encryption key. The key must be unique (never previously
    /// used). To also update the p2p address, use setNetworkConfig instead.
    /// @param x25519Key The new x25519 encryption key (must be unique, never previously used)
    function setX25519Key(bytes32 x25519Key) external virtual whenNotPaused {
        ensureValidatorActive(msg.sender);
        require(x25519Key != bytes32(0), InvalidX25519Key());
        require(!x25519Keys[x25519Key], X25519KeyAlreadyUsed());
        // Old x25519 keys are intentionally not freed. Key operations are rare for ~100 validators.
        x25519Keys[x25519Key] = true;
        emit X25519KeyUpdated(msg.sender, x25519Key);
    }

    /// @notice Update the p2p address. Use for operational changes like server migration.
    /// @param p2pAddr The new p2p address (host:port)
    function setP2pAddr(string memory p2pAddr) external virtual whenNotPaused {
        ensureValidatorActive(msg.sender);
        validateP2pAddr(p2pAddr);
        emit P2pAddrUpdated(msg.sender, p2pAddr);
    }

    /// @notice Set x25519 key and p2p address for an active validator.
    ///
    /// Primary use: initial configuration for validators registered before V3. Also usable to
    /// rotate the x25519 key. The x25519 key must be new (never used before); the p2p address
    /// may be the same as the current one.
    ///
    /// Emits both X25519KeyUpdated and P2pAddrUpdated.
    ///
    /// @param x25519Key The new x25519 encryption key (must be unique, never previously used)
    /// @param p2pAddr The p2p address (host:port)
    function setNetworkConfig(bytes32 x25519Key, string memory p2pAddr)
        external
        virtual
        whenNotPaused
    {
        ensureValidatorActive(msg.sender);

        require(x25519Key != bytes32(0), InvalidX25519Key());
        require(!x25519Keys[x25519Key], X25519KeyAlreadyUsed());
        // Old x25519 keys are intentionally not freed. Key operations are rare for ~100 validators.
        x25519Keys[x25519Key] = true;
        emit X25519KeyUpdated(msg.sender, x25519Key);

        validateP2pAddr(p2pAddr);
        emit P2pAddrUpdated(msg.sender, p2pAddr);
    }
}
