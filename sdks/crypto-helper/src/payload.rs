use std::{ffi::CString, os::raw::c_char};

use espresso_types::{NsTable, Payload};
use hotshot_types::traits::BlockPayload;

#[repr(C)]
pub struct Transaction {
    // Transaction Namespace ID
    namespace: u64,
    // Transaction payload ptr
    payload_ptr: *const u8,
    // Transaction payload len
    payload_len: usize,
    // Transaction payload capacity (for freeing)
    payload_cap: usize,
}

#[repr(C)]
pub struct DecodingResult {
    // The operation succeeded
    pub success: bool,
    // The error message if the operation failed, otherwise null
    pub error: *mut c_char,
    // Pointer to the array of transactions (null on error)
    pub transactions: *mut Transaction,
    // Number of transactions in the array
    pub transactions_len: usize,
    // Capacity of the transactions array (for freeing)
    pub transactions_cap: usize,
}

impl DecodingResult {
    fn err(msg: &str) -> DecodingResult {
        let ptr = CString::new(msg)
            .unwrap_or(c"<invalid error string>".to_owned())
            .into_raw();
        DecodingResult {
            success: false,
            error: ptr,
            transactions: std::ptr::null_mut(),
            transactions_len: 0,
            transactions_cap: 0,
        }
    }

    fn success(transactions: Vec<Transaction>) -> DecodingResult {
        let len = transactions.len();
        let cap = transactions.capacity();
        let ptr = transactions.leak().as_mut_ptr();
        DecodingResult {
            success: true,
            error: std::ptr::null_mut(),
            transactions: ptr,
            transactions_len: len,
            transactions_cap: cap,
        }
    }
}

#[no_mangle]
/// Decode a payload from payload bytes and namespace table bytes.
///
/// # Safety
///
/// payload_ptr and ns_table_ptr must be valid pointers to initialized slices,
/// valid for the duration of the call. It is okay to pass null pointers if the length is zero.
pub unsafe extern "C" fn decode_payload(
    mut payload_ptr: *const u8,
    payload_len: usize,
    mut ns_table_ptr: *const u8,
    ns_table_len: usize,
) -> DecodingResult {
    // Pointers must be non-null even for zero-length slices
    if payload_len == 0 {
        payload_ptr = std::ptr::dangling();
    }
    if ns_table_len == 0 {
        ns_table_ptr = std::ptr::dangling();
    }

    // Pointers must be non-null
    if payload_ptr.is_null() {
        return DecodingResult::err("Invalid payload pointer or length");
    }
    if ns_table_ptr.is_null() {
        return DecodingResult::err("Invalid ns_table pointer or length");
    }

    // Safety:
    //
    // We have ensured that the pointers are non-null, even for zero-length slices
    // It's on caller to pass initialized slices and not do anything weird with them
    // in another thread
    let payload_bytes = unsafe { std::slice::from_raw_parts(payload_ptr, payload_len) };
    let ns_table_bytes = unsafe { std::slice::from_raw_parts(ns_table_ptr, ns_table_len) };

    let ns_table = NsTable::from_bytes_unchecked(ns_table_bytes);

    let payload = Payload::from_bytes(payload_bytes, &ns_table);
    if let Err(err) = ns_table.validate(&payload.byte_len()) {
        return DecodingResult::err(&format!("Invalid namespace table: {:?}", err));
    }

    let transactions: Vec<Transaction> = payload
        .transactions(&ns_table)
        .map(|tx| {
            let namespace = tx.namespace().into();

            let tx_payload = tx.into_payload();
            let payload_ptr = tx_payload.as_ptr();
            let payload_len = tx_payload.len();
            let payload_cap = tx_payload.capacity();

            // Leak the payload to prevent it from being freed
            std::mem::forget(tx_payload);

            Transaction {
                namespace,
                payload_ptr,
                payload_len,
                payload_cap,
            }
        })
        .collect();

    DecodingResult::success(transactions)
}

/// Free a TransactionVecResult that was allocated via FFI.
/// This function will free:
/// - The error string if present
/// - The transactions array
/// - Each transaction's payload within the array
///
/// Caller is responsible for ensuring this is called only on TransactionVecResult allocated
/// by this library.
///
/// # Safety
/// Caller promised this was allocated by us,
#[no_mangle]
pub unsafe extern "C" fn free_transaction_vec_result(result: DecodingResult) {
    // Free the error string if present
    if !result.error.is_null() {
        let _ = CString::from_raw(result.error);
    }

    // Free the transactions array and payloads if present
    if !result.transactions.is_null() {
        // First free each transaction's payload
        let transactions = Vec::from_raw_parts(
            result.transactions,
            result.transactions_len,
            result.transactions_cap,
        );

        for transaction in transactions.iter() {
            if !transaction.payload_ptr.is_null() {
                let _ = Vec::from_raw_parts(
                    transaction.payload_ptr as *mut u8,
                    transaction.payload_len,
                    transaction.payload_cap,
                );
            }
        }

        // The transactions vector itself is dropped when it goes out of scope
    }
}
