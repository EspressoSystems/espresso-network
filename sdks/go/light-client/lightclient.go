// Code generated - DO NOT EDIT.
// This file is a generated binding and any manual changes will be lost.

package lightclient

import (
	"errors"
	"math/big"
	"strings"

	ethereum "github.com/ethereum/go-ethereum"
	"github.com/ethereum/go-ethereum/accounts/abi"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/event"
)

// Reference imports to suppress errors if they are not otherwise used.
var (
	_ = errors.New
	_ = big.NewInt
	_ = strings.NewReader
	_ = ethereum.NotFound
	_ = bind.Bind
	_ = common.Big1
	_ = types.BloomLookup
	_ = event.NewSubscription
	_ = abi.ConvertType
)

// BN254G1Point is an auto generated low-level Go binding around an user-defined struct.
type BN254G1Point struct {
	X *big.Int
	Y *big.Int
}

// IPlonkVerifierPlonkProof is an auto generated low-level Go binding around an user-defined struct.
type IPlonkVerifierPlonkProof struct {
	Wire0                 BN254G1Point
	Wire1                 BN254G1Point
	Wire2                 BN254G1Point
	Wire3                 BN254G1Point
	Wire4                 BN254G1Point
	ProdPerm              BN254G1Point
	Split0                BN254G1Point
	Split1                BN254G1Point
	Split2                BN254G1Point
	Split3                BN254G1Point
	Split4                BN254G1Point
	Zeta                  BN254G1Point
	ZetaOmega             BN254G1Point
	WireEval0             *big.Int
	WireEval1             *big.Int
	WireEval2             *big.Int
	WireEval3             *big.Int
	WireEval4             *big.Int
	SigmaEval0            *big.Int
	SigmaEval1            *big.Int
	SigmaEval2            *big.Int
	SigmaEval3            *big.Int
	ProdPermZetaOmegaEval *big.Int
}

// IPlonkVerifierVerifyingKey is an auto generated low-level Go binding around an user-defined struct.
type IPlonkVerifierVerifyingKey struct {
	DomainSize *big.Int
	NumInputs  *big.Int
	Sigma0     BN254G1Point
	Sigma1     BN254G1Point
	Sigma2     BN254G1Point
	Sigma3     BN254G1Point
	Sigma4     BN254G1Point
	Q1         BN254G1Point
	Q2         BN254G1Point
	Q3         BN254G1Point
	Q4         BN254G1Point
	QM12       BN254G1Point
	QM34       BN254G1Point
	QO         BN254G1Point
	QC         BN254G1Point
	QH1        BN254G1Point
	QH2        BN254G1Point
	QH3        BN254G1Point
	QH4        BN254G1Point
	QEcc       BN254G1Point
	G2LSB      [32]byte
	G2MSB      [32]byte
}

// LightClientLightClientState is an auto generated low-level Go binding around an user-defined struct.
type LightClientLightClientState struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}

// LightClientStakeTableState is an auto generated low-level Go binding around an user-defined struct.
type LightClientStakeTableState struct {
	Threshold      *big.Int
	BlsKeyComm     *big.Int
	SchnorrKeyComm *big.Int
	AmountComm     *big.Int
}

// LightclientMetaData contains all meta data concerning the Lightclient contract.
var LightclientMetaData = &bind.MetaData{
	ABI: "[{\"type\":\"constructor\",\"inputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"UPGRADE_INTERFACE_VERSION\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"string\",\"internalType\":\"string\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"_getVk\",\"inputs\":[],\"outputs\":[{\"name\":\"vk\",\"type\":\"tuple\",\"internalType\":\"structIPlonkVerifier.VerifyingKey\",\"components\":[{\"name\":\"domainSize\",\"type\":\"uint256\",\"internalType\":\"uint256\"},{\"name\":\"numInputs\",\"type\":\"uint256\",\"internalType\":\"uint256\"},{\"name\":\"sigma0\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"sigma1\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"sigma2\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"sigma3\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"sigma4\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"q1\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"q2\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"q3\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"q4\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qM12\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qM34\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qO\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qC\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qH1\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qH2\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qH3\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qH4\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"qEcc\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"g2LSB\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"},{\"name\":\"g2MSB\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"}]}],\"stateMutability\":\"pure\"},{\"type\":\"function\",\"name\":\"currentBlockNumber\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"disablePermissionedProverMode\",\"inputs\":[],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"finalizedState\",\"inputs\":[],\"outputs\":[{\"name\":\"viewNum\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"genesisStakeTableState\",\"inputs\":[],\"outputs\":[{\"name\":\"threshold\",\"type\":\"uint256\",\"internalType\":\"uint256\"},{\"name\":\"blsKeyComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"schnorrKeyComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"amountComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"genesisState\",\"inputs\":[],\"outputs\":[{\"name\":\"viewNum\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"getHotShotCommitment\",\"inputs\":[{\"name\":\"hotShotBlockHeight\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"outputs\":[{\"name\":\"hotShotBlockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"hotshotBlockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"getStateHistoryCount\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"getVersion\",\"inputs\":[],\"outputs\":[{\"name\":\"majorVersion\",\"type\":\"uint8\",\"internalType\":\"uint8\"},{\"name\":\"minorVersion\",\"type\":\"uint8\",\"internalType\":\"uint8\"},{\"name\":\"patchVersion\",\"type\":\"uint8\",\"internalType\":\"uint8\"}],\"stateMutability\":\"pure\"},{\"type\":\"function\",\"name\":\"initialize\",\"inputs\":[{\"name\":\"_genesis\",\"type\":\"tuple\",\"internalType\":\"structLightClient.LightClientState\",\"components\":[{\"name\":\"viewNum\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}]},{\"name\":\"_genesisStakeTableState\",\"type\":\"tuple\",\"internalType\":\"structLightClient.StakeTableState\",\"components\":[{\"name\":\"threshold\",\"type\":\"uint256\",\"internalType\":\"uint256\"},{\"name\":\"blsKeyComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"schnorrKeyComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"amountComm\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}]},{\"name\":\"_stateHistoryRetentionPeriod\",\"type\":\"uint32\",\"internalType\":\"uint32\"},{\"name\":\"owner\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"isPermissionedProverEnabled\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"bool\",\"internalType\":\"bool\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"lagOverEscapeHatchThreshold\",\"inputs\":[{\"name\":\"blockNumber\",\"type\":\"uint256\",\"internalType\":\"uint256\"},{\"name\":\"blockThreshold\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"outputs\":[{\"name\":\"\",\"type\":\"bool\",\"internalType\":\"bool\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"newFinalizedState\",\"inputs\":[{\"name\":\"newState\",\"type\":\"tuple\",\"internalType\":\"structLightClient.LightClientState\",\"components\":[{\"name\":\"viewNum\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"blockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}]},{\"name\":\"proof\",\"type\":\"tuple\",\"internalType\":\"structIPlonkVerifier.PlonkProof\",\"components\":[{\"name\":\"wire0\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"wire1\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"wire2\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"wire3\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"wire4\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"prodPerm\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"split0\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"split1\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"split2\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"split3\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"split4\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"zeta\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"zetaOmega\",\"type\":\"tuple\",\"internalType\":\"structBN254.G1Point\",\"components\":[{\"name\":\"x\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"},{\"name\":\"y\",\"type\":\"uint256\",\"internalType\":\"BN254.BaseField\"}]},{\"name\":\"wireEval0\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"wireEval1\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"wireEval2\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"wireEval3\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"wireEval4\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"sigmaEval0\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"sigmaEval1\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"sigmaEval2\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"sigmaEval3\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"},{\"name\":\"prodPermZetaOmegaEval\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}]}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"owner\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"address\",\"internalType\":\"address\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"permissionedProver\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"address\",\"internalType\":\"address\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"proxiableUUID\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"renounceOwnership\",\"inputs\":[],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"setPermissionedProver\",\"inputs\":[{\"name\":\"prover\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"setstateHistoryRetentionPeriod\",\"inputs\":[{\"name\":\"historySeconds\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"stateHistoryCommitments\",\"inputs\":[{\"name\":\"\",\"type\":\"uint256\",\"internalType\":\"uint256\"}],\"outputs\":[{\"name\":\"l1BlockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"l1BlockTimestamp\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"hotShotBlockHeight\",\"type\":\"uint64\",\"internalType\":\"uint64\"},{\"name\":\"hotShotBlockCommRoot\",\"type\":\"uint256\",\"internalType\":\"BN254.ScalarField\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"stateHistoryFirstIndex\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint64\",\"internalType\":\"uint64\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"stateHistoryRetentionPeriod\",\"inputs\":[],\"outputs\":[{\"name\":\"\",\"type\":\"uint32\",\"internalType\":\"uint32\"}],\"stateMutability\":\"view\"},{\"type\":\"function\",\"name\":\"transferOwnership\",\"inputs\":[{\"name\":\"newOwner\",\"type\":\"address\",\"internalType\":\"address\"}],\"outputs\":[],\"stateMutability\":\"nonpayable\"},{\"type\":\"function\",\"name\":\"upgradeToAndCall\",\"inputs\":[{\"name\":\"newImplementation\",\"type\":\"address\",\"internalType\":\"address\"},{\"name\":\"data\",\"type\":\"bytes\",\"internalType\":\"bytes\"}],\"outputs\":[],\"stateMutability\":\"payable\"},{\"type\":\"event\",\"name\":\"Initialized\",\"inputs\":[{\"name\":\"version\",\"type\":\"uint64\",\"indexed\":false,\"internalType\":\"uint64\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"NewState\",\"inputs\":[{\"name\":\"viewNum\",\"type\":\"uint64\",\"indexed\":true,\"internalType\":\"uint64\"},{\"name\":\"blockHeight\",\"type\":\"uint64\",\"indexed\":true,\"internalType\":\"uint64\"},{\"name\":\"blockCommRoot\",\"type\":\"uint256\",\"indexed\":false,\"internalType\":\"BN254.ScalarField\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"OwnershipTransferred\",\"inputs\":[{\"name\":\"previousOwner\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"},{\"name\":\"newOwner\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"PermissionedProverNotRequired\",\"inputs\":[],\"anonymous\":false},{\"type\":\"event\",\"name\":\"PermissionedProverRequired\",\"inputs\":[{\"name\":\"permissionedProver\",\"type\":\"address\",\"indexed\":false,\"internalType\":\"address\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"Upgrade\",\"inputs\":[{\"name\":\"implementation\",\"type\":\"address\",\"indexed\":false,\"internalType\":\"address\"}],\"anonymous\":false},{\"type\":\"event\",\"name\":\"Upgraded\",\"inputs\":[{\"name\":\"implementation\",\"type\":\"address\",\"indexed\":true,\"internalType\":\"address\"}],\"anonymous\":false},{\"type\":\"error\",\"name\":\"AddressEmptyCode\",\"inputs\":[{\"name\":\"target\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"ERC1967InvalidImplementation\",\"inputs\":[{\"name\":\"implementation\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"ERC1967NonPayable\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"FailedInnerCall\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InsufficientSnapshotHistory\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidAddress\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidArgs\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidHotShotBlockForCommitmentCheck\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidInitialization\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidMaxStateHistory\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"InvalidProof\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"NoChangeRequired\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"NotInitializing\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"OutdatedState\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"OwnableInvalidOwner\",\"inputs\":[{\"name\":\"owner\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"OwnableUnauthorizedAccount\",\"inputs\":[{\"name\":\"account\",\"type\":\"address\",\"internalType\":\"address\"}]},{\"type\":\"error\",\"name\":\"ProverNotPermissioned\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"UUPSUnauthorizedCallContext\",\"inputs\":[]},{\"type\":\"error\",\"name\":\"UUPSUnsupportedProxiableUUID\",\"inputs\":[{\"name\":\"slot\",\"type\":\"bytes32\",\"internalType\":\"bytes32\"}]},{\"type\":\"error\",\"name\":\"WrongStakeTableUsed\",\"inputs\":[]}]",
}

// LightclientABI is the input ABI used to generate the binding from.
// Deprecated: Use LightclientMetaData.ABI instead.
var LightclientABI = LightclientMetaData.ABI

// Lightclient is an auto generated Go binding around an Ethereum contract.
type Lightclient struct {
	LightclientCaller     // Read-only binding to the contract
	LightclientTransactor // Write-only binding to the contract
	LightclientFilterer   // Log filterer for contract events
}

// LightclientCaller is an auto generated read-only Go binding around an Ethereum contract.
type LightclientCaller struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// LightclientTransactor is an auto generated write-only Go binding around an Ethereum contract.
type LightclientTransactor struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// LightclientFilterer is an auto generated log filtering Go binding around an Ethereum contract events.
type LightclientFilterer struct {
	contract *bind.BoundContract // Generic contract wrapper for the low level calls
}

// LightclientSession is an auto generated Go binding around an Ethereum contract,
// with pre-set call and transact options.
type LightclientSession struct {
	Contract     *Lightclient      // Generic contract binding to set the session for
	CallOpts     bind.CallOpts     // Call options to use throughout this session
	TransactOpts bind.TransactOpts // Transaction auth options to use throughout this session
}

// LightclientCallerSession is an auto generated read-only Go binding around an Ethereum contract,
// with pre-set call options.
type LightclientCallerSession struct {
	Contract *LightclientCaller // Generic contract caller binding to set the session for
	CallOpts bind.CallOpts      // Call options to use throughout this session
}

// LightclientTransactorSession is an auto generated write-only Go binding around an Ethereum contract,
// with pre-set transact options.
type LightclientTransactorSession struct {
	Contract     *LightclientTransactor // Generic contract transactor binding to set the session for
	TransactOpts bind.TransactOpts      // Transaction auth options to use throughout this session
}

// LightclientRaw is an auto generated low-level Go binding around an Ethereum contract.
type LightclientRaw struct {
	Contract *Lightclient // Generic contract binding to access the raw methods on
}

// LightclientCallerRaw is an auto generated low-level read-only Go binding around an Ethereum contract.
type LightclientCallerRaw struct {
	Contract *LightclientCaller // Generic read-only contract binding to access the raw methods on
}

// LightclientTransactorRaw is an auto generated low-level write-only Go binding around an Ethereum contract.
type LightclientTransactorRaw struct {
	Contract *LightclientTransactor // Generic write-only contract binding to access the raw methods on
}

// NewLightclient creates a new instance of Lightclient, bound to a specific deployed contract.
func NewLightclient(address common.Address, backend bind.ContractBackend) (*Lightclient, error) {
	contract, err := bindLightclient(address, backend, backend, backend)
	if err != nil {
		return nil, err
	}
	return &Lightclient{LightclientCaller: LightclientCaller{contract: contract}, LightclientTransactor: LightclientTransactor{contract: contract}, LightclientFilterer: LightclientFilterer{contract: contract}}, nil
}

// NewLightclientCaller creates a new read-only instance of Lightclient, bound to a specific deployed contract.
func NewLightclientCaller(address common.Address, caller bind.ContractCaller) (*LightclientCaller, error) {
	contract, err := bindLightclient(address, caller, nil, nil)
	if err != nil {
		return nil, err
	}
	return &LightclientCaller{contract: contract}, nil
}

// NewLightclientTransactor creates a new write-only instance of Lightclient, bound to a specific deployed contract.
func NewLightclientTransactor(address common.Address, transactor bind.ContractTransactor) (*LightclientTransactor, error) {
	contract, err := bindLightclient(address, nil, transactor, nil)
	if err != nil {
		return nil, err
	}
	return &LightclientTransactor{contract: contract}, nil
}

// NewLightclientFilterer creates a new log filterer instance of Lightclient, bound to a specific deployed contract.
func NewLightclientFilterer(address common.Address, filterer bind.ContractFilterer) (*LightclientFilterer, error) {
	contract, err := bindLightclient(address, nil, nil, filterer)
	if err != nil {
		return nil, err
	}
	return &LightclientFilterer{contract: contract}, nil
}

// bindLightclient binds a generic wrapper to an already deployed contract.
func bindLightclient(address common.Address, caller bind.ContractCaller, transactor bind.ContractTransactor, filterer bind.ContractFilterer) (*bind.BoundContract, error) {
	parsed, err := LightclientMetaData.GetAbi()
	if err != nil {
		return nil, err
	}
	return bind.NewBoundContract(address, *parsed, caller, transactor, filterer), nil
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_Lightclient *LightclientRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _Lightclient.Contract.LightclientCaller.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_Lightclient *LightclientRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _Lightclient.Contract.LightclientTransactor.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_Lightclient *LightclientRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _Lightclient.Contract.LightclientTransactor.contract.Transact(opts, method, params...)
}

// Call invokes the (constant) contract method with params as input values and
// sets the output to result. The result type might be a single field for simple
// returns, a slice of interfaces for anonymous returns and a struct for named
// returns.
func (_Lightclient *LightclientCallerRaw) Call(opts *bind.CallOpts, result *[]interface{}, method string, params ...interface{}) error {
	return _Lightclient.Contract.contract.Call(opts, result, method, params...)
}

// Transfer initiates a plain transaction to move funds to the contract, calling
// its default method if one is available.
func (_Lightclient *LightclientTransactorRaw) Transfer(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _Lightclient.Contract.contract.Transfer(opts)
}

// Transact invokes the (paid) contract method with params as input values.
func (_Lightclient *LightclientTransactorRaw) Transact(opts *bind.TransactOpts, method string, params ...interface{}) (*types.Transaction, error) {
	return _Lightclient.Contract.contract.Transact(opts, method, params...)
}

// UPGRADEINTERFACEVERSION is a free data retrieval call binding the contract method 0xad3cb1cc.
//
// Solidity: function UPGRADE_INTERFACE_VERSION() view returns(string)
func (_Lightclient *LightclientCaller) UPGRADEINTERFACEVERSION(opts *bind.CallOpts) (string, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "UPGRADE_INTERFACE_VERSION")

	if err != nil {
		return *new(string), err
	}

	out0 := *abi.ConvertType(out[0], new(string)).(*string)

	return out0, err

}

// UPGRADEINTERFACEVERSION is a free data retrieval call binding the contract method 0xad3cb1cc.
//
// Solidity: function UPGRADE_INTERFACE_VERSION() view returns(string)
func (_Lightclient *LightclientSession) UPGRADEINTERFACEVERSION() (string, error) {
	return _Lightclient.Contract.UPGRADEINTERFACEVERSION(&_Lightclient.CallOpts)
}

// UPGRADEINTERFACEVERSION is a free data retrieval call binding the contract method 0xad3cb1cc.
//
// Solidity: function UPGRADE_INTERFACE_VERSION() view returns(string)
func (_Lightclient *LightclientCallerSession) UPGRADEINTERFACEVERSION() (string, error) {
	return _Lightclient.Contract.UPGRADEINTERFACEVERSION(&_Lightclient.CallOpts)
}

// GetVk is a free data retrieval call binding the contract method 0x12173c2c.
//
// Solidity: function _getVk() pure returns((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32) vk)
func (_Lightclient *LightclientCaller) GetVk(opts *bind.CallOpts) (IPlonkVerifierVerifyingKey, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "_getVk")

	if err != nil {
		return *new(IPlonkVerifierVerifyingKey), err
	}

	out0 := *abi.ConvertType(out[0], new(IPlonkVerifierVerifyingKey)).(*IPlonkVerifierVerifyingKey)

	return out0, err

}

// GetVk is a free data retrieval call binding the contract method 0x12173c2c.
//
// Solidity: function _getVk() pure returns((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32) vk)
func (_Lightclient *LightclientSession) GetVk() (IPlonkVerifierVerifyingKey, error) {
	return _Lightclient.Contract.GetVk(&_Lightclient.CallOpts)
}

// GetVk is a free data retrieval call binding the contract method 0x12173c2c.
//
// Solidity: function _getVk() pure returns((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32) vk)
func (_Lightclient *LightclientCallerSession) GetVk() (IPlonkVerifierVerifyingKey, error) {
	return _Lightclient.Contract.GetVk(&_Lightclient.CallOpts)
}

// CurrentBlockNumber is a free data retrieval call binding the contract method 0x378ec23b.
//
// Solidity: function currentBlockNumber() view returns(uint256)
func (_Lightclient *LightclientCaller) CurrentBlockNumber(opts *bind.CallOpts) (*big.Int, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "currentBlockNumber")

	if err != nil {
		return *new(*big.Int), err
	}

	out0 := *abi.ConvertType(out[0], new(*big.Int)).(**big.Int)

	return out0, err

}

// CurrentBlockNumber is a free data retrieval call binding the contract method 0x378ec23b.
//
// Solidity: function currentBlockNumber() view returns(uint256)
func (_Lightclient *LightclientSession) CurrentBlockNumber() (*big.Int, error) {
	return _Lightclient.Contract.CurrentBlockNumber(&_Lightclient.CallOpts)
}

// CurrentBlockNumber is a free data retrieval call binding the contract method 0x378ec23b.
//
// Solidity: function currentBlockNumber() view returns(uint256)
func (_Lightclient *LightclientCallerSession) CurrentBlockNumber() (*big.Int, error) {
	return _Lightclient.Contract.CurrentBlockNumber(&_Lightclient.CallOpts)
}

// FinalizedState is a free data retrieval call binding the contract method 0x9fdb54a7.
//
// Solidity: function finalizedState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientCaller) FinalizedState(opts *bind.CallOpts) (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "finalizedState")

	outstruct := new(struct {
		ViewNum       uint64
		BlockHeight   uint64
		BlockCommRoot *big.Int
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.ViewNum = *abi.ConvertType(out[0], new(uint64)).(*uint64)
	outstruct.BlockHeight = *abi.ConvertType(out[1], new(uint64)).(*uint64)
	outstruct.BlockCommRoot = *abi.ConvertType(out[2], new(*big.Int)).(**big.Int)

	return *outstruct, err

}

// FinalizedState is a free data retrieval call binding the contract method 0x9fdb54a7.
//
// Solidity: function finalizedState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientSession) FinalizedState() (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.FinalizedState(&_Lightclient.CallOpts)
}

// FinalizedState is a free data retrieval call binding the contract method 0x9fdb54a7.
//
// Solidity: function finalizedState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientCallerSession) FinalizedState() (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.FinalizedState(&_Lightclient.CallOpts)
}

// GenesisStakeTableState is a free data retrieval call binding the contract method 0x426d3194.
//
// Solidity: function genesisStakeTableState() view returns(uint256 threshold, uint256 blsKeyComm, uint256 schnorrKeyComm, uint256 amountComm)
func (_Lightclient *LightclientCaller) GenesisStakeTableState(opts *bind.CallOpts) (struct {
	Threshold      *big.Int
	BlsKeyComm     *big.Int
	SchnorrKeyComm *big.Int
	AmountComm     *big.Int
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "genesisStakeTableState")

	outstruct := new(struct {
		Threshold      *big.Int
		BlsKeyComm     *big.Int
		SchnorrKeyComm *big.Int
		AmountComm     *big.Int
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.Threshold = *abi.ConvertType(out[0], new(*big.Int)).(**big.Int)
	outstruct.BlsKeyComm = *abi.ConvertType(out[1], new(*big.Int)).(**big.Int)
	outstruct.SchnorrKeyComm = *abi.ConvertType(out[2], new(*big.Int)).(**big.Int)
	outstruct.AmountComm = *abi.ConvertType(out[3], new(*big.Int)).(**big.Int)

	return *outstruct, err

}

// GenesisStakeTableState is a free data retrieval call binding the contract method 0x426d3194.
//
// Solidity: function genesisStakeTableState() view returns(uint256 threshold, uint256 blsKeyComm, uint256 schnorrKeyComm, uint256 amountComm)
func (_Lightclient *LightclientSession) GenesisStakeTableState() (struct {
	Threshold      *big.Int
	BlsKeyComm     *big.Int
	SchnorrKeyComm *big.Int
	AmountComm     *big.Int
}, error) {
	return _Lightclient.Contract.GenesisStakeTableState(&_Lightclient.CallOpts)
}

// GenesisStakeTableState is a free data retrieval call binding the contract method 0x426d3194.
//
// Solidity: function genesisStakeTableState() view returns(uint256 threshold, uint256 blsKeyComm, uint256 schnorrKeyComm, uint256 amountComm)
func (_Lightclient *LightclientCallerSession) GenesisStakeTableState() (struct {
	Threshold      *big.Int
	BlsKeyComm     *big.Int
	SchnorrKeyComm *big.Int
	AmountComm     *big.Int
}, error) {
	return _Lightclient.Contract.GenesisStakeTableState(&_Lightclient.CallOpts)
}

// GenesisState is a free data retrieval call binding the contract method 0xd24d933d.
//
// Solidity: function genesisState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientCaller) GenesisState(opts *bind.CallOpts) (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "genesisState")

	outstruct := new(struct {
		ViewNum       uint64
		BlockHeight   uint64
		BlockCommRoot *big.Int
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.ViewNum = *abi.ConvertType(out[0], new(uint64)).(*uint64)
	outstruct.BlockHeight = *abi.ConvertType(out[1], new(uint64)).(*uint64)
	outstruct.BlockCommRoot = *abi.ConvertType(out[2], new(*big.Int)).(**big.Int)

	return *outstruct, err

}

// GenesisState is a free data retrieval call binding the contract method 0xd24d933d.
//
// Solidity: function genesisState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientSession) GenesisState() (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.GenesisState(&_Lightclient.CallOpts)
}

// GenesisState is a free data retrieval call binding the contract method 0xd24d933d.
//
// Solidity: function genesisState() view returns(uint64 viewNum, uint64 blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientCallerSession) GenesisState() (struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.GenesisState(&_Lightclient.CallOpts)
}

// GetHotShotCommitment is a free data retrieval call binding the contract method 0x8584d23f.
//
// Solidity: function getHotShotCommitment(uint256 hotShotBlockHeight) view returns(uint256 hotShotBlockCommRoot, uint64 hotshotBlockHeight)
func (_Lightclient *LightclientCaller) GetHotShotCommitment(opts *bind.CallOpts, hotShotBlockHeight *big.Int) (struct {
	HotShotBlockCommRoot *big.Int
	HotshotBlockHeight   uint64
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "getHotShotCommitment", hotShotBlockHeight)

	outstruct := new(struct {
		HotShotBlockCommRoot *big.Int
		HotshotBlockHeight   uint64
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.HotShotBlockCommRoot = *abi.ConvertType(out[0], new(*big.Int)).(**big.Int)
	outstruct.HotshotBlockHeight = *abi.ConvertType(out[1], new(uint64)).(*uint64)

	return *outstruct, err

}

// GetHotShotCommitment is a free data retrieval call binding the contract method 0x8584d23f.
//
// Solidity: function getHotShotCommitment(uint256 hotShotBlockHeight) view returns(uint256 hotShotBlockCommRoot, uint64 hotshotBlockHeight)
func (_Lightclient *LightclientSession) GetHotShotCommitment(hotShotBlockHeight *big.Int) (struct {
	HotShotBlockCommRoot *big.Int
	HotshotBlockHeight   uint64
}, error) {
	return _Lightclient.Contract.GetHotShotCommitment(&_Lightclient.CallOpts, hotShotBlockHeight)
}

// GetHotShotCommitment is a free data retrieval call binding the contract method 0x8584d23f.
//
// Solidity: function getHotShotCommitment(uint256 hotShotBlockHeight) view returns(uint256 hotShotBlockCommRoot, uint64 hotshotBlockHeight)
func (_Lightclient *LightclientCallerSession) GetHotShotCommitment(hotShotBlockHeight *big.Int) (struct {
	HotShotBlockCommRoot *big.Int
	HotshotBlockHeight   uint64
}, error) {
	return _Lightclient.Contract.GetHotShotCommitment(&_Lightclient.CallOpts, hotShotBlockHeight)
}

// GetStateHistoryCount is a free data retrieval call binding the contract method 0xf9e50d19.
//
// Solidity: function getStateHistoryCount() view returns(uint256)
func (_Lightclient *LightclientCaller) GetStateHistoryCount(opts *bind.CallOpts) (*big.Int, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "getStateHistoryCount")

	if err != nil {
		return *new(*big.Int), err
	}

	out0 := *abi.ConvertType(out[0], new(*big.Int)).(**big.Int)

	return out0, err

}

// GetStateHistoryCount is a free data retrieval call binding the contract method 0xf9e50d19.
//
// Solidity: function getStateHistoryCount() view returns(uint256)
func (_Lightclient *LightclientSession) GetStateHistoryCount() (*big.Int, error) {
	return _Lightclient.Contract.GetStateHistoryCount(&_Lightclient.CallOpts)
}

// GetStateHistoryCount is a free data retrieval call binding the contract method 0xf9e50d19.
//
// Solidity: function getStateHistoryCount() view returns(uint256)
func (_Lightclient *LightclientCallerSession) GetStateHistoryCount() (*big.Int, error) {
	return _Lightclient.Contract.GetStateHistoryCount(&_Lightclient.CallOpts)
}

// GetVersion is a free data retrieval call binding the contract method 0x0d8e6e2c.
//
// Solidity: function getVersion() pure returns(uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
func (_Lightclient *LightclientCaller) GetVersion(opts *bind.CallOpts) (struct {
	MajorVersion uint8
	MinorVersion uint8
	PatchVersion uint8
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "getVersion")

	outstruct := new(struct {
		MajorVersion uint8
		MinorVersion uint8
		PatchVersion uint8
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.MajorVersion = *abi.ConvertType(out[0], new(uint8)).(*uint8)
	outstruct.MinorVersion = *abi.ConvertType(out[1], new(uint8)).(*uint8)
	outstruct.PatchVersion = *abi.ConvertType(out[2], new(uint8)).(*uint8)

	return *outstruct, err

}

// GetVersion is a free data retrieval call binding the contract method 0x0d8e6e2c.
//
// Solidity: function getVersion() pure returns(uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
func (_Lightclient *LightclientSession) GetVersion() (struct {
	MajorVersion uint8
	MinorVersion uint8
	PatchVersion uint8
}, error) {
	return _Lightclient.Contract.GetVersion(&_Lightclient.CallOpts)
}

// GetVersion is a free data retrieval call binding the contract method 0x0d8e6e2c.
//
// Solidity: function getVersion() pure returns(uint8 majorVersion, uint8 minorVersion, uint8 patchVersion)
func (_Lightclient *LightclientCallerSession) GetVersion() (struct {
	MajorVersion uint8
	MinorVersion uint8
	PatchVersion uint8
}, error) {
	return _Lightclient.Contract.GetVersion(&_Lightclient.CallOpts)
}

// IsPermissionedProverEnabled is a free data retrieval call binding the contract method 0x826e41fc.
//
// Solidity: function isPermissionedProverEnabled() view returns(bool)
func (_Lightclient *LightclientCaller) IsPermissionedProverEnabled(opts *bind.CallOpts) (bool, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "isPermissionedProverEnabled")

	if err != nil {
		return *new(bool), err
	}

	out0 := *abi.ConvertType(out[0], new(bool)).(*bool)

	return out0, err

}

// IsPermissionedProverEnabled is a free data retrieval call binding the contract method 0x826e41fc.
//
// Solidity: function isPermissionedProverEnabled() view returns(bool)
func (_Lightclient *LightclientSession) IsPermissionedProverEnabled() (bool, error) {
	return _Lightclient.Contract.IsPermissionedProverEnabled(&_Lightclient.CallOpts)
}

// IsPermissionedProverEnabled is a free data retrieval call binding the contract method 0x826e41fc.
//
// Solidity: function isPermissionedProverEnabled() view returns(bool)
func (_Lightclient *LightclientCallerSession) IsPermissionedProverEnabled() (bool, error) {
	return _Lightclient.Contract.IsPermissionedProverEnabled(&_Lightclient.CallOpts)
}

// LagOverEscapeHatchThreshold is a free data retrieval call binding the contract method 0xe0303301.
//
// Solidity: function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) view returns(bool)
func (_Lightclient *LightclientCaller) LagOverEscapeHatchThreshold(opts *bind.CallOpts, blockNumber *big.Int, blockThreshold *big.Int) (bool, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "lagOverEscapeHatchThreshold", blockNumber, blockThreshold)

	if err != nil {
		return *new(bool), err
	}

	out0 := *abi.ConvertType(out[0], new(bool)).(*bool)

	return out0, err

}

// LagOverEscapeHatchThreshold is a free data retrieval call binding the contract method 0xe0303301.
//
// Solidity: function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) view returns(bool)
func (_Lightclient *LightclientSession) LagOverEscapeHatchThreshold(blockNumber *big.Int, blockThreshold *big.Int) (bool, error) {
	return _Lightclient.Contract.LagOverEscapeHatchThreshold(&_Lightclient.CallOpts, blockNumber, blockThreshold)
}

// LagOverEscapeHatchThreshold is a free data retrieval call binding the contract method 0xe0303301.
//
// Solidity: function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) view returns(bool)
func (_Lightclient *LightclientCallerSession) LagOverEscapeHatchThreshold(blockNumber *big.Int, blockThreshold *big.Int) (bool, error) {
	return _Lightclient.Contract.LagOverEscapeHatchThreshold(&_Lightclient.CallOpts, blockNumber, blockThreshold)
}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_Lightclient *LightclientCaller) Owner(opts *bind.CallOpts) (common.Address, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "owner")

	if err != nil {
		return *new(common.Address), err
	}

	out0 := *abi.ConvertType(out[0], new(common.Address)).(*common.Address)

	return out0, err

}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_Lightclient *LightclientSession) Owner() (common.Address, error) {
	return _Lightclient.Contract.Owner(&_Lightclient.CallOpts)
}

// Owner is a free data retrieval call binding the contract method 0x8da5cb5b.
//
// Solidity: function owner() view returns(address)
func (_Lightclient *LightclientCallerSession) Owner() (common.Address, error) {
	return _Lightclient.Contract.Owner(&_Lightclient.CallOpts)
}

// PermissionedProver is a free data retrieval call binding the contract method 0x313df7b1.
//
// Solidity: function permissionedProver() view returns(address)
func (_Lightclient *LightclientCaller) PermissionedProver(opts *bind.CallOpts) (common.Address, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "permissionedProver")

	if err != nil {
		return *new(common.Address), err
	}

	out0 := *abi.ConvertType(out[0], new(common.Address)).(*common.Address)

	return out0, err

}

// PermissionedProver is a free data retrieval call binding the contract method 0x313df7b1.
//
// Solidity: function permissionedProver() view returns(address)
func (_Lightclient *LightclientSession) PermissionedProver() (common.Address, error) {
	return _Lightclient.Contract.PermissionedProver(&_Lightclient.CallOpts)
}

// PermissionedProver is a free data retrieval call binding the contract method 0x313df7b1.
//
// Solidity: function permissionedProver() view returns(address)
func (_Lightclient *LightclientCallerSession) PermissionedProver() (common.Address, error) {
	return _Lightclient.Contract.PermissionedProver(&_Lightclient.CallOpts)
}

// ProxiableUUID is a free data retrieval call binding the contract method 0x52d1902d.
//
// Solidity: function proxiableUUID() view returns(bytes32)
func (_Lightclient *LightclientCaller) ProxiableUUID(opts *bind.CallOpts) ([32]byte, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "proxiableUUID")

	if err != nil {
		return *new([32]byte), err
	}

	out0 := *abi.ConvertType(out[0], new([32]byte)).(*[32]byte)

	return out0, err

}

// ProxiableUUID is a free data retrieval call binding the contract method 0x52d1902d.
//
// Solidity: function proxiableUUID() view returns(bytes32)
func (_Lightclient *LightclientSession) ProxiableUUID() ([32]byte, error) {
	return _Lightclient.Contract.ProxiableUUID(&_Lightclient.CallOpts)
}

// ProxiableUUID is a free data retrieval call binding the contract method 0x52d1902d.
//
// Solidity: function proxiableUUID() view returns(bytes32)
func (_Lightclient *LightclientCallerSession) ProxiableUUID() ([32]byte, error) {
	return _Lightclient.Contract.ProxiableUUID(&_Lightclient.CallOpts)
}

// StateHistoryCommitments is a free data retrieval call binding the contract method 0x02b592f3.
//
// Solidity: function stateHistoryCommitments(uint256 ) view returns(uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, uint256 hotShotBlockCommRoot)
func (_Lightclient *LightclientCaller) StateHistoryCommitments(opts *bind.CallOpts, arg0 *big.Int) (struct {
	L1BlockHeight        uint64
	L1BlockTimestamp     uint64
	HotShotBlockHeight   uint64
	HotShotBlockCommRoot *big.Int
}, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "stateHistoryCommitments", arg0)

	outstruct := new(struct {
		L1BlockHeight        uint64
		L1BlockTimestamp     uint64
		HotShotBlockHeight   uint64
		HotShotBlockCommRoot *big.Int
	})
	if err != nil {
		return *outstruct, err
	}

	outstruct.L1BlockHeight = *abi.ConvertType(out[0], new(uint64)).(*uint64)
	outstruct.L1BlockTimestamp = *abi.ConvertType(out[1], new(uint64)).(*uint64)
	outstruct.HotShotBlockHeight = *abi.ConvertType(out[2], new(uint64)).(*uint64)
	outstruct.HotShotBlockCommRoot = *abi.ConvertType(out[3], new(*big.Int)).(**big.Int)

	return *outstruct, err

}

// StateHistoryCommitments is a free data retrieval call binding the contract method 0x02b592f3.
//
// Solidity: function stateHistoryCommitments(uint256 ) view returns(uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, uint256 hotShotBlockCommRoot)
func (_Lightclient *LightclientSession) StateHistoryCommitments(arg0 *big.Int) (struct {
	L1BlockHeight        uint64
	L1BlockTimestamp     uint64
	HotShotBlockHeight   uint64
	HotShotBlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.StateHistoryCommitments(&_Lightclient.CallOpts, arg0)
}

// StateHistoryCommitments is a free data retrieval call binding the contract method 0x02b592f3.
//
// Solidity: function stateHistoryCommitments(uint256 ) view returns(uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, uint256 hotShotBlockCommRoot)
func (_Lightclient *LightclientCallerSession) StateHistoryCommitments(arg0 *big.Int) (struct {
	L1BlockHeight        uint64
	L1BlockTimestamp     uint64
	HotShotBlockHeight   uint64
	HotShotBlockCommRoot *big.Int
}, error) {
	return _Lightclient.Contract.StateHistoryCommitments(&_Lightclient.CallOpts, arg0)
}

// StateHistoryFirstIndex is a free data retrieval call binding the contract method 0x2f79889d.
//
// Solidity: function stateHistoryFirstIndex() view returns(uint64)
func (_Lightclient *LightclientCaller) StateHistoryFirstIndex(opts *bind.CallOpts) (uint64, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "stateHistoryFirstIndex")

	if err != nil {
		return *new(uint64), err
	}

	out0 := *abi.ConvertType(out[0], new(uint64)).(*uint64)

	return out0, err

}

// StateHistoryFirstIndex is a free data retrieval call binding the contract method 0x2f79889d.
//
// Solidity: function stateHistoryFirstIndex() view returns(uint64)
func (_Lightclient *LightclientSession) StateHistoryFirstIndex() (uint64, error) {
	return _Lightclient.Contract.StateHistoryFirstIndex(&_Lightclient.CallOpts)
}

// StateHistoryFirstIndex is a free data retrieval call binding the contract method 0x2f79889d.
//
// Solidity: function stateHistoryFirstIndex() view returns(uint64)
func (_Lightclient *LightclientCallerSession) StateHistoryFirstIndex() (uint64, error) {
	return _Lightclient.Contract.StateHistoryFirstIndex(&_Lightclient.CallOpts)
}

// StateHistoryRetentionPeriod is a free data retrieval call binding the contract method 0xc23b9e9e.
//
// Solidity: function stateHistoryRetentionPeriod() view returns(uint32)
func (_Lightclient *LightclientCaller) StateHistoryRetentionPeriod(opts *bind.CallOpts) (uint32, error) {
	var out []interface{}
	err := _Lightclient.contract.Call(opts, &out, "stateHistoryRetentionPeriod")

	if err != nil {
		return *new(uint32), err
	}

	out0 := *abi.ConvertType(out[0], new(uint32)).(*uint32)

	return out0, err

}

// StateHistoryRetentionPeriod is a free data retrieval call binding the contract method 0xc23b9e9e.
//
// Solidity: function stateHistoryRetentionPeriod() view returns(uint32)
func (_Lightclient *LightclientSession) StateHistoryRetentionPeriod() (uint32, error) {
	return _Lightclient.Contract.StateHistoryRetentionPeriod(&_Lightclient.CallOpts)
}

// StateHistoryRetentionPeriod is a free data retrieval call binding the contract method 0xc23b9e9e.
//
// Solidity: function stateHistoryRetentionPeriod() view returns(uint32)
func (_Lightclient *LightclientCallerSession) StateHistoryRetentionPeriod() (uint32, error) {
	return _Lightclient.Contract.StateHistoryRetentionPeriod(&_Lightclient.CallOpts)
}

// DisablePermissionedProverMode is a paid mutator transaction binding the contract method 0x69cc6a04.
//
// Solidity: function disablePermissionedProverMode() returns()
func (_Lightclient *LightclientTransactor) DisablePermissionedProverMode(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "disablePermissionedProverMode")
}

// DisablePermissionedProverMode is a paid mutator transaction binding the contract method 0x69cc6a04.
//
// Solidity: function disablePermissionedProverMode() returns()
func (_Lightclient *LightclientSession) DisablePermissionedProverMode() (*types.Transaction, error) {
	return _Lightclient.Contract.DisablePermissionedProverMode(&_Lightclient.TransactOpts)
}

// DisablePermissionedProverMode is a paid mutator transaction binding the contract method 0x69cc6a04.
//
// Solidity: function disablePermissionedProverMode() returns()
func (_Lightclient *LightclientTransactorSession) DisablePermissionedProverMode() (*types.Transaction, error) {
	return _Lightclient.Contract.DisablePermissionedProverMode(&_Lightclient.TransactOpts)
}

// Initialize is a paid mutator transaction binding the contract method 0x9baa3cc9.
//
// Solidity: function initialize((uint64,uint64,uint256) _genesis, (uint256,uint256,uint256,uint256) _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) returns()
func (_Lightclient *LightclientTransactor) Initialize(opts *bind.TransactOpts, _genesis LightClientLightClientState, _genesisStakeTableState LightClientStakeTableState, _stateHistoryRetentionPeriod uint32, owner common.Address) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "initialize", _genesis, _genesisStakeTableState, _stateHistoryRetentionPeriod, owner)
}

// Initialize is a paid mutator transaction binding the contract method 0x9baa3cc9.
//
// Solidity: function initialize((uint64,uint64,uint256) _genesis, (uint256,uint256,uint256,uint256) _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) returns()
func (_Lightclient *LightclientSession) Initialize(_genesis LightClientLightClientState, _genesisStakeTableState LightClientStakeTableState, _stateHistoryRetentionPeriod uint32, owner common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.Initialize(&_Lightclient.TransactOpts, _genesis, _genesisStakeTableState, _stateHistoryRetentionPeriod, owner)
}

// Initialize is a paid mutator transaction binding the contract method 0x9baa3cc9.
//
// Solidity: function initialize((uint64,uint64,uint256) _genesis, (uint256,uint256,uint256,uint256) _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) returns()
func (_Lightclient *LightclientTransactorSession) Initialize(_genesis LightClientLightClientState, _genesisStakeTableState LightClientStakeTableState, _stateHistoryRetentionPeriod uint32, owner common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.Initialize(&_Lightclient.TransactOpts, _genesis, _genesisStakeTableState, _stateHistoryRetentionPeriod, owner)
}

// NewFinalizedState is a paid mutator transaction binding the contract method 0x2063d4f7.
//
// Solidity: function newFinalizedState((uint64,uint64,uint256) newState, ((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256) proof) returns()
func (_Lightclient *LightclientTransactor) NewFinalizedState(opts *bind.TransactOpts, newState LightClientLightClientState, proof IPlonkVerifierPlonkProof) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "newFinalizedState", newState, proof)
}

// NewFinalizedState is a paid mutator transaction binding the contract method 0x2063d4f7.
//
// Solidity: function newFinalizedState((uint64,uint64,uint256) newState, ((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256) proof) returns()
func (_Lightclient *LightclientSession) NewFinalizedState(newState LightClientLightClientState, proof IPlonkVerifierPlonkProof) (*types.Transaction, error) {
	return _Lightclient.Contract.NewFinalizedState(&_Lightclient.TransactOpts, newState, proof)
}

// NewFinalizedState is a paid mutator transaction binding the contract method 0x2063d4f7.
//
// Solidity: function newFinalizedState((uint64,uint64,uint256) newState, ((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256) proof) returns()
func (_Lightclient *LightclientTransactorSession) NewFinalizedState(newState LightClientLightClientState, proof IPlonkVerifierPlonkProof) (*types.Transaction, error) {
	return _Lightclient.Contract.NewFinalizedState(&_Lightclient.TransactOpts, newState, proof)
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_Lightclient *LightclientTransactor) RenounceOwnership(opts *bind.TransactOpts) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "renounceOwnership")
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_Lightclient *LightclientSession) RenounceOwnership() (*types.Transaction, error) {
	return _Lightclient.Contract.RenounceOwnership(&_Lightclient.TransactOpts)
}

// RenounceOwnership is a paid mutator transaction binding the contract method 0x715018a6.
//
// Solidity: function renounceOwnership() returns()
func (_Lightclient *LightclientTransactorSession) RenounceOwnership() (*types.Transaction, error) {
	return _Lightclient.Contract.RenounceOwnership(&_Lightclient.TransactOpts)
}

// SetPermissionedProver is a paid mutator transaction binding the contract method 0x013fa5fc.
//
// Solidity: function setPermissionedProver(address prover) returns()
func (_Lightclient *LightclientTransactor) SetPermissionedProver(opts *bind.TransactOpts, prover common.Address) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "setPermissionedProver", prover)
}

// SetPermissionedProver is a paid mutator transaction binding the contract method 0x013fa5fc.
//
// Solidity: function setPermissionedProver(address prover) returns()
func (_Lightclient *LightclientSession) SetPermissionedProver(prover common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.SetPermissionedProver(&_Lightclient.TransactOpts, prover)
}

// SetPermissionedProver is a paid mutator transaction binding the contract method 0x013fa5fc.
//
// Solidity: function setPermissionedProver(address prover) returns()
func (_Lightclient *LightclientTransactorSession) SetPermissionedProver(prover common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.SetPermissionedProver(&_Lightclient.TransactOpts, prover)
}

// SetstateHistoryRetentionPeriod is a paid mutator transaction binding the contract method 0x96c1ca61.
//
// Solidity: function setstateHistoryRetentionPeriod(uint32 historySeconds) returns()
func (_Lightclient *LightclientTransactor) SetstateHistoryRetentionPeriod(opts *bind.TransactOpts, historySeconds uint32) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "setstateHistoryRetentionPeriod", historySeconds)
}

// SetstateHistoryRetentionPeriod is a paid mutator transaction binding the contract method 0x96c1ca61.
//
// Solidity: function setstateHistoryRetentionPeriod(uint32 historySeconds) returns()
func (_Lightclient *LightclientSession) SetstateHistoryRetentionPeriod(historySeconds uint32) (*types.Transaction, error) {
	return _Lightclient.Contract.SetstateHistoryRetentionPeriod(&_Lightclient.TransactOpts, historySeconds)
}

// SetstateHistoryRetentionPeriod is a paid mutator transaction binding the contract method 0x96c1ca61.
//
// Solidity: function setstateHistoryRetentionPeriod(uint32 historySeconds) returns()
func (_Lightclient *LightclientTransactorSession) SetstateHistoryRetentionPeriod(historySeconds uint32) (*types.Transaction, error) {
	return _Lightclient.Contract.SetstateHistoryRetentionPeriod(&_Lightclient.TransactOpts, historySeconds)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_Lightclient *LightclientTransactor) TransferOwnership(opts *bind.TransactOpts, newOwner common.Address) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "transferOwnership", newOwner)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_Lightclient *LightclientSession) TransferOwnership(newOwner common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.TransferOwnership(&_Lightclient.TransactOpts, newOwner)
}

// TransferOwnership is a paid mutator transaction binding the contract method 0xf2fde38b.
//
// Solidity: function transferOwnership(address newOwner) returns()
func (_Lightclient *LightclientTransactorSession) TransferOwnership(newOwner common.Address) (*types.Transaction, error) {
	return _Lightclient.Contract.TransferOwnership(&_Lightclient.TransactOpts, newOwner)
}

// UpgradeToAndCall is a paid mutator transaction binding the contract method 0x4f1ef286.
//
// Solidity: function upgradeToAndCall(address newImplementation, bytes data) payable returns()
func (_Lightclient *LightclientTransactor) UpgradeToAndCall(opts *bind.TransactOpts, newImplementation common.Address, data []byte) (*types.Transaction, error) {
	return _Lightclient.contract.Transact(opts, "upgradeToAndCall", newImplementation, data)
}

// UpgradeToAndCall is a paid mutator transaction binding the contract method 0x4f1ef286.
//
// Solidity: function upgradeToAndCall(address newImplementation, bytes data) payable returns()
func (_Lightclient *LightclientSession) UpgradeToAndCall(newImplementation common.Address, data []byte) (*types.Transaction, error) {
	return _Lightclient.Contract.UpgradeToAndCall(&_Lightclient.TransactOpts, newImplementation, data)
}

// UpgradeToAndCall is a paid mutator transaction binding the contract method 0x4f1ef286.
//
// Solidity: function upgradeToAndCall(address newImplementation, bytes data) payable returns()
func (_Lightclient *LightclientTransactorSession) UpgradeToAndCall(newImplementation common.Address, data []byte) (*types.Transaction, error) {
	return _Lightclient.Contract.UpgradeToAndCall(&_Lightclient.TransactOpts, newImplementation, data)
}

// LightclientInitializedIterator is returned from FilterInitialized and is used to iterate over the raw logs and unpacked data for Initialized events raised by the Lightclient contract.
type LightclientInitializedIterator struct {
	Event *LightclientInitialized // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientInitializedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientInitialized)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientInitialized)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientInitializedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientInitializedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientInitialized represents a Initialized event raised by the Lightclient contract.
type LightclientInitialized struct {
	Version uint64
	Raw     types.Log // Blockchain specific contextual infos
}

// FilterInitialized is a free log retrieval operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_Lightclient *LightclientFilterer) FilterInitialized(opts *bind.FilterOpts) (*LightclientInitializedIterator, error) {

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "Initialized")
	if err != nil {
		return nil, err
	}
	return &LightclientInitializedIterator{contract: _Lightclient.contract, event: "Initialized", logs: logs, sub: sub}, nil
}

// WatchInitialized is a free log subscription operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_Lightclient *LightclientFilterer) WatchInitialized(opts *bind.WatchOpts, sink chan<- *LightclientInitialized) (event.Subscription, error) {

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "Initialized")
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientInitialized)
				if err := _Lightclient.contract.UnpackLog(event, "Initialized", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseInitialized is a log parse operation binding the contract event 0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2.
//
// Solidity: event Initialized(uint64 version)
func (_Lightclient *LightclientFilterer) ParseInitialized(log types.Log) (*LightclientInitialized, error) {
	event := new(LightclientInitialized)
	if err := _Lightclient.contract.UnpackLog(event, "Initialized", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientNewStateIterator is returned from FilterNewState and is used to iterate over the raw logs and unpacked data for NewState events raised by the Lightclient contract.
type LightclientNewStateIterator struct {
	Event *LightclientNewState // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientNewStateIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientNewState)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientNewState)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientNewStateIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientNewStateIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientNewState represents a NewState event raised by the Lightclient contract.
type LightclientNewState struct {
	ViewNum       uint64
	BlockHeight   uint64
	BlockCommRoot *big.Int
	Raw           types.Log // Blockchain specific contextual infos
}

// FilterNewState is a free log retrieval operation binding the contract event 0xa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae6.
//
// Solidity: event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientFilterer) FilterNewState(opts *bind.FilterOpts, viewNum []uint64, blockHeight []uint64) (*LightclientNewStateIterator, error) {

	var viewNumRule []interface{}
	for _, viewNumItem := range viewNum {
		viewNumRule = append(viewNumRule, viewNumItem)
	}
	var blockHeightRule []interface{}
	for _, blockHeightItem := range blockHeight {
		blockHeightRule = append(blockHeightRule, blockHeightItem)
	}

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "NewState", viewNumRule, blockHeightRule)
	if err != nil {
		return nil, err
	}
	return &LightclientNewStateIterator{contract: _Lightclient.contract, event: "NewState", logs: logs, sub: sub}, nil
}

// WatchNewState is a free log subscription operation binding the contract event 0xa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae6.
//
// Solidity: event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientFilterer) WatchNewState(opts *bind.WatchOpts, sink chan<- *LightclientNewState, viewNum []uint64, blockHeight []uint64) (event.Subscription, error) {

	var viewNumRule []interface{}
	for _, viewNumItem := range viewNum {
		viewNumRule = append(viewNumRule, viewNumItem)
	}
	var blockHeightRule []interface{}
	for _, blockHeightItem := range blockHeight {
		blockHeightRule = append(blockHeightRule, blockHeightItem)
	}

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "NewState", viewNumRule, blockHeightRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientNewState)
				if err := _Lightclient.contract.UnpackLog(event, "NewState", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseNewState is a log parse operation binding the contract event 0xa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae6.
//
// Solidity: event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, uint256 blockCommRoot)
func (_Lightclient *LightclientFilterer) ParseNewState(log types.Log) (*LightclientNewState, error) {
	event := new(LightclientNewState)
	if err := _Lightclient.contract.UnpackLog(event, "NewState", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientOwnershipTransferredIterator is returned from FilterOwnershipTransferred and is used to iterate over the raw logs and unpacked data for OwnershipTransferred events raised by the Lightclient contract.
type LightclientOwnershipTransferredIterator struct {
	Event *LightclientOwnershipTransferred // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientOwnershipTransferredIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientOwnershipTransferred)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientOwnershipTransferred)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientOwnershipTransferredIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientOwnershipTransferredIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientOwnershipTransferred represents a OwnershipTransferred event raised by the Lightclient contract.
type LightclientOwnershipTransferred struct {
	PreviousOwner common.Address
	NewOwner      common.Address
	Raw           types.Log // Blockchain specific contextual infos
}

// FilterOwnershipTransferred is a free log retrieval operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_Lightclient *LightclientFilterer) FilterOwnershipTransferred(opts *bind.FilterOpts, previousOwner []common.Address, newOwner []common.Address) (*LightclientOwnershipTransferredIterator, error) {

	var previousOwnerRule []interface{}
	for _, previousOwnerItem := range previousOwner {
		previousOwnerRule = append(previousOwnerRule, previousOwnerItem)
	}
	var newOwnerRule []interface{}
	for _, newOwnerItem := range newOwner {
		newOwnerRule = append(newOwnerRule, newOwnerItem)
	}

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "OwnershipTransferred", previousOwnerRule, newOwnerRule)
	if err != nil {
		return nil, err
	}
	return &LightclientOwnershipTransferredIterator{contract: _Lightclient.contract, event: "OwnershipTransferred", logs: logs, sub: sub}, nil
}

// WatchOwnershipTransferred is a free log subscription operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_Lightclient *LightclientFilterer) WatchOwnershipTransferred(opts *bind.WatchOpts, sink chan<- *LightclientOwnershipTransferred, previousOwner []common.Address, newOwner []common.Address) (event.Subscription, error) {

	var previousOwnerRule []interface{}
	for _, previousOwnerItem := range previousOwner {
		previousOwnerRule = append(previousOwnerRule, previousOwnerItem)
	}
	var newOwnerRule []interface{}
	for _, newOwnerItem := range newOwner {
		newOwnerRule = append(newOwnerRule, newOwnerItem)
	}

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "OwnershipTransferred", previousOwnerRule, newOwnerRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientOwnershipTransferred)
				if err := _Lightclient.contract.UnpackLog(event, "OwnershipTransferred", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseOwnershipTransferred is a log parse operation binding the contract event 0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0.
//
// Solidity: event OwnershipTransferred(address indexed previousOwner, address indexed newOwner)
func (_Lightclient *LightclientFilterer) ParseOwnershipTransferred(log types.Log) (*LightclientOwnershipTransferred, error) {
	event := new(LightclientOwnershipTransferred)
	if err := _Lightclient.contract.UnpackLog(event, "OwnershipTransferred", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientPermissionedProverNotRequiredIterator is returned from FilterPermissionedProverNotRequired and is used to iterate over the raw logs and unpacked data for PermissionedProverNotRequired events raised by the Lightclient contract.
type LightclientPermissionedProverNotRequiredIterator struct {
	Event *LightclientPermissionedProverNotRequired // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientPermissionedProverNotRequiredIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientPermissionedProverNotRequired)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientPermissionedProverNotRequired)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientPermissionedProverNotRequiredIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientPermissionedProverNotRequiredIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientPermissionedProverNotRequired represents a PermissionedProverNotRequired event raised by the Lightclient contract.
type LightclientPermissionedProverNotRequired struct {
	Raw types.Log // Blockchain specific contextual infos
}

// FilterPermissionedProverNotRequired is a free log retrieval operation binding the contract event 0x9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450.
//
// Solidity: event PermissionedProverNotRequired()
func (_Lightclient *LightclientFilterer) FilterPermissionedProverNotRequired(opts *bind.FilterOpts) (*LightclientPermissionedProverNotRequiredIterator, error) {

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "PermissionedProverNotRequired")
	if err != nil {
		return nil, err
	}
	return &LightclientPermissionedProverNotRequiredIterator{contract: _Lightclient.contract, event: "PermissionedProverNotRequired", logs: logs, sub: sub}, nil
}

// WatchPermissionedProverNotRequired is a free log subscription operation binding the contract event 0x9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450.
//
// Solidity: event PermissionedProverNotRequired()
func (_Lightclient *LightclientFilterer) WatchPermissionedProverNotRequired(opts *bind.WatchOpts, sink chan<- *LightclientPermissionedProverNotRequired) (event.Subscription, error) {

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "PermissionedProverNotRequired")
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientPermissionedProverNotRequired)
				if err := _Lightclient.contract.UnpackLog(event, "PermissionedProverNotRequired", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParsePermissionedProverNotRequired is a log parse operation binding the contract event 0x9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450.
//
// Solidity: event PermissionedProverNotRequired()
func (_Lightclient *LightclientFilterer) ParsePermissionedProverNotRequired(log types.Log) (*LightclientPermissionedProverNotRequired, error) {
	event := new(LightclientPermissionedProverNotRequired)
	if err := _Lightclient.contract.UnpackLog(event, "PermissionedProverNotRequired", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientPermissionedProverRequiredIterator is returned from FilterPermissionedProverRequired and is used to iterate over the raw logs and unpacked data for PermissionedProverRequired events raised by the Lightclient contract.
type LightclientPermissionedProverRequiredIterator struct {
	Event *LightclientPermissionedProverRequired // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientPermissionedProverRequiredIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientPermissionedProverRequired)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientPermissionedProverRequired)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientPermissionedProverRequiredIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientPermissionedProverRequiredIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientPermissionedProverRequired represents a PermissionedProverRequired event raised by the Lightclient contract.
type LightclientPermissionedProverRequired struct {
	PermissionedProver common.Address
	Raw                types.Log // Blockchain specific contextual infos
}

// FilterPermissionedProverRequired is a free log retrieval operation binding the contract event 0x8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072.
//
// Solidity: event PermissionedProverRequired(address permissionedProver)
func (_Lightclient *LightclientFilterer) FilterPermissionedProverRequired(opts *bind.FilterOpts) (*LightclientPermissionedProverRequiredIterator, error) {

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "PermissionedProverRequired")
	if err != nil {
		return nil, err
	}
	return &LightclientPermissionedProverRequiredIterator{contract: _Lightclient.contract, event: "PermissionedProverRequired", logs: logs, sub: sub}, nil
}

// WatchPermissionedProverRequired is a free log subscription operation binding the contract event 0x8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072.
//
// Solidity: event PermissionedProverRequired(address permissionedProver)
func (_Lightclient *LightclientFilterer) WatchPermissionedProverRequired(opts *bind.WatchOpts, sink chan<- *LightclientPermissionedProverRequired) (event.Subscription, error) {

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "PermissionedProverRequired")
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientPermissionedProverRequired)
				if err := _Lightclient.contract.UnpackLog(event, "PermissionedProverRequired", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParsePermissionedProverRequired is a log parse operation binding the contract event 0x8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072.
//
// Solidity: event PermissionedProverRequired(address permissionedProver)
func (_Lightclient *LightclientFilterer) ParsePermissionedProverRequired(log types.Log) (*LightclientPermissionedProverRequired, error) {
	event := new(LightclientPermissionedProverRequired)
	if err := _Lightclient.contract.UnpackLog(event, "PermissionedProverRequired", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientUpgradeIterator is returned from FilterUpgrade and is used to iterate over the raw logs and unpacked data for Upgrade events raised by the Lightclient contract.
type LightclientUpgradeIterator struct {
	Event *LightclientUpgrade // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientUpgradeIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientUpgrade)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientUpgrade)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientUpgradeIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientUpgradeIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientUpgrade represents a Upgrade event raised by the Lightclient contract.
type LightclientUpgrade struct {
	Implementation common.Address
	Raw            types.Log // Blockchain specific contextual infos
}

// FilterUpgrade is a free log retrieval operation binding the contract event 0xf78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d.
//
// Solidity: event Upgrade(address implementation)
func (_Lightclient *LightclientFilterer) FilterUpgrade(opts *bind.FilterOpts) (*LightclientUpgradeIterator, error) {

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "Upgrade")
	if err != nil {
		return nil, err
	}
	return &LightclientUpgradeIterator{contract: _Lightclient.contract, event: "Upgrade", logs: logs, sub: sub}, nil
}

// WatchUpgrade is a free log subscription operation binding the contract event 0xf78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d.
//
// Solidity: event Upgrade(address implementation)
func (_Lightclient *LightclientFilterer) WatchUpgrade(opts *bind.WatchOpts, sink chan<- *LightclientUpgrade) (event.Subscription, error) {

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "Upgrade")
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientUpgrade)
				if err := _Lightclient.contract.UnpackLog(event, "Upgrade", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseUpgrade is a log parse operation binding the contract event 0xf78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d.
//
// Solidity: event Upgrade(address implementation)
func (_Lightclient *LightclientFilterer) ParseUpgrade(log types.Log) (*LightclientUpgrade, error) {
	event := new(LightclientUpgrade)
	if err := _Lightclient.contract.UnpackLog(event, "Upgrade", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}

// LightclientUpgradedIterator is returned from FilterUpgraded and is used to iterate over the raw logs and unpacked data for Upgraded events raised by the Lightclient contract.
type LightclientUpgradedIterator struct {
	Event *LightclientUpgraded // Event containing the contract specifics and raw log

	contract *bind.BoundContract // Generic contract to use for unpacking event data
	event    string              // Event name to use for unpacking event data

	logs chan types.Log        // Log channel receiving the found contract events
	sub  ethereum.Subscription // Subscription for errors, completion and termination
	done bool                  // Whether the subscription completed delivering logs
	fail error                 // Occurred error to stop iteration
}

// Next advances the iterator to the subsequent event, returning whether there
// are any more events found. In case of a retrieval or parsing error, false is
// returned and Error() can be queried for the exact failure.
func (it *LightclientUpgradedIterator) Next() bool {
	// If the iterator failed, stop iterating
	if it.fail != nil {
		return false
	}
	// If the iterator completed, deliver directly whatever's available
	if it.done {
		select {
		case log := <-it.logs:
			it.Event = new(LightclientUpgraded)
			if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
				it.fail = err
				return false
			}
			it.Event.Raw = log
			return true

		default:
			return false
		}
	}
	// Iterator still in progress, wait for either a data or an error event
	select {
	case log := <-it.logs:
		it.Event = new(LightclientUpgraded)
		if err := it.contract.UnpackLog(it.Event, it.event, log); err != nil {
			it.fail = err
			return false
		}
		it.Event.Raw = log
		return true

	case err := <-it.sub.Err():
		it.done = true
		it.fail = err
		return it.Next()
	}
}

// Error returns any retrieval or parsing error occurred during filtering.
func (it *LightclientUpgradedIterator) Error() error {
	return it.fail
}

// Close terminates the iteration process, releasing any pending underlying
// resources.
func (it *LightclientUpgradedIterator) Close() error {
	it.sub.Unsubscribe()
	return nil
}

// LightclientUpgraded represents a Upgraded event raised by the Lightclient contract.
type LightclientUpgraded struct {
	Implementation common.Address
	Raw            types.Log // Blockchain specific contextual infos
}

// FilterUpgraded is a free log retrieval operation binding the contract event 0xbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b.
//
// Solidity: event Upgraded(address indexed implementation)
func (_Lightclient *LightclientFilterer) FilterUpgraded(opts *bind.FilterOpts, implementation []common.Address) (*LightclientUpgradedIterator, error) {

	var implementationRule []interface{}
	for _, implementationItem := range implementation {
		implementationRule = append(implementationRule, implementationItem)
	}

	logs, sub, err := _Lightclient.contract.FilterLogs(opts, "Upgraded", implementationRule)
	if err != nil {
		return nil, err
	}
	return &LightclientUpgradedIterator{contract: _Lightclient.contract, event: "Upgraded", logs: logs, sub: sub}, nil
}

// WatchUpgraded is a free log subscription operation binding the contract event 0xbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b.
//
// Solidity: event Upgraded(address indexed implementation)
func (_Lightclient *LightclientFilterer) WatchUpgraded(opts *bind.WatchOpts, sink chan<- *LightclientUpgraded, implementation []common.Address) (event.Subscription, error) {

	var implementationRule []interface{}
	for _, implementationItem := range implementation {
		implementationRule = append(implementationRule, implementationItem)
	}

	logs, sub, err := _Lightclient.contract.WatchLogs(opts, "Upgraded", implementationRule)
	if err != nil {
		return nil, err
	}
	return event.NewSubscription(func(quit <-chan struct{}) error {
		defer sub.Unsubscribe()
		for {
			select {
			case log := <-logs:
				// New log arrived, parse the event and forward to the user
				event := new(LightclientUpgraded)
				if err := _Lightclient.contract.UnpackLog(event, "Upgraded", log); err != nil {
					return err
				}
				event.Raw = log

				select {
				case sink <- event:
				case err := <-sub.Err():
					return err
				case <-quit:
					return nil
				}
			case err := <-sub.Err():
				return err
			case <-quit:
				return nil
			}
		}
	}), nil
}

// ParseUpgraded is a log parse operation binding the contract event 0xbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b.
//
// Solidity: event Upgraded(address indexed implementation)
func (_Lightclient *LightclientFilterer) ParseUpgraded(log types.Log) (*LightclientUpgraded, error) {
	event := new(LightclientUpgraded)
	if err := _Lightclient.contract.UnpackLog(event, "Upgraded", log); err != nil {
		return nil, err
	}
	event.Raw = log
	return event, nil
}
