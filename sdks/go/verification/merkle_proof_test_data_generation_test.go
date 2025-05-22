package verification

// import (
// 	"context"
// 	"encoding/json"
// 	"fmt"
// 	"testing"
// 	"time"

// 	espressoClient "github.com/EspressoSystems/espresso-network/sdks/go/client"
// 	lightclient "github.com/EspressoSystems/espresso-network/sdks/go/light-client"
// 	espressoTypes "github.com/EspressoSystems/espresso-network/sdks/go/types"
// 	"github.com/ethereum/go-ethereum/accounts/abi/bind"
// 	"github.com/ethereum/go-ethereum/common"
// 	"github.com/ethereum/go-ethereum/ethclient"
// )

// type merkleProoftData struct {
// 	Proof             json.RawMessage `json:"proof"`
// 	Header            json.RawMessage `json:"header"`
// 	BlockMerkleRoot   string          `json:"block_merkle_root"`
// 	HotShotCommitment []uint8         `json:"hotshot_commitment"`
// }

// // go test ./espressocrypto -run ^TestGenerateMerkleProofTestData$
// //
// // Make sure the espresso network and L1 is running
// // If you are using dev node, visit http://localhost:{port}/v0/api/dev-info to get the light client address
// func TestGenerateMerkleProofTestData(t *testing.T) {
// 	fmt.Println("Generating merkle proof test data...")
// 	ctx := context.Background()
// 	hotshotUrl := "http://localhost:21000"
// 	l1Url := "http://localhost:8545"
// 	lightClientAddr := "0x9fe46736679d2d9a65f0992f2272de9f3c7fa6e0"

// 	tx := espressoTypes.Transaction{
// 		Namespace: 100,
// 		Payload:   []byte("test"),
// 	}

// 	hotshotClient := espressoClient.NewClient(hotshotUrl)
// 	txHash, err := hotshotClient.SubmitTransaction(ctx, tx)
// 	if err != nil {
// 		t.Fatalf("Failed to submit transaction: %v", err)
// 	}

// 	var txData espressoTypes.TransactionQueryData
// 	limit := 30
// 	for {
// 		txData, err = hotshotClient.FetchTransactionByHash(ctx, txHash)
// 		if err == nil {
// 			break
// 		}
// 		limit--
// 		if limit <= 0 {
// 			t.Fatalf("Failed to fetch transaction")
// 		}
// 		time.Sleep(1 * time.Second)
// 	}

// 	header, err := hotshotClient.FetchRawHeaderByHeight(ctx, txData.BlockHeight)
// 	if err != nil {
// 		t.Fatalf("Failed to fetch header: %v", err)
// 	}

// 	l1Client, err := ethclient.Dial(l1Url)
// 	if err != nil {
// 		t.Fatalf("Failed to dial L1 client: %v", err)
// 	}
// 	lightClientReader, err := lightclient.NewLightClientReader(common.HexToAddress(lightClientAddr), l1Client)
// 	if err != nil {
// 		t.Fatalf("Failed to create light client reader: %v", err)
// 	}

// 	var nextHeight uint64
// 	var commitment espressoTypes.Commitment
// 	limit = 30
// 	for {
// 		fmt.Printf("Fetching snapshot at height: %v\n", txData.BlockHeight)
// 		snapshot, err := lightClientReader.FetchMerkleRoot(txData.BlockHeight, &bind.CallOpts{})
// 		fmt.Printf("LightClientReader error: %v\n", err)
// 		if err == nil && snapshot.Height > 0 {
// 			nextHeight = snapshot.Height
// 			commitment = snapshot.Root
// 			break
// 		}
// 		limit--
// 		if limit <= 0 {
// 			t.Fatalf("Failed to fetch merkle root")
// 		}
// 		time.Sleep(15 * time.Second)
// 	}

// 	fmt.Println("snapshot height:", nextHeight)
// 	fmt.Println("Fetching block merkle proof...")

// 	proof, err := hotshotClient.FetchBlockMerkleProof(ctx, nextHeight, txData.BlockHeight)
// 	if err != nil {
// 		t.Fatalf("Failed to fetch block merkle proof: %v", err)
// 	}

// 	nextHeader, err := hotshotClient.FetchHeaderByHeight(ctx, nextHeight)
// 	if err != nil {
// 		t.Fatalf("Failed to fetch header: %v", err)
// 	}

// 	testData := merkleProoftData{
// 		Proof:             proof.Proof,
// 		Header:            header,
// 		BlockMerkleRoot:   nextHeader.Header.GetBlockMerkleTreeRoot().String(),
// 		HotShotCommitment: commitment[:],
// 	}

// 	// filePath := "merkle_proof_test_data.json"
// 	// file, err := os.Create(filePath)
// 	// if err != nil {
// 	// 	t.Fatalf("Failed to create file: %v", err)
// 	// }
// 	// defer file.Close()

// 	// json.NewEncoder(file).Encode(testData)

// }
