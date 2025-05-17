// This file contains functions that help people interact with light client mock contract.
package lightclientmock

import (
	"context"
	"math/big"
	"testing"

	"github.com/ethereum/go-ethereum/accounts/abi/bind"
	"github.com/ethereum/go-ethereum/common"
)

func FreezeL1Height(t *testing.T, client bind.ContractBackend, contractAddress common.Address, txOpts *bind.TransactOpts) error {
	mockLightClient, err := NewLightclientmock(contractAddress, client)
	if err != nil {
		return err
	}

	header, err := client.HeaderByNumber(context.Background(), nil)
	if err != nil {
		return err
	}

	_, err = mockLightClient.SetHotShotDownSince(txOpts, header.Number)
	if err != nil {
		return err
	}

	return nil
}

func UnfreezeL1Height(t *testing.T, client bind.ContractBackend, contractAddress common.Address, txOpts *bind.TransactOpts) error {
	mockLightClient, err := NewLightclientmock(contractAddress, client)
	if err != nil {
		return err
	}
	_, err = mockLightClient.SetHotShotUp(txOpts)

	return err
}

func IsHotShotLive(t *testing.T, client bind.ContractBackend, contractAddress common.Address, threshold uint64) (bool, error) {
	mockLightClient, err := NewLightclientmock(contractAddress, client)
	if err != nil {
		return false, err
	}

	header, err := client.HeaderByNumber(context.Background(), nil)
	if err != nil {
		return false, err
	}

	isDown, err := mockLightClient.LagOverEscapeHatchThreshold(nil, header.Number, new(big.Int).SetUint64(threshold))
	if err != nil {
		return false, err
	}

	return !isDown, nil
}
