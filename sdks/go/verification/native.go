package verification

/*
#cgo darwin,amd64 LDFLAGS: -L${SRCDIR}/target/lib/ -lespresso_crypto_helper-x86_64-apple-darwin -lm
#cgo darwin,arm64 LDFLAGS: -L${SRCDIR}/target/lib/ -lespresso_crypto_helper-aarch64-apple-darwin -lm
#cgo linux,amd64 LDFLAGS: -L${SRCDIR}/target/lib/ -lespresso_crypto_helper-x86_64-unknown-linux-gnu -lm
#cgo linux,arm64 LDFLAGS: -L${SRCDIR}/target/lib/ -lespresso_crypto_helper-aarch64-unknown-linux-gnu -lm
#include <stdbool.h>
#include <stdint.h>

typedef struct {
    bool success;
    char* error;
} VerificationResult;


extern void free_error_string(char* s);
extern VerificationResult verify_merkle_proof_helper(
    const uint8_t* proof_ptr, size_t proof_len,
    const uint8_t* header_ptr, size_t header_len,
    const uint8_t* block_comm_ptr, size_t block_comm_len,
    const uint8_t* circuit_block_ptr, size_t circuit_block_len
);
extern VerificationResult verify_namespace_helper(
    uint64_t namespace,
    const uint8_t* proof_ptr, size_t proof_len,
    const uint8_t* commit_ptr, size_t commit_len,
    const uint8_t* ns_table_ptr, size_t ns_table_len,
    const uint8_t* tx_comm_ptr, size_t tx_comm_len,
    const uint8_t* common_data_ptr, size_t common_data_len
);
*/
import "C"
import (
	"errors"
	"unsafe"
)

func verifyNamespace(namespace uint64, proof []byte, blockComm []byte, nsTable []byte, txComm []byte, commonData []byte) (bool, error) {
	c_namespace := C.uint64_t(namespace)

	proofPtr := (*C.uint8_t)(unsafe.Pointer(&proof[0]))
	proofLen := C.size_t(len(proof))

	blockCommPtr := (*C.uint8_t)(unsafe.Pointer(&blockComm[0]))
	blockCommLen := C.size_t(len(blockComm))

	nsTablePtr := (*C.uint8_t)(unsafe.Pointer(&nsTable[0]))
	nsTableLen := C.size_t(len(nsTable))

	txCommPtr := (*C.uint8_t)(unsafe.Pointer(&txComm[0]))
	txCommLen := C.size_t(len(txComm))

	commonDataPtr := (*C.uint8_t)(unsafe.Pointer(&commonData[0]))
	commonDataLen := C.size_t(len(commonData))

	result := C.verify_namespace_helper(
		c_namespace, proofPtr, proofLen, blockCommPtr, blockCommLen, nsTablePtr, nsTableLen, txCommPtr, txCommLen, commonDataPtr, commonDataLen)
	defer C.free_error_string(result.error)
	if bool(result.success) {
		return true, nil
	}
	// Allocate a new string in go, so we can free the C string
	// See https://go.dev/wiki/cgo#go-strings-and-c-strings
	msg := C.GoString(result.error)
	return false, errors.New(msg)
}

func verifyMerkleProof(proof []byte, header []byte, blockComm []byte, circuitBlock []byte) (bool, error) {

	proofPtr := (*C.uint8_t)(unsafe.Pointer(&proof[0]))
	proofLen := C.size_t(len(proof))

	headerPtr := (*C.uint8_t)(unsafe.Pointer(&header[0]))
	headerLen := C.size_t(len(header))

	blockCommPtr := (*C.uint8_t)(unsafe.Pointer(&blockComm[0]))
	blockCommLen := C.size_t(len(blockComm))

	circuitBlockPtr := (*C.uint8_t)(unsafe.Pointer(&circuitBlock[0]))
	circuitBlockLen := C.size_t(len(circuitBlock))

	result := C.verify_merkle_proof_helper(proofPtr, proofLen, headerPtr, headerLen, blockCommPtr, blockCommLen, circuitBlockPtr, circuitBlockLen)
	defer C.free_error_string(result.error)
	if bool(result.success) {
		return true, nil
	}
	// Allocate a new string in go, so we can free the C string
	// See https://go.dev/wiki/cgo#go-strings-and-c-strings
	msg := C.GoString(result.error)
	return false, errors.New(msg)
}
