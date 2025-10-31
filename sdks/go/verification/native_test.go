package verification

import (
	"encoding/json"
	"io"
	"log"
	"os"
	"strings"
	"testing"

	"github.com/EspressoSystems/espresso-network/sdks/go/types/common"
)

type merkleProofTestData struct {
	Proof             json.RawMessage `json:"proof"`
	Header            json.RawMessage `json:"header"`
	BlockMerkleRoot   string          `json:"block_merkle_root"`
	HotShotCommitment []uint8         `json:"hotshot_commitment"`
}

func TestMerkleProofVerification(t *testing.T) {
	file, err := os.Open("./merkle_proof_test_data.json")
	if err != nil {
		log.Fatalf("Failed to open file: %v", err)
	}
	defer file.Close()

	bytes, err := io.ReadAll(file)
	if err != nil {
		log.Fatalf("Failed to read file")
	}

	var data merkleProofTestData

	if err := json.Unmarshal(bytes, &data); err != nil {
		log.Fatalf("Failed to unmarshal the test data")
	}

	r, err := verifyMerkleProof(data.Proof, data.Header, []byte(data.BlockMerkleRoot), data.HotShotCommitment)
	if !r {
		log.Fatalf("Failed to verify the merkle proof %v", err)
	}

	// Tamper with the correct data and see if it will return false
	data.HotShotCommitment[0] = 1

	r, err = verifyMerkleProof(data.Proof, data.Header, []byte(data.BlockMerkleRoot), data.HotShotCommitment)
	if r {
		log.Fatalf("Merkle proof should have failed to verify but succeeded")
	}
	// Check that we received the expected error
	if err == nil {
		log.Fatalf("Expected an error but got nil")
	}
	msg := "circuit commitment mismatch"
	if !strings.Contains(err.Error(), msg) {
		log.Fatalf("Expected error message to contains '%v', got: %v", msg, err.Error())
	}

}

type namespaceProofTestData struct {
	NsProof   json.RawMessage `json:"ns_proof"`
	VidCommit string          `json:"vid_commit"`
	VidCommon json.RawMessage `json:"vid_common"`
	Namespace uint64          `json:"namespace"`
	NsTable   []uint8         `json:"ns_table"`
	TxCommit  string          `json:"tx_commit"`
}

func TestNamespaceProofVerification(t *testing.T) {
	file, err := os.Open("./namespace_proof_test_data.json")
	if err != nil {
		log.Fatalf("Failed to open file: %v", err)
	}
	defer file.Close()

	bytes, err := io.ReadAll(file)
	if err != nil {
		log.Fatalf("Failed to read file")
	}

	var data namespaceProofTestData

	if err := json.Unmarshal(bytes, &data); err != nil {
		log.Fatalf("Failed to unmarshal the test data: %v", err)
	}

	r, err := verifyNamespace(data.Namespace, data.NsProof, []byte(data.VidCommit), data.NsTable, []byte(data.TxCommit), data.VidCommon)
	if !r {
		log.Fatalf("Failed to verify the namespace proof %v", err)
	}

	// Tamper with the correct data and see if it will return false
	data.Namespace = 1

	r, err = verifyNamespace(data.Namespace, data.NsProof, []byte(data.VidCommit), data.NsTable, []byte(data.TxCommit), data.VidCommon)
	if r {
		log.Fatalf("Failed to verify the namespace proof")
	}
	if err == nil {
		log.Fatalf("Expected an error but got nil")
	}
	msg := "namespace mismatch"
	if !strings.Contains(err.Error(), msg) {
		log.Fatalf("Expected error message to contain '%v', got: %v", msg, err.Error())
	}
}

func TestDecodePayload(t *testing.T) {
	file, err := os.Open("./payload_test_data.json")
	if err != nil {
		log.Fatalf("Failed to open file: %v", err)
	}
	defer file.Close()

	bytes, err := io.ReadAll(file)
	if err != nil {
		log.Fatalf("Failed to read file")
	}

	var data common.BlockPayload

	if err := json.Unmarshal(bytes, &data); err != nil {
		log.Fatalf("Failed to unmarshal the test data: %v", err)
	}

	// Test successful decoding
	transactions, err := decodePayload(data.RawPayload, data.NsTable.Bytes)
	if err != nil {
		t.Fatalf("Failed to decode payload: %v", err)
	}

	// Verify we got transactions
	if len(transactions) != 1 {
		t.Fatalf("Expected to decode a transaction, got none")
	}

	// Test with empty payload
	emptyTransactions, err := decodePayload([]byte{}, []byte{00,00,00,00})
	if err != nil {
		t.Fatalf("Failed to decode empty payload: %v", err)
	}
	if len(emptyTransactions) != 0 {
		t.Fatalf("Expected 0 transactions from empty payload, got %d", len(emptyTransactions))
	}

	// Test with invalid ns_table (too short to be valid)
	_, err = decodePayload([]byte{}, []byte{})
	if err == nil {
		t.Fatalf("Expected error with invalid ns_table, but got none")
	}
}
