//! Atomic libcall shims for riscv32im (no A extension).
//!
//! The SP1 zkVM is single-threaded, so plain loads and stores are sound.

macro_rules! atomic_load_store {
    ($ty:ty, $load:ident, $store:ident) => {
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $load(src: *const $ty, _order: i32) -> $ty {
            unsafe { src.read() }
        }

        #[unsafe(no_mangle)]
        unsafe extern "C" fn $store(dst: *mut $ty, val: $ty, _order: i32) {
            unsafe { dst.write(val) }
        }
    };
}

atomic_load_store!(u8, __atomic_load_1, __atomic_store_1);
atomic_load_store!(u32, __atomic_load_4, __atomic_store_4);
atomic_load_store!(u64, __atomic_load_8, __atomic_store_8);

macro_rules! atomic_compare_exchange {
    ($ty:ty, $name:ident) => {
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name(
            dst: *mut $ty,
            expected: *mut $ty,
            desired: $ty,
            _weak: bool,
            _success: i32,
            _failure: i32,
        ) -> bool {
            unsafe {
                let current = dst.read();
                if current == expected.read() {
                    dst.write(desired);
                    true
                } else {
                    expected.write(current);
                    false
                }
            }
        }
    };
}

atomic_compare_exchange!(u8, __atomic_compare_exchange_1);
atomic_compare_exchange!(u32, __atomic_compare_exchange_4);

#[unsafe(no_mangle)]
unsafe extern "C" fn __atomic_exchange_4(dst: *mut u32, val: u32, _order: i32) -> u32 {
    unsafe {
        let old = dst.read();
        dst.write(val);
        old
    }
}

macro_rules! atomic_fetch_op {
    ($name:ident, $op:expr) => {
        #[unsafe(no_mangle)]
        unsafe extern "C" fn $name(dst: *mut u32, val: u32, _order: i32) -> u32 {
            unsafe {
                let old = dst.read();
                let op: fn(u32, u32) -> u32 = $op;
                dst.write(op(old, val));
                old
            }
        }
    };
}

atomic_fetch_op!(__atomic_fetch_add_4, u32::wrapping_add);
atomic_fetch_op!(__atomic_fetch_sub_4, u32::wrapping_sub);
atomic_fetch_op!(__atomic_fetch_and_4, |a, b| a & b);
atomic_fetch_op!(__atomic_fetch_or_4, |a, b| a | b);
