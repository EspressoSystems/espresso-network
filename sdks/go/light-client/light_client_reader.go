package lightclient

import (
	"context"
	"math/big"

	common_types "github.com/EspressoSystems/espresso-network-go/types/common"
	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
)

var _ LightClientReaderInterface = (*LightClientReader)(nil)

type LightClientReaderInterface interface {
	ValidatedHeight() (validatedHeight uint64, l1Height uint64, err error)
	FetchMerkleRoot(hotShotHeight uint64, opts *bind.CallOpts) (common_types.BlockMerkleSnapshot, error)

	IsHotShotLive(delayThreshold uint64) (bool, error)
	IsHotShotLiveAtHeight(height, delayThreshold uint64) (bool, error)
}

type LightClientReader struct {
	LightClient Lightclient
	L1Client    bind.ContractBackend
}

func NewLightClientReader(lightClientAddr common.Address, l1client bind.ContractBackend) (*LightClientReader, error) {
	lightclient, err := NewLightclient(lightClientAddr, l1client)
	if err != nil {
		return nil, err
	}

	return &LightClientReader{
		LightClient: *lightclient,
		L1Client:    l1client,
	}, nil

}

// Returns the L1 block number where the light client has validated a particular
// hotshot block number
func (l *LightClientReader) ValidatedHeight() (validatedHeight uint64, l1Height uint64, err error) {
	header, err := l.L1Client.HeaderByNumber(context.Background(), nil)
	if err != nil {
		return 0, 0, err
	}
	state, err := l.LightClient.FinalizedState(&bind.CallOpts{BlockNumber: header.Number})
	if err != nil {
		return 0, 0, err
	}
	return state.BlockHeight, header.Number.Uint64(), nil
}

// Fetch the merkle root at the first light client snapshot that proves the provided hotshot leaf height.
// CallOpt included as a parameter in case validators need to fetch historical merkle roots if they are catching up.
func (l *LightClientReader) FetchMerkleRoot(hotShotHeight uint64, opts *bind.CallOpts) (common_types.BlockMerkleSnapshot, error) {
	snapshot, err := l.LightClient.GetHotShotCommitment(opts, new(big.Int).SetUint64(hotShotHeight))
	if err != nil {
		return common_types.BlockMerkleSnapshot{}, err
	}
	root, err := common_types.CommitmentFromUint256(common_types.NewU256().SetBigInt(snapshot.HotShotBlockCommRoot))
	if err != nil {
		return common_types.BlockMerkleSnapshot{}, err
	}
	return common_types.BlockMerkleSnapshot{
		Root:   root,
		Height: snapshot.HotshotBlockHeight,
	}, nil
}

func (l *LightClientReader) IsHotShotLive(delayThreshold uint64) (bool, error) {
	header, err := l.L1Client.HeaderByNumber(context.Background(), nil)
	if err != nil {
		return false, err
	}
	threshold := new(big.Int).SetUint64(delayThreshold)
	isDown, err := l.LightClient.LagOverEscapeHatchThreshold(&bind.CallOpts{}, header.Number, threshold)
	if err != nil {
		return false, err
	}
	return !isDown, nil
}

func (l *LightClientReader) IsHotShotLiveAtHeight(h, delayThreshold uint64) (bool, error) {
	threshold := new(big.Int).SetUint64(delayThreshold)
	height := new(big.Int).SetUint64(h)
	isDown, err := l.LightClient.LagOverEscapeHatchThreshold(&bind.CallOpts{}, height, threshold)
	if err != nil {
		return false, err
	}
	return !isDown, nil
}
