package verification

import (
	"encoding/json"
	"io"
	"os"
	"testing"

	"github.com/EspressoSystems/espresso-network/sdks/go/types"
)

func TestVerifyNamespaceWithRealData(t *testing.T) {
	bytes, err := readResponse("./resp/transaction_in_block.json")
	if err != nil {
		t.Fatalf("Failed to read file: %v", err)
	}
	var res TransactionInBlock
	if err := json.Unmarshal(bytes, &res); err != nil {
		t.Fatalf("Failed to unmarshal: %v", err)
	}

	var txes []types.Bytes
	for _, tx := range res.Transactions {
		txes = append(txes, tx.Payload)
	}

	vidCommonBytes, err := readResponse("./resp/vid_common.json")
	if err != nil {
		t.Fatalf("Failed to read file: %v", err)
	}
	var vidCommon types.VidCommonQueryData
	if err := json.Unmarshal(vidCommonBytes, &vidCommon); err != nil {
		t.Fatalf("Failed to unmarshal: %v", err)
	}

	headerBytes, err := readResponse("./resp/header.json")
	if err != nil {
		t.Fatalf("Failed to read file: %v", err)
	}
	var header types.HeaderImpl
	if err := json.Unmarshal(headerBytes, &header); err != nil {
		t.Fatalf("Failed to unmarshal: %v", err)
	}

	success, err := VerifyNamespace(
		1918988905,
		res.Proof,
		*header.Header.GetPayloadCommitment(),
		*header.Header.GetNsTable(),
		txes,
		json.RawMessage(vidCommon.Common),
	)
	if !success {
		t.Fatalf("Failed to verify namespace: %v", err)
	}
}

func readResponse(path string) (json.RawMessage, error) {
	file, err := os.Open(path)
	if err != nil {
		return nil, err
	}
	defer file.Close()

	bytes, err := io.ReadAll(file)
	if err != nil {
		return nil, err
	}
	return bytes, nil
}

type TransactionInBlock struct {
	Proof        json.RawMessage     `json:"proof"`
	Transactions []types.Transaction `json:"transactions"`
}
