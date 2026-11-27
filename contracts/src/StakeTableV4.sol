// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;

import { StakeTableV3 } from "./StakeTableV3.sol";
import { EdOnBN254 } from "./libraries/EdOnBn254.sol";
import { BN254 } from "bn254/BN254.sol";
import { BLSSig } from "./libraries/BLSSig.sol";

/// @title StakeTableV4 - Adds an SLH-DSA (post-quantum) key to validator registration.
///
/// @dev Proof of concept. All functions are virtual so that future upgrades can override them.
///
/// @notice V4 adds:
/// 1. `registerValidatorV4`, which replaces `registerValidatorV3` and also takes an SLH-DSA key.
///    It emits the unchanged `ValidatorRegisteredV3` event, so nodes that do not know V4 still
///    see the new validator, and then `SlhDsaKeyUpdated`.
/// 2. `updateSlhDsaKey`, for validators registered before V4 and for key rotation.
/// 3. SLH-DSA key uniqueness. The proof of possession is not checked here: nodes check it.
contract StakeTableV4 is StakeTableV3 {
    // === Storage ===

    /// @notice SLH-DSA keys that have been registered, by keccak256 hash of the key
    /// @dev Ensures uniqueness of SLH-DSA keys across validators. Keys are never freed.
    mapping(bytes32 slhDsaKeyHash => bool used) public slhDsaKeys;

    // === Constants ===

    /// @notice Length of an SLH-DSA-SHA2-128s public key in bytes
    uint256 public constant SLH_DSA_KEY_LENGTH = 32;

    /// @notice Length of an SLH-DSA-SHA2-128s signature in bytes
    uint256 public constant SLH_DSA_SIG_LENGTH = 7856;

    // === Events ===

    /// @notice A validator set its SLH-DSA key
    /// @param validator The address of the validator
    /// @param slhDsaVK The SLH-DSA public key
    /// @param slhDsaSig The SLH-DSA signature over the validator address (proof of possession)
    event SlhDsaKeyUpdated(address indexed validator, bytes slhDsaVK, bytes slhDsaSig);

    // === Errors ===

    /// The SLH-DSA key does not have the expected length
    error InvalidSlhDsaKey();

    /// The SLH-DSA signature does not have the expected length
    error InvalidSlhDsaSig();

    /// The SLH-DSA key has been previously registered
    error SlhDsaKeyAlreadyUsed();

    // === Constructor ===

    /// @notice Constructor
    /// @dev Disables initializers to prevent implementation contract from being initialized
    constructor() {
        _disableInitializers();
    }

    // === Initializer ===

    /// @notice Reinitialize the contract for V4
    function initializeV4() public virtual onlyOwner reinitializer(4) { }

    // === Version ===

    /// @notice Get the version of the contract
    function getVersion()
        public
        pure
        virtual
        override
        returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
    {
        return (4, 0, 0);
    }

    // === Registration ===

    /// @notice Register a validator with x25519 key, p2p address and SLH-DSA key
    ///
    /// @param blsVK The BLS verification key
    /// @param schnorrVK The Schnorr verification key
    /// @param blsSig The BLS signature that authenticates the BLS VK
    /// @param schnorrSig The Schnorr signature that authenticates the Schnorr VK
    /// @param commission in % with 2 decimals, from 0.00% (value 0) to 100% (value 10_000)
    /// @param metadataUri The metadata URI for the validator
    /// @param x25519Key The x25519 encryption key (raw little-endian bytes, canonical encoding)
    /// @param p2pAddr The p2p address (host:port) for the validator
    /// @param slhDsaVK The SLH-DSA public key (must be unique, never previously used)
    /// @param slhDsaSig The SLH-DSA signature over the validator address, checked by nodes
    function registerValidatorV4(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission,
        string memory metadataUri,
        bytes32 x25519Key,
        string memory p2pAddr,
        bytes memory slhDsaVK,
        bytes memory slhDsaSig
    ) external virtual whenNotPaused {
        _registerValidator(
            blsVK, schnorrVK, blsSig, schnorrSig, commission, metadataUri, x25519Key, p2pAddr
        );
        _setSlhDsaKey(msg.sender, slhDsaVK, slhDsaSig);
    }

    /// @notice Deprecate registerValidatorV3
    /// @dev Users must call registerValidatorV4 instead
    function registerValidatorV3(
        BN254.G2Point memory,
        EdOnBN254.EdOnBN254Point memory,
        BN254.G1Point memory,
        bytes memory,
        uint16,
        string memory,
        bytes32,
        string memory
    ) external pure override {
        revert DeprecatedFunction();
    }

    // === SLH-DSA key ===

    /// @notice Set or rotate the SLH-DSA key of an active validator. The key must be new.
    /// @param slhDsaVK The SLH-DSA public key (must be unique, never previously used)
    /// @param slhDsaSig The SLH-DSA signature over the validator address, checked by nodes
    function updateSlhDsaKey(bytes memory slhDsaVK, bytes memory slhDsaSig)
        external
        virtual
        whenNotPaused
    {
        ensureValidatorActive(msg.sender);
        _setSlhDsaKey(msg.sender, slhDsaVK, slhDsaSig);
    }

    // === Internal ===

    /// @dev The body of `StakeTableV3.registerValidatorV3`, which is external and so cannot be
    /// called from here. It emits `ValidatorRegisteredV3`, the event nodes without V4 read.
    function _registerValidator(
        BN254.G2Point memory blsVK,
        EdOnBN254.EdOnBN254Point memory schnorrVK,
        BN254.G1Point memory blsSig,
        bytes memory schnorrSig,
        uint16 commission,
        string memory metadataUri,
        bytes32 x25519Key,
        string memory p2pAddr
    ) internal {
        address validator = msg.sender;

        ensureValidatorNotRegistered(validator);
        ensureNonZeroSchnorrKey(schnorrVK);
        ensureNewKeys(blsVK, schnorrVK);

        bytes memory message = abi.encode(validator);
        BLSSig.verifyBlsSig(message, blsSig, blsVK);

        require(schnorrSig.length == 64, InvalidSchnorrSig());
        require(commission <= MAX_COMMISSION_BPS, InvalidCommission());
        validateMetadataUri(metadataUri);

        ensureCanonicalX25519Key(x25519Key);
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

    /// @dev Check and record a new SLH-DSA key. The signature is only length-checked: verifying
    /// SLH-DSA on L1 costs too much gas, so nodes verify it and drop events that fail.
    function _setSlhDsaKey(address validator, bytes memory slhDsaVK, bytes memory slhDsaSig)
        internal
    {
        require(slhDsaVK.length == SLH_DSA_KEY_LENGTH, InvalidSlhDsaKey());
        require(slhDsaSig.length == SLH_DSA_SIG_LENGTH, InvalidSlhDsaSig());
        bytes32 keyHash = keccak256(slhDsaVK);
        require(!slhDsaKeys[keyHash], SlhDsaKeyAlreadyUsed());

        // Old keys are intentionally not freed, as for x25519 keys.
        slhDsaKeys[keyHash] = true;
        emit SlhDsaKeyUpdated(validator, slhDsaVK, slhDsaSig);
    }
}
