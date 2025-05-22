package tagged_base64

import (
	"fmt"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestStringParse(t *testing.T) {
	tb64, err := New("T", []byte("123"))
	require.NoError(t, err)

	str := tb64.String()
	parsed, err := Parse(str)
	require.NoError(t, err)

	require.Equal(t, tb64, parsed)
}

// Checks basic construction, printing, and parsing:
// - Can construct from a tag string and a binary value
// - Tag and value match the supplied values
// - String representation can be generated
// - Generated string can be parsed
// - Accessors and parsed string match the supplied values
func checkTb64(t *testing.T, tag string, value []byte) {
	tb64, err := New(tag, value)
	require.NoError(t, err)
	str := tb64.String()

	parsed, err := Parse(str)
	require.NoError(t, err)

	require.Equal(t, tb64, parsed)

	// Do we get back the tag we supplied?
	require.Equal(t, parsed.Tag(), tag)

	// Do we get back the binary value we supplied?
	require.Equal(t, parsed.Value(), value)
}

func TestParse(t *testing.T) {
	// The empty string is not a valid TaggedBase64.
	_, err := Parse("")
	require.NotNil(t, err)

	// The tag is alphanumeric with hyphen and underscore.
	// The value here is the base64 encoding of foobar, but
	// the encoding doesn't include the required checksum.
	_, err = Parse("-_~Zm9vYmFy")
	require.NotNil(t, err)

	// An invalid tag should err.
	_, err = Parse("&_~wA")
	require.NotNil(t, err)

	// A null value is not allowed.
	b64Null := BASE64.EncodeToString([]byte{})
	tagged := fmt.Sprintf("a~%s", b64Null)
	_, err = Parse(tagged)
	require.NotNil(t, err)

	// The tag can be empty, but the value cannot because the value
	// includes the checksum.
	_, err = Parse("~")
	require.NotNil(t, err)

	checkTb64(t, "mytag", []byte("mytag"))

	// Only base64 characters are allowed in the tag. No restrictions on
	// the value because it will get base64 encoded.
	checkTb64(
		t,
		"abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789",
		[]byte("~Yeah, we can have spaces and odd stuff—😀 here. ¯⧵_(ツ)_/¯"),
	)
	checkTb64(
		t,
		"",
		[]byte("abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789~"),
	)

	// All the following have invalid characters in the tag.
	_, err = New("~", []byte{})
	require.NotNil(t, err)
	_, err = New("a~", []byte{})
	require.NotNil(t, err)
	_, err = New("~b", []byte{})
	require.NotNil(t, err)
	_, err = New("c~d", []byte{})
	require.NotNil(t, err)
	_, err = New("e~f~", []byte{})
	require.NotNil(t, err)
	_, err = New("g~h~i", []byte{})
	require.NotNil(t, err)
	_, err = New("Oh, no!", []byte{})
	require.NotNil(t, err)
	_, err = New("Σ", []byte{})
	require.NotNil(t, err)
}

func TestCompat(t *testing.T) {
	// A hard-coded example, to check compatibility with the original, Rust implementation.
	tag := "abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789"
	data := []byte("~Yeah, we can have spaces and odd stuff—😀 here. ¯⧵_(ツ)_/¯")
	expected := "abcdefghijklmnopqrstuvwxyz-ABCDEFGHIJKLMNOPQRSTUVWXYZ_0123456789~flllYWgsIHdlIGNhbiBoYXZlIHNwYWNlcyBhbmQgb2RkIHN0dWZm4oCU8J-YgCBoZXJlLiDCr-KntV8o44OEKV8vwq_6"

	tb64, err := New(tag, data)
	require.NoError(t, err)
	s := tb64.String()
	require.Equal(t, s, expected)
	parsed, err := Parse(expected)
	require.NoError(t, err)
	require.Equal(t, tb64, parsed)
}
