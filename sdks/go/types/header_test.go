package types

import (
	"encoding/json"
	"io"
	"os"
	"testing"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"
	common_types "github.com/EspressoSystems/espresso-network/sdks/go/types/common"
	"github.com/stretchr/testify/require"
)

func TestVersion(t *testing.T) {
	s := `{"Version":{"major":0,"minor":3}}`
	data := []byte(s)
	var v common_types.Version
	err := json.Unmarshal(data, &v)
	if err != nil {
		t.Fatal("Failed to marshal JSON", err)
	}

	if !(v.Major == 0 && v.Minor == 3) {
		t.Fatal("Get the wrong version", v)
	}

	bytes, err := json.Marshal(v)
	if err != nil {
		t.Fatal("Failed to marshal version", err)
	}
	var a common_types.Version
	if err = json.Unmarshal(bytes, &a); err != nil {
		t.Fatal("Failed to unmarshal version", err)
	}
}

func TestHeader0_1(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v1/header.json", t)

	if header.Version().Major != 0 || header.Version().Minor != 1 {
		t.Fatal("Wrong version", header.Version())
	}

	testHeaderFields(header, t)

	require.Equal(t, header.Commit(), common_types.Commitment{118, 29, 74, 165, 219, 239, 197, 43, 231, 156, 250, 78, 139, 108, 136, 220, 51, 160, 242, 30, 165, 182, 189, 138, 191, 93, 226, 71, 54, 208, 190, 211})
}

func TestHeader0_2(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v2/header.json", t)

	if header.Version().Major != 0 || header.Version().Minor != 2 {
		t.Fatal("Wrong version", header.Version())
	}

	testHeaderFields(header, t)

	require.Equal(t, header.Commit(), common_types.Commitment{87, 65, 137, 140, 189, 125, 156, 42, 229, 155, 217, 245, 205, 158, 160, 104, 226, 132, 122, 68, 140, 9, 62, 174, 71, 147, 254, 135, 177, 162, 233, 66})
}

func TestHeader0_3(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v3/header.json", t)

	if header.Version().Major != 0 || header.Version().Minor != 3 {
		t.Fatal("Wrong version", header.Version())
	}

	testHeaderFields(header, t)

	require.Equal(t, header.Commit(), common_types.Commitment{0xa8, 0xa6, 0xf4, 0x6b, 0x16, 0x3d, 0x37, 0xa, 0x6e, 0xb0, 0x99, 0xf9, 0x64, 0x58, 0x63, 0x24, 0xc, 0x86, 0xf0, 0x86, 0x29, 0x26, 0x91, 0xcd, 0xdb, 0xe0, 0x43, 0x22, 0xc2, 0x24, 0x86, 0xb1})
}

// The expected commitments are the `REFERENCE_V*_HEADER_COMMITMENT` constants in
// `crates/espresso/types/src/reference_tests.rs`, which pin the Rust side to the same vectors.
func TestHeader0_4(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v4/header.json", t)

	require.Equal(t, common_types.Version{Major: 0, Minor: 4}, header.Version())
	testHeaderFields(header, t)
	requireCommitment(t, header, "BLOCK~hPVq9NasWW1vVYGGGr0PSRv1TV3nUV_8ARw5fWHlQLx3")
}

func TestHeader0_5(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v5/header.json", t)

	require.Equal(t, common_types.Version{Major: 0, Minor: 5}, header.Version())
	testHeaderFields(header, t)
	requireCommitment(t, header, "BLOCK~yYZmWrTIWJerGV7VA-EeKWL4tnsdJya1BpK4HWdvnwAA")
}

func TestHeader0_6(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v6/header.json", t)

	require.Equal(t, common_types.Version{Major: 0, Minor: 6}, header.Version())
	testHeaderFields(header, t)
	requireCommitment(t, header, "BLOCK~nAVIoY9ekw8WPwzHnLwTgsPZ1qvBox-WQev4nhcrLoZP")
}

// 0.7 dropped `builder_commitment`.
func TestHeader0_7(t *testing.T) {
	header := getHeaderFromTestFile("../../../data/v7/header.json", t)

	require.Equal(t, common_types.Version{Major: 0, Minor: 7}, header.Version())
	require.Equal(t, uint64(42), header.GetBlockHeight())
	require.Nil(t, header.GetBuilderCommitment())
	requireCommitment(t, header, "BLOCK~Y05GqjVqwmnMOza3LMLFagoyQxwEDFcYQCRRnzpho5a1")
}

// A header from mainnet, which runs 0.6, against the hash the query service reports for its
// block. Unlike the reference vectors it has no `next_stake_table_hash` and a large
// `total_reward_distributed`.
func TestHeaderMainnet0_6(t *testing.T) {
	header := getHeaderFromTestFile("testdata/mainnet_header_26050100.json", t)

	require.Equal(t, common_types.Version{Major: 0, Minor: 6}, header.Version())
	require.Equal(t, uint64(26050100), header.GetBlockHeight())
	requireCommitment(t, header, "BLOCK~5Iyf4o7gd7bDnxwuZFb1GjEvhX-yjavRLGyx2QAUHtkF")
}

func TestHeaderImplMarshalAndUnmarshal(t *testing.T) {
	for _, version := range []string{"v1", "v2", "v3", "v4", "v5", "v6", "v7"} {
		header := getHeaderFromTestFile("../../../data/"+version+"/header.json", t)
		testHeaderImplMarshalAndUnmarshal(header, t)
	}
}

// The round trip must keep every field the commitment covers.
func testHeaderImplMarshalAndUnmarshal(header HeaderInterface, t *testing.T) {
	headerImpl := HeaderImpl{Header: header}
	bytes, err := json.Marshal(headerImpl)
	if err != nil {
		t.Fatal("Failed to marshal header", err)
	}
	var actualHeaderImpl HeaderImpl
	err = json.Unmarshal(bytes, &actualHeaderImpl)
	if err != nil {
		t.Fatal("failed to unmarshal header", err)
	}
	require.Equal(t, header.Version(), actualHeaderImpl.Header.Version())
	require.Equal(t, header.Commit(), actualHeaderImpl.Header.Commit())
}

func testHeaderFields(header HeaderInterface, t *testing.T) {
	if header.GetBlockHeight() != 42 {
		t.Fatal("Wrong block height", header.GetBlockHeight())
	}

	if header.GetBuilderCommitment().String() != "BUILDER_COMMITMENT~jlEvJoHPETCSwXF6UKcD22zOjfoHGuyVFTVkP_BNc-no" {
		t.Fatal("Wrong builder commitment", header.GetBuilderCommitment().String())
	}

}

func requireCommitment(t *testing.T, header HeaderInterface, expected string) {
	tagged, err := tagged_base64.Parse(expected)
	require.NoError(t, err)
	actual := header.Commit()
	require.Equal(t, tagged.Value(), actual[:])
}

func getHeaderFromTestFile(path string, t *testing.T) HeaderInterface {
	file, err := os.Open(path)
	if err != nil {
		t.Fatal("failed to open file:", err)
	}
	defer file.Close()

	data, err := io.ReadAll(file)
	if err != nil {
		t.Fatal("Error reading file:", err)
	}

	var headerImpl HeaderImpl
	err = json.Unmarshal(data, &headerImpl)
	if err != nil {
		t.Fatal("Error unmarshalling:", err)
	}

	return headerImpl.Header
}

func TestUnmarshalSignature(t *testing.T) {
	// `r` ans `s` are hex string of odd length.
	// It should be unmarshalled successfully
	data := `
{
    "r": "0xa1c",
    "s": "0x202",
    "v": 27
}
	`
	var signature Signature
	err := json.Unmarshal([]byte(data), &signature)
	if err != nil {
		t.Fatal("Error unmarshalling:", err)
	}
	expectedR := int64(2588)
	expectedS := int64(514)
	expectedV := uint64(27)

	if expectedR != signature.R.Int64() {
		t.Fatal("getting a wrong r in unmarshal signature")
	}

	if expectedS != signature.S.Int64() {
		t.Fatal("getting a wrong r in unmarshal signature")
	}

	if expectedV != signature.V {
		t.Fatal("getting a wrong r in unmarshal signature")
	}

}
