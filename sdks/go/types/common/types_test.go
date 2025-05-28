package common

import (
	"encoding/json"
	"strings"
	"testing"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"

	"github.com/ethereum/go-ethereum/common"

	"github.com/stretchr/testify/require"
)

// Reference data taken from the reference sequencer implementation
// (https://github.com/EspressoSystems/espresso-network/blob/main/data)

var ReferenceL1BLockInfo L1BlockInfo = L1BlockInfo{
	Number:    123,
	Timestamp: *NewU256().SetUint64(0x456),
	Hash:      common.Hash{0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef},
}

var ReferenceNsTable NsTable = NsTable{
	Bytes: []byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
}

var ReferencePayloadCommitment, _ = tagged_base64.Parse("HASH~1yS-KEtL3oDZDBJdsW51Pd7zywIiHesBZsTbpOzrxOfu")
var ReferenceBuilderCommitment, _ = tagged_base64.Parse("BUILDER_COMMITMENT~1yS-KEtL3oDZDBJdsW51Pd7zywIiHesBZsTbpOzrxOdZ")
var ReferenceBlockMerkleTreeRoot, _ = tagged_base64.Parse("MERKLE_COMM~yB4_Aqa35_PoskgTpcCR1oVLh6BUdLHIs7erHKWi-usUAAAAAAAAAAEAAAAAAAAAJg")
var ReferenceFeeMerkleTreeRoot, _ = tagged_base64.Parse("MERKLE_COMM~VJ9z239aP9GZDrHp3VxwPd_0l28Hc5KEAB1pFeCIxhYgAAAAAAAAAAIAAAAAAAAAdA")

var ReferenceTransaction Transaction = Transaction{
	Namespace: 12648430,
	Payload:   []byte{76, 111, 114, 101, 109, 32, 105, 112, 115, 117, 109, 32, 100, 111, 108, 111, 114, 32, 115, 105, 116, 32, 97, 109, 101, 116, 44, 32, 99, 111, 110, 115, 101, 99, 116, 101, 116, 117, 114, 32, 97, 100, 105, 112, 105, 115, 99, 105, 110, 103, 32, 101, 108, 105, 116, 46, 32, 68, 111, 110, 101, 99, 32, 108, 101, 99, 116, 117, 115, 32, 118, 101, 108, 105, 116, 44, 32, 99, 111, 109, 109, 111, 100, 111, 32, 101, 103, 101, 116, 32, 116, 101, 108, 108, 117, 115, 32, 118, 105, 116, 97, 101, 44, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 109, 97, 120, 105, 109, 117, 115, 32, 116, 117, 114, 112, 105, 115, 46, 32, 77, 97, 101, 99, 101, 110, 97, 115, 32, 108, 97, 99, 117, 115, 32, 109, 97, 117, 114, 105, 115, 44, 32, 97, 117, 99, 116, 111, 114, 32, 113, 117, 105, 115, 32, 108, 97, 99, 117, 115, 32, 97, 116, 44, 32, 97, 117, 99, 116, 111, 114, 32, 118, 111, 108, 117, 116, 112, 97, 116, 32, 110, 105, 115, 105, 46, 32, 70, 117, 115, 99, 101, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 117, 114, 110, 97, 32, 115, 105, 116, 32, 97, 109, 101, 116, 32, 113, 117, 97, 109, 32, 105, 109, 112, 101, 114, 100, 105, 101, 116, 32, 115, 117, 115, 99, 105, 112, 105, 116, 46, 32, 68, 111, 110, 101, 99, 32, 101, 108, 105, 116, 32, 108, 101, 99, 116, 117, 115, 44, 32, 100, 97, 112, 105, 98, 117, 115, 32, 105, 110, 32, 105, 112, 115, 117, 109, 32, 101, 116, 44, 32, 118, 105, 118, 101, 114, 114, 97, 32, 112, 104, 97, 114, 101, 116, 114, 97, 32, 102, 101, 108, 105, 115, 46, 32, 83, 101, 100, 32, 115, 101, 100, 32, 115, 101, 109, 32, 115, 101, 100, 32, 108, 105, 98, 101, 114, 111, 32, 115, 101, 109, 112, 101, 114, 32, 112, 111, 115, 117, 101, 114, 101, 46, 32, 85, 116, 32, 101, 117, 105, 115, 109, 111, 100, 32, 112, 117, 114, 117, 115, 32, 97, 116, 32, 109, 111, 108, 101, 115, 116, 105, 101, 32, 118, 111, 108, 117, 116, 112, 97, 116, 46, 32, 78, 117, 110, 99, 32, 101, 117, 105, 115, 109, 111, 100, 32, 105, 100, 32, 101, 115, 116, 32, 110, 101, 99, 32, 101, 117, 105, 115, 109, 111, 100, 46, 32, 65, 108, 105, 113, 117, 97, 109, 32, 113, 117, 105, 115, 32, 101, 114, 97, 116, 32, 98, 105, 98, 101, 110, 100, 117, 109, 44, 32, 101, 103, 101, 115, 116, 97, 115, 32, 97, 117, 103, 117, 101, 32, 113, 117, 105, 115, 44, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 116, 101, 108, 108, 117, 115, 46, 32, 68, 117, 105, 115, 32, 100, 97, 112, 105, 98, 117, 115, 32, 97, 99, 32, 106, 117, 115, 116, 111, 32, 117, 116, 32, 114, 104, 111, 110, 99, 117, 115, 46, 32, 78, 117, 108, 108, 97, 32, 118, 101, 104, 105, 99, 117, 108, 97, 32, 97, 117, 103, 117, 101, 32, 110, 111, 110, 32, 97, 114, 99, 117, 32, 118, 101, 115, 116, 105, 98, 117, 108, 117, 109, 32, 116, 101, 109, 112, 117, 115, 46, 32, 68, 117, 105, 115, 32, 117, 108, 108, 97, 109, 99, 111, 114, 112, 101, 114, 32, 115, 105, 116, 32, 97, 109, 101, 116, 32, 108, 97, 99, 117, 115, 32, 101, 116, 32, 100, 105, 103, 110, 105, 115, 115, 105, 109, 46, 32, 77, 97, 117, 114, 105, 115, 32, 97, 117, 99, 116, 111, 114, 32, 115, 111, 108, 108, 105, 99, 105, 116, 117, 100, 105, 110, 32, 102, 101, 117, 103, 105, 97, 116, 46, 32, 70, 117, 115, 99, 101, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 99, 111, 110, 100, 105, 109, 101, 110, 116, 117, 109, 32, 100, 97, 112, 105, 98, 117, 115, 46, 32, 65, 108, 105, 113, 117, 97, 109, 32, 97, 114, 99, 117, 32, 108, 101, 99, 116, 117, 115, 44, 32, 98, 108, 97, 110, 100, 105, 116, 32, 115, 101, 100, 32, 115, 101, 109, 32, 115, 105, 116, 32, 97, 109, 101, 116, 44, 32, 102, 101, 114, 109, 101, 110, 116, 117, 109, 32, 118, 101, 104, 105, 99, 117, 108, 97, 32, 109, 101, 116, 117, 115, 46, 32, 77, 97, 101, 99, 101, 110, 97, 115, 32, 116, 117, 114, 112, 105, 115, 32, 110, 101, 113, 117, 101, 44, 32, 116, 114, 105, 115, 116, 105, 113, 117, 101, 32, 101, 103, 101, 116, 32, 116, 105, 110, 99, 105, 100, 117, 110, 116, 32, 117, 116, 44, 32, 115, 99, 101, 108, 101, 114, 105, 115, 113, 117, 101, 32, 101, 117, 32, 108, 97, 99, 117, 115, 46, 32, 85, 116, 32, 98, 108, 97, 110, 100, 105, 116, 32, 101, 117, 32, 108, 101, 111, 32, 118, 105, 116, 97, 101, 32, 118, 111, 108, 117, 116, 112, 97, 116, 46},
}

func removeWhitespace(s string) string {
	// Split the string on whitespace then concatenate the segments
	return strings.Join(strings.Fields(s), "")
}
func TestEspressoTypesL1BLockInfoJson(t *testing.T) {
	data := []byte(removeWhitespace(`{
		"number": 123,
		"timestamp": "0x456",
		"hash": "0x0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
	}`))

	// Check encoding.
	encoded, err := json.Marshal(ReferenceL1BLockInfo)
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}
	require.Equal(t, encoded, data)

	// Check decoding
	var decoded L1BlockInfo
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal JSON: %v", err)
	}
	require.Equal(t, decoded, ReferenceL1BLockInfo)

	CheckJsonRequiredFields[L1BlockInfo](t, data, "number", "timestamp", "hash")
}

func TestEspressoTransactionJson(t *testing.T) {
	data := []byte(removeWhitespace(`{
		"namespace": 12648430,
		"payload": "TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGFtZXQsIGNvbnNlY3RldHVyIGFkaXBpc2NpbmcgZWxpdC4gRG9uZWMgbGVjdHVzIHZlbGl0LCBjb21tb2RvIGVnZXQgdGVsbHVzIHZpdGFlLCBtb2xlc3RpZSBtYXhpbXVzIHR1cnBpcy4gTWFlY2VuYXMgbGFjdXMgbWF1cmlzLCBhdWN0b3IgcXVpcyBsYWN1cyBhdCwgYXVjdG9yIHZvbHV0cGF0IG5pc2kuIEZ1c2NlIG1vbGVzdGllIHVybmEgc2l0IGFtZXQgcXVhbSBpbXBlcmRpZXQgc3VzY2lwaXQuIERvbmVjIGVsaXQgbGVjdHVzLCBkYXBpYnVzIGluIGlwc3VtIGV0LCB2aXZlcnJhIHBoYXJldHJhIGZlbGlzLiBTZWQgc2VkIHNlbSBzZWQgbGliZXJvIHNlbXBlciBwb3N1ZXJlLiBVdCBldWlzbW9kIHB1cnVzIGF0IG1vbGVzdGllIHZvbHV0cGF0LiBOdW5jIGV1aXNtb2QgaWQgZXN0IG5lYyBldWlzbW9kLiBBbGlxdWFtIHF1aXMgZXJhdCBiaWJlbmR1bSwgZWdlc3RhcyBhdWd1ZSBxdWlzLCB0aW5jaWR1bnQgdGVsbHVzLiBEdWlzIGRhcGlidXMgYWMganVzdG8gdXQgcmhvbmN1cy4gTnVsbGEgdmVoaWN1bGEgYXVndWUgbm9uIGFyY3UgdmVzdGlidWx1bSB0ZW1wdXMuIER1aXMgdWxsYW1jb3JwZXIgc2l0IGFtZXQgbGFjdXMgZXQgZGlnbmlzc2ltLiBNYXVyaXMgYXVjdG9yIHNvbGxpY2l0dWRpbiBmZXVnaWF0LiBGdXNjZSB0aW5jaWR1bnQgY29uZGltZW50dW0gZGFwaWJ1cy4gQWxpcXVhbSBhcmN1IGxlY3R1cywgYmxhbmRpdCBzZWQgc2VtIHNpdCBhbWV0LCBmZXJtZW50dW0gdmVoaWN1bGEgbWV0dXMuIE1hZWNlbmFzIHR1cnBpcyBuZXF1ZSwgdHJpc3RpcXVlIGVnZXQgdGluY2lkdW50IHV0LCBzY2VsZXJpc3F1ZSBldSBsYWN1cy4gVXQgYmxhbmRpdCBldSBsZW8gdml0YWUgdm9sdXRwYXQu"
	}`))

	tx := ReferenceTransaction
	// Check encoding.
	encoded, err := json.Marshal(tx)
	if err != nil {
		t.Fatalf("Failed to marshal JSON: %v", err)
	}
	require.Equal(t, encoded, data)

	// Check decoding
	var decoded Transaction
	if err := json.Unmarshal(data, &decoded); err != nil {
		t.Fatalf("Failed to unmarshal JSON: %v", err)
	}
	require.Equal(t, decoded, tx)

	CheckJsonRequiredFields[Transaction](t, data, "namespace", "payload")
}

// Commitment tests ported from the reference sequencer implementation
// (https://github.com/EspressoSystems/espresso-network/blob/main/sequencer/src/block.rs)

func TestEspressoTypesL1BlockInfoCommit(t *testing.T) {
	require.Equal(t, ReferenceL1BLockInfo.Commit(), Commitment{224, 122, 115, 150, 226, 202, 216, 139, 51, 221, 23, 79, 54, 243, 107, 12, 12, 144, 113, 99, 133, 217, 207, 73, 120, 182, 115, 84, 210, 230, 126, 148})
}

func TestEspressoTypesNsTable(t *testing.T) {
	require.Equal(t, ReferenceNsTable.Commit(), Commitment{24, 191, 165, 16, 16, 48, 53, 144, 229, 119, 16, 233, 201, 36, 89, 64, 40, 77, 158, 105, 253, 188, 220, 221, 32, 2, 252, 91, 209, 13, 58, 232})
}

func TestEspressoTypesTransaction(t *testing.T) {
	require.Equal(t, ReferenceTransaction.Commit(), Commitment{239, 188, 78, 127, 214, 247, 253, 27, 70, 194, 164, 59, 255, 51, 143, 122, 226, 81, 75, 72, 153, 192, 94, 196, 38, 37, 127, 55, 51, 175, 226, 226})
}

func TestEspressoCommitmentFromU256TrailingZero(t *testing.T) {
	comm := Commitment{209, 146, 197, 195, 145, 148, 17, 211, 52, 72, 28, 120, 88, 182, 204, 206, 77, 36, 56, 35, 3, 143, 77, 186, 69, 233, 104, 30, 90, 105, 48, 0}
	roundTrip, err := CommitmentFromUint256(comm.Uint256())
	require.Nil(t, err)
	require.Equal(t, comm, roundTrip)
}

func CheckJsonRequiredFields[T any](t *testing.T, data []byte, fields ...string) {
	// Parse the JSON object into a map so we can selectively delete fields.
	var obj map[string]json.RawMessage
	if err := json.Unmarshal(data, &obj); err != nil {
		t.Fatalf("failed to unmarshal JSON: %v", err)
	}

	for _, field := range fields {
		data, err := json.Marshal(withoutKey(obj, field))
		require.Nil(t, err, "failed to marshal JSON")

		var dec T
		err = json.Unmarshal(data, &dec)
		require.NotNil(t, err, "unmarshalling without required field %s should fail", field)
	}
}

func withoutKey[K comparable, V any](m map[K]V, key K) map[K]V {
	copied := make(map[K]V)
	for k, v := range m {
		if k != key {
			copied[k] = v
		}
	}
	return copied
}
