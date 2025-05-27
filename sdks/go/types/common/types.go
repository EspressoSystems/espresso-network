package common

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"math/big"

	tagged_base64 "github.com/EspressoSystems/espresso-network/sdks/go/tagged-base64"

	"github.com/ethereum/go-ethereum/common"
)

type TaggedBase64 = tagged_base64.TaggedBase64

type VidCommon = json.RawMessage

type VidCommonQueryData struct {
	Height      uint64        `json:"height"`
	BlockHash   *TaggedBase64 `json:"block_hash"`
	PayloadHash *TaggedBase64 `json:"payload_hash"`
	Common      VidCommon     `json:"common"`
}

type TransactionQueryData struct {
	Transaction Transaction     `json:"transaction"`
	Hash        *TaggedBase64   `json:"hash"`
	Index       uint64          `json:"index"`
	Proof       json.RawMessage `json:"proof"`
	BlockHash   *TaggedBase64   `json:"block_hash"`
	BlockHeight uint64          `json:"block_height"`
}

type L1BlockInfo struct {
	Number    uint64      `json:"number"`
	Timestamp U256        `json:"timestamp"`
	Hash      common.Hash `json:"hash"`
}

func (i *L1BlockInfo) UnmarshalJSON(b []byte) error {
	// Parse using pointers so we can distinguish between missing and default fields.
	type Dec struct {
		Number    *uint64      `json:"number"`
		Timestamp *U256        `json:"timestamp"`
		Hash      *common.Hash `json:"hash"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.Number == nil {
		return fmt.Errorf("Field number of type L1BlockInfo is required")
	}
	i.Number = *dec.Number

	if dec.Timestamp == nil {
		return fmt.Errorf("Field timestamp of type L1BlockInfo is required")
	}
	i.Timestamp = *dec.Timestamp

	if dec.Hash == nil {
		return fmt.Errorf("Field hash of type L1BlockInfo is required")
	}
	i.Hash = *dec.Hash

	return nil
}

func (self *L1BlockInfo) Commit() Commitment {
	return NewRawCommitmentBuilder("L1BLOCK").
		Uint64Field("number", self.Number).
		Uint256Field("timestamp", &self.Timestamp).
		FixedSizeField("hash", self.Hash[:]).
		Finalize()
}

type NsTable struct {
	Bytes Bytes `json:"bytes"`
}

type NamespaceProof = json.RawMessage

type BlockMerkleSnapshot struct {
	Root   BlockMerkleRoot
	Height uint64
}

type BlockMerkleRoot = Commitment

type HotShotBlockMerkleProof struct {
	Proof json.RawMessage `json:"proof"`
}

// Validates a block merkle proof, returning the validated HotShot block height. This is mocked until we have real
// merkle tree snapshot support.
func (p *HotShotBlockMerkleProof) Verify(root BlockMerkleRoot) (uint64, error) {
	return 0, nil
}

func (r *NsTable) UnmarshalJSON(b []byte) error {
	// Parse using pointers so we can distinguish between missing and default fields.
	type Dec struct {
		Bytes *Bytes `json:"bytes"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.Bytes == nil {
		return fmt.Errorf("Field root of type RawPayload is required")
	}
	r.Bytes = *dec.Bytes

	return nil
}

func (self *NsTable) Commit() Commitment {
	return NewRawCommitmentBuilder("NSTABLE").
		VarSizeBytes(self.Bytes).
		Finalize()
}

type Transaction struct {
	Namespace uint64 `json:"namespace"`
	Payload   Bytes  `json:"payload"`
}

func (self *Transaction) Commit() Commitment {
	return NewRawCommitmentBuilder("Transaction").
		Uint64Field("namespace", self.Namespace).
		VarSizeBytes(self.Payload).
		Finalize()
}

func (t *Transaction) UnmarshalJSON(b []byte) error {
	// Parse using pointers so we can distinguish between missing and default fields.
	type Dec struct {
		Namespace *uint64 `json:"namespace"`
		Payload   *Bytes  `json:"payload"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	if dec.Namespace == nil {
		return fmt.Errorf("Field vm of type Transaction is required")
	}
	t.Namespace = *dec.Namespace

	if dec.Payload == nil {
		return fmt.Errorf("Field payload of type Transaction is required")
	}
	t.Payload = *dec.Payload

	return nil
}

// A bytes type which serializes to JSON as an array, rather than a base64 string. This ensures
// compatibility with the Espresso APIs.
type Bytes []byte

func (b Bytes) MarshalJSON() ([]byte, error) {
	s := base64.StdEncoding.EncodeToString(b)
	return json.Marshal(s)
}

func (b *Bytes) UnmarshalJSON(in []byte) error {
	var s string
	if err := json.Unmarshal(in, &s); err != nil {
		return err
	}
	bytes, err := base64.StdEncoding.DecodeString(s)
	if err != nil {
		return err
	}
	*b = bytes
	return nil
}

// A readable decimal format for U256. Please use the struct `U256` to initialize
// the number first and use the `ToDecimal` to convert.
type U256Decimal struct {
	big.Int
}

func (i U256Decimal) MarshalJSON() ([]byte, error) {
	return json.Marshal(i.Text(10))
}

func (i *U256Decimal) UnmarshalJSON(in []byte) error {
	var s string
	if err := json.Unmarshal(in, &s); err != nil {
		return err
	}
	if _, err := fmt.Sscanf(s, "%d", &i.Int); err != nil {
		return err
	}
	return nil
}

func (i *U256Decimal) ToU256() *U256 {
	return &U256{i.Int}
}

// A BigInt type which serializes to JSON a a hex string. This ensures compatibility with the
// Espresso APIs.
type U256 struct {
	big.Int
}

func NewU256() *U256 {
	return new(U256)
}

func (i U256) Equal(other U256) bool {
	return i.Int.Cmp(&other.Int) == 0
}

func (i *U256) SetBigInt(n *big.Int) *U256 {
	i.Int.Set(n)
	return i
}

func (i *U256) SetUint64(n uint64) *U256 {
	i.Int.SetUint64(n)
	return i
}

func (i *U256) SetBytes(buf [32]byte) *U256 {
	i.Int.SetBytes(buf[:])
	return i
}

func (i U256) MarshalJSON() ([]byte, error) {
	return json.Marshal(fmt.Sprintf("0x%s", i.Text(16)))
}

func (i *U256) UnmarshalJSON(in []byte) error {
	var s string
	if err := json.Unmarshal(in, &s); err != nil {
		return err
	}
	if _, err := fmt.Sscanf(s, "0x%x", &i.Int); err != nil {
		return err
	}
	return nil
}

func (i *U256) ToDecimal() *U256Decimal {
	return &U256Decimal{i.Int}
}

type FeeInfo struct {
	Account common.Address `json:"account"`
	Amount  U256Decimal    `json:"amount"`
}

func (self *FeeInfo) Commit() Commitment {
	return NewRawCommitmentBuilder("FEE_INFO").
		FixedSizeField("account", self.Account.Bytes()).
		Uint256Field("amount", self.Amount.ToU256()).
		Finalize()
}

type Signature struct {
	R U256   `json:"r"`
	S U256   `json:"s"`
	V uint64 `json:"v"`
}

func (s *Signature) Bytes() [65]byte {
	var sig [65]byte
	copy(sig[:32], s.R.Bytes())
	copy(sig[32:64], s.S.Bytes())
	sig[64] = byte(s.V)
	return sig
}

type Version struct {
	Major uint16 `json:"major"`
	Minor uint16 `json:"minor"`
}

func (v *Version) UnmarshalJSON(b []byte) error {
	// Use an alias type to avoid recursive calls of this function
	type Alias Version

	type Dec struct {
		Ver Alias `json:"Version"`
	}

	var dec Dec
	if err := json.Unmarshal(b, &dec); err != nil {
		return err
	}

	v.Major = dec.Ver.Major
	v.Minor = dec.Ver.Minor

	return nil
}

func (v Version) MarshalJSON() ([]byte, error) {
	type Alias Version

	type Dec struct {
		Ver Alias `json:"Version"`
	}

	var dec Dec
	dec.Ver.Major = v.Major
	dec.Ver.Minor = v.Minor

	return json.Marshal(dec)
}
