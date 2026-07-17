//! Serialization (and deserialization) of primitive unsigned integer types to
//! (and from) an arbitrary fixed-length byte array.
//!
use std::mem::size_of;

use paste::paste;

// Use an ugly macro because it's difficult or impossible to be generic over
// primitive types such as `usize`, `u64`.
macro_rules! uint_bytes_impl {
        ($T:ty) => {
            paste! {
                /// Serialize `n` into `BYTE_LEN` bytes in little-endian form, padding with
                /// 0 as needed.
                ///
                /// # Panics
                /// If `n` cannot fit into `BYTE_LEN` bytes.
                pub fn [<$T _to_bytes>]<const BYTE_LEN: usize>(n: $T) -> [u8; BYTE_LEN] {
                    if size_of::<$T>() > BYTE_LEN {
                        assert!(
                            [<$T _fits>](n, BYTE_LEN),
                            "n {n} cannot fit into {BYTE_LEN} bytes"
                        );
                        n.to_le_bytes()[..BYTE_LEN].try_into().unwrap() // panic is impossible
                    } else {
                        // convert `n` to bytes and pad with 0
                        let mut result = [0; BYTE_LEN];
                        result[..size_of::<$T>()].copy_from_slice(&n.to_le_bytes()[..]);
                        result
                    }
                }

                /// Deserialize `bytes` in little-endian form into a `$T`, padding with 0
                /// as needed.
                ///
                /// # Panics
                /// If `bytes.len()` is too large to fit into a `$T`.
                pub fn [<$T _from_bytes>]<const BYTE_LEN: usize>(bytes: &[u8]) -> $T {
                    assert!(bytes.len() <= BYTE_LEN, "bytes len {} exceeds BYTE_LEN {BYTE_LEN}", bytes.len());
                    assert!(
                        BYTE_LEN <= size_of::<$T>(),
                        "BYTE_LEN {BYTE_LEN} cannot fit into {}",
                        stringify!($T)
                    );
                    let mut [<$T _bytes>] = [0; size_of::<$T>()];
                    [<$T _bytes>][..bytes.len()].copy_from_slice(bytes);
                    $T::from_le_bytes([<$T _bytes>])
                }

                /// Return the largest `$T` value that can fit into `byte_len` bytes.
                pub const fn [<$T _max_from_byte_len>](byte_len: usize) -> $T {
                    if byte_len >= size_of::<$T>() {
                        $T::MAX
                    } else {
                        // overflow cannot occur because `byte_len < size_of::<$T>()`
                        (1 << (byte_len * 8)) - 1
                    }
                }

                /// Can `n` fit into `byte_len` bytes?
                pub const fn [<$T _fits>](n: $T, byte_len: usize) -> bool {
                    n <= [<$T _max_from_byte_len>](byte_len)
                }
            }
        };
    }

uint_bytes_impl!(usize);
uint_bytes_impl!(u32);

/// Impl [`serde`] for type `$T` with methods named `$to_bytes`, `$from_bytes`
/// of the form
/// ```ignore
/// $T::$to_bytes(&self) -> $B
/// $T::$from_bytes(bytes: &[u8]) -> Self
/// ```
/// where `$B` is any type that impls [`serde::Deserialize`] and has a method
/// `as_ref` of the form
/// ```ignore
/// $B::as_ref(&self) -> &[u8]
/// ```
/// Typical examples of `$B` include array `[u8; N]`, slice `&[u8]`, or
/// `Vec<u8>`.
macro_rules! bytes_serde_impl {
    ($T:ty, $to_bytes:ident, $B:ty, $from_bytes:ident) => {
        impl Serialize for $T {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.$to_bytes().serialize(serializer)
            }
        }

        impl<'de> Deserialize<'de> for $T {
            fn deserialize<D>(deserializer: D) -> Result<$T, D::Error>
            where
                D: Deserializer<'de>,
            {
                <$B as Deserialize>::deserialize(deserializer)
                    .map(|bytes| <$T>::$from_bytes(bytes.as_ref()))
            }
        }
    };
}

pub(super) use bytes_serde_impl;

#[cfg(test)]
mod test {
    use std::mem::size_of;

    use fluent_asserter::prelude::*;
    use paste::paste;

    macro_rules! uint_bytes_test_impl {
            ($T:ty) => {
                paste! {
                    use super::{[<$T _max_from_byte_len>], [<$T _to_bytes>], [<$T _from_bytes>]};

                    #[test]
                    fn [<$T _max_from_byte_len_correctness>]() {
                        // test byte lengths 0 to size_of::<$T>()
                        let mut bytes = [0; size_of::<$T>()];
                        assert_eq!([<$T _max_from_byte_len>](0), 0);
                        for i in 0..bytes.len() {
                            bytes[i] = 0xff;
                            assert_eq!([<$T _max_from_byte_len>](i + 1).to_le_bytes(), bytes);
                        }

                        // test byte lengths size_of::<$T>() to twice that length
                        for i in size_of::<$T>()..2 * size_of::<$T>() {
                            assert_eq!([<$T _max_from_byte_len>](i + 1), $T::MAX);
                        }
                    }

                    #[test]
                    fn [<$T _to_bytes_correctness>]() {
                        // byte length 0
                        assert_eq!([<$T _to_bytes>](0), [0; 0]);
                        assert_that_code!(|| [<$T _to_bytes>]::<0>(1)).panics();

                        // byte length 1
                        assert_eq!([<$T _to_bytes>](0), [0; 1]);
                        assert_eq!([<$T _to_bytes>](255), [255; 1]);
                        assert_that_code!(|| [<$T _to_bytes>]::<1>(256)).panics();

                        // byte length 2
                        assert_eq!([<$T _to_bytes>](0), [0; 2]);
                        assert_eq!([<$T _to_bytes>](65535), [255; 2]);
                        assert_that_code!(|| [<$T _to_bytes>]::<2>(65536)).panics();

                        // byte length size_of::<$T>()
                        assert_eq!([<$T _to_bytes>](0), [0; size_of::<$T>()]);
                        assert_eq!([<$T _to_bytes>]($T::MAX), [255; size_of::<$T>()]);

                        // byte length size_of::<$T>() + 1
                        assert_eq!([<$T _to_bytes>](0), [0; size_of::<$T>() + 1]);
                        let [<$T _max_bytes>] = {
                            let mut bytes = [255; size_of::<$T>() + 1];
                            bytes[bytes.len() - 1] = 0;
                            bytes
                        };
                        assert_eq!([<$T _to_bytes>]($T::MAX), [<$T _max_bytes>]);
                    }

                    #[test]
                    fn [<$T _from_bytes_correctness>]() {
                        let bytes = [255; size_of::<$T>() + 1];

                        // It would be nice to iterate through
                        // `0..size_of::<$T>()` but this is not possible with
                        // const generics for `[<$T _from_bytes>]`. We could
                        // use `seq-macro` crate but it requires an integer
                        // literal whereas our range includes `size_of::<$T>()`.
                        //
                        // Instead we just hard code four constants:
                        // `0`, `1`, `size_of::<$T>() - 1`, `size_of::<$T>()`.
                        assert_eq!(
                            [<$T _from_bytes>]::<0>(&bytes[..0]),
                            [<$T _max_from_byte_len>](0)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<1>(&bytes[..1]),
                            [<$T _max_from_byte_len>](1)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>() - 1}>(&bytes[..size_of::<$T>() - 1]),
                            [<$T _max_from_byte_len>](size_of::<$T>() - 1)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..size_of::<$T>()]),
                            [<$T _max_from_byte_len>](size_of::<$T>())
                        );

                        assert_that_code!(|| [<$T _from_bytes>]::<{size_of::<$T>() + 1}>(&bytes[..])).panics();
                    }

                    #[test]
                    fn [<$T _from_bytes_allows_smaller_byte_lens>]() {
                        // This test same as `xxx_from_bytes_correctness` except
                        // we set the const param `BYTE_LEN` to
                        // `size_of::<$T>()` in all cases. Why? To ensure that
                        // `xxx_from_bytes` allows its arg to have length
                        // smaller than `BYTE_LEN`.
                        let bytes = [255; size_of::<$T>() + 1];

                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..0]),
                            [<$T _max_from_byte_len>](0)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..1]),
                            [<$T _max_from_byte_len>](1)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..size_of::<$T>() - 1]),
                            [<$T _max_from_byte_len>](size_of::<$T>() - 1)
                        );
                        assert_eq!(
                            [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..size_of::<$T>()]),
                            [<$T _max_from_byte_len>](size_of::<$T>())
                        );

                        assert_that_code!(|| [<$T _from_bytes>]::<{size_of::<$T>()}>(&bytes[..])).panics();
                    }
                }
            };
        }

    uint_bytes_test_impl!(usize);
    uint_bytes_test_impl!(u32);
}
