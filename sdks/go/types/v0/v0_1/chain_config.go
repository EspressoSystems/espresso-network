package v0_1

import (
	"encoding/json"
	"fmt"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"
	common_types "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	"github.com/ethereum/go-ethereum/common"
)

type ChainConfig struct {
	ChainId      common_types.U256Decimal `json:"chain_id"`
	MaxBlockSize common_types.U256Decimal `json:"max_block_size"`
	BaseFee      common_types.U256Decimal `json:"base_fee"`
	FeeContract  *common.Address          `json:"fee_contract"`
	FeeRecipient common.Address           `json:"fee_recipient"`
}

func (self *ChainConfig) Commit() common_types.Commitment {
	builder := common_types.NewRawCommitmentBuilder("CHAIN_CONFIG").
		Uint256Field("chain_id", self.ChainId.ToU256()).
		Uint64Field("max_block_size", self.MaxBlockSize.Uint64()).
		Uint256Field("base_fee", self.BaseFee.ToU256()).
		FixedSizeField("fee_recipient", self.FeeRecipient.Bytes())
	if self.FeeContract == nil {
		return builder.Finalize()
	}
	return builder.Uint64Field("fee_contract", 1).FixedSizeBytes(self.FeeContract.Bytes()).Finalize()
}

type TaggedBase64 = tagged_base64.TaggedBase64

type ResolvableChainConfig struct {
	ChainConfig EitherChainConfig `json:"chain_config"`
}

func (self *ResolvableChainConfig) Commit() common_types.Commitment {
	config := self.ChainConfig
	if config.Left != nil {
		return config.Left.Commit()
	}
	if config.Right != nil {
		right := *config.Right
		r := (*right).Value()
		bytes := [32]byte{}
		copy(bytes[:], r)
		return common_types.Commitment(bytes)
	}

	// It shouldn't happen
	return common_types.Commitment{}
}

type EitherChainConfig struct {
	Left  *ChainConfig   `json:"Left"`
	Right **TaggedBase64 `json:"Right"`
}

func (i *EitherChainConfig) UnmarshalJSON(b []byte) error {
	type Dec struct {
		Left  *ChainConfig   `json:"Left"`
		Right **TaggedBase64 `json:"Right"`
	}
	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}
	if dec.Left != nil {
		i.Left = dec.Left
	}

	if dec.Right != nil {
		i.Right = dec.Right
	}

	if i.Left == nil && i.Right == nil {
		return fmt.Errorf("either Left or Right variant for EitherChainConfig is required")
	}

	return nil
}

func (i *EitherChainConfig) MarshalJSON() ([]byte, error) {
	type Left struct {
		Left *ChainConfig `json:"Left"`
	}
	type Right struct {
		Right **TaggedBase64 `json:"Right"`
	}

	if i.Left != nil {
		return json.Marshal(Left{Left: i.Left})
	}

	return json.Marshal(Right{Right: i.Right})
}
