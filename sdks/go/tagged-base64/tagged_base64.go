package tagged_base64

import (
	"encoding/base64"
	"encoding/json"
	"fmt"
	"strings"
	"unicode"

	"github.com/sigurn/crc8"
)

var (
	BASE64 *base64.Encoding = base64.RawURLEncoding
)

type TaggedBase64 struct {
	tag      string
	value    []byte
	checksum byte
}

func New(tag string, value []byte) (*TaggedBase64, error) {
	if err := checkSafeBase64Tag(tag); err != nil {
		return nil, err
	}
	return &TaggedBase64{
		tag:      tag,
		value:    value,
		checksum: calcChecksum(tag, value),
	}, nil
}

func (t *TaggedBase64) String() string {
	data := append(t.value, t.checksum)
	return fmt.Sprintf("%s~%s", t.tag, BASE64.EncodeToString(data))
}

func Parse(s string) (*TaggedBase64, error) {
	// Split tag and data.
	tokens := strings.Split(s, "~")
	if len(tokens) < 2 {
		return nil, fmt.Errorf("missing delimiter")
	} else if len(tokens) > 2 {
		return nil, fmt.Errorf("too many delimiters")
	}
	tag := tokens[0]
	base64 := tokens[1]

	if err := checkSafeBase64Tag(tag); err != nil {
		return nil, err
	}

	// Decode data from base64.
	data, err := BASE64.DecodeString(base64)
	if err != nil {
		return nil, fmt.Errorf("invalid base 64: %w", err)
	}

	// Verify checksum.
	if len(data) == 0 {
		return nil, fmt.Errorf("missing checksum")
	}
	value := data[:len(data)-1]
	cs := data[len(data)-1]
	if cs != calcChecksum(tag, value) {
		return nil, fmt.Errorf("incorrect checksum")
	}

	return &TaggedBase64{
		tag:      tag,
		value:    value,
		checksum: cs,
	}, nil
}

func (t *TaggedBase64) MarshalJSON() ([]byte, error) {
	return json.Marshal(t.String())
}

func (t *TaggedBase64) UnmarshalJSON(in []byte) error {
	var s string
	if err := json.Unmarshal(in, &s); err != nil {
		return err
	}
	parsed, err := Parse(s)
	if err != nil {
		return err
	}
	*t = *parsed
	return nil
}

func (t *TaggedBase64) Tag() string {
	return t.tag
}

func (t *TaggedBase64) Value() []byte {
	return t.value
}

func checkSafeBase64Tag(tag string) error {
	for _, c := range tag {
		if err := checkSafeBase64Ascii(c); err != nil {
			return fmt.Errorf("invalid tag %s: %w", tag, err)
		}
	}
	return nil
}

func checkSafeBase64Ascii(c rune) error {
	if c > unicode.MaxASCII {
		return fmt.Errorf("non-ASCII character %c", c)
	}
	// Allow alphanumeric characters, plus - and _
	if !(unicode.IsLetter(c) || unicode.IsDigit(c) || c == '-' || c == '_') {
		return fmt.Errorf("disallowed character %c", c)
	}
	return nil
}

func calcChecksum(tag string, value []byte) byte {
	table := crc8.MakeTable(crc8.CRC8)
	cs := crc8.Init(table)
	cs = crc8.Update(cs, []byte(tag), table)
	cs = crc8.Update(cs, value, table)
	return byte(crc8.Complete(cs, table)) ^ byte(len(value))
}
