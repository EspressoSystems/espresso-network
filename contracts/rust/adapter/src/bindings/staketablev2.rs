///Module containing a contract's types and functions.
/**

```solidity
library BN254 {
    type BaseField is uint256;
    struct G1Point { BaseField x; BaseField y; }
    struct G2Point { BaseField x0; BaseField x1; BaseField y0; BaseField y1; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod BN254 {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BaseField(alloy::sol_types::private::primitives::aliases::U256);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<BaseField>
        for alloy::sol_types::private::primitives::aliases::U256 {
            #[inline]
            fn stv_to_tokens(
                &self,
            ) -> <alloy::sol_types::sol_data::Uint<
                256,
            > as alloy_sol_types::SolType>::Token<'_> {
                alloy_sol_types::private::SolTypeValue::<
                    alloy::sol_types::sol_data::Uint<256>,
                >::stv_to_tokens(self)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::SolType>::tokenize(self)
                    .0
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(self, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::SolType>::abi_encoded_size(self)
            }
        }
        #[automatically_derived]
        impl BaseField {
            /// The Solidity type name.
            pub const NAME: &'static str = stringify!(@ name);
            /// Convert from the underlying value type.
            #[inline]
            pub const fn from(
                value: alloy::sol_types::private::primitives::aliases::U256,
            ) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into(
                self,
            ) -> alloy::sol_types::private::primitives::aliases::U256 {
                self.0
            }
            /// Return the single encoding of this value, delegating to the
            /// underlying type.
            #[inline]
            pub fn abi_encode(&self) -> alloy_sol_types::private::Vec<u8> {
                <Self as alloy_sol_types::SolType>::abi_encode(&self.0)
            }
            /// Return the packed encoding of this value, delegating to the
            /// underlying type.
            #[inline]
            pub fn abi_encode_packed(&self) -> alloy_sol_types::private::Vec<u8> {
                <Self as alloy_sol_types::SolType>::abi_encode_packed(&self.0)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for BaseField {
            type RustType = alloy::sol_types::private::primitives::aliases::U256;
            type Token<'a> = <alloy::sol_types::sol_data::Uint<
                256,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = Self::NAME;
            const ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                256,
            > as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                256,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                Self::type_check(token).is_ok()
            }
            #[inline]
            fn type_check(token: &Self::Token<'_>) -> alloy_sol_types::Result<()> {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::SolType>::type_check(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::SolType>::detokenize(token)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for BaseField {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::topic_preimage_length(rust)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, out)
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> alloy_sol_types::abi::token::WordToken {
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic(rust)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**```solidity
struct G1Point { BaseField x; BaseField y; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct G1Point {
        #[allow(missing_docs)]
        pub x: <BaseField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub y: <BaseField as alloy::sol_types::SolType>::RustType,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (BaseField, BaseField);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            <BaseField as alloy::sol_types::SolType>::RustType,
            <BaseField as alloy::sol_types::SolType>::RustType,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<G1Point> for UnderlyingRustTuple<'_> {
            fn from(value: G1Point) -> Self {
                (value.x, value.y)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for G1Point {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { x: tuple.0, y: tuple.1 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for G1Point {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for G1Point {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.x),
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.y),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_packed_encoded_size(&tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for G1Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolStruct for G1Point {
            const NAME: &'static str = "G1Point";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed("G1Point(uint256 x,uint256 y)")
            }
            #[inline]
            fn eip712_components() -> alloy_sol_types::private::Vec<
                alloy_sol_types::private::Cow<'static, str>,
            > {
                alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> alloy_sol_types::private::Cow<'static, str> {
                <Self as alloy_sol_types::SolStruct>::eip712_root_type()
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.x).0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.y).0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for G1Point {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.x,
                    )
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.y,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.x,
                    out,
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.y,
                    out,
                );
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                alloy_sol_types::abi::token::WordToken(
                    alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**```solidity
struct G2Point { BaseField x0; BaseField x1; BaseField y0; BaseField y1; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct G2Point {
        #[allow(missing_docs)]
        pub x0: <BaseField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub x1: <BaseField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub y0: <BaseField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub y1: <BaseField as alloy::sol_types::SolType>::RustType,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (BaseField, BaseField, BaseField, BaseField);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            <BaseField as alloy::sol_types::SolType>::RustType,
            <BaseField as alloy::sol_types::SolType>::RustType,
            <BaseField as alloy::sol_types::SolType>::RustType,
            <BaseField as alloy::sol_types::SolType>::RustType,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<G2Point> for UnderlyingRustTuple<'_> {
            fn from(value: G2Point) -> Self {
                (value.x0, value.x1, value.y0, value.y1)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for G2Point {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    x0: tuple.0,
                    x1: tuple.1,
                    y0: tuple.2,
                    y1: tuple.3,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for G2Point {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for G2Point {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.x0),
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.x1),
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.y0),
                    <BaseField as alloy_sol_types::SolType>::tokenize(&self.y1),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_packed_encoded_size(&tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for G2Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolStruct for G2Point {
            const NAME: &'static str = "G2Point";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "G2Point(uint256 x0,uint256 x1,uint256 y0,uint256 y1)",
                )
            }
            #[inline]
            fn eip712_components() -> alloy_sol_types::private::Vec<
                alloy_sol_types::private::Cow<'static, str>,
            > {
                alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> alloy_sol_types::private::Cow<'static, str> {
                <Self as alloy_sol_types::SolStruct>::eip712_root_type()
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.x0)
                        .0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.x1)
                        .0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.y0)
                        .0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.y1).0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for G2Point {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.x0,
                    )
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.x1,
                    )
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.y0,
                    )
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.y1,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.x0,
                    out,
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.x1,
                    out,
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.y0,
                    out,
                );
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.y1,
                    out,
                );
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                alloy_sol_types::abi::token::WordToken(
                    alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`BN254`](self) contract instance.

See the [wrapper's documentation](`BN254Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> BN254Instance<T, P, N> {
        BN254Instance::<T, P, N>::new(address, provider)
    }
    /**A [`BN254`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`BN254`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct BN254Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for BN254Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("BN254Instance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > BN254Instance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`BN254`](self) contract instance.

See the [wrapper's documentation](`BN254Instance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            provider: P,
        ) -> Self {
            Self {
                address,
                provider,
                _network_transport: ::core::marker::PhantomData,
            }
        }
        /// Returns a reference to the address.
        #[inline]
        pub const fn address(&self) -> &alloy_sol_types::private::Address {
            &self.address
        }
        /// Sets the address.
        #[inline]
        pub fn set_address(&mut self, address: alloy_sol_types::private::Address) {
            self.address = address;
        }
        /// Sets the address and returns `self`.
        pub fn at(mut self, address: alloy_sol_types::private::Address) -> Self {
            self.set_address(address);
            self
        }
        /// Returns a reference to the provider.
        #[inline]
        pub const fn provider(&self) -> &P {
            &self.provider
        }
    }
    impl<T, P: ::core::clone::Clone, N> BN254Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> BN254Instance<T, P, N> {
            BN254Instance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network_transport: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > BN254Instance<T, P, N> {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<T, &P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > BN254Instance<T, P, N> {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<T, &P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
    }
}
///Module containing a contract's types and functions.
/**

```solidity
library EdOnBN254 {
    struct EdOnBN254Point { uint256 x; uint256 y; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod EdOnBN254 {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**```solidity
struct EdOnBN254Point { uint256 x; uint256 y; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EdOnBN254Point {
        #[allow(missing_docs)]
        pub x: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub y: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            alloy::sol_types::sol_data::Uint<256>,
            alloy::sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            alloy::sol_types::private::primitives::aliases::U256,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<EdOnBN254Point> for UnderlyingRustTuple<'_> {
            fn from(value: EdOnBN254Point) -> Self {
                (value.x, value.y)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for EdOnBN254Point {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { x: tuple.0, y: tuple.1 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for EdOnBN254Point {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for EdOnBN254Point {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.x),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.y),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(&tuple, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple = <UnderlyingRustTuple<
                    '_,
                > as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_packed_encoded_size(&tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for EdOnBN254Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <UnderlyingSolTuple<
                '_,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<
                    '_,
                > as alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolStruct for EdOnBN254Point {
            const NAME: &'static str = "EdOnBN254Point";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "EdOnBN254Point(uint256 x,uint256 y)",
                )
            }
            #[inline]
            fn eip712_components() -> alloy_sol_types::private::Vec<
                alloy_sol_types::private::Cow<'static, str>,
            > {
                alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> alloy_sol_types::private::Cow<'static, str> {
                <Self as alloy_sol_types::SolStruct>::eip712_root_type()
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.x)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.y)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for EdOnBN254Point {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.x)
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.y)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.x, out);
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.y, out);
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    rust,
                    &mut out,
                );
                alloy_sol_types::abi::token::WordToken(
                    alloy_sol_types::private::keccak256(out),
                )
            }
        }
    };
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`EdOnBN254`](self) contract instance.

See the [wrapper's documentation](`EdOnBN254Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> EdOnBN254Instance<T, P, N> {
        EdOnBN254Instance::<T, P, N>::new(address, provider)
    }
    /**A [`EdOnBN254`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`EdOnBN254`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct EdOnBN254Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for EdOnBN254Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("EdOnBN254Instance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > EdOnBN254Instance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`EdOnBN254`](self) contract instance.

See the [wrapper's documentation](`EdOnBN254Instance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            provider: P,
        ) -> Self {
            Self {
                address,
                provider,
                _network_transport: ::core::marker::PhantomData,
            }
        }
        /// Returns a reference to the address.
        #[inline]
        pub const fn address(&self) -> &alloy_sol_types::private::Address {
            &self.address
        }
        /// Sets the address.
        #[inline]
        pub fn set_address(&mut self, address: alloy_sol_types::private::Address) {
            self.address = address;
        }
        /// Sets the address and returns `self`.
        pub fn at(mut self, address: alloy_sol_types::private::Address) -> Self {
            self.set_address(address);
            self
        }
        /// Returns a reference to the provider.
        #[inline]
        pub const fn provider(&self) -> &P {
            &self.provider
        }
    }
    impl<T, P: ::core::clone::Clone, N> EdOnBN254Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> EdOnBN254Instance<T, P, N> {
            EdOnBN254Instance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network_transport: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > EdOnBN254Instance<T, P, N> {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<T, &P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > EdOnBN254Instance<T, P, N> {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<T, &P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
    }
}
///Module containing a contract's types and functions.
/**

```solidity
library StakeTable {
    type ValidatorStatus is uint8;
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod StakeTable {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorStatus(u8);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<ValidatorStatus> for u8 {
            #[inline]
            fn stv_to_tokens(
                &self,
            ) -> <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::Token<'_> {
                alloy_sol_types::private::SolTypeValue::<
                    alloy::sol_types::sol_data::Uint<8>,
                >::stv_to_tokens(self)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::tokenize(self)
                    .0
            }
            #[inline]
            fn stv_abi_encode_packed_to(
                &self,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(self, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::abi_encoded_size(self)
            }
        }
        #[automatically_derived]
        impl ValidatorStatus {
            /// The Solidity type name.
            pub const NAME: &'static str = stringify!(@ name);
            /// Convert from the underlying value type.
            #[inline]
            pub const fn from(value: u8) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into(self) -> u8 {
                self.0
            }
            /// Return the single encoding of this value, delegating to the
            /// underlying type.
            #[inline]
            pub fn abi_encode(&self) -> alloy_sol_types::private::Vec<u8> {
                <Self as alloy_sol_types::SolType>::abi_encode(&self.0)
            }
            /// Return the packed encoding of this value, delegating to the
            /// underlying type.
            #[inline]
            pub fn abi_encode_packed(&self) -> alloy_sol_types::private::Vec<u8> {
                <Self as alloy_sol_types::SolType>::abi_encode_packed(&self.0)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for ValidatorStatus {
            type RustType = u8;
            type Token<'a> = <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = Self::NAME;
            const ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                Self::type_check(token).is_ok()
            }
            #[inline]
            fn type_check(token: &Self::Token<'_>) -> alloy_sol_types::Result<()> {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::type_check(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::detokenize(token)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for ValidatorStatus {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::EventTopic>::topic_preimage_length(rust)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, out)
            }
            #[inline]
            fn encode_topic(
                rust: &Self::RustType,
            ) -> alloy_sol_types::abi::token::WordToken {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::EventTopic>::encode_topic(rust)
            }
        }
    };
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`StakeTable`](self) contract instance.

See the [wrapper's documentation](`StakeTableInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> StakeTableInstance<T, P, N> {
        StakeTableInstance::<T, P, N>::new(address, provider)
    }
    /**A [`StakeTable`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`StakeTable`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct StakeTableInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for StakeTableInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("StakeTableInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`StakeTable`](self) contract instance.

See the [wrapper's documentation](`StakeTableInstance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            provider: P,
        ) -> Self {
            Self {
                address,
                provider,
                _network_transport: ::core::marker::PhantomData,
            }
        }
        /// Returns a reference to the address.
        #[inline]
        pub const fn address(&self) -> &alloy_sol_types::private::Address {
            &self.address
        }
        /// Sets the address.
        #[inline]
        pub fn set_address(&mut self, address: alloy_sol_types::private::Address) {
            self.address = address;
        }
        /// Sets the address and returns `self`.
        pub fn at(mut self, address: alloy_sol_types::private::Address) -> Self {
            self.set_address(address);
            self
        }
        /// Returns a reference to the provider.
        #[inline]
        pub const fn provider(&self) -> &P {
            &self.provider
        }
    }
    impl<T, P: ::core::clone::Clone, N> StakeTableInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> StakeTableInstance<T, P, N> {
            StakeTableInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network_transport: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableInstance<T, P, N> {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<T, &P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableInstance<T, P, N> {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<T, &P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
    }
}
/**

Generated by the following Solidity interface...
```solidity
library BN254 {
    type BaseField is uint256;
    struct G1Point {
        BaseField x;
        BaseField y;
    }
    struct G2Point {
        BaseField x0;
        BaseField x1;
        BaseField y0;
        BaseField y1;
    }
}

library EdOnBN254 {
    struct EdOnBN254Point {
        uint256 x;
        uint256 y;
    }
}

library StakeTable {
    type ValidatorStatus is uint8;
}

interface StakeTableV2 {
    error AccessControlBadConfirmation();
    error AccessControlUnauthorizedAccount(address account, bytes32 neededRole);
    error AddressEmptyCode(address target);
    error BLSSigVerificationFailed();
    error BlsKeyAlreadyUsed();
    error DeprecatedFunction();
    error ERC1967InvalidImplementation(address implementation);
    error ERC1967NonPayable();
    error EnforcedPause();
    error ExitEscrowPeriodInvalid();
    error ExpectedPause();
    error FailedInnerCall();
    error InsufficientAllowance(uint256, uint256);
    error InsufficientBalance(uint256);
    error InvalidCommission();
    error InvalidInitialization();
    error InvalidSchnorrSig();
    error InvalidSchnorrVK();
    error NotInitializing();
    error NothingToWithdraw();
    error OwnableInvalidOwner(address owner);
    error OwnableUnauthorizedAccount(address account);
    error PrematureWithdrawal();
    error SchnorrKeyAlreadyUsed();
    error UUPSUnauthorizedCallContext();
    error UUPSUnsupportedProxiableUUID(bytes32 slot);
    error UndelegationAlreadyExists();
    error ValidatorAlreadyExited();
    error ValidatorAlreadyRegistered();
    error ValidatorInactive();
    error ValidatorNotExited();
    error ZeroAddress();
    error ZeroAmount();

    event ConsensusKeysUpdated(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
    event ConsensusKeysUpdatedV2(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK, BN254.G1Point blsSig, bytes schnorrSig);
    event Delegated(address indexed delegator, address indexed validator, uint256 amount);
    event ExitEscrowPeriodUpdated(uint64 newExitEscrowPeriod);
    event Initialized(uint64 version);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event Paused(address account);
    event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole);
    event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
    event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
    event Undelegated(address indexed delegator, address indexed validator, uint256 amount);
    event Unpaused(address account);
    event Upgrade(address implementation);
    event Upgraded(address indexed implementation);
    event ValidatorExit(address indexed validator);
    event ValidatorRegistered(address indexed account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
    event ValidatorRegisteredV2(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK, uint16 commission, BN254.G1Point blsSig, bytes schnorrSig);
    event Withdrawal(address indexed account, uint256 amount);

    constructor();

    function DEFAULT_ADMIN_ROLE() external view returns (bytes32);
    function PAUSER_ROLE() external view returns (bytes32);
    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    function _hashBlsKey(BN254.G2Point memory blsVK) external pure returns (bytes32);
    function blsKeys(bytes32 blsKeyHash) external view returns (bool used);
    function claimValidatorExit(address validator) external;
    function claimWithdrawal(address validator) external;
    function delegate(address validator, uint256 amount) external;
    function delegations(address validator, address delegator) external view returns (uint256 amount);
    function deregisterValidator() external;
    function exitEscrowPeriod() external view returns (uint256);
    function getRoleAdmin(bytes32 role) external view returns (bytes32);
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    function grantRole(bytes32 role, address account) external;
    function hasRole(bytes32 role, address account) external view returns (bool);
    function initialize(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod, address _timelock) external;
    function initializeV2(address pauser, address admin) external;
    function initializedAtBlock() external view returns (uint256);
    function lightClient() external view returns (address);
    function owner() external view returns (address);
    function pause() external;
    function paused() external view returns (bool);
    function proxiableUUID() external view returns (bytes32);
    function registerValidator(BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory, uint16) external pure;
    function registerValidatorV2(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, bytes memory schnorrSig, uint16 commission) external;
    function renounceOwnership() external;
    function renounceRole(bytes32 role, address callerConfirmation) external;
    function revokeRole(bytes32 role, address account) external;
    function schnorrKeys(bytes32 schnorrKey) external view returns (bool used);
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
    function token() external view returns (address);
    function transferOwnership(address newOwner) external;
    function undelegate(address validator, uint256 amount) external;
    function undelegations(address validator, address delegator) external view returns (uint256 amount, uint256 unlocksAt);
    function unpause() external;
    function updateConsensusKeys(BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory) external pure;
    function updateConsensusKeysV2(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, bytes memory schnorrSig) external;
    function updateExitEscrowPeriod(uint64 newExitEscrowPeriod) external;
    function upgradeToAndCall(address newImplementation, bytes memory data) external payable;
    function validatorExits(address validator) external view returns (uint256 unlocksAt);
    function validators(address account) external view returns (uint256 delegatedAmount, StakeTable.ValidatorStatus status);
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "constructor",
    "inputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "DEFAULT_ADMIN_ROLE",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "PAUSER_ROLE",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "UPGRADE_INTERFACE_VERSION",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "string",
        "internalType": "string"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "_hashBlsKey",
    "inputs": [
      {
        "name": "blsVK",
        "type": "tuple",
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "blsKeys",
    "inputs": [
      {
        "name": "blsKeyHash",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "used",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "claimValidatorExit",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "claimWithdrawal",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "delegate",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "delegations",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "delegator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "amount",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "deregisterValidator",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "exitEscrowPeriod",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getRoleAdmin",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getVersion",
    "inputs": [],
    "outputs": [
      {
        "name": "majorVersion",
        "type": "uint8",
        "internalType": "uint8"
      },
      {
        "name": "minorVersion",
        "type": "uint8",
        "internalType": "uint8"
      },
      {
        "name": "patchVersion",
        "type": "uint8",
        "internalType": "uint8"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "grantRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "hasRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "initialize",
    "inputs": [
      {
        "name": "_tokenAddress",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "_lightClientAddress",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "_exitEscrowPeriod",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "_timelock",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "initializeV2",
    "inputs": [
      {
        "name": "pauser",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "admin",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "initializedAtBlock",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "lightClient",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "contract ILightClient"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "owner",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "address"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "pause",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "paused",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "proxiableUUID",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "registerValidator",
    "inputs": [
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "",
        "type": "uint16",
        "internalType": "uint16"
      }
    ],
    "outputs": [],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "registerValidatorV2",
    "inputs": [
      {
        "name": "blsVK",
        "type": "tuple",
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVK",
        "type": "tuple",
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "blsSig",
        "type": "tuple",
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrSig",
        "type": "bytes",
        "internalType": "bytes"
      },
      {
        "name": "commission",
        "type": "uint16",
        "internalType": "uint16"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "renounceOwnership",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "renounceRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "callerConfirmation",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "revokeRole",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "schnorrKeys",
    "inputs": [
      {
        "name": "schnorrKey",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [
      {
        "name": "used",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "supportsInterface",
    "inputs": [
      {
        "name": "interfaceId",
        "type": "bytes4",
        "internalType": "bytes4"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "token",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "address",
        "internalType": "contract ERC20"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "transferOwnership",
    "inputs": [
      {
        "name": "newOwner",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "undelegate",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "undelegations",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "delegator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "amount",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "unlocksAt",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "unpause",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "updateConsensusKeys",
    "inputs": [
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      }
    ],
    "outputs": [],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "updateConsensusKeysV2",
    "inputs": [
      {
        "name": "blsVK",
        "type": "tuple",
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVK",
        "type": "tuple",
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "blsSig",
        "type": "tuple",
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrSig",
        "type": "bytes",
        "internalType": "bytes"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "updateExitEscrowPeriod",
    "inputs": [
      {
        "name": "newExitEscrowPeriod",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "upgradeToAndCall",
    "inputs": [
      {
        "name": "newImplementation",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "data",
        "type": "bytes",
        "internalType": "bytes"
      }
    ],
    "outputs": [],
    "stateMutability": "payable"
  },
  {
    "type": "function",
    "name": "validatorExits",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "unlocksAt",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "validators",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "delegatedAmount",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "status",
        "type": "uint8",
        "internalType": "enum StakeTable.ValidatorStatus"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "event",
    "name": "ConsensusKeysUpdated",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "blsVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "ConsensusKeysUpdatedV2",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "blsVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "blsSig",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrSig",
        "type": "bytes",
        "indexed": false,
        "internalType": "bytes"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Delegated",
    "inputs": [
      {
        "name": "delegator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "validator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "ExitEscrowPeriodUpdated",
    "inputs": [
      {
        "name": "newExitEscrowPeriod",
        "type": "uint64",
        "indexed": false,
        "internalType": "uint64"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Initialized",
    "inputs": [
      {
        "name": "version",
        "type": "uint64",
        "indexed": false,
        "internalType": "uint64"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "OwnershipTransferred",
    "inputs": [
      {
        "name": "previousOwner",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "newOwner",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Paused",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": false,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleAdminChanged",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "previousAdminRole",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "newAdminRole",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleGranted",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "sender",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "RoleRevoked",
    "inputs": [
      {
        "name": "role",
        "type": "bytes32",
        "indexed": true,
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "sender",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Undelegated",
    "inputs": [
      {
        "name": "delegator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "validator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Unpaused",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": false,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Upgrade",
    "inputs": [
      {
        "name": "implementation",
        "type": "address",
        "indexed": false,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Upgraded",
    "inputs": [
      {
        "name": "implementation",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "ValidatorExit",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "ValidatorRegistered",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "blsVk",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVk",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "commission",
        "type": "uint16",
        "indexed": false,
        "internalType": "uint16"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "ValidatorRegisteredV2",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "blsVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G2Point",
        "components": [
          {
            "name": "x0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "x1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y0",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y1",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrVK",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct EdOnBN254.EdOnBN254Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "uint256"
          }
        ]
      },
      {
        "name": "commission",
        "type": "uint16",
        "indexed": false,
        "internalType": "uint16"
      },
      {
        "name": "blsSig",
        "type": "tuple",
        "indexed": false,
        "internalType": "struct BN254.G1Point",
        "components": [
          {
            "name": "x",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          },
          {
            "name": "y",
            "type": "uint256",
            "internalType": "BN254.BaseField"
          }
        ]
      },
      {
        "name": "schnorrSig",
        "type": "bytes",
        "indexed": false,
        "internalType": "bytes"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "Withdrawal",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "error",
    "name": "AccessControlBadConfirmation",
    "inputs": []
  },
  {
    "type": "error",
    "name": "AccessControlUnauthorizedAccount",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "neededRole",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
  },
  {
    "type": "error",
    "name": "AddressEmptyCode",
    "inputs": [
      {
        "name": "target",
        "type": "address",
        "internalType": "address"
      }
    ]
  },
  {
    "type": "error",
    "name": "BLSSigVerificationFailed",
    "inputs": []
  },
  {
    "type": "error",
    "name": "BlsKeyAlreadyUsed",
    "inputs": []
  },
  {
    "type": "error",
    "name": "DeprecatedFunction",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ERC1967InvalidImplementation",
    "inputs": [
      {
        "name": "implementation",
        "type": "address",
        "internalType": "address"
      }
    ]
  },
  {
    "type": "error",
    "name": "ERC1967NonPayable",
    "inputs": []
  },
  {
    "type": "error",
    "name": "EnforcedPause",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ExitEscrowPeriodInvalid",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ExpectedPause",
    "inputs": []
  },
  {
    "type": "error",
    "name": "FailedInnerCall",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InsufficientAllowance",
    "inputs": [
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "InsufficientBalance",
    "inputs": [
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "InvalidCommission",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidInitialization",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidSchnorrSig",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidSchnorrVK",
    "inputs": []
  },
  {
    "type": "error",
    "name": "NotInitializing",
    "inputs": []
  },
  {
    "type": "error",
    "name": "NothingToWithdraw",
    "inputs": []
  },
  {
    "type": "error",
    "name": "OwnableInvalidOwner",
    "inputs": [
      {
        "name": "owner",
        "type": "address",
        "internalType": "address"
      }
    ]
  },
  {
    "type": "error",
    "name": "OwnableUnauthorizedAccount",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ]
  },
  {
    "type": "error",
    "name": "PrematureWithdrawal",
    "inputs": []
  },
  {
    "type": "error",
    "name": "SchnorrKeyAlreadyUsed",
    "inputs": []
  },
  {
    "type": "error",
    "name": "UUPSUnauthorizedCallContext",
    "inputs": []
  },
  {
    "type": "error",
    "name": "UUPSUnsupportedProxiableUUID",
    "inputs": [
      {
        "name": "slot",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
  },
  {
    "type": "error",
    "name": "UndelegationAlreadyExists",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ValidatorAlreadyExited",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ValidatorAlreadyRegistered",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ValidatorInactive",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ValidatorNotExited",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ZeroAddress",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ZeroAmount",
    "inputs": []
  }
]
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod StakeTableV2 {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x60a060405230608052348015610013575f5ffd5b5061001c610031565b610024610031565b61002c610031565b6100e3565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff16156100815760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b03908116146100e05780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b6080516136d56101095f395f818161188e015281816118b70152611a2f01526136d55ff3fe60806040526004361061023e575f3560e01c8063715018a611610134578063ad3cb1cc116100b3578063c64814dd11610078578063c64814dd146106f7578063d547741f1461072d578063e63ab1e91461074c578063f2fde38b1461076c578063fa52c7d81461078b578063fc0c546a146107ce575f5ffd5b8063ad3cb1cc14610623578063b3e6ebd514610660578063b5700e681461068e578063b5ecb344146106ad578063be203094146106d8575f5ffd5b80639e9a8f31116100f95780639e9a8f311461056b578063a217fddf14610580578063a2d78dd514610593578063a3066aab146105e5578063ac5c2ad014610604575f5ffd5b8063715018a6146104b55780638456cb59146104c95780638da5cb5b146104dd57806391d148541461052d5780639b30a5e61461054c575f5ffd5b806336568abe116101c057806352d1902d1161018557806352d1902d146104225780635544c2f1146104365780635c975abb146104505780635f8754a6146104735780636a911ccf146104a1575f5ffd5b806336568abe146103a95780633e9df9b5146103c85780633f4ba83a146103dc5780634d99dd16146103f05780634f1ef2861461040f575f5ffd5b80631a20cd63116102065780631a20cd63146103005780632140fecd1461031f578063248a9ca31461033e5780632f2ff15d1461036b578063304240bf1461038a575f5ffd5b806301ffc9a714610242578063026e402b146102765780630840ba72146102975780630d8e6e2c146102b657806313b9057a146102e1575b5f5ffd5b34801561024d575f5ffd5b5061026161025c366004612cf3565b6107ed565b60405190151581526020015b60405180910390f35b348015610281575f5ffd5b50610295610290366004612d35565b610823565b005b3480156102a2575f5ffd5b506102956102b1366004612d5d565b610839565b3480156102c1575f5ffd5b5060408051600281525f602082018190529181019190915260600161026d565b3480156102ec575f5ffd5b506102956102fb366004612e6a565b610926565b34801561030b575f5ffd5b5061029561031a366004612ed4565b61093f565b34801561032a575f5ffd5b50610295610339366004612eef565b610a53565b348015610349575f5ffd5b5061035d610358366004612f08565b610a67565b60405190815260200161026d565b348015610376575f5ffd5b50610295610385366004612f1f565b610a87565b348015610395575f5ffd5b506102956103a4366004612fc9565b610aa9565b3480156103b4575f5ffd5b506102956103c3366004612f1f565b610c6f565b3480156103d3575f5ffd5b5061035d5f5481565b3480156103e7575f5ffd5b50610295610ca7565b3480156103fb575f5ffd5b5061029561040a366004612d35565b610cc6565b61029561041d366004613049565b610cd8565b34801561042d575f5ffd5b5061035d610cf3565b348015610441575f5ffd5b506102956102fb366004613093565b34801561045b575f5ffd5b505f5160206136895f395f51905f525460ff16610261565b34801561047e575f5ffd5b5061026161048d366004612f08565b60096020525f908152604090205460ff1681565b3480156104ac575f5ffd5b50610295610d0e565b3480156104c0575f5ffd5b50610295610d20565b3480156104d4575f5ffd5b50610295610d31565b3480156104e8575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03165b6040516001600160a01b03909116815260200161026d565b348015610538575f5ffd5b50610261610547366004612f1f565b610d50565b348015610557575f5ffd5b5061035d6105663660046130d7565b610d86565b348015610576575f5ffd5b5061035d60085481565b34801561058b575f5ffd5b5061035d5f81565b34801561059e575f5ffd5b506105d06105ad366004612d5d565b600760209081525f92835260408084209091529082529020805460019091015482565b6040805192835260208301919091520161026d565b3480156105f0575f5ffd5b506102956105ff366004612eef565b610de0565b34801561060f575f5ffd5b5061029561061e3660046130f1565b610df1565b34801561062e575f5ffd5b50610653604051806040016040528060058152602001640352e302e360dc1b81525081565b60405161026d919061318e565b34801561066b575f5ffd5b5061026161067a366004612f08565b60046020525f908152604090205460ff1681565b348015610699575f5ffd5b50600154610515906001600160a01b031681565b3480156106b8575f5ffd5b5061035d6106c7366004612eef565b60056020525f908152604090205481565b3480156106e3575f5ffd5b506102956106f23660046131a0565b610ec8565b348015610702575f5ffd5b5061035d610711366004612d5d565b600660209081525f928352604080842090915290825290205481565b348015610738575f5ffd5b50610295610747366004612f1f565b610fdf565b348015610757575f5ffd5b5061035d5f5160206136495f395f51905f5281565b348015610777575f5ffd5b50610295610786366004612eef565b610ffb565b348015610796575f5ffd5b506107c06107a5366004612eef565b60036020525f90815260409020805460019091015460ff1682565b60405161026d9291906131f3565b3480156107d9575f5ffd5b50600254610515906001600160a01b031681565b5f6001600160e01b03198216637965db0b60e01b148061081d57506301ffc9a760e01b6001600160e01b03198316145b92915050565b61082b61103a565b610835828261106a565b5050565b5f5160206136a95f395f51905f52805460029190600160401b900460ff168061086f575080546001600160401b03808416911610155b1561088d5760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b03831617600160401b1781556108b6611208565b6108c05f84611210565b506108d85f5160206136495f395f51905f5285611210565b50805460ff60401b191681556040516001600160401b03831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b60405163c2d7f81360e01b815260040160405180910390fd5b6109476112b8565b60015460408051633c1a081560e21b815290515f926001600160a01b03169163f06820549160048083019260209291908290030181865afa15801561098e573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906109b29190613223565b6109bd90600f613252565b9050621275006001600160401b0380831690841610806109ee5750806001600160401b0316836001600160401b0316115b15610a0c5760405163b57e21df60e01b815260040160405180910390fd5b6001600160401b03831660088190556040519081527f793e3b1e1bcd677bb11900c83124d3c44c9946ea8ddf978a0ca250b034ec9dde9060200160405180910390a1505050565b610a5b61103a565b610a6481611313565b50565b5f9081525f5160206136695f395f51905f52602052604090206001015490565b610a9082610a67565b610a9981611434565b610aa38383611210565b50505050565b610ab161103a565b33610abb8161143e565b610ac48561148b565b610ace86866114c6565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610b0081868961153f565b8351604014610b2257604051634247068760e01b815260040160405180910390fd5b6127108361ffff161115610b495760405163dc81db8560e01b815260040160405180910390fd5b600160045f610b578a610d86565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550600160095f610b88896115d4565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f815260200160016002811115610bcf57610bcf6131df565b90526001600160a01b0383165f908152600360209081526040909120825181559082015160018083018054909160ff1990911690836002811115610c1557610c156131df565b0217905550905050816001600160a01b03167ff057d4ea81e98628653a8f90788541972078137334a92b42347eaccda80fc40a8888868989604051610c5e959493929190613274565b60405180910390a250505050505050565b6001600160a01b0381163314610c985760405163334bd91960e11b815260040160405180910390fd5b610ca282826115f8565b505050565b5f5160206136495f395f51905f52610cbe81611434565b610a64611671565b610cce61103a565b61083582826116d1565b610ce0611883565b610ce982611927565b6108358282611968565b5f610cfc611a24565b505f5160206136295f395f51905f5290565b610d1661103a565b610d1e611a6d565b565b610d286112b8565b610d1e5f611afb565b5f5160206136495f395f51905f52610d4881611434565b610a64611b6b565b5f9182525f5160206136695f395f51905f52602090815260408084206001600160a01b0393909316845291905290205460ff1690565b5f815f0151826020015183604001518460600151604051602001610dc3949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b610de861103a565b610a6481611bb3565b610df961103a565b33610e0381611cc3565b610e0c8461148b565b610e1685856114c6565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610e4881858861153f565b600160045f610e5689610d86565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550816001600160a01b03167fc8c5b37aec7f2ddbd3a13c51361e54a0a8df3bca256ab758a77f5ad741d281e587878787604051610eb894939291906132e6565b60405180910390a2505050505050565b5f5160206136a95f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015610ef95750825b90505f826001600160401b03166001148015610f145750303b155b905081158015610f22575080155b15610f405760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610f6a57845460ff60401b1916600160401b1785555b610f7386611d44565b610f7b611208565b610f83611d55565b610f8e898989611e47565b8315610fd457845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b610fe882610a67565b610ff181611434565b610aa383836115f8565b6110036112b8565b6001600160a01b03811661103157604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610a6481611afb565b5f5160206136895f395f51905f525460ff1615610d1e5760405163d93c066560e01b815260040160405180910390fd5b61107382611cc3565b335f82900361109557604051631f2a200560e01b815260040160405180910390fd5b600254604051636eb1769f60e11b81526001600160a01b0383811660048301523060248301525f92169063dd62ed3e90604401602060405180830381865afa1580156110e3573d5f5f3e3d5ffd5b505050506040513d601f19601f82011682018060405250810190611107919061334c565b9050828110156111345760405163054365bb60e31b81526004810182905260248101849052604401611028565b60025461114c906001600160a01b0316833086611ef0565b6001600160a01b0384165f9081526003602052604081208054859290611173908490613363565b90915550506001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906111ae908490613363565b92505081905550836001600160a01b0316826001600160a01b03167fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b856040516111fa91815260200190565b60405180910390a350505050565b610d1e611f8d565b5f5f5160206136695f395f51905f526112298484610d50565b6112a8575f848152602082815260408083206001600160a01b03871684529091529020805460ff1916600117905561125e3390565b6001600160a01b0316836001600160a01b0316857f2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d60405160405180910390a4600191505061081d565b5f91505061081d565b5092915050565b336112ea7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610d1e5760405163118cdaa760e01b8152336004820152602401611028565b6001600160a01b0381165f90815260056020526040812054339181900361134d576040516379298a5360e11b815260040160405180910390fd5b8042101561136e57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f908152600660209081526040808320938616835292905290812054908190036113b657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038085165f90815260066020908152604080832087851684529091528120556002546113eb91168483611fc3565b826001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b658260405161142691815260200190565b60405180910390a250505050565b610a64813361204c565b6001600160a01b0381165f9081526003602052604081206001015460ff16600281111561146d5761146d6131df565b14610a645760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f80825260208201526114a88282612085565b15610835576040516306cf438f60e01b815260040160405180910390fd5b60045f6114d284610d86565b815260208101919091526040015f205460ff16156115025760405162da8a5760e11b815260040160405180910390fd5b60095f61150e836115d4565b815260208101919091526040015f205460ff16156108355760405163ae493b0360e01b815260040160405180910390fd5b611548826120a6565b5f6040518060600160405280602481526020016135e56024913990505f848260405160200161157892919061338d565b60405160208183030381529060405290505f6115938261213c565b90506115b081856115a388612229565b6115ab6122a0565b61236d565b6115cc5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f815f01518260200151604051602001610dc3929190918252602082015260400190565b5f5f5160206136695f395f51905f526116118484610d50565b156112a8575f848152602082815260408083206001600160a01b0387168085529252808320805460ff1916905551339287917ff6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b9190a4600191505061081d565b61167961244b565b5f5160206136895f395f51905f52805460ff191681557f5db9ee0a495bf2e6ff9c91a7834c1ba4fdd244a5e8aa4e537bd38aeae4b073aa335b6040516001600160a01b0390911681526020015b60405180910390a150565b6116da82611cc3565b335f8290036116fc57604051631f2a200560e01b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320938516835292905220541561173f5760405163d423a4f160e01b815260040160405180910390fd5b6001600160a01b038084165f908152600660209081526040808320938516835292905220548281101561178857604051639266535160e01b815260048101829052602401611028565b6001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906117be9084906133a9565b925050819055506040518060400160405280848152602001600854426117e49190613363565b90526001600160a01b038086165f818152600760209081526040808320948816835293815283822085518155948101516001909501949094559081526003909252812080548592906118379084906133a9565b92505081905550836001600160a01b0316826001600160a01b03167f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c856040516111fa91815260200190565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061190957507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b03166118fd5f5160206136295f395f51905f52546001600160a01b031690565b6001600160a01b031614155b15610d1e5760405163703e46dd60e11b815260040160405180910390fd5b61192f6112b8565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d906020016116c6565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa9250505080156119c2575060408051601f3d908101601f191682019092526119bf9181019061334c565b60015b6119ea57604051634c9c8ce360e01b81526001600160a01b0383166004820152602401611028565b5f5160206136295f395f51905f528114611a1a57604051632a87526960e21b815260048101829052602401611028565b610ca2838361247a565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610d1e5760405163703e46dd60e11b815260040160405180910390fd5b33611a7781611cc3565b6001600160a01b0381165f908152600360205260409020600101805460ff19166002179055600854611aa99042613363565b6001600160a01b0382165f8181526005602090815260408083209490945560039052828120819055915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a250565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611b7361103a565b5f5160206136895f395f51905f52805460ff191660011781557f62e78cea01bee320cd4e420270b5ea74000d11b0c9f74754ebdbfc544b05a258336116b2565b6001600160a01b0381165f9081526007602090815260408083203380855292528220549091819003611bf857604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f90815260076020908152604080832093861683529290522060010154421015611c4057604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f9081526007602090815260408083208685168452909152812081815560010155600254611c7b91168383611fc3565b816001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b6582604051611cb691815260200190565b60405180910390a2505050565b6001600160a01b0381165f9081526003602052604081206001015460ff1690816002811115611cf457611cf46131df565b03611d125760405163508a793f60e01b815260040160405180910390fd5b6002816002811115611d2657611d266131df565b036108355760405163eab4a96360e01b815260040160405180910390fd5b611d4c611f8d565b610a64816124cf565b5f5160206136a95f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015611d865750825b90505f826001600160401b03166001148015611da15750303b155b905081158015611daf575080155b15611dcd5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315611df757845460ff60401b1916600160401b1785555b435f558315611e4057845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b5050505050565b6001600160a01b038316611e6e5760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b038216611e955760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b038086166001600160a01b0319928316179092556001805492851692909116919091179055605a80821015611ee85760405163b57e21df60e01b815260040160405180910390fd5b506008555050565b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f511416151615611f495750833b153d17155b80611e405760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401611028565b5f5160206136a95f395f51905f5254600160401b900460ff16610d1e57604051631afcd79f60e31b815260040160405180910390fd5b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f51141615161561200d5750823b153d17155b80610aa35760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401611028565b6120568282610d50565b6108355760405163e2517d3f60e01b81526001600160a01b038216600482015260248101839052604401611028565b805182515f9114801561209f575081602001518360200151145b9392505050565b805160208201515f915f5160206136095f395f51905f529115901516156120cc57505050565b825160208401518260038485858609850908838283091483821084841016169350505081610ca25760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401611028565b604080518082019091525f80825260208201525f612159836124d7565b90505f5160206136095f395f51905f5260035f8284850990508280612180576121806133bc565b84820990508280612193576121936133bc565b82820890505f5f6121a3836126df565b925090505b8061220c5784806121bb576121bb6133bc565b60018708955084806121cf576121cf6133bc565b868709925084806121e2576121e26133bc565b868409925084806121f5576121f56133bc565b8484089250612203836126df565b925090506121a8565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f8082526020820152815160208301511590151615612250575090565b6040518060400160405280835f015181526020015f5160206136095f395f51905f52846020015161228191906133d0565b612298905f5160206136095f395f51905f526133a9565b905292915050565b6122c760405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f5f5f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f5191508061243f5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401611028565b50151595945050505050565b5f5160206136895f395f51905f525460ff16610d1e57604051638dfc202b60e01b815260040160405180910390fd5b612483826127d6565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a28051156124c757610ca28282612839565b6108356128ab565b611003611f8d565b5f5f6124e2836128ca565b8051909150603081146124f7576124f76133ef565b5f816001600160401b0381111561251057612510612d8e565b6040519080825280601f01601f19166020018201604052801561253a576020820181803683370190505b5090505f5b828110156125a95783600161255483866133a9565b61255e91906133a9565b8151811061256e5761256e613403565b602001015160f81c60f81b82828151811061258b5761258b613403565b60200101906001600160f81b03191690815f1a90535060010161253f565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b828110156126395783816125e585886133a9565b6125ef9190613363565b815181106125ff576125ff613403565b602001015160f81c60f81b60f81c82828151811061261f5761261f613403565b60ff909216602092830291909101909101526001016125d1565b505f61264482612c15565b90506101005f5160206136095f395f51905f525f61266286896133a9565b90505f5b818110156126cf575f88600161267c84866133a9565b61268691906133a9565b8151811061269657612696613403565b016020015160f81c905083806126ae576126ae6133bc565b858709955083806126c1576126c16133bc565b818708955050600101612666565b50929a9950505050505050505050565b5f5f5f5f5f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f5160206136095f395f51905f52905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f5192508361279c5760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401611028565b80600184901b11156127b5576127b283826133a9565b92505b80806127c3576127c36133bc565b8384099690961496919550909350505050565b806001600160a01b03163b5f0361280b57604051634c9c8ce360e01b81526001600160a01b0382166004820152602401611028565b5f5160206136295f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b0316846040516128559190613417565b5f60405180830381855af49150503d805f811461288d576040519150601f19603f3d011682016040523d82523d5f602084013e612892565b606091505b50915091506128a2858383612c75565b95945050505050565b3415610d1e5760405163b398979f60e01b815260040160405180910390fd5b604080516030808252606082810190935290602090600160f91b905f9084602082018180368337019050509050808660405160200161290a92919061338d565b6040516020818303038152906040529050808460f81b604051602001612931929190613422565b604051602081830303815290604052905080604051602001612953919061344c565b60408051601f1981840301815290829052915061010160f01b9061297d9083908390602001613464565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff88166001600160401b038111156129ec576129ec612d8e565b6040519080825280601f01601f191660200182016040528015612a16576020820181803683370190505b5090505f82604051602001612a2d91815260200190565b60408051601f1981840301815291905290505f5b8151811015612a9757818181518110612a5c57612a5c613403565b602001015160f81c60f81b838281518110612a7957612a79613403565b60200101906001600160f81b03191690815f1a905350600101612a41565b505f84604051602001612aac91815260200190565b60408051601f19818403018152602083019091525f80835291985091505b89811015612b3e575f838281518110612ae557612ae5613403565b602001015160f81c60f81b838381518110612b0257612b02613403565b602001015160f81c60f81b1890508881604051602001612b23929190613488565b60408051601f19818403018152919052985050600101612aca565b50868887604051602001612b54939291906134ac565b60405160208183030381529060405296508680519060200120935083604051602001612b8291815260200190565b60408051601f1981840301815291905291505f5b612ba38a60ff8d166133a9565b811015612c0457828181518110612bbc57612bbc613403565b01602001516001600160f81b03191684612bd6838d613363565b81518110612be657612be6613403565b60200101906001600160f81b03191690815f1a905350600101612b96565b50919b9a5050505050505050505050565b5f80805b83518110156112b157838181518110612c3457612c34613403565b602002602001015160ff16816008612c4c91906134df565b612c579060026135d9565b612c6191906134df565b612c6b9083613363565b9150600101612c19565b606082612c8a57612c8582612cca565b61209f565b8151158015612ca157506001600160a01b0384163b155b156112b157604051639996b31560e01b81526001600160a01b0385166004820152602401611028565b805115612cda5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b5f60208284031215612d03575f5ffd5b81356001600160e01b03198116811461209f575f5ffd5b80356001600160a01b0381168114612d30575f5ffd5b919050565b5f5f60408385031215612d46575f5ffd5b612d4f83612d1a565b946020939093013593505050565b5f5f60408385031215612d6e575f5ffd5b612d7783612d1a565b9150612d8560208401612d1a565b90509250929050565b634e487b7160e01b5f52604160045260245ffd5b604080519081016001600160401b0381118282101715612dc457612dc4612d8e565b60405290565b5f60808284031215612dda575f5ffd5b604051608081016001600160401b0381118282101715612dfc57612dfc612d8e565b6040908152833582526020808501359083015283810135908201526060928301359281019290925250919050565b5f60408284031215612e3a575f5ffd5b612e42612da2565b823581526020928301359281019290925250919050565b803561ffff81168114612d30575f5ffd5b5f5f5f5f6101208587031215612e7e575f5ffd5b612e888686612dca565b9350612e978660808701612e2a565b9250612ea68660c08701612e2a565b9150612eb56101008601612e59565b905092959194509250565b6001600160401b0381168114610a64575f5ffd5b5f60208284031215612ee4575f5ffd5b813561209f81612ec0565b5f60208284031215612eff575f5ffd5b61209f82612d1a565b5f60208284031215612f18575f5ffd5b5035919050565b5f5f60408385031215612f30575f5ffd5b82359150612d8560208401612d1a565b5f82601f830112612f4f575f5ffd5b81356001600160401b03811115612f6857612f68612d8e565b604051601f8201601f19908116603f011681016001600160401b0381118282101715612f9657612f96612d8e565b604052818152838201602001851015612fad575f5ffd5b816020850160208301375f918101602001919091529392505050565b5f5f5f5f5f6101408688031215612fde575f5ffd5b612fe88787612dca565b9450612ff78760808801612e2a565b93506130068760c08801612e2a565b92506101008601356001600160401b03811115613021575f5ffd5b61302d88828901612f40565b92505061303d6101208701612e59565b90509295509295909350565b5f5f6040838503121561305a575f5ffd5b61306383612d1a565b915060208301356001600160401b0381111561307d575f5ffd5b61308985828601612f40565b9150509250929050565b5f5f5f61010084860312156130a6575f5ffd5b6130b08585612dca565b92506130bf8560808601612e2a565b91506130ce8560c08601612e2a565b90509250925092565b5f608082840312156130e7575f5ffd5b61209f8383612dca565b5f5f5f5f6101208587031215613105575f5ffd5b61310f8686612dca565b935061311e8660808701612e2a565b925061312d8660c08701612e2a565b91506101008501356001600160401b03811115613148575f5ffd5b61315487828801612f40565b91505092959194509250565b5f81518084528060208401602086015e5f602082860101526020601f19601f83011685010191505092915050565b602081525f61209f6020830184613160565b5f5f5f5f608085870312156131b3575f5ffd5b6131bc85612d1a565b93506131ca60208601612d1a565b925060408501359150612eb560608601612d1a565b634e487b7160e01b5f52602160045260245ffd5b828152604081016003831061321657634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b5f60208284031215613233575f5ffd5b815161209f81612ec0565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b0381811683821602908116908181146112b1576112b161323e565b8551815260208087015190820152604080870151908201526060808701519082015284516080820152602085015160a082015261ffff841660c0820152825160e082015260208301516101008201526101406101208201525f6132db610140830184613160565b979650505050505050565b8451815260208086015190820152604080860151908201526060808601519082015283516080820152602084015160a0820152825160c0820152602083015160e08201526101206101008201525f613342610120830184613160565b9695505050505050565b5f6020828403121561335c575f5ffd5b5051919050565b8082018082111561081d5761081d61323e565b5f81518060208401855e5f93019283525090919050565b5f6133a161339b8386613376565b84613376565b949350505050565b8181038181111561081d5761081d61323e565b634e487b7160e01b5f52601260045260245ffd5b5f826133ea57634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f61209f8284613376565b5f61342d8285613376565b5f81526001600160f81b03199390931660018401525050600201919050565b5f6134578284613376565b5f81526001019392505050565b5f61346f8285613376565b6001600160f01b03199390931683525050600201919050565b5f6134938285613376565b6001600160f81b03199390931683525050600101919050565b5f6134b78286613376565b6001600160f81b031994909416845250506001600160f01b0319166001820152600301919050565b808202811582820484141761081d5761081d61323e565b6001815b6001841115613531578085048111156135155761351561323e565b600184161561352357908102905b60019390931c9280026134fa565b935093915050565b5f826135475750600161081d565b8161355357505f61081d565b816001811461356957600281146135735761358f565b600191505061081d565b60ff8411156135845761358461323e565b50506001821b61081d565b5060208310610133831016604e8410600b84101617156135b2575081810a61081d565b6135be5f1984846134f6565b805f19048211156135d1576135d161323e565b029392505050565b5f61209f838361353956fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc65d7a28e3265b37a6474929f336521b332c1681b933f6cb9f3376673440d862a02dd7bc7dec4dceedda775e58dd541e08a116c6c53815c0bd028192f7b626800cd5ed15c6e187e77e9aee88184c21f4f2182ab5827cb3b7e07fbedcd63f03300f0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15a\0\x13W__\xFD[Pa\0\x1Ca\x001V[a\0$a\x001V[a\0,a\x001V[a\0\xE3V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15a\0\x81W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14a\0\xE0W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80Qa6\xD5a\x01\t_9_\x81\x81a\x18\x8E\x01R\x81\x81a\x18\xB7\x01Ra\x1A/\x01Ra6\xD5_\xF3\xFE`\x80`@R`\x046\x10a\x02>W_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x014W\x80c\xAD<\xB1\xCC\x11a\0\xB3W\x80c\xC6H\x14\xDD\x11a\0xW\x80c\xC6H\x14\xDD\x14a\x06\xF7W\x80c\xD5Gt\x1F\x14a\x07-W\x80c\xE6:\xB1\xE9\x14a\x07LW\x80c\xF2\xFD\xE3\x8B\x14a\x07lW\x80c\xFAR\xC7\xD8\x14a\x07\x8BW\x80c\xFC\x0CTj\x14a\x07\xCEW__\xFD[\x80c\xAD<\xB1\xCC\x14a\x06#W\x80c\xB3\xE6\xEB\xD5\x14a\x06`W\x80c\xB5p\x0Eh\x14a\x06\x8EW\x80c\xB5\xEC\xB3D\x14a\x06\xADW\x80c\xBE 0\x94\x14a\x06\xD8W__\xFD[\x80c\x9E\x9A\x8F1\x11a\0\xF9W\x80c\x9E\x9A\x8F1\x14a\x05kW\x80c\xA2\x17\xFD\xDF\x14a\x05\x80W\x80c\xA2\xD7\x8D\xD5\x14a\x05\x93W\x80c\xA3\x06j\xAB\x14a\x05\xE5W\x80c\xAC\\*\xD0\x14a\x06\x04W__\xFD[\x80cqP\x18\xA6\x14a\x04\xB5W\x80c\x84V\xCBY\x14a\x04\xC9W\x80c\x8D\xA5\xCB[\x14a\x04\xDDW\x80c\x91\xD1HT\x14a\x05-W\x80c\x9B0\xA5\xE6\x14a\x05LW__\xFD[\x80c6V\x8A\xBE\x11a\x01\xC0W\x80cR\xD1\x90-\x11a\x01\x85W\x80cR\xD1\x90-\x14a\x04\"W\x80cUD\xC2\xF1\x14a\x046W\x80c\\\x97Z\xBB\x14a\x04PW\x80c_\x87T\xA6\x14a\x04sW\x80cj\x91\x1C\xCF\x14a\x04\xA1W__\xFD[\x80c6V\x8A\xBE\x14a\x03\xA9W\x80c>\x9D\xF9\xB5\x14a\x03\xC8W\x80c?K\xA8:\x14a\x03\xDCW\x80cM\x99\xDD\x16\x14a\x03\xF0W\x80cO\x1E\xF2\x86\x14a\x04\x0FW__\xFD[\x80c\x1A \xCDc\x11a\x02\x06W\x80c\x1A \xCDc\x14a\x03\0W\x80c!@\xFE\xCD\x14a\x03\x1FW\x80c$\x8A\x9C\xA3\x14a\x03>W\x80c//\xF1]\x14a\x03kW\x80c0B@\xBF\x14a\x03\x8AW__\xFD[\x80c\x01\xFF\xC9\xA7\x14a\x02BW\x80c\x02n@+\x14a\x02vW\x80c\x08@\xBAr\x14a\x02\x97W\x80c\r\x8En,\x14a\x02\xB6W\x80c\x13\xB9\x05z\x14a\x02\xE1W[__\xFD[4\x80\x15a\x02MW__\xFD[Pa\x02aa\x02\\6`\x04a,\xF3V[a\x07\xEDV[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x81W__\xFD[Pa\x02\x95a\x02\x906`\x04a-5V[a\x08#V[\0[4\x80\x15a\x02\xA2W__\xFD[Pa\x02\x95a\x02\xB16`\x04a-]V[a\x089V[4\x80\x15a\x02\xC1W__\xFD[P`@\x80Q`\x02\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02mV[4\x80\x15a\x02\xECW__\xFD[Pa\x02\x95a\x02\xFB6`\x04a.jV[a\t&V[4\x80\x15a\x03\x0BW__\xFD[Pa\x02\x95a\x03\x1A6`\x04a.\xD4V[a\t?V[4\x80\x15a\x03*W__\xFD[Pa\x02\x95a\x0396`\x04a.\xEFV[a\nSV[4\x80\x15a\x03IW__\xFD[Pa\x03]a\x03X6`\x04a/\x08V[a\ngV[`@Q\x90\x81R` \x01a\x02mV[4\x80\x15a\x03vW__\xFD[Pa\x02\x95a\x03\x856`\x04a/\x1FV[a\n\x87V[4\x80\x15a\x03\x95W__\xFD[Pa\x02\x95a\x03\xA46`\x04a/\xC9V[a\n\xA9V[4\x80\x15a\x03\xB4W__\xFD[Pa\x02\x95a\x03\xC36`\x04a/\x1FV[a\x0CoV[4\x80\x15a\x03\xD3W__\xFD[Pa\x03]_T\x81V[4\x80\x15a\x03\xE7W__\xFD[Pa\x02\x95a\x0C\xA7V[4\x80\x15a\x03\xFBW__\xFD[Pa\x02\x95a\x04\n6`\x04a-5V[a\x0C\xC6V[a\x02\x95a\x04\x1D6`\x04a0IV[a\x0C\xD8V[4\x80\x15a\x04-W__\xFD[Pa\x03]a\x0C\xF3V[4\x80\x15a\x04AW__\xFD[Pa\x02\x95a\x02\xFB6`\x04a0\x93V[4\x80\x15a\x04[W__\xFD[P_Q` a6\x89_9_Q\x90_RT`\xFF\x16a\x02aV[4\x80\x15a\x04~W__\xFD[Pa\x02aa\x04\x8D6`\x04a/\x08V[`\t` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[4\x80\x15a\x04\xACW__\xFD[Pa\x02\x95a\r\x0EV[4\x80\x15a\x04\xC0W__\xFD[Pa\x02\x95a\r V[4\x80\x15a\x04\xD4W__\xFD[Pa\x02\x95a\r1V[4\x80\x15a\x04\xE8W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02mV[4\x80\x15a\x058W__\xFD[Pa\x02aa\x05G6`\x04a/\x1FV[a\rPV[4\x80\x15a\x05WW__\xFD[Pa\x03]a\x05f6`\x04a0\xD7V[a\r\x86V[4\x80\x15a\x05vW__\xFD[Pa\x03]`\x08T\x81V[4\x80\x15a\x05\x8BW__\xFD[Pa\x03]_\x81V[4\x80\x15a\x05\x9EW__\xFD[Pa\x05\xD0a\x05\xAD6`\x04a-]V[`\x07` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 \x80T`\x01\x90\x91\x01T\x82V[`@\x80Q\x92\x83R` \x83\x01\x91\x90\x91R\x01a\x02mV[4\x80\x15a\x05\xF0W__\xFD[Pa\x02\x95a\x05\xFF6`\x04a.\xEFV[a\r\xE0V[4\x80\x15a\x06\x0FW__\xFD[Pa\x02\x95a\x06\x1E6`\x04a0\xF1V[a\r\xF1V[4\x80\x15a\x06.W__\xFD[Pa\x06S`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02m\x91\x90a1\x8EV[4\x80\x15a\x06kW__\xFD[Pa\x02aa\x06z6`\x04a/\x08V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[4\x80\x15a\x06\x99W__\xFD[P`\x01Ta\x05\x15\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x06\xB8W__\xFD[Pa\x03]a\x06\xC76`\x04a.\xEFV[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x06\xE3W__\xFD[Pa\x02\x95a\x06\xF26`\x04a1\xA0V[a\x0E\xC8V[4\x80\x15a\x07\x02W__\xFD[Pa\x03]a\x07\x116`\x04a-]V[`\x06` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 T\x81V[4\x80\x15a\x078W__\xFD[Pa\x02\x95a\x07G6`\x04a/\x1FV[a\x0F\xDFV[4\x80\x15a\x07WW__\xFD[Pa\x03]_Q` a6I_9_Q\x90_R\x81V[4\x80\x15a\x07wW__\xFD[Pa\x02\x95a\x07\x866`\x04a.\xEFV[a\x0F\xFBV[4\x80\x15a\x07\x96W__\xFD[Pa\x07\xC0a\x07\xA56`\x04a.\xEFV[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x02m\x92\x91\x90a1\xF3V[4\x80\x15a\x07\xD9W__\xFD[P`\x02Ta\x05\x15\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16cye\xDB\x0B`\xE0\x1B\x14\x80a\x08\x1DWPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x08+a\x10:V[a\x085\x82\x82a\x10jV[PPV[_Q` a6\xA9_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x08oWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x08\x8DW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x83\x16\x17`\x01`@\x1B\x17\x81Ua\x08\xB6a\x12\x08V[a\x08\xC0_\x84a\x12\x10V[Pa\x08\xD8_Q` a6I_9_Q\x90_R\x85a\x12\x10V[P\x80T`\xFF`@\x1B\x19\x16\x81U`@Q`\x01`\x01`@\x1B\x03\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[`@Qc\xC2\xD7\xF8\x13`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\tGa\x12\xB8V[`\x01T`@\x80Qc<\x1A\x08\x15`\xE2\x1B\x81R\x90Q_\x92`\x01`\x01`\xA0\x1B\x03\x16\x91c\xF0h T\x91`\x04\x80\x83\x01\x92` \x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\t\x8EW=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\t\xB2\x91\x90a2#V[a\t\xBD\x90`\x0Fa2RV[\x90Pb\x12u\0`\x01`\x01`@\x1B\x03\x80\x83\x16\x90\x84\x16\x10\x80a\t\xEEWP\x80`\x01`\x01`@\x1B\x03\x16\x83`\x01`\x01`@\x1B\x03\x16\x11[\x15a\n\x0CW`@Qc\xB5~!\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`@\x1B\x03\x83\x16`\x08\x81\x90U`@Q\x90\x81R\x7Fy>;\x1E\x1B\xCDg{\xB1\x19\0\xC81$\xD3\xC4L\x99F\xEA\x8D\xDF\x97\x8A\x0C\xA2P\xB04\xEC\x9D\xDE\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPV[a\n[a\x10:V[a\nd\x81a\x13\x13V[PV[_\x90\x81R_Q` a6i_9_Q\x90_R` R`@\x90 `\x01\x01T\x90V[a\n\x90\x82a\ngV[a\n\x99\x81a\x144V[a\n\xA3\x83\x83a\x12\x10V[PPPPV[a\n\xB1a\x10:V[3a\n\xBB\x81a\x14>V[a\n\xC4\x85a\x14\x8BV[a\n\xCE\x86\x86a\x14\xC6V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0B\0\x81\x86\x89a\x15?V[\x83Q`@\x14a\x0B\"W`@QcBG\x06\x87`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a'\x10\x83a\xFF\xFF\x16\x11\x15a\x0BIW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x0BW\x8Aa\r\x86V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`\x01`\t_a\x0B\x88\x89a\x15\xD4V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\x0B\xCFWa\x0B\xCFa1\xDFV[\x90R`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\x0C\x15Wa\x0C\x15a1\xDFV[\x02\x17\x90UP\x90PP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\xF0W\xD4\xEA\x81\xE9\x86(e:\x8F\x90x\x85A\x97 x\x13s4\xA9+B4~\xAC\xCD\xA8\x0F\xC4\n\x88\x88\x86\x89\x89`@Qa\x0C^\x95\x94\x93\x92\x91\x90a2tV[`@Q\x80\x91\x03\x90\xA2PPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x163\x14a\x0C\x98W`@Qc3K\xD9\x19`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0C\xA2\x82\x82a\x15\xF8V[PPPV[_Q` a6I_9_Q\x90_Ra\x0C\xBE\x81a\x144V[a\nda\x16qV[a\x0C\xCEa\x10:V[a\x085\x82\x82a\x16\xD1V[a\x0C\xE0a\x18\x83V[a\x0C\xE9\x82a\x19'V[a\x085\x82\x82a\x19hV[_a\x0C\xFCa\x1A$V[P_Q` a6)_9_Q\x90_R\x90V[a\r\x16a\x10:V[a\r\x1Ea\x1AmV[V[a\r(a\x12\xB8V[a\r\x1E_a\x1A\xFBV[_Q` a6I_9_Q\x90_Ra\rH\x81a\x144V[a\nda\x1BkV[_\x91\x82R_Q` a6i_9_Q\x90_R` \x90\x81R`@\x80\x84 `\x01`\x01`\xA0\x1B\x03\x93\x90\x93\x16\x84R\x91\x90R\x90 T`\xFF\x16\x90V[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\r\xC3\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\r\xE8a\x10:V[a\nd\x81a\x1B\xB3V[a\r\xF9a\x10:V[3a\x0E\x03\x81a\x1C\xC3V[a\x0E\x0C\x84a\x14\x8BV[a\x0E\x16\x85\x85a\x14\xC6V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0EH\x81\x85\x88a\x15?V[`\x01`\x04_a\x0EV\x89a\r\x86V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\xC8\xC5\xB3z\xEC\x7F-\xDB\xD3\xA1<Q6\x1ET\xA0\xA8\xDF;\xCA%j\xB7X\xA7\x7FZ\xD7A\xD2\x81\xE5\x87\x87\x87\x87`@Qa\x0E\xB8\x94\x93\x92\x91\x90a2\xE6V[`@Q\x80\x91\x03\x90\xA2PPPPPPV[_Q` a6\xA9_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x0E\xF9WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x0F\x14WP0;\x15[\x90P\x81\x15\x80\x15a\x0F\"WP\x80\x15[\x15a\x0F@W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x0FjW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x0Fs\x86a\x1DDV[a\x0F{a\x12\x08V[a\x0F\x83a\x1DUV[a\x0F\x8E\x89\x89\x89a\x1EGV[\x83\x15a\x0F\xD4W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[a\x0F\xE8\x82a\ngV[a\x0F\xF1\x81a\x144V[a\n\xA3\x83\x83a\x15\xF8V[a\x10\x03a\x12\xB8V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x101W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\nd\x81a\x1A\xFBV[_Q` a6\x89_9_Q\x90_RT`\xFF\x16\x15a\r\x1EW`@Qc\xD9<\x06e`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x10s\x82a\x1C\xC3V[3_\x82\x90\x03a\x10\x95W`@Qc\x1F* \x05`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R0`$\x83\x01R_\x92\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x10\xE3W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x11\x07\x91\x90a3LV[\x90P\x82\x81\x10\x15a\x114W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x84\x90R`D\x01a\x10(V[`\x02Ta\x11L\x90`\x01`\x01`\xA0\x1B\x03\x16\x830\x86a\x1E\xF0V[`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x03` R`@\x81 \x80T\x85\x92\x90a\x11s\x90\x84\x90a3cV[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x11\xAE\x90\x84\x90a3cV[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x85`@Qa\x11\xFA\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[a\r\x1Ea\x1F\x8DV[__Q` a6i_9_Q\x90_Ra\x12)\x84\x84a\rPV[a\x12\xA8W_\x84\x81R` \x82\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x87\x16\x84R\x90\x91R\x90 \x80T`\xFF\x19\x16`\x01\x17\x90Ua\x12^3\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x85\x7F/\x87\x88\x11~~\xFF\x1D\x82\xE9&\xECyI\x01\xD1|x\x02JP'\t@0E@\xA73eo\r`@Q`@Q\x80\x91\x03\x90\xA4`\x01\x91PPa\x08\x1DV[_\x91PPa\x08\x1DV[P\x92\x91PPV[3a\x12\xEA\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\r\x1EW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x10(V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T3\x91\x81\x90\x03a\x13MW`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x13nW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 T\x90\x81\x90\x03a\x13\xB6W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x87\x85\x16\x84R\x90\x91R\x81 U`\x02Ta\x13\xEB\x91\x16\x84\x83a\x1F\xC3V[\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x14&\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPPV[a\nd\x813a LV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x14mWa\x14ma1\xDFV[\x14a\ndW`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x14\xA8\x82\x82a \x85V[\x15a\x085W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x14\xD2\x84a\r\x86V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x15\x02W`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\t_a\x15\x0E\x83a\x15\xD4V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x085W`@Qc\xAEI;\x03`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x15H\x82a \xA6V[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a5\xE5`$\x919\x90P_\x84\x82`@Q` \x01a\x15x\x92\x91\x90a3\x8DV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x15\x93\x82a!<V[\x90Pa\x15\xB0\x81\x85a\x15\xA3\x88a\")V[a\x15\xABa\"\xA0V[a#mV[a\x15\xCCW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_\x81_\x01Q\x82` \x01Q`@Q` \x01a\r\xC3\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[__Q` a6i_9_Q\x90_Ra\x16\x11\x84\x84a\rPV[\x15a\x12\xA8W_\x84\x81R` \x82\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x87\x16\x80\x85R\x92R\x80\x83 \x80T`\xFF\x19\x16\x90UQ3\x92\x87\x91\x7F\xF69\x1F\\2\xD9\xC6\x9D*G\xEAg\x0BD)t\xB595\xD1\xED\xC7\xFDd\xEB!\xE0G\xA89\x17\x1B\x91\x90\xA4`\x01\x91PPa\x08\x1DV[a\x16ya$KV[_Q` a6\x89_9_Q\x90_R\x80T`\xFF\x19\x16\x81U\x7F]\xB9\xEE\nI[\xF2\xE6\xFF\x9C\x91\xA7\x83L\x1B\xA4\xFD\xD2D\xA5\xE8\xAANS{\xD3\x8A\xEA\xE4\xB0s\xAA3[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xA1PV[a\x16\xDA\x82a\x1C\xC3V[3_\x82\x90\x03a\x16\xFCW`@Qc\x1F* \x05`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x15a\x17?W`@Qc\xD4#\xA4\xF1`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x82\x81\x10\x15a\x17\x88W`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x10(V[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x17\xBE\x90\x84\x90a3\xA9V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x84\x81R` \x01`\x08TBa\x17\xE4\x91\x90a3cV[\x90R`\x01`\x01`\xA0\x1B\x03\x80\x86\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 \x94\x88\x16\x83R\x93\x81R\x83\x82 \x85Q\x81U\x94\x81\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x90\x81R`\x03\x90\x92R\x81 \x80T\x85\x92\x90a\x187\x90\x84\x90a3\xA9V[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x85`@Qa\x11\xFA\x91\x81R` \x01\x90V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x19\tWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x18\xFD_Q` a6)_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\r\x1EW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x19/a\x12\xB8V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x16\xC6V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x19\xC2WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x19\xBF\x91\x81\x01\x90a3LV[`\x01[a\x19\xEAW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x10(V[_Q` a6)_9_Q\x90_R\x81\x14a\x1A\x1AW`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x10(V[a\x0C\xA2\x83\x83a$zV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\r\x1EW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[3a\x1Aw\x81a\x1C\xC3V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\x08Ta\x1A\xA9\x90Ba3cV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 \x94\x90\x94U`\x03\x90R\x82\x81 \x81\x90U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2PV[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1Bsa\x10:V[_Q` a6\x89_9_Q\x90_R\x80T`\xFF\x19\x16`\x01\x17\x81U\x7Fb\xE7\x8C\xEA\x01\xBE\xE3 \xCDNB\x02p\xB5\xEAt\0\r\x11\xB0\xC9\xF7GT\xEB\xDB\xFCTK\x05\xA2X3a\x16\xB2V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 T\x90\x91\x81\x90\x03a\x1B\xF8W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R `\x01\x01TB\x10\x15a\x1C@W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x86\x85\x16\x84R\x90\x91R\x81 \x81\x81U`\x01\x01U`\x02Ta\x1C{\x91\x16\x83\x83a\x1F\xC3V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x1C\xB6\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16\x90\x81`\x02\x81\x11\x15a\x1C\xF4Wa\x1C\xF4a1\xDFV[\x03a\x1D\x12W`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x81`\x02\x81\x11\x15a\x1D&Wa\x1D&a1\xDFV[\x03a\x085W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1DLa\x1F\x8DV[a\nd\x81a$\xCFV[_Q` a6\xA9_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x1D\x86WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x1D\xA1WP0;\x15[\x90P\x81\x15\x80\x15a\x1D\xAFWP\x80\x15[\x15a\x1D\xCDW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x1D\xF7W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a\x1E@W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x1EnW`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x1E\x95W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x80\x86\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90\x92U`\x01\x80T\x92\x85\x16\x92\x90\x91\x16\x91\x90\x91\x17\x90U`Z\x80\x82\x10\x15a\x1E\xE8W`@Qc\xB5~!\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x08UPPV[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x1FIWP\x83;\x15=\x17\x15[\x80a\x1E@W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x10(V[_Q` a6\xA9_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\r\x1EW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a \rWP\x82;\x15=\x17\x15[\x80a\n\xA3W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x10(V[a V\x82\x82a\rPV[a\x085W`@Qc\xE2Q}?`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x10(V[\x80Q\x82Q_\x91\x14\x80\x15a \x9FWP\x81` \x01Q\x83` \x01Q\x14[\x93\x92PPPV[\x80Q` \x82\x01Q_\x91_Q` a6\t_9_Q\x90_R\x91\x15\x90\x15\x16\x15a \xCCWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0C\xA2W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x10(V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a!Y\x83a$\xD7V[\x90P_Q` a6\t_9_Q\x90_R`\x03_\x82\x84\x85\t\x90P\x82\x80a!\x80Wa!\x80a3\xBCV[\x84\x82\t\x90P\x82\x80a!\x93Wa!\x93a3\xBCV[\x82\x82\x08\x90P__a!\xA3\x83a&\xDFV[\x92P\x90P[\x80a\"\x0CW\x84\x80a!\xBBWa!\xBBa3\xBCV[`\x01\x87\x08\x95P\x84\x80a!\xCFWa!\xCFa3\xBCV[\x86\x87\t\x92P\x84\x80a!\xE2Wa!\xE2a3\xBCV[\x86\x84\t\x92P\x84\x80a!\xF5Wa!\xF5a3\xBCV[\x84\x84\x08\x92Pa\"\x03\x83a&\xDFV[\x92P\x90Pa!\xA8V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\"PWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_Q` a6\t_9_Q\x90_R\x84` \x01Qa\"\x81\x91\x90a3\xD0V[a\"\x98\x90_Q` a6\t_9_Q\x90_Ra3\xA9V[\x90R\x92\x91PPV[a\"\xC7`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[___`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a$?W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x10(V[P\x15\x15\x95\x94PPPPPV[_Q` a6\x89_9_Q\x90_RT`\xFF\x16a\r\x1EW`@Qc\x8D\xFC +`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a$\x83\x82a'\xD6V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a$\xC7Wa\x0C\xA2\x82\x82a(9V[a\x085a(\xABV[a\x10\x03a\x1F\x8DV[__a$\xE2\x83a(\xCAV[\x80Q\x90\x91P`0\x81\x14a$\xF7Wa$\xF7a3\xEFV[_\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a%\x10Wa%\x10a-\x8EV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a%:W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a%\xA9W\x83`\x01a%T\x83\x86a3\xA9V[a%^\x91\x90a3\xA9V[\x81Q\x81\x10a%nWa%na4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a%\x8BWa%\x8Ba4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a%?V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a&9W\x83\x81a%\xE5\x85\x88a3\xA9V[a%\xEF\x91\x90a3cV[\x81Q\x81\x10a%\xFFWa%\xFFa4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a&\x1FWa&\x1Fa4\x03V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a%\xD1V[P_a&D\x82a,\x15V[\x90Pa\x01\0_Q` a6\t_9_Q\x90_R_a&b\x86\x89a3\xA9V[\x90P_[\x81\x81\x10\x15a&\xCFW_\x88`\x01a&|\x84\x86a3\xA9V[a&\x86\x91\x90a3\xA9V[\x81Q\x81\x10a&\x96Wa&\x96a4\x03V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a&\xAEWa&\xAEa3\xBCV[\x85\x87\t\x95P\x83\x80a&\xC1Wa&\xC1a3\xBCV[\x81\x87\x08\x95PP`\x01\x01a&fV[P\x92\x9A\x99PPPPPPPPPPV[_____\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__Q` a6\t_9_Q\x90_R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a'\x9CW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x10(V[\x80`\x01\x84\x90\x1B\x11\x15a'\xB5Wa'\xB2\x83\x82a3\xA9V[\x92P[\x80\x80a'\xC3Wa'\xC3a3\xBCV[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a(\x0BW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x10(V[_Q` a6)_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa(U\x91\x90a4\x17V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a(\x8DW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a(\x92V[``\x91P[P\x91P\x91Pa(\xA2\x85\x83\x83a,uV[\x95\x94PPPPPV[4\x15a\r\x1EW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a)\n\x92\x91\x90a3\x8DV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a)1\x92\x91\x90a4\"V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a)S\x91\x90a4LV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a)}\x90\x83\x90\x83\x90` \x01a4dV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16`\x01`\x01`@\x1B\x03\x81\x11\x15a)\xECWa)\xECa-\x8EV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a*\x16W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a*-\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x90P_[\x81Q\x81\x10\x15a*\x97W\x81\x81\x81Q\x81\x10a*\\Wa*\\a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a*yWa*ya4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a*AV[P_\x84`@Q` \x01a*\xAC\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a+>W_\x83\x82\x81Q\x81\x10a*\xE5Wa*\xE5a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a+\x02Wa+\x02a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a+#\x92\x91\x90a4\x88V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a*\xCAV[P\x86\x88\x87`@Q` \x01a+T\x93\x92\x91\x90a4\xACV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a+\x82\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x91P_[a+\xA3\x8A`\xFF\x8D\x16a3\xA9V[\x81\x10\x15a,\x04W\x82\x81\x81Q\x81\x10a+\xBCWa+\xBCa4\x03V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a+\xD6\x83\x8Da3cV[\x81Q\x81\x10a+\xE6Wa+\xE6a4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a+\x96V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x12\xB1W\x83\x81\x81Q\x81\x10a,4Wa,4a4\x03V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a,L\x91\x90a4\xDFV[a,W\x90`\x02a5\xD9V[a,a\x91\x90a4\xDFV[a,k\x90\x83a3cV[\x91P`\x01\x01a,\x19V[``\x82a,\x8AWa,\x85\x82a,\xCAV[a \x9FV[\x81Q\x15\x80\x15a,\xA1WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a\x12\xB1W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x10(V[\x80Q\x15a,\xDAW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_` \x82\x84\x03\x12\x15a-\x03W__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a \x9FW__\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a-0W__\xFD[\x91\x90PV[__`@\x83\x85\x03\x12\x15a-FW__\xFD[a-O\x83a-\x1AV[\x94` \x93\x90\x93\x015\x93PPPV[__`@\x83\x85\x03\x12\x15a-nW__\xFD[a-w\x83a-\x1AV[\x91Pa-\x85` \x84\x01a-\x1AV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a-\xC4Wa-\xC4a-\x8EV[`@R\x90V[_`\x80\x82\x84\x03\x12\x15a-\xDAW__\xFD[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a-\xFCWa-\xFCa-\x8EV[`@\x90\x81R\x835\x82R` \x80\x85\x015\x90\x83\x01R\x83\x81\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_`@\x82\x84\x03\x12\x15a.:W__\xFD[a.Ba-\xA2V[\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[\x805a\xFF\xFF\x81\x16\x81\x14a-0W__\xFD[____a\x01 \x85\x87\x03\x12\x15a.~W__\xFD[a.\x88\x86\x86a-\xCAV[\x93Pa.\x97\x86`\x80\x87\x01a.*V[\x92Pa.\xA6\x86`\xC0\x87\x01a.*V[\x91Pa.\xB5a\x01\0\x86\x01a.YV[\x90P\x92\x95\x91\x94P\x92PV[`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\ndW__\xFD[_` \x82\x84\x03\x12\x15a.\xE4W__\xFD[\x815a \x9F\x81a.\xC0V[_` \x82\x84\x03\x12\x15a.\xFFW__\xFD[a \x9F\x82a-\x1AV[_` \x82\x84\x03\x12\x15a/\x18W__\xFD[P5\x91\x90PV[__`@\x83\x85\x03\x12\x15a/0W__\xFD[\x825\x91Pa-\x85` \x84\x01a-\x1AV[_\x82`\x1F\x83\x01\x12a/OW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a/hWa/ha-\x8EV[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/\x96Wa/\x96a-\x8EV[`@R\x81\x81R\x83\x82\x01` \x01\x85\x10\x15a/\xADW__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[_____a\x01@\x86\x88\x03\x12\x15a/\xDEW__\xFD[a/\xE8\x87\x87a-\xCAV[\x94Pa/\xF7\x87`\x80\x88\x01a.*V[\x93Pa0\x06\x87`\xC0\x88\x01a.*V[\x92Pa\x01\0\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a0!W__\xFD[a0-\x88\x82\x89\x01a/@V[\x92PPa0=a\x01 \x87\x01a.YV[\x90P\x92\x95P\x92\x95\x90\x93PV[__`@\x83\x85\x03\x12\x15a0ZW__\xFD[a0c\x83a-\x1AV[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a0}W__\xFD[a0\x89\x85\x82\x86\x01a/@V[\x91PP\x92P\x92\x90PV[___a\x01\0\x84\x86\x03\x12\x15a0\xA6W__\xFD[a0\xB0\x85\x85a-\xCAV[\x92Pa0\xBF\x85`\x80\x86\x01a.*V[\x91Pa0\xCE\x85`\xC0\x86\x01a.*V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a0\xE7W__\xFD[a \x9F\x83\x83a-\xCAV[____a\x01 \x85\x87\x03\x12\x15a1\x05W__\xFD[a1\x0F\x86\x86a-\xCAV[\x93Pa1\x1E\x86`\x80\x87\x01a.*V[\x92Pa1-\x86`\xC0\x87\x01a.*V[\x91Pa\x01\0\x85\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a1HW__\xFD[a1T\x87\x82\x88\x01a/@V[\x91PP\x92\x95\x91\x94P\x92PV[_\x81Q\x80\x84R\x80` \x84\x01` \x86\x01^_` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[` \x81R_a \x9F` \x83\x01\x84a1`V[____`\x80\x85\x87\x03\x12\x15a1\xB3W__\xFD[a1\xBC\x85a-\x1AV[\x93Pa1\xCA` \x86\x01a-\x1AV[\x92P`@\x85\x015\x91Pa.\xB5``\x86\x01a-\x1AV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a2\x16WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15a23W__\xFD[\x81Qa \x9F\x81a.\xC0V[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x02\x90\x81\x16\x90\x81\x81\x14a\x12\xB1Wa\x12\xB1a2>V[\x85Q\x81R` \x80\x87\x01Q\x90\x82\x01R`@\x80\x87\x01Q\x90\x82\x01R``\x80\x87\x01Q\x90\x82\x01R\x84Q`\x80\x82\x01R` \x85\x01Q`\xA0\x82\x01Ra\xFF\xFF\x84\x16`\xC0\x82\x01R\x82Q`\xE0\x82\x01R` \x83\x01Qa\x01\0\x82\x01Ra\x01@a\x01 \x82\x01R_a2\xDBa\x01@\x83\x01\x84a1`V[\x97\x96PPPPPPPV[\x84Q\x81R` \x80\x86\x01Q\x90\x82\x01R`@\x80\x86\x01Q\x90\x82\x01R``\x80\x86\x01Q\x90\x82\x01R\x83Q`\x80\x82\x01R` \x84\x01Q`\xA0\x82\x01R\x82Q`\xC0\x82\x01R` \x83\x01Q`\xE0\x82\x01Ra\x01 a\x01\0\x82\x01R_a3Ba\x01 \x83\x01\x84a1`V[\x96\x95PPPPPPV[_` \x82\x84\x03\x12\x15a3\\W__\xFD[PQ\x91\x90PV[\x80\x82\x01\x80\x82\x11\x15a\x08\x1DWa\x08\x1Da2>V[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a3\xA1a3\x9B\x83\x86a3vV[\x84a3vV[\x94\x93PPPPV[\x81\x81\x03\x81\x81\x11\x15a\x08\x1DWa\x08\x1Da2>V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a3\xEAWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a \x9F\x82\x84a3vV[_a4-\x82\x85a3vV[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a4W\x82\x84a3vV[_\x81R`\x01\x01\x93\x92PPPV[_a4o\x82\x85a3vV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a4\x93\x82\x85a3vV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a4\xB7\x82\x86a3vV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x08\x1DWa\x08\x1Da2>V[`\x01\x81[`\x01\x84\x11\x15a51W\x80\x85\x04\x81\x11\x15a5\x15Wa5\x15a2>V[`\x01\x84\x16\x15a5#W\x90\x81\x02\x90[`\x01\x93\x90\x93\x1C\x92\x80\x02a4\xFAV[\x93P\x93\x91PPV[_\x82a5GWP`\x01a\x08\x1DV[\x81a5SWP_a\x08\x1DV[\x81`\x01\x81\x14a5iW`\x02\x81\x14a5sWa5\x8FV[`\x01\x91PPa\x08\x1DV[`\xFF\x84\x11\x15a5\x84Wa5\x84a2>V[PP`\x01\x82\x1Ba\x08\x1DV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a5\xB2WP\x81\x81\na\x08\x1DV[a5\xBE_\x19\x84\x84a4\xF6V[\x80_\x19\x04\x82\x11\x15a5\xD1Wa5\xD1a2>V[\x02\x93\x92PPPV[_a \x9F\x83\x83a59V\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBCe\xD7\xA2\x8E2e\xB3zdt\x92\x9F3e!\xB32\xC1h\x1B\x93?l\xB9\xF37fsD\r\x86*\x02\xDD{\xC7\xDE\xC4\xDC\xEE\xDD\xA7u\xE5\x8D\xD5A\xE0\x8A\x11llS\x81\\\x0B\xD0(\x19/{bh\0\xCD^\xD1\\n\x18~w\xE9\xAE\xE8\x81\x84\xC2\x1FO!\x82\xABX'\xCB;~\x07\xFB\xED\xCDc\xF03\0\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x60806040526004361061023e575f3560e01c8063715018a611610134578063ad3cb1cc116100b3578063c64814dd11610078578063c64814dd146106f7578063d547741f1461072d578063e63ab1e91461074c578063f2fde38b1461076c578063fa52c7d81461078b578063fc0c546a146107ce575f5ffd5b8063ad3cb1cc14610623578063b3e6ebd514610660578063b5700e681461068e578063b5ecb344146106ad578063be203094146106d8575f5ffd5b80639e9a8f31116100f95780639e9a8f311461056b578063a217fddf14610580578063a2d78dd514610593578063a3066aab146105e5578063ac5c2ad014610604575f5ffd5b8063715018a6146104b55780638456cb59146104c95780638da5cb5b146104dd57806391d148541461052d5780639b30a5e61461054c575f5ffd5b806336568abe116101c057806352d1902d1161018557806352d1902d146104225780635544c2f1146104365780635c975abb146104505780635f8754a6146104735780636a911ccf146104a1575f5ffd5b806336568abe146103a95780633e9df9b5146103c85780633f4ba83a146103dc5780634d99dd16146103f05780634f1ef2861461040f575f5ffd5b80631a20cd63116102065780631a20cd63146103005780632140fecd1461031f578063248a9ca31461033e5780632f2ff15d1461036b578063304240bf1461038a575f5ffd5b806301ffc9a714610242578063026e402b146102765780630840ba72146102975780630d8e6e2c146102b657806313b9057a146102e1575b5f5ffd5b34801561024d575f5ffd5b5061026161025c366004612cf3565b6107ed565b60405190151581526020015b60405180910390f35b348015610281575f5ffd5b50610295610290366004612d35565b610823565b005b3480156102a2575f5ffd5b506102956102b1366004612d5d565b610839565b3480156102c1575f5ffd5b5060408051600281525f602082018190529181019190915260600161026d565b3480156102ec575f5ffd5b506102956102fb366004612e6a565b610926565b34801561030b575f5ffd5b5061029561031a366004612ed4565b61093f565b34801561032a575f5ffd5b50610295610339366004612eef565b610a53565b348015610349575f5ffd5b5061035d610358366004612f08565b610a67565b60405190815260200161026d565b348015610376575f5ffd5b50610295610385366004612f1f565b610a87565b348015610395575f5ffd5b506102956103a4366004612fc9565b610aa9565b3480156103b4575f5ffd5b506102956103c3366004612f1f565b610c6f565b3480156103d3575f5ffd5b5061035d5f5481565b3480156103e7575f5ffd5b50610295610ca7565b3480156103fb575f5ffd5b5061029561040a366004612d35565b610cc6565b61029561041d366004613049565b610cd8565b34801561042d575f5ffd5b5061035d610cf3565b348015610441575f5ffd5b506102956102fb366004613093565b34801561045b575f5ffd5b505f5160206136895f395f51905f525460ff16610261565b34801561047e575f5ffd5b5061026161048d366004612f08565b60096020525f908152604090205460ff1681565b3480156104ac575f5ffd5b50610295610d0e565b3480156104c0575f5ffd5b50610295610d20565b3480156104d4575f5ffd5b50610295610d31565b3480156104e8575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03165b6040516001600160a01b03909116815260200161026d565b348015610538575f5ffd5b50610261610547366004612f1f565b610d50565b348015610557575f5ffd5b5061035d6105663660046130d7565b610d86565b348015610576575f5ffd5b5061035d60085481565b34801561058b575f5ffd5b5061035d5f81565b34801561059e575f5ffd5b506105d06105ad366004612d5d565b600760209081525f92835260408084209091529082529020805460019091015482565b6040805192835260208301919091520161026d565b3480156105f0575f5ffd5b506102956105ff366004612eef565b610de0565b34801561060f575f5ffd5b5061029561061e3660046130f1565b610df1565b34801561062e575f5ffd5b50610653604051806040016040528060058152602001640352e302e360dc1b81525081565b60405161026d919061318e565b34801561066b575f5ffd5b5061026161067a366004612f08565b60046020525f908152604090205460ff1681565b348015610699575f5ffd5b50600154610515906001600160a01b031681565b3480156106b8575f5ffd5b5061035d6106c7366004612eef565b60056020525f908152604090205481565b3480156106e3575f5ffd5b506102956106f23660046131a0565b610ec8565b348015610702575f5ffd5b5061035d610711366004612d5d565b600660209081525f928352604080842090915290825290205481565b348015610738575f5ffd5b50610295610747366004612f1f565b610fdf565b348015610757575f5ffd5b5061035d5f5160206136495f395f51905f5281565b348015610777575f5ffd5b50610295610786366004612eef565b610ffb565b348015610796575f5ffd5b506107c06107a5366004612eef565b60036020525f90815260409020805460019091015460ff1682565b60405161026d9291906131f3565b3480156107d9575f5ffd5b50600254610515906001600160a01b031681565b5f6001600160e01b03198216637965db0b60e01b148061081d57506301ffc9a760e01b6001600160e01b03198316145b92915050565b61082b61103a565b610835828261106a565b5050565b5f5160206136a95f395f51905f52805460029190600160401b900460ff168061086f575080546001600160401b03808416911610155b1561088d5760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b03831617600160401b1781556108b6611208565b6108c05f84611210565b506108d85f5160206136495f395f51905f5285611210565b50805460ff60401b191681556040516001600160401b03831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b60405163c2d7f81360e01b815260040160405180910390fd5b6109476112b8565b60015460408051633c1a081560e21b815290515f926001600160a01b03169163f06820549160048083019260209291908290030181865afa15801561098e573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906109b29190613223565b6109bd90600f613252565b9050621275006001600160401b0380831690841610806109ee5750806001600160401b0316836001600160401b0316115b15610a0c5760405163b57e21df60e01b815260040160405180910390fd5b6001600160401b03831660088190556040519081527f793e3b1e1bcd677bb11900c83124d3c44c9946ea8ddf978a0ca250b034ec9dde9060200160405180910390a1505050565b610a5b61103a565b610a6481611313565b50565b5f9081525f5160206136695f395f51905f52602052604090206001015490565b610a9082610a67565b610a9981611434565b610aa38383611210565b50505050565b610ab161103a565b33610abb8161143e565b610ac48561148b565b610ace86866114c6565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610b0081868961153f565b8351604014610b2257604051634247068760e01b815260040160405180910390fd5b6127108361ffff161115610b495760405163dc81db8560e01b815260040160405180910390fd5b600160045f610b578a610d86565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550600160095f610b88896115d4565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f815260200160016002811115610bcf57610bcf6131df565b90526001600160a01b0383165f908152600360209081526040909120825181559082015160018083018054909160ff1990911690836002811115610c1557610c156131df565b0217905550905050816001600160a01b03167ff057d4ea81e98628653a8f90788541972078137334a92b42347eaccda80fc40a8888868989604051610c5e959493929190613274565b60405180910390a250505050505050565b6001600160a01b0381163314610c985760405163334bd91960e11b815260040160405180910390fd5b610ca282826115f8565b505050565b5f5160206136495f395f51905f52610cbe81611434565b610a64611671565b610cce61103a565b61083582826116d1565b610ce0611883565b610ce982611927565b6108358282611968565b5f610cfc611a24565b505f5160206136295f395f51905f5290565b610d1661103a565b610d1e611a6d565b565b610d286112b8565b610d1e5f611afb565b5f5160206136495f395f51905f52610d4881611434565b610a64611b6b565b5f9182525f5160206136695f395f51905f52602090815260408084206001600160a01b0393909316845291905290205460ff1690565b5f815f0151826020015183604001518460600151604051602001610dc3949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b610de861103a565b610a6481611bb3565b610df961103a565b33610e0381611cc3565b610e0c8461148b565b610e1685856114c6565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610e4881858861153f565b600160045f610e5689610d86565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550816001600160a01b03167fc8c5b37aec7f2ddbd3a13c51361e54a0a8df3bca256ab758a77f5ad741d281e587878787604051610eb894939291906132e6565b60405180910390a2505050505050565b5f5160206136a95f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015610ef95750825b90505f826001600160401b03166001148015610f145750303b155b905081158015610f22575080155b15610f405760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610f6a57845460ff60401b1916600160401b1785555b610f7386611d44565b610f7b611208565b610f83611d55565b610f8e898989611e47565b8315610fd457845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b610fe882610a67565b610ff181611434565b610aa383836115f8565b6110036112b8565b6001600160a01b03811661103157604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610a6481611afb565b5f5160206136895f395f51905f525460ff1615610d1e5760405163d93c066560e01b815260040160405180910390fd5b61107382611cc3565b335f82900361109557604051631f2a200560e01b815260040160405180910390fd5b600254604051636eb1769f60e11b81526001600160a01b0383811660048301523060248301525f92169063dd62ed3e90604401602060405180830381865afa1580156110e3573d5f5f3e3d5ffd5b505050506040513d601f19601f82011682018060405250810190611107919061334c565b9050828110156111345760405163054365bb60e31b81526004810182905260248101849052604401611028565b60025461114c906001600160a01b0316833086611ef0565b6001600160a01b0384165f9081526003602052604081208054859290611173908490613363565b90915550506001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906111ae908490613363565b92505081905550836001600160a01b0316826001600160a01b03167fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b856040516111fa91815260200190565b60405180910390a350505050565b610d1e611f8d565b5f5f5160206136695f395f51905f526112298484610d50565b6112a8575f848152602082815260408083206001600160a01b03871684529091529020805460ff1916600117905561125e3390565b6001600160a01b0316836001600160a01b0316857f2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d60405160405180910390a4600191505061081d565b5f91505061081d565b5092915050565b336112ea7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610d1e5760405163118cdaa760e01b8152336004820152602401611028565b6001600160a01b0381165f90815260056020526040812054339181900361134d576040516379298a5360e11b815260040160405180910390fd5b8042101561136e57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f908152600660209081526040808320938616835292905290812054908190036113b657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038085165f90815260066020908152604080832087851684529091528120556002546113eb91168483611fc3565b826001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b658260405161142691815260200190565b60405180910390a250505050565b610a64813361204c565b6001600160a01b0381165f9081526003602052604081206001015460ff16600281111561146d5761146d6131df565b14610a645760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f80825260208201526114a88282612085565b15610835576040516306cf438f60e01b815260040160405180910390fd5b60045f6114d284610d86565b815260208101919091526040015f205460ff16156115025760405162da8a5760e11b815260040160405180910390fd5b60095f61150e836115d4565b815260208101919091526040015f205460ff16156108355760405163ae493b0360e01b815260040160405180910390fd5b611548826120a6565b5f6040518060600160405280602481526020016135e56024913990505f848260405160200161157892919061338d565b60405160208183030381529060405290505f6115938261213c565b90506115b081856115a388612229565b6115ab6122a0565b61236d565b6115cc5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f815f01518260200151604051602001610dc3929190918252602082015260400190565b5f5f5160206136695f395f51905f526116118484610d50565b156112a8575f848152602082815260408083206001600160a01b0387168085529252808320805460ff1916905551339287917ff6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b9190a4600191505061081d565b61167961244b565b5f5160206136895f395f51905f52805460ff191681557f5db9ee0a495bf2e6ff9c91a7834c1ba4fdd244a5e8aa4e537bd38aeae4b073aa335b6040516001600160a01b0390911681526020015b60405180910390a150565b6116da82611cc3565b335f8290036116fc57604051631f2a200560e01b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320938516835292905220541561173f5760405163d423a4f160e01b815260040160405180910390fd5b6001600160a01b038084165f908152600660209081526040808320938516835292905220548281101561178857604051639266535160e01b815260048101829052602401611028565b6001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906117be9084906133a9565b925050819055506040518060400160405280848152602001600854426117e49190613363565b90526001600160a01b038086165f818152600760209081526040808320948816835293815283822085518155948101516001909501949094559081526003909252812080548592906118379084906133a9565b92505081905550836001600160a01b0316826001600160a01b03167f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c856040516111fa91815260200190565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061190957507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b03166118fd5f5160206136295f395f51905f52546001600160a01b031690565b6001600160a01b031614155b15610d1e5760405163703e46dd60e11b815260040160405180910390fd5b61192f6112b8565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d906020016116c6565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa9250505080156119c2575060408051601f3d908101601f191682019092526119bf9181019061334c565b60015b6119ea57604051634c9c8ce360e01b81526001600160a01b0383166004820152602401611028565b5f5160206136295f395f51905f528114611a1a57604051632a87526960e21b815260048101829052602401611028565b610ca2838361247a565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610d1e5760405163703e46dd60e11b815260040160405180910390fd5b33611a7781611cc3565b6001600160a01b0381165f908152600360205260409020600101805460ff19166002179055600854611aa99042613363565b6001600160a01b0382165f8181526005602090815260408083209490945560039052828120819055915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a250565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611b7361103a565b5f5160206136895f395f51905f52805460ff191660011781557f62e78cea01bee320cd4e420270b5ea74000d11b0c9f74754ebdbfc544b05a258336116b2565b6001600160a01b0381165f9081526007602090815260408083203380855292528220549091819003611bf857604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f90815260076020908152604080832093861683529290522060010154421015611c4057604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f9081526007602090815260408083208685168452909152812081815560010155600254611c7b91168383611fc3565b816001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b6582604051611cb691815260200190565b60405180910390a2505050565b6001600160a01b0381165f9081526003602052604081206001015460ff1690816002811115611cf457611cf46131df565b03611d125760405163508a793f60e01b815260040160405180910390fd5b6002816002811115611d2657611d266131df565b036108355760405163eab4a96360e01b815260040160405180910390fd5b611d4c611f8d565b610a64816124cf565b5f5160206136a95f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015611d865750825b90505f826001600160401b03166001148015611da15750303b155b905081158015611daf575080155b15611dcd5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315611df757845460ff60401b1916600160401b1785555b435f558315611e4057845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b5050505050565b6001600160a01b038316611e6e5760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b038216611e955760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b038086166001600160a01b0319928316179092556001805492851692909116919091179055605a80821015611ee85760405163b57e21df60e01b815260040160405180910390fd5b506008555050565b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f511416151615611f495750833b153d17155b80611e405760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401611028565b5f5160206136a95f395f51905f5254600160401b900460ff16610d1e57604051631afcd79f60e31b815260040160405180910390fd5b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f51141615161561200d5750823b153d17155b80610aa35760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401611028565b6120568282610d50565b6108355760405163e2517d3f60e01b81526001600160a01b038216600482015260248101839052604401611028565b805182515f9114801561209f575081602001518360200151145b9392505050565b805160208201515f915f5160206136095f395f51905f529115901516156120cc57505050565b825160208401518260038485858609850908838283091483821084841016169350505081610ca25760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401611028565b604080518082019091525f80825260208201525f612159836124d7565b90505f5160206136095f395f51905f5260035f8284850990508280612180576121806133bc565b84820990508280612193576121936133bc565b82820890505f5f6121a3836126df565b925090505b8061220c5784806121bb576121bb6133bc565b60018708955084806121cf576121cf6133bc565b868709925084806121e2576121e26133bc565b868409925084806121f5576121f56133bc565b8484089250612203836126df565b925090506121a8565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f8082526020820152815160208301511590151615612250575090565b6040518060400160405280835f015181526020015f5160206136095f395f51905f52846020015161228191906133d0565b612298905f5160206136095f395f51905f526133a9565b905292915050565b6122c760405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f5f5f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f5191508061243f5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401611028565b50151595945050505050565b5f5160206136895f395f51905f525460ff16610d1e57604051638dfc202b60e01b815260040160405180910390fd5b612483826127d6565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a28051156124c757610ca28282612839565b6108356128ab565b611003611f8d565b5f5f6124e2836128ca565b8051909150603081146124f7576124f76133ef565b5f816001600160401b0381111561251057612510612d8e565b6040519080825280601f01601f19166020018201604052801561253a576020820181803683370190505b5090505f5b828110156125a95783600161255483866133a9565b61255e91906133a9565b8151811061256e5761256e613403565b602001015160f81c60f81b82828151811061258b5761258b613403565b60200101906001600160f81b03191690815f1a90535060010161253f565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b828110156126395783816125e585886133a9565b6125ef9190613363565b815181106125ff576125ff613403565b602001015160f81c60f81b60f81c82828151811061261f5761261f613403565b60ff909216602092830291909101909101526001016125d1565b505f61264482612c15565b90506101005f5160206136095f395f51905f525f61266286896133a9565b90505f5b818110156126cf575f88600161267c84866133a9565b61268691906133a9565b8151811061269657612696613403565b016020015160f81c905083806126ae576126ae6133bc565b858709955083806126c1576126c16133bc565b818708955050600101612666565b50929a9950505050505050505050565b5f5f5f5f5f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f5160206136095f395f51905f52905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f5192508361279c5760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401611028565b80600184901b11156127b5576127b283826133a9565b92505b80806127c3576127c36133bc565b8384099690961496919550909350505050565b806001600160a01b03163b5f0361280b57604051634c9c8ce360e01b81526001600160a01b0382166004820152602401611028565b5f5160206136295f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b0316846040516128559190613417565b5f60405180830381855af49150503d805f811461288d576040519150601f19603f3d011682016040523d82523d5f602084013e612892565b606091505b50915091506128a2858383612c75565b95945050505050565b3415610d1e5760405163b398979f60e01b815260040160405180910390fd5b604080516030808252606082810190935290602090600160f91b905f9084602082018180368337019050509050808660405160200161290a92919061338d565b6040516020818303038152906040529050808460f81b604051602001612931929190613422565b604051602081830303815290604052905080604051602001612953919061344c565b60408051601f1981840301815290829052915061010160f01b9061297d9083908390602001613464565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff88166001600160401b038111156129ec576129ec612d8e565b6040519080825280601f01601f191660200182016040528015612a16576020820181803683370190505b5090505f82604051602001612a2d91815260200190565b60408051601f1981840301815291905290505f5b8151811015612a9757818181518110612a5c57612a5c613403565b602001015160f81c60f81b838281518110612a7957612a79613403565b60200101906001600160f81b03191690815f1a905350600101612a41565b505f84604051602001612aac91815260200190565b60408051601f19818403018152602083019091525f80835291985091505b89811015612b3e575f838281518110612ae557612ae5613403565b602001015160f81c60f81b838381518110612b0257612b02613403565b602001015160f81c60f81b1890508881604051602001612b23929190613488565b60408051601f19818403018152919052985050600101612aca565b50868887604051602001612b54939291906134ac565b60405160208183030381529060405296508680519060200120935083604051602001612b8291815260200190565b60408051601f1981840301815291905291505f5b612ba38a60ff8d166133a9565b811015612c0457828181518110612bbc57612bbc613403565b01602001516001600160f81b03191684612bd6838d613363565b81518110612be657612be6613403565b60200101906001600160f81b03191690815f1a905350600101612b96565b50919b9a5050505050505050505050565b5f80805b83518110156112b157838181518110612c3457612c34613403565b602002602001015160ff16816008612c4c91906134df565b612c579060026135d9565b612c6191906134df565b612c6b9083613363565b9150600101612c19565b606082612c8a57612c8582612cca565b61209f565b8151158015612ca157506001600160a01b0384163b155b156112b157604051639996b31560e01b81526001600160a01b0385166004820152602401611028565b805115612cda5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b5f60208284031215612d03575f5ffd5b81356001600160e01b03198116811461209f575f5ffd5b80356001600160a01b0381168114612d30575f5ffd5b919050565b5f5f60408385031215612d46575f5ffd5b612d4f83612d1a565b946020939093013593505050565b5f5f60408385031215612d6e575f5ffd5b612d7783612d1a565b9150612d8560208401612d1a565b90509250929050565b634e487b7160e01b5f52604160045260245ffd5b604080519081016001600160401b0381118282101715612dc457612dc4612d8e565b60405290565b5f60808284031215612dda575f5ffd5b604051608081016001600160401b0381118282101715612dfc57612dfc612d8e565b6040908152833582526020808501359083015283810135908201526060928301359281019290925250919050565b5f60408284031215612e3a575f5ffd5b612e42612da2565b823581526020928301359281019290925250919050565b803561ffff81168114612d30575f5ffd5b5f5f5f5f6101208587031215612e7e575f5ffd5b612e888686612dca565b9350612e978660808701612e2a565b9250612ea68660c08701612e2a565b9150612eb56101008601612e59565b905092959194509250565b6001600160401b0381168114610a64575f5ffd5b5f60208284031215612ee4575f5ffd5b813561209f81612ec0565b5f60208284031215612eff575f5ffd5b61209f82612d1a565b5f60208284031215612f18575f5ffd5b5035919050565b5f5f60408385031215612f30575f5ffd5b82359150612d8560208401612d1a565b5f82601f830112612f4f575f5ffd5b81356001600160401b03811115612f6857612f68612d8e565b604051601f8201601f19908116603f011681016001600160401b0381118282101715612f9657612f96612d8e565b604052818152838201602001851015612fad575f5ffd5b816020850160208301375f918101602001919091529392505050565b5f5f5f5f5f6101408688031215612fde575f5ffd5b612fe88787612dca565b9450612ff78760808801612e2a565b93506130068760c08801612e2a565b92506101008601356001600160401b03811115613021575f5ffd5b61302d88828901612f40565b92505061303d6101208701612e59565b90509295509295909350565b5f5f6040838503121561305a575f5ffd5b61306383612d1a565b915060208301356001600160401b0381111561307d575f5ffd5b61308985828601612f40565b9150509250929050565b5f5f5f61010084860312156130a6575f5ffd5b6130b08585612dca565b92506130bf8560808601612e2a565b91506130ce8560c08601612e2a565b90509250925092565b5f608082840312156130e7575f5ffd5b61209f8383612dca565b5f5f5f5f6101208587031215613105575f5ffd5b61310f8686612dca565b935061311e8660808701612e2a565b925061312d8660c08701612e2a565b91506101008501356001600160401b03811115613148575f5ffd5b61315487828801612f40565b91505092959194509250565b5f81518084528060208401602086015e5f602082860101526020601f19601f83011685010191505092915050565b602081525f61209f6020830184613160565b5f5f5f5f608085870312156131b3575f5ffd5b6131bc85612d1a565b93506131ca60208601612d1a565b925060408501359150612eb560608601612d1a565b634e487b7160e01b5f52602160045260245ffd5b828152604081016003831061321657634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b5f60208284031215613233575f5ffd5b815161209f81612ec0565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b0381811683821602908116908181146112b1576112b161323e565b8551815260208087015190820152604080870151908201526060808701519082015284516080820152602085015160a082015261ffff841660c0820152825160e082015260208301516101008201526101406101208201525f6132db610140830184613160565b979650505050505050565b8451815260208086015190820152604080860151908201526060808601519082015283516080820152602084015160a0820152825160c0820152602083015160e08201526101206101008201525f613342610120830184613160565b9695505050505050565b5f6020828403121561335c575f5ffd5b5051919050565b8082018082111561081d5761081d61323e565b5f81518060208401855e5f93019283525090919050565b5f6133a161339b8386613376565b84613376565b949350505050565b8181038181111561081d5761081d61323e565b634e487b7160e01b5f52601260045260245ffd5b5f826133ea57634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f61209f8284613376565b5f61342d8285613376565b5f81526001600160f81b03199390931660018401525050600201919050565b5f6134578284613376565b5f81526001019392505050565b5f61346f8285613376565b6001600160f01b03199390931683525050600201919050565b5f6134938285613376565b6001600160f81b03199390931683525050600101919050565b5f6134b78286613376565b6001600160f81b031994909416845250506001600160f01b0319166001820152600301919050565b808202811582820484141761081d5761081d61323e565b6001815b6001841115613531578085048111156135155761351561323e565b600184161561352357908102905b60019390931c9280026134fa565b935093915050565b5f826135475750600161081d565b8161355357505f61081d565b816001811461356957600281146135735761358f565b600191505061081d565b60ff8411156135845761358461323e565b50506001821b61081d565b5060208310610133831016604e8410600b84101617156135b2575081810a61081d565b6135be5f1984846134f6565b805f19048211156135d1576135d161323e565b029392505050565b5f61209f838361353956fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbc65d7a28e3265b37a6474929f336521b332c1681b933f6cb9f3376673440d862a02dd7bc7dec4dceedda775e58dd541e08a116c6c53815c0bd028192f7b626800cd5ed15c6e187e77e9aee88184c21f4f2182ab5827cb3b7e07fbedcd63f03300f0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x02>W_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x014W\x80c\xAD<\xB1\xCC\x11a\0\xB3W\x80c\xC6H\x14\xDD\x11a\0xW\x80c\xC6H\x14\xDD\x14a\x06\xF7W\x80c\xD5Gt\x1F\x14a\x07-W\x80c\xE6:\xB1\xE9\x14a\x07LW\x80c\xF2\xFD\xE3\x8B\x14a\x07lW\x80c\xFAR\xC7\xD8\x14a\x07\x8BW\x80c\xFC\x0CTj\x14a\x07\xCEW__\xFD[\x80c\xAD<\xB1\xCC\x14a\x06#W\x80c\xB3\xE6\xEB\xD5\x14a\x06`W\x80c\xB5p\x0Eh\x14a\x06\x8EW\x80c\xB5\xEC\xB3D\x14a\x06\xADW\x80c\xBE 0\x94\x14a\x06\xD8W__\xFD[\x80c\x9E\x9A\x8F1\x11a\0\xF9W\x80c\x9E\x9A\x8F1\x14a\x05kW\x80c\xA2\x17\xFD\xDF\x14a\x05\x80W\x80c\xA2\xD7\x8D\xD5\x14a\x05\x93W\x80c\xA3\x06j\xAB\x14a\x05\xE5W\x80c\xAC\\*\xD0\x14a\x06\x04W__\xFD[\x80cqP\x18\xA6\x14a\x04\xB5W\x80c\x84V\xCBY\x14a\x04\xC9W\x80c\x8D\xA5\xCB[\x14a\x04\xDDW\x80c\x91\xD1HT\x14a\x05-W\x80c\x9B0\xA5\xE6\x14a\x05LW__\xFD[\x80c6V\x8A\xBE\x11a\x01\xC0W\x80cR\xD1\x90-\x11a\x01\x85W\x80cR\xD1\x90-\x14a\x04\"W\x80cUD\xC2\xF1\x14a\x046W\x80c\\\x97Z\xBB\x14a\x04PW\x80c_\x87T\xA6\x14a\x04sW\x80cj\x91\x1C\xCF\x14a\x04\xA1W__\xFD[\x80c6V\x8A\xBE\x14a\x03\xA9W\x80c>\x9D\xF9\xB5\x14a\x03\xC8W\x80c?K\xA8:\x14a\x03\xDCW\x80cM\x99\xDD\x16\x14a\x03\xF0W\x80cO\x1E\xF2\x86\x14a\x04\x0FW__\xFD[\x80c\x1A \xCDc\x11a\x02\x06W\x80c\x1A \xCDc\x14a\x03\0W\x80c!@\xFE\xCD\x14a\x03\x1FW\x80c$\x8A\x9C\xA3\x14a\x03>W\x80c//\xF1]\x14a\x03kW\x80c0B@\xBF\x14a\x03\x8AW__\xFD[\x80c\x01\xFF\xC9\xA7\x14a\x02BW\x80c\x02n@+\x14a\x02vW\x80c\x08@\xBAr\x14a\x02\x97W\x80c\r\x8En,\x14a\x02\xB6W\x80c\x13\xB9\x05z\x14a\x02\xE1W[__\xFD[4\x80\x15a\x02MW__\xFD[Pa\x02aa\x02\\6`\x04a,\xF3V[a\x07\xEDV[`@Q\x90\x15\x15\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x81W__\xFD[Pa\x02\x95a\x02\x906`\x04a-5V[a\x08#V[\0[4\x80\x15a\x02\xA2W__\xFD[Pa\x02\x95a\x02\xB16`\x04a-]V[a\x089V[4\x80\x15a\x02\xC1W__\xFD[P`@\x80Q`\x02\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02mV[4\x80\x15a\x02\xECW__\xFD[Pa\x02\x95a\x02\xFB6`\x04a.jV[a\t&V[4\x80\x15a\x03\x0BW__\xFD[Pa\x02\x95a\x03\x1A6`\x04a.\xD4V[a\t?V[4\x80\x15a\x03*W__\xFD[Pa\x02\x95a\x0396`\x04a.\xEFV[a\nSV[4\x80\x15a\x03IW__\xFD[Pa\x03]a\x03X6`\x04a/\x08V[a\ngV[`@Q\x90\x81R` \x01a\x02mV[4\x80\x15a\x03vW__\xFD[Pa\x02\x95a\x03\x856`\x04a/\x1FV[a\n\x87V[4\x80\x15a\x03\x95W__\xFD[Pa\x02\x95a\x03\xA46`\x04a/\xC9V[a\n\xA9V[4\x80\x15a\x03\xB4W__\xFD[Pa\x02\x95a\x03\xC36`\x04a/\x1FV[a\x0CoV[4\x80\x15a\x03\xD3W__\xFD[Pa\x03]_T\x81V[4\x80\x15a\x03\xE7W__\xFD[Pa\x02\x95a\x0C\xA7V[4\x80\x15a\x03\xFBW__\xFD[Pa\x02\x95a\x04\n6`\x04a-5V[a\x0C\xC6V[a\x02\x95a\x04\x1D6`\x04a0IV[a\x0C\xD8V[4\x80\x15a\x04-W__\xFD[Pa\x03]a\x0C\xF3V[4\x80\x15a\x04AW__\xFD[Pa\x02\x95a\x02\xFB6`\x04a0\x93V[4\x80\x15a\x04[W__\xFD[P_Q` a6\x89_9_Q\x90_RT`\xFF\x16a\x02aV[4\x80\x15a\x04~W__\xFD[Pa\x02aa\x04\x8D6`\x04a/\x08V[`\t` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[4\x80\x15a\x04\xACW__\xFD[Pa\x02\x95a\r\x0EV[4\x80\x15a\x04\xC0W__\xFD[Pa\x02\x95a\r V[4\x80\x15a\x04\xD4W__\xFD[Pa\x02\x95a\r1V[4\x80\x15a\x04\xE8W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02mV[4\x80\x15a\x058W__\xFD[Pa\x02aa\x05G6`\x04a/\x1FV[a\rPV[4\x80\x15a\x05WW__\xFD[Pa\x03]a\x05f6`\x04a0\xD7V[a\r\x86V[4\x80\x15a\x05vW__\xFD[Pa\x03]`\x08T\x81V[4\x80\x15a\x05\x8BW__\xFD[Pa\x03]_\x81V[4\x80\x15a\x05\x9EW__\xFD[Pa\x05\xD0a\x05\xAD6`\x04a-]V[`\x07` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 \x80T`\x01\x90\x91\x01T\x82V[`@\x80Q\x92\x83R` \x83\x01\x91\x90\x91R\x01a\x02mV[4\x80\x15a\x05\xF0W__\xFD[Pa\x02\x95a\x05\xFF6`\x04a.\xEFV[a\r\xE0V[4\x80\x15a\x06\x0FW__\xFD[Pa\x02\x95a\x06\x1E6`\x04a0\xF1V[a\r\xF1V[4\x80\x15a\x06.W__\xFD[Pa\x06S`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02m\x91\x90a1\x8EV[4\x80\x15a\x06kW__\xFD[Pa\x02aa\x06z6`\x04a/\x08V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[4\x80\x15a\x06\x99W__\xFD[P`\x01Ta\x05\x15\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x06\xB8W__\xFD[Pa\x03]a\x06\xC76`\x04a.\xEFV[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x06\xE3W__\xFD[Pa\x02\x95a\x06\xF26`\x04a1\xA0V[a\x0E\xC8V[4\x80\x15a\x07\x02W__\xFD[Pa\x03]a\x07\x116`\x04a-]V[`\x06` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 T\x81V[4\x80\x15a\x078W__\xFD[Pa\x02\x95a\x07G6`\x04a/\x1FV[a\x0F\xDFV[4\x80\x15a\x07WW__\xFD[Pa\x03]_Q` a6I_9_Q\x90_R\x81V[4\x80\x15a\x07wW__\xFD[Pa\x02\x95a\x07\x866`\x04a.\xEFV[a\x0F\xFBV[4\x80\x15a\x07\x96W__\xFD[Pa\x07\xC0a\x07\xA56`\x04a.\xEFV[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x02m\x92\x91\x90a1\xF3V[4\x80\x15a\x07\xD9W__\xFD[P`\x02Ta\x05\x15\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[_`\x01`\x01`\xE0\x1B\x03\x19\x82\x16cye\xDB\x0B`\xE0\x1B\x14\x80a\x08\x1DWPc\x01\xFF\xC9\xA7`\xE0\x1B`\x01`\x01`\xE0\x1B\x03\x19\x83\x16\x14[\x92\x91PPV[a\x08+a\x10:V[a\x085\x82\x82a\x10jV[PPV[_Q` a6\xA9_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x08oWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x08\x8DW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x83\x16\x17`\x01`@\x1B\x17\x81Ua\x08\xB6a\x12\x08V[a\x08\xC0_\x84a\x12\x10V[Pa\x08\xD8_Q` a6I_9_Q\x90_R\x85a\x12\x10V[P\x80T`\xFF`@\x1B\x19\x16\x81U`@Q`\x01`\x01`@\x1B\x03\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[`@Qc\xC2\xD7\xF8\x13`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\tGa\x12\xB8V[`\x01T`@\x80Qc<\x1A\x08\x15`\xE2\x1B\x81R\x90Q_\x92`\x01`\x01`\xA0\x1B\x03\x16\x91c\xF0h T\x91`\x04\x80\x83\x01\x92` \x92\x91\x90\x82\x90\x03\x01\x81\x86Z\xFA\x15\x80\x15a\t\x8EW=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\t\xB2\x91\x90a2#V[a\t\xBD\x90`\x0Fa2RV[\x90Pb\x12u\0`\x01`\x01`@\x1B\x03\x80\x83\x16\x90\x84\x16\x10\x80a\t\xEEWP\x80`\x01`\x01`@\x1B\x03\x16\x83`\x01`\x01`@\x1B\x03\x16\x11[\x15a\n\x0CW`@Qc\xB5~!\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`@\x1B\x03\x83\x16`\x08\x81\x90U`@Q\x90\x81R\x7Fy>;\x1E\x1B\xCDg{\xB1\x19\0\xC81$\xD3\xC4L\x99F\xEA\x8D\xDF\x97\x8A\x0C\xA2P\xB04\xEC\x9D\xDE\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPV[a\n[a\x10:V[a\nd\x81a\x13\x13V[PV[_\x90\x81R_Q` a6i_9_Q\x90_R` R`@\x90 `\x01\x01T\x90V[a\n\x90\x82a\ngV[a\n\x99\x81a\x144V[a\n\xA3\x83\x83a\x12\x10V[PPPPV[a\n\xB1a\x10:V[3a\n\xBB\x81a\x14>V[a\n\xC4\x85a\x14\x8BV[a\n\xCE\x86\x86a\x14\xC6V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0B\0\x81\x86\x89a\x15?V[\x83Q`@\x14a\x0B\"W`@QcBG\x06\x87`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a'\x10\x83a\xFF\xFF\x16\x11\x15a\x0BIW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x0BW\x8Aa\r\x86V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`\x01`\t_a\x0B\x88\x89a\x15\xD4V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\x0B\xCFWa\x0B\xCFa1\xDFV[\x90R`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\x0C\x15Wa\x0C\x15a1\xDFV[\x02\x17\x90UP\x90PP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\xF0W\xD4\xEA\x81\xE9\x86(e:\x8F\x90x\x85A\x97 x\x13s4\xA9+B4~\xAC\xCD\xA8\x0F\xC4\n\x88\x88\x86\x89\x89`@Qa\x0C^\x95\x94\x93\x92\x91\x90a2tV[`@Q\x80\x91\x03\x90\xA2PPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x163\x14a\x0C\x98W`@Qc3K\xD9\x19`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0C\xA2\x82\x82a\x15\xF8V[PPPV[_Q` a6I_9_Q\x90_Ra\x0C\xBE\x81a\x144V[a\nda\x16qV[a\x0C\xCEa\x10:V[a\x085\x82\x82a\x16\xD1V[a\x0C\xE0a\x18\x83V[a\x0C\xE9\x82a\x19'V[a\x085\x82\x82a\x19hV[_a\x0C\xFCa\x1A$V[P_Q` a6)_9_Q\x90_R\x90V[a\r\x16a\x10:V[a\r\x1Ea\x1AmV[V[a\r(a\x12\xB8V[a\r\x1E_a\x1A\xFBV[_Q` a6I_9_Q\x90_Ra\rH\x81a\x144V[a\nda\x1BkV[_\x91\x82R_Q` a6i_9_Q\x90_R` \x90\x81R`@\x80\x84 `\x01`\x01`\xA0\x1B\x03\x93\x90\x93\x16\x84R\x91\x90R\x90 T`\xFF\x16\x90V[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\r\xC3\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\r\xE8a\x10:V[a\nd\x81a\x1B\xB3V[a\r\xF9a\x10:V[3a\x0E\x03\x81a\x1C\xC3V[a\x0E\x0C\x84a\x14\x8BV[a\x0E\x16\x85\x85a\x14\xC6V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0EH\x81\x85\x88a\x15?V[`\x01`\x04_a\x0EV\x89a\r\x86V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\xC8\xC5\xB3z\xEC\x7F-\xDB\xD3\xA1<Q6\x1ET\xA0\xA8\xDF;\xCA%j\xB7X\xA7\x7FZ\xD7A\xD2\x81\xE5\x87\x87\x87\x87`@Qa\x0E\xB8\x94\x93\x92\x91\x90a2\xE6V[`@Q\x80\x91\x03\x90\xA2PPPPPPV[_Q` a6\xA9_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x0E\xF9WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x0F\x14WP0;\x15[\x90P\x81\x15\x80\x15a\x0F\"WP\x80\x15[\x15a\x0F@W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x0FjW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x0Fs\x86a\x1DDV[a\x0F{a\x12\x08V[a\x0F\x83a\x1DUV[a\x0F\x8E\x89\x89\x89a\x1EGV[\x83\x15a\x0F\xD4W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[a\x0F\xE8\x82a\ngV[a\x0F\xF1\x81a\x144V[a\n\xA3\x83\x83a\x15\xF8V[a\x10\x03a\x12\xB8V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x101W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\nd\x81a\x1A\xFBV[_Q` a6\x89_9_Q\x90_RT`\xFF\x16\x15a\r\x1EW`@Qc\xD9<\x06e`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x10s\x82a\x1C\xC3V[3_\x82\x90\x03a\x10\x95W`@Qc\x1F* \x05`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R0`$\x83\x01R_\x92\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x10\xE3W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x11\x07\x91\x90a3LV[\x90P\x82\x81\x10\x15a\x114W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x84\x90R`D\x01a\x10(V[`\x02Ta\x11L\x90`\x01`\x01`\xA0\x1B\x03\x16\x830\x86a\x1E\xF0V[`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x03` R`@\x81 \x80T\x85\x92\x90a\x11s\x90\x84\x90a3cV[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x11\xAE\x90\x84\x90a3cV[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x85`@Qa\x11\xFA\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPV[a\r\x1Ea\x1F\x8DV[__Q` a6i_9_Q\x90_Ra\x12)\x84\x84a\rPV[a\x12\xA8W_\x84\x81R` \x82\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x87\x16\x84R\x90\x91R\x90 \x80T`\xFF\x19\x16`\x01\x17\x90Ua\x12^3\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x83`\x01`\x01`\xA0\x1B\x03\x16\x85\x7F/\x87\x88\x11~~\xFF\x1D\x82\xE9&\xECyI\x01\xD1|x\x02JP'\t@0E@\xA73eo\r`@Q`@Q\x80\x91\x03\x90\xA4`\x01\x91PPa\x08\x1DV[_\x91PPa\x08\x1DV[P\x92\x91PPV[3a\x12\xEA\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\r\x1EW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x10(V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T3\x91\x81\x90\x03a\x13MW`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x13nW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 T\x90\x81\x90\x03a\x13\xB6W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x87\x85\x16\x84R\x90\x91R\x81 U`\x02Ta\x13\xEB\x91\x16\x84\x83a\x1F\xC3V[\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x14&\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPPV[a\nd\x813a LV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x14mWa\x14ma1\xDFV[\x14a\ndW`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x14\xA8\x82\x82a \x85V[\x15a\x085W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x14\xD2\x84a\r\x86V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x15\x02W`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\t_a\x15\x0E\x83a\x15\xD4V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x085W`@Qc\xAEI;\x03`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x15H\x82a \xA6V[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a5\xE5`$\x919\x90P_\x84\x82`@Q` \x01a\x15x\x92\x91\x90a3\x8DV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x15\x93\x82a!<V[\x90Pa\x15\xB0\x81\x85a\x15\xA3\x88a\")V[a\x15\xABa\"\xA0V[a#mV[a\x15\xCCW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_\x81_\x01Q\x82` \x01Q`@Q` \x01a\r\xC3\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[__Q` a6i_9_Q\x90_Ra\x16\x11\x84\x84a\rPV[\x15a\x12\xA8W_\x84\x81R` \x82\x81R`@\x80\x83 `\x01`\x01`\xA0\x1B\x03\x87\x16\x80\x85R\x92R\x80\x83 \x80T`\xFF\x19\x16\x90UQ3\x92\x87\x91\x7F\xF69\x1F\\2\xD9\xC6\x9D*G\xEAg\x0BD)t\xB595\xD1\xED\xC7\xFDd\xEB!\xE0G\xA89\x17\x1B\x91\x90\xA4`\x01\x91PPa\x08\x1DV[a\x16ya$KV[_Q` a6\x89_9_Q\x90_R\x80T`\xFF\x19\x16\x81U\x7F]\xB9\xEE\nI[\xF2\xE6\xFF\x9C\x91\xA7\x83L\x1B\xA4\xFD\xD2D\xA5\xE8\xAANS{\xD3\x8A\xEA\xE4\xB0s\xAA3[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01[`@Q\x80\x91\x03\x90\xA1PV[a\x16\xDA\x82a\x1C\xC3V[3_\x82\x90\x03a\x16\xFCW`@Qc\x1F* \x05`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x15a\x17?W`@Qc\xD4#\xA4\xF1`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x82\x81\x10\x15a\x17\x88W`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x10(V[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x17\xBE\x90\x84\x90a3\xA9V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x84\x81R` \x01`\x08TBa\x17\xE4\x91\x90a3cV[\x90R`\x01`\x01`\xA0\x1B\x03\x80\x86\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 \x94\x88\x16\x83R\x93\x81R\x83\x82 \x85Q\x81U\x94\x81\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x90\x81R`\x03\x90\x92R\x81 \x80T\x85\x92\x90a\x187\x90\x84\x90a3\xA9V[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x85`@Qa\x11\xFA\x91\x81R` \x01\x90V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x19\tWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x18\xFD_Q` a6)_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\r\x1EW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x19/a\x12\xB8V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x16\xC6V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x19\xC2WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x19\xBF\x91\x81\x01\x90a3LV[`\x01[a\x19\xEAW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x10(V[_Q` a6)_9_Q\x90_R\x81\x14a\x1A\x1AW`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x10(V[a\x0C\xA2\x83\x83a$zV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\r\x1EW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[3a\x1Aw\x81a\x1C\xC3V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\x08Ta\x1A\xA9\x90Ba3cV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 \x94\x90\x94U`\x03\x90R\x82\x81 \x81\x90U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2PV[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1Bsa\x10:V[_Q` a6\x89_9_Q\x90_R\x80T`\xFF\x19\x16`\x01\x17\x81U\x7Fb\xE7\x8C\xEA\x01\xBE\xE3 \xCDNB\x02p\xB5\xEAt\0\r\x11\xB0\xC9\xF7GT\xEB\xDB\xFCTK\x05\xA2X3a\x16\xB2V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 T\x90\x91\x81\x90\x03a\x1B\xF8W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R `\x01\x01TB\x10\x15a\x1C@W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x86\x85\x16\x84R\x90\x91R\x81 \x81\x81U`\x01\x01U`\x02Ta\x1C{\x91\x16\x83\x83a\x1F\xC3V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x1C\xB6\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16\x90\x81`\x02\x81\x11\x15a\x1C\xF4Wa\x1C\xF4a1\xDFV[\x03a\x1D\x12W`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x81`\x02\x81\x11\x15a\x1D&Wa\x1D&a1\xDFV[\x03a\x085W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1DLa\x1F\x8DV[a\nd\x81a$\xCFV[_Q` a6\xA9_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x1D\x86WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x1D\xA1WP0;\x15[\x90P\x81\x15\x80\x15a\x1D\xAFWP\x80\x15[\x15a\x1D\xCDW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x1D\xF7W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a\x1E@W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x1EnW`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x1E\x95W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x80\x86\x16`\x01`\x01`\xA0\x1B\x03\x19\x92\x83\x16\x17\x90\x92U`\x01\x80T\x92\x85\x16\x92\x90\x91\x16\x91\x90\x91\x17\x90U`Z\x80\x82\x10\x15a\x1E\xE8W`@Qc\xB5~!\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`\x08UPPV[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x1FIWP\x83;\x15=\x17\x15[\x80a\x1E@W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x10(V[_Q` a6\xA9_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\r\x1EW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a \rWP\x82;\x15=\x17\x15[\x80a\n\xA3W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x10(V[a V\x82\x82a\rPV[a\x085W`@Qc\xE2Q}?`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x10(V[\x80Q\x82Q_\x91\x14\x80\x15a \x9FWP\x81` \x01Q\x83` \x01Q\x14[\x93\x92PPPV[\x80Q` \x82\x01Q_\x91_Q` a6\t_9_Q\x90_R\x91\x15\x90\x15\x16\x15a \xCCWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0C\xA2W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x10(V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a!Y\x83a$\xD7V[\x90P_Q` a6\t_9_Q\x90_R`\x03_\x82\x84\x85\t\x90P\x82\x80a!\x80Wa!\x80a3\xBCV[\x84\x82\t\x90P\x82\x80a!\x93Wa!\x93a3\xBCV[\x82\x82\x08\x90P__a!\xA3\x83a&\xDFV[\x92P\x90P[\x80a\"\x0CW\x84\x80a!\xBBWa!\xBBa3\xBCV[`\x01\x87\x08\x95P\x84\x80a!\xCFWa!\xCFa3\xBCV[\x86\x87\t\x92P\x84\x80a!\xE2Wa!\xE2a3\xBCV[\x86\x84\t\x92P\x84\x80a!\xF5Wa!\xF5a3\xBCV[\x84\x84\x08\x92Pa\"\x03\x83a&\xDFV[\x92P\x90Pa!\xA8V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\"PWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_Q` a6\t_9_Q\x90_R\x84` \x01Qa\"\x81\x91\x90a3\xD0V[a\"\x98\x90_Q` a6\t_9_Q\x90_Ra3\xA9V[\x90R\x92\x91PPV[a\"\xC7`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[___`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a$?W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x10(V[P\x15\x15\x95\x94PPPPPV[_Q` a6\x89_9_Q\x90_RT`\xFF\x16a\r\x1EW`@Qc\x8D\xFC +`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a$\x83\x82a'\xD6V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a$\xC7Wa\x0C\xA2\x82\x82a(9V[a\x085a(\xABV[a\x10\x03a\x1F\x8DV[__a$\xE2\x83a(\xCAV[\x80Q\x90\x91P`0\x81\x14a$\xF7Wa$\xF7a3\xEFV[_\x81`\x01`\x01`@\x1B\x03\x81\x11\x15a%\x10Wa%\x10a-\x8EV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a%:W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a%\xA9W\x83`\x01a%T\x83\x86a3\xA9V[a%^\x91\x90a3\xA9V[\x81Q\x81\x10a%nWa%na4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a%\x8BWa%\x8Ba4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a%?V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a&9W\x83\x81a%\xE5\x85\x88a3\xA9V[a%\xEF\x91\x90a3cV[\x81Q\x81\x10a%\xFFWa%\xFFa4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a&\x1FWa&\x1Fa4\x03V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a%\xD1V[P_a&D\x82a,\x15V[\x90Pa\x01\0_Q` a6\t_9_Q\x90_R_a&b\x86\x89a3\xA9V[\x90P_[\x81\x81\x10\x15a&\xCFW_\x88`\x01a&|\x84\x86a3\xA9V[a&\x86\x91\x90a3\xA9V[\x81Q\x81\x10a&\x96Wa&\x96a4\x03V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a&\xAEWa&\xAEa3\xBCV[\x85\x87\t\x95P\x83\x80a&\xC1Wa&\xC1a3\xBCV[\x81\x87\x08\x95PP`\x01\x01a&fV[P\x92\x9A\x99PPPPPPPPPPV[_____\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__Q` a6\t_9_Q\x90_R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a'\x9CW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x10(V[\x80`\x01\x84\x90\x1B\x11\x15a'\xB5Wa'\xB2\x83\x82a3\xA9V[\x92P[\x80\x80a'\xC3Wa'\xC3a3\xBCV[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a(\x0BW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x10(V[_Q` a6)_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa(U\x91\x90a4\x17V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a(\x8DW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a(\x92V[``\x91P[P\x91P\x91Pa(\xA2\x85\x83\x83a,uV[\x95\x94PPPPPV[4\x15a\r\x1EW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a)\n\x92\x91\x90a3\x8DV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a)1\x92\x91\x90a4\"V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a)S\x91\x90a4LV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a)}\x90\x83\x90\x83\x90` \x01a4dV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16`\x01`\x01`@\x1B\x03\x81\x11\x15a)\xECWa)\xECa-\x8EV[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a*\x16W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a*-\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x90P_[\x81Q\x81\x10\x15a*\x97W\x81\x81\x81Q\x81\x10a*\\Wa*\\a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a*yWa*ya4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a*AV[P_\x84`@Q` \x01a*\xAC\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a+>W_\x83\x82\x81Q\x81\x10a*\xE5Wa*\xE5a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a+\x02Wa+\x02a4\x03V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a+#\x92\x91\x90a4\x88V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a*\xCAV[P\x86\x88\x87`@Q` \x01a+T\x93\x92\x91\x90a4\xACV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a+\x82\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x91P_[a+\xA3\x8A`\xFF\x8D\x16a3\xA9V[\x81\x10\x15a,\x04W\x82\x81\x81Q\x81\x10a+\xBCWa+\xBCa4\x03V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a+\xD6\x83\x8Da3cV[\x81Q\x81\x10a+\xE6Wa+\xE6a4\x03V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a+\x96V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x12\xB1W\x83\x81\x81Q\x81\x10a,4Wa,4a4\x03V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a,L\x91\x90a4\xDFV[a,W\x90`\x02a5\xD9V[a,a\x91\x90a4\xDFV[a,k\x90\x83a3cV[\x91P`\x01\x01a,\x19V[``\x82a,\x8AWa,\x85\x82a,\xCAV[a \x9FV[\x81Q\x15\x80\x15a,\xA1WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a\x12\xB1W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x10(V[\x80Q\x15a,\xDAW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_` \x82\x84\x03\x12\x15a-\x03W__\xFD[\x815`\x01`\x01`\xE0\x1B\x03\x19\x81\x16\x81\x14a \x9FW__\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a-0W__\xFD[\x91\x90PV[__`@\x83\x85\x03\x12\x15a-FW__\xFD[a-O\x83a-\x1AV[\x94` \x93\x90\x93\x015\x93PPPV[__`@\x83\x85\x03\x12\x15a-nW__\xFD[a-w\x83a-\x1AV[\x91Pa-\x85` \x84\x01a-\x1AV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a-\xC4Wa-\xC4a-\x8EV[`@R\x90V[_`\x80\x82\x84\x03\x12\x15a-\xDAW__\xFD[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a-\xFCWa-\xFCa-\x8EV[`@\x90\x81R\x835\x82R` \x80\x85\x015\x90\x83\x01R\x83\x81\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_`@\x82\x84\x03\x12\x15a.:W__\xFD[a.Ba-\xA2V[\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[\x805a\xFF\xFF\x81\x16\x81\x14a-0W__\xFD[____a\x01 \x85\x87\x03\x12\x15a.~W__\xFD[a.\x88\x86\x86a-\xCAV[\x93Pa.\x97\x86`\x80\x87\x01a.*V[\x92Pa.\xA6\x86`\xC0\x87\x01a.*V[\x91Pa.\xB5a\x01\0\x86\x01a.YV[\x90P\x92\x95\x91\x94P\x92PV[`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\ndW__\xFD[_` \x82\x84\x03\x12\x15a.\xE4W__\xFD[\x815a \x9F\x81a.\xC0V[_` \x82\x84\x03\x12\x15a.\xFFW__\xFD[a \x9F\x82a-\x1AV[_` \x82\x84\x03\x12\x15a/\x18W__\xFD[P5\x91\x90PV[__`@\x83\x85\x03\x12\x15a/0W__\xFD[\x825\x91Pa-\x85` \x84\x01a-\x1AV[_\x82`\x1F\x83\x01\x12a/OW__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a/hWa/ha-\x8EV[`@Q`\x1F\x82\x01`\x1F\x19\x90\x81\x16`?\x01\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/\x96Wa/\x96a-\x8EV[`@R\x81\x81R\x83\x82\x01` \x01\x85\x10\x15a/\xADW__\xFD[\x81` \x85\x01` \x83\x017_\x91\x81\x01` \x01\x91\x90\x91R\x93\x92PPPV[_____a\x01@\x86\x88\x03\x12\x15a/\xDEW__\xFD[a/\xE8\x87\x87a-\xCAV[\x94Pa/\xF7\x87`\x80\x88\x01a.*V[\x93Pa0\x06\x87`\xC0\x88\x01a.*V[\x92Pa\x01\0\x86\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a0!W__\xFD[a0-\x88\x82\x89\x01a/@V[\x92PPa0=a\x01 \x87\x01a.YV[\x90P\x92\x95P\x92\x95\x90\x93PV[__`@\x83\x85\x03\x12\x15a0ZW__\xFD[a0c\x83a-\x1AV[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a0}W__\xFD[a0\x89\x85\x82\x86\x01a/@V[\x91PP\x92P\x92\x90PV[___a\x01\0\x84\x86\x03\x12\x15a0\xA6W__\xFD[a0\xB0\x85\x85a-\xCAV[\x92Pa0\xBF\x85`\x80\x86\x01a.*V[\x91Pa0\xCE\x85`\xC0\x86\x01a.*V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a0\xE7W__\xFD[a \x9F\x83\x83a-\xCAV[____a\x01 \x85\x87\x03\x12\x15a1\x05W__\xFD[a1\x0F\x86\x86a-\xCAV[\x93Pa1\x1E\x86`\x80\x87\x01a.*V[\x92Pa1-\x86`\xC0\x87\x01a.*V[\x91Pa\x01\0\x85\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a1HW__\xFD[a1T\x87\x82\x88\x01a/@V[\x91PP\x92\x95\x91\x94P\x92PV[_\x81Q\x80\x84R\x80` \x84\x01` \x86\x01^_` \x82\x86\x01\x01R` `\x1F\x19`\x1F\x83\x01\x16\x85\x01\x01\x91PP\x92\x91PPV[` \x81R_a \x9F` \x83\x01\x84a1`V[____`\x80\x85\x87\x03\x12\x15a1\xB3W__\xFD[a1\xBC\x85a-\x1AV[\x93Pa1\xCA` \x86\x01a-\x1AV[\x92P`@\x85\x015\x91Pa.\xB5``\x86\x01a-\x1AV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a2\x16WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15a23W__\xFD[\x81Qa \x9F\x81a.\xC0V[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x02\x90\x81\x16\x90\x81\x81\x14a\x12\xB1Wa\x12\xB1a2>V[\x85Q\x81R` \x80\x87\x01Q\x90\x82\x01R`@\x80\x87\x01Q\x90\x82\x01R``\x80\x87\x01Q\x90\x82\x01R\x84Q`\x80\x82\x01R` \x85\x01Q`\xA0\x82\x01Ra\xFF\xFF\x84\x16`\xC0\x82\x01R\x82Q`\xE0\x82\x01R` \x83\x01Qa\x01\0\x82\x01Ra\x01@a\x01 \x82\x01R_a2\xDBa\x01@\x83\x01\x84a1`V[\x97\x96PPPPPPPV[\x84Q\x81R` \x80\x86\x01Q\x90\x82\x01R`@\x80\x86\x01Q\x90\x82\x01R``\x80\x86\x01Q\x90\x82\x01R\x83Q`\x80\x82\x01R` \x84\x01Q`\xA0\x82\x01R\x82Q`\xC0\x82\x01R` \x83\x01Q`\xE0\x82\x01Ra\x01 a\x01\0\x82\x01R_a3Ba\x01 \x83\x01\x84a1`V[\x96\x95PPPPPPV[_` \x82\x84\x03\x12\x15a3\\W__\xFD[PQ\x91\x90PV[\x80\x82\x01\x80\x82\x11\x15a\x08\x1DWa\x08\x1Da2>V[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a3\xA1a3\x9B\x83\x86a3vV[\x84a3vV[\x94\x93PPPPV[\x81\x81\x03\x81\x81\x11\x15a\x08\x1DWa\x08\x1Da2>V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a3\xEAWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a \x9F\x82\x84a3vV[_a4-\x82\x85a3vV[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a4W\x82\x84a3vV[_\x81R`\x01\x01\x93\x92PPPV[_a4o\x82\x85a3vV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a4\x93\x82\x85a3vV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a4\xB7\x82\x86a3vV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x08\x1DWa\x08\x1Da2>V[`\x01\x81[`\x01\x84\x11\x15a51W\x80\x85\x04\x81\x11\x15a5\x15Wa5\x15a2>V[`\x01\x84\x16\x15a5#W\x90\x81\x02\x90[`\x01\x93\x90\x93\x1C\x92\x80\x02a4\xFAV[\x93P\x93\x91PPV[_\x82a5GWP`\x01a\x08\x1DV[\x81a5SWP_a\x08\x1DV[\x81`\x01\x81\x14a5iW`\x02\x81\x14a5sWa5\x8FV[`\x01\x91PPa\x08\x1DV[`\xFF\x84\x11\x15a5\x84Wa5\x84a2>V[PP`\x01\x82\x1Ba\x08\x1DV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a5\xB2WP\x81\x81\na\x08\x1DV[a5\xBE_\x19\x84\x84a4\xF6V[\x80_\x19\x04\x82\x11\x15a5\xD1Wa5\xD1a2>V[\x02\x93\x92PPPV[_a \x9F\x83\x83a59V\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBCe\xD7\xA2\x8E2e\xB3zdt\x92\x9F3e!\xB32\xC1h\x1B\x93?l\xB9\xF37fsD\r\x86*\x02\xDD{\xC7\xDE\xC4\xDC\xEE\xDD\xA7u\xE5\x8D\xD5A\xE0\x8A\x11llS\x81\\\x0B\xD0(\x19/{bh\0\xCD^\xD1\\n\x18~w\xE9\xAE\xE8\x81\x84\xC2\x1FO!\x82\xABX'\xCB;~\x07\xFB\xED\xCDc\xF03\0\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `AccessControlBadConfirmation()` and selector `0x6697b232`.
```solidity
error AccessControlBadConfirmation();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct AccessControlBadConfirmation {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<AccessControlBadConfirmation>
        for UnderlyingRustTuple<'_> {
            fn from(value: AccessControlBadConfirmation) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for AccessControlBadConfirmation {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for AccessControlBadConfirmation {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "AccessControlBadConfirmation()";
            const SELECTOR: [u8; 4] = [102u8, 151u8, 178u8, 50u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `AccessControlUnauthorizedAccount(address,bytes32)` and selector `0xe2517d3f`.
```solidity
error AccessControlUnauthorizedAccount(address account, bytes32 neededRole);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct AccessControlUnauthorizedAccount {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub neededRole: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            alloy::sol_types::sol_data::Address,
            alloy::sol_types::sol_data::FixedBytes<32>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::Address,
            alloy::sol_types::private::FixedBytes<32>,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<AccessControlUnauthorizedAccount>
        for UnderlyingRustTuple<'_> {
            fn from(value: AccessControlUnauthorizedAccount) -> Self {
                (value.account, value.neededRole)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for AccessControlUnauthorizedAccount {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    account: tuple.0,
                    neededRole: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for AccessControlUnauthorizedAccount {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "AccessControlUnauthorizedAccount(address,bytes32)";
            const SELECTOR: [u8; 4] = [226u8, 81u8, 125u8, 63u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.neededRole),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `AddressEmptyCode(address)` and selector `0x9996b315`.
```solidity
error AddressEmptyCode(address target);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct AddressEmptyCode {
        #[allow(missing_docs)]
        pub target: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<AddressEmptyCode> for UnderlyingRustTuple<'_> {
            fn from(value: AddressEmptyCode) -> Self {
                (value.target,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for AddressEmptyCode {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { target: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for AddressEmptyCode {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "AddressEmptyCode(address)";
            const SELECTOR: [u8; 4] = [153u8, 150u8, 179u8, 21u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.target,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `BLSSigVerificationFailed()` and selector `0x0ced3e50`.
```solidity
error BLSSigVerificationFailed();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BLSSigVerificationFailed {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<BLSSigVerificationFailed>
        for UnderlyingRustTuple<'_> {
            fn from(value: BLSSigVerificationFailed) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for BLSSigVerificationFailed {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for BLSSigVerificationFailed {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BLSSigVerificationFailed()";
            const SELECTOR: [u8; 4] = [12u8, 237u8, 62u8, 80u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `BlsKeyAlreadyUsed()` and selector `0x01b514ae`.
```solidity
error BlsKeyAlreadyUsed();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BlsKeyAlreadyUsed {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<BlsKeyAlreadyUsed> for UnderlyingRustTuple<'_> {
            fn from(value: BlsKeyAlreadyUsed) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for BlsKeyAlreadyUsed {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for BlsKeyAlreadyUsed {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BlsKeyAlreadyUsed()";
            const SELECTOR: [u8; 4] = [1u8, 181u8, 20u8, 174u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `DeprecatedFunction()` and selector `0xc2d7f813`.
```solidity
error DeprecatedFunction();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DeprecatedFunction {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<DeprecatedFunction> for UnderlyingRustTuple<'_> {
            fn from(value: DeprecatedFunction) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for DeprecatedFunction {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for DeprecatedFunction {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "DeprecatedFunction()";
            const SELECTOR: [u8; 4] = [194u8, 215u8, 248u8, 19u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ERC1967InvalidImplementation(address)` and selector `0x4c9c8ce3`.
```solidity
error ERC1967InvalidImplementation(address implementation);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ERC1967InvalidImplementation {
        #[allow(missing_docs)]
        pub implementation: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ERC1967InvalidImplementation>
        for UnderlyingRustTuple<'_> {
            fn from(value: ERC1967InvalidImplementation) -> Self {
                (value.implementation,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for ERC1967InvalidImplementation {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { implementation: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ERC1967InvalidImplementation {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ERC1967InvalidImplementation(address)";
            const SELECTOR: [u8; 4] = [76u8, 156u8, 140u8, 227u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.implementation,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ERC1967NonPayable()` and selector `0xb398979f`.
```solidity
error ERC1967NonPayable();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ERC1967NonPayable {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ERC1967NonPayable> for UnderlyingRustTuple<'_> {
            fn from(value: ERC1967NonPayable) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ERC1967NonPayable {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ERC1967NonPayable {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ERC1967NonPayable()";
            const SELECTOR: [u8; 4] = [179u8, 152u8, 151u8, 159u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `EnforcedPause()` and selector `0xd93c0665`.
```solidity
error EnforcedPause();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EnforcedPause {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<EnforcedPause> for UnderlyingRustTuple<'_> {
            fn from(value: EnforcedPause) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for EnforcedPause {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for EnforcedPause {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "EnforcedPause()";
            const SELECTOR: [u8; 4] = [217u8, 60u8, 6u8, 101u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ExitEscrowPeriodInvalid()` and selector `0xb57e21df`.
```solidity
error ExitEscrowPeriodInvalid();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ExitEscrowPeriodInvalid {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ExitEscrowPeriodInvalid> for UnderlyingRustTuple<'_> {
            fn from(value: ExitEscrowPeriodInvalid) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ExitEscrowPeriodInvalid {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ExitEscrowPeriodInvalid {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ExitEscrowPeriodInvalid()";
            const SELECTOR: [u8; 4] = [181u8, 126u8, 33u8, 223u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ExpectedPause()` and selector `0x8dfc202b`.
```solidity
error ExpectedPause();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ExpectedPause {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ExpectedPause> for UnderlyingRustTuple<'_> {
            fn from(value: ExpectedPause) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ExpectedPause {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ExpectedPause {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ExpectedPause()";
            const SELECTOR: [u8; 4] = [141u8, 252u8, 32u8, 43u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `FailedInnerCall()` and selector `0x1425ea42`.
```solidity
error FailedInnerCall();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct FailedInnerCall {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<FailedInnerCall> for UnderlyingRustTuple<'_> {
            fn from(value: FailedInnerCall) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for FailedInnerCall {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for FailedInnerCall {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "FailedInnerCall()";
            const SELECTOR: [u8; 4] = [20u8, 37u8, 234u8, 66u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InsufficientAllowance(uint256,uint256)` and selector `0x2a1b2dd8`.
```solidity
error InsufficientAllowance(uint256, uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InsufficientAllowance {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub _1: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (
            alloy::sol_types::sol_data::Uint<256>,
            alloy::sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            alloy::sol_types::private::primitives::aliases::U256,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InsufficientAllowance> for UnderlyingRustTuple<'_> {
            fn from(value: InsufficientAllowance) -> Self {
                (value._0, value._1)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InsufficientAllowance {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { _0: tuple.0, _1: tuple.1 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InsufficientAllowance {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InsufficientAllowance(uint256,uint256)";
            const SELECTOR: [u8; 4] = [42u8, 27u8, 45u8, 216u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self._0),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self._1),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InsufficientBalance(uint256)` and selector `0x92665351`.
```solidity
error InsufficientBalance(uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InsufficientBalance {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InsufficientBalance> for UnderlyingRustTuple<'_> {
            fn from(value: InsufficientBalance) -> Self {
                (value._0,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InsufficientBalance {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { _0: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InsufficientBalance {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InsufficientBalance(uint256)";
            const SELECTOR: [u8; 4] = [146u8, 102u8, 83u8, 81u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self._0),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidCommission()` and selector `0xdc81db85`.
```solidity
error InvalidCommission();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidCommission {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InvalidCommission> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidCommission) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidCommission {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidCommission {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidCommission()";
            const SELECTOR: [u8; 4] = [220u8, 129u8, 219u8, 133u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidInitialization()` and selector `0xf92ee8a9`.
```solidity
error InvalidInitialization();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidInitialization {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InvalidInitialization> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidInitialization) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidInitialization {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidInitialization {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidInitialization()";
            const SELECTOR: [u8; 4] = [249u8, 46u8, 232u8, 169u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidSchnorrSig()` and selector `0x42470687`.
```solidity
error InvalidSchnorrSig();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidSchnorrSig {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InvalidSchnorrSig> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidSchnorrSig) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidSchnorrSig {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidSchnorrSig {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidSchnorrSig()";
            const SELECTOR: [u8; 4] = [66u8, 71u8, 6u8, 135u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidSchnorrVK()` and selector `0x06cf438f`.
```solidity
error InvalidSchnorrVK();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidSchnorrVK {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InvalidSchnorrVK> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidSchnorrVK) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidSchnorrVK {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidSchnorrVK {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidSchnorrVK()";
            const SELECTOR: [u8; 4] = [6u8, 207u8, 67u8, 143u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `NotInitializing()` and selector `0xd7e6bcf8`.
```solidity
error NotInitializing();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct NotInitializing {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<NotInitializing> for UnderlyingRustTuple<'_> {
            fn from(value: NotInitializing) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for NotInitializing {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for NotInitializing {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "NotInitializing()";
            const SELECTOR: [u8; 4] = [215u8, 230u8, 188u8, 248u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `NothingToWithdraw()` and selector `0xd0d04f60`.
```solidity
error NothingToWithdraw();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct NothingToWithdraw {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<NothingToWithdraw> for UnderlyingRustTuple<'_> {
            fn from(value: NothingToWithdraw) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for NothingToWithdraw {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for NothingToWithdraw {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "NothingToWithdraw()";
            const SELECTOR: [u8; 4] = [208u8, 208u8, 79u8, 96u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `OwnableInvalidOwner(address)` and selector `0x1e4fbdf7`.
```solidity
error OwnableInvalidOwner(address owner);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct OwnableInvalidOwner {
        #[allow(missing_docs)]
        pub owner: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<OwnableInvalidOwner> for UnderlyingRustTuple<'_> {
            fn from(value: OwnableInvalidOwner) -> Self {
                (value.owner,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for OwnableInvalidOwner {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { owner: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for OwnableInvalidOwner {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "OwnableInvalidOwner(address)";
            const SELECTOR: [u8; 4] = [30u8, 79u8, 189u8, 247u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.owner,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `OwnableUnauthorizedAccount(address)` and selector `0x118cdaa7`.
```solidity
error OwnableUnauthorizedAccount(address account);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct OwnableUnauthorizedAccount {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<OwnableUnauthorizedAccount>
        for UnderlyingRustTuple<'_> {
            fn from(value: OwnableUnauthorizedAccount) -> Self {
                (value.account,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for OwnableUnauthorizedAccount {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { account: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for OwnableUnauthorizedAccount {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "OwnableUnauthorizedAccount(address)";
            const SELECTOR: [u8; 4] = [17u8, 140u8, 218u8, 167u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `PrematureWithdrawal()` and selector `0x5a774357`.
```solidity
error PrematureWithdrawal();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PrematureWithdrawal {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<PrematureWithdrawal> for UnderlyingRustTuple<'_> {
            fn from(value: PrematureWithdrawal) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for PrematureWithdrawal {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for PrematureWithdrawal {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "PrematureWithdrawal()";
            const SELECTOR: [u8; 4] = [90u8, 119u8, 67u8, 87u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `SchnorrKeyAlreadyUsed()` and selector `0xae493b03`.
```solidity
error SchnorrKeyAlreadyUsed();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct SchnorrKeyAlreadyUsed {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<SchnorrKeyAlreadyUsed> for UnderlyingRustTuple<'_> {
            fn from(value: SchnorrKeyAlreadyUsed) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for SchnorrKeyAlreadyUsed {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for SchnorrKeyAlreadyUsed {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "SchnorrKeyAlreadyUsed()";
            const SELECTOR: [u8; 4] = [174u8, 73u8, 59u8, 3u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `UUPSUnauthorizedCallContext()` and selector `0xe07c8dba`.
```solidity
error UUPSUnauthorizedCallContext();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UUPSUnauthorizedCallContext {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UUPSUnauthorizedCallContext>
        for UnderlyingRustTuple<'_> {
            fn from(value: UUPSUnauthorizedCallContext) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for UUPSUnauthorizedCallContext {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UUPSUnauthorizedCallContext {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UUPSUnauthorizedCallContext()";
            const SELECTOR: [u8; 4] = [224u8, 124u8, 141u8, 186u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `UUPSUnsupportedProxiableUUID(bytes32)` and selector `0xaa1d49a4`.
```solidity
error UUPSUnsupportedProxiableUUID(bytes32 slot);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UUPSUnsupportedProxiableUUID {
        #[allow(missing_docs)]
        pub slot: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UUPSUnsupportedProxiableUUID>
        for UnderlyingRustTuple<'_> {
            fn from(value: UUPSUnsupportedProxiableUUID) -> Self {
                (value.slot,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for UUPSUnsupportedProxiableUUID {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { slot: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UUPSUnsupportedProxiableUUID {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UUPSUnsupportedProxiableUUID(bytes32)";
            const SELECTOR: [u8; 4] = [170u8, 29u8, 73u8, 164u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.slot),
                )
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `UndelegationAlreadyExists()` and selector `0xd423a4f1`.
```solidity
error UndelegationAlreadyExists();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UndelegationAlreadyExists {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UndelegationAlreadyExists>
        for UnderlyingRustTuple<'_> {
            fn from(value: UndelegationAlreadyExists) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for UndelegationAlreadyExists {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UndelegationAlreadyExists {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UndelegationAlreadyExists()";
            const SELECTOR: [u8; 4] = [212u8, 35u8, 164u8, 241u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ValidatorAlreadyExited()` and selector `0xeab4a963`.
```solidity
error ValidatorAlreadyExited();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorAlreadyExited {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorAlreadyExited> for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorAlreadyExited) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ValidatorAlreadyExited {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorAlreadyExited {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ValidatorAlreadyExited()";
            const SELECTOR: [u8; 4] = [234u8, 180u8, 169u8, 99u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ValidatorAlreadyRegistered()` and selector `0x9973f7d8`.
```solidity
error ValidatorAlreadyRegistered();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorAlreadyRegistered {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorAlreadyRegistered>
        for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorAlreadyRegistered) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for ValidatorAlreadyRegistered {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorAlreadyRegistered {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ValidatorAlreadyRegistered()";
            const SELECTOR: [u8; 4] = [153u8, 115u8, 247u8, 216u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ValidatorInactive()` and selector `0x508a793f`.
```solidity
error ValidatorInactive();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorInactive {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorInactive> for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorInactive) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ValidatorInactive {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorInactive {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ValidatorInactive()";
            const SELECTOR: [u8; 4] = [80u8, 138u8, 121u8, 63u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ValidatorNotExited()` and selector `0xf25314a6`.
```solidity
error ValidatorNotExited();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorNotExited {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorNotExited> for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorNotExited) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ValidatorNotExited {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorNotExited {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ValidatorNotExited()";
            const SELECTOR: [u8; 4] = [242u8, 83u8, 20u8, 166u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ZeroAddress()` and selector `0xd92e233d`.
```solidity
error ZeroAddress();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ZeroAddress {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ZeroAddress> for UnderlyingRustTuple<'_> {
            fn from(value: ZeroAddress) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ZeroAddress {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ZeroAddress {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ZeroAddress()";
            const SELECTOR: [u8; 4] = [217u8, 46u8, 35u8, 61u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ZeroAmount()` and selector `0x1f2a2005`.
```solidity
error ZeroAmount();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ZeroAmount {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        type UnderlyingSolTuple<'a> = ();
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = ();
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(
            _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
        ) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ZeroAmount> for UnderlyingRustTuple<'_> {
            fn from(value: ZeroAmount) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ZeroAmount {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ZeroAmount {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ZeroAmount()";
            const SELECTOR: [u8; 4] = [31u8, 42u8, 32u8, 5u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive()]
    /**Event with signature `ConsensusKeysUpdated(address,(uint256,uint256,uint256,uint256),(uint256,uint256))` and selector `0x80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d`.
```solidity
event ConsensusKeysUpdated(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ConsensusKeysUpdated {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ConsensusKeysUpdated {
            type DataTuple<'a> = (BN254::G2Point, EdOnBN254::EdOnBN254Point);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ConsensusKeysUpdated(address,(uint256,uint256,uint256,uint256),(uint256,uint256))";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                128u8, 216u8, 164u8, 161u8, 102u8, 51u8, 40u8, 169u8, 152u8, 212u8, 85u8,
                91u8, 162u8, 29u8, 139u8, 186u8, 110u8, 241u8, 87u8, 106u8, 140u8, 94u8,
                157u8, 39u8, 249u8, 197u8, 69u8, 241u8, 163u8, 213u8, 43u8, 29u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    account: topics.1,
                    blsVK: data.0,
                    schnorrVK: data.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.account.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ConsensusKeysUpdated {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ConsensusKeysUpdated> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &ConsensusKeysUpdated) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive()]
    /**Event with signature `ConsensusKeysUpdatedV2(address,(uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes)` and selector `0xc8c5b37aec7f2ddbd3a13c51361e54a0a8df3bca256ab758a77f5ad741d281e5`.
```solidity
event ConsensusKeysUpdatedV2(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK, BN254.G1Point blsSig, bytes schnorrSig);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ConsensusKeysUpdatedV2 {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrSig: alloy::sol_types::private::Bytes,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ConsensusKeysUpdatedV2 {
            type DataTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
            );
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ConsensusKeysUpdatedV2(address,(uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                200u8, 197u8, 179u8, 122u8, 236u8, 127u8, 45u8, 219u8, 211u8, 161u8,
                60u8, 81u8, 54u8, 30u8, 84u8, 160u8, 168u8, 223u8, 59u8, 202u8, 37u8,
                106u8, 183u8, 88u8, 167u8, 127u8, 90u8, 215u8, 65u8, 210u8, 129u8, 229u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    account: topics.1,
                    blsVK: data.0,
                    schnorrVK: data.1,
                    blsSig: data.2,
                    schnorrSig: data.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.blsSig),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrSig,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.account.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ConsensusKeysUpdatedV2 {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ConsensusKeysUpdatedV2> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &ConsensusKeysUpdatedV2) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Delegated(address,address,uint256)` and selector `0xe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b`.
```solidity
event Delegated(address indexed delegator, address indexed validator, uint256 amount);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Delegated {
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Delegated {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Delegated(address,address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                229u8, 84u8, 26u8, 107u8, 97u8, 3u8, 212u8, 250u8, 126u8, 2u8, 30u8,
                213u8, 79u8, 173u8, 57u8, 198u8, 111u8, 39u8, 167u8, 107u8, 209u8, 61u8,
                55u8, 76u8, 246u8, 36u8, 10u8, 230u8, 189u8, 11u8, 183u8, 43u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    delegator: topics.1,
                    validator: topics.2,
                    amount: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.delegator.clone(),
                    self.validator.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.delegator,
                );
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.validator,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Delegated {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Delegated> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Delegated) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `ExitEscrowPeriodUpdated(uint64)` and selector `0x793e3b1e1bcd677bb11900c83124d3c44c9946ea8ddf978a0ca250b034ec9dde`.
```solidity
event ExitEscrowPeriodUpdated(uint64 newExitEscrowPeriod);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ExitEscrowPeriodUpdated {
        #[allow(missing_docs)]
        pub newExitEscrowPeriod: u64,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ExitEscrowPeriodUpdated {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "ExitEscrowPeriodUpdated(uint64)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                121u8, 62u8, 59u8, 30u8, 27u8, 205u8, 103u8, 123u8, 177u8, 25u8, 0u8,
                200u8, 49u8, 36u8, 211u8, 196u8, 76u8, 153u8, 70u8, 234u8, 141u8, 223u8,
                151u8, 138u8, 12u8, 162u8, 80u8, 176u8, 52u8, 236u8, 157u8, 222u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    newExitEscrowPeriod: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.newExitEscrowPeriod),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ExitEscrowPeriodUpdated {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ExitEscrowPeriodUpdated> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(
                this: &ExitEscrowPeriodUpdated,
            ) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Initialized(uint64)` and selector `0xc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d2`.
```solidity
event Initialized(uint64 version);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Initialized {
        #[allow(missing_docs)]
        pub version: u64,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Initialized {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Initialized(uint64)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                199u8, 245u8, 5u8, 178u8, 243u8, 113u8, 174u8, 33u8, 117u8, 238u8, 73u8,
                19u8, 244u8, 73u8, 158u8, 31u8, 38u8, 51u8, 167u8, 181u8, 147u8, 99u8,
                33u8, 238u8, 209u8, 205u8, 174u8, 182u8, 17u8, 81u8, 129u8, 210u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { version: data.0 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.version),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Initialized {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Initialized> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Initialized) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `OwnershipTransferred(address,address)` and selector `0x8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0`.
```solidity
event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct OwnershipTransferred {
        #[allow(missing_docs)]
        pub previousOwner: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub newOwner: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for OwnershipTransferred {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "OwnershipTransferred(address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8,
                31u8, 208u8, 164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8,
                218u8, 175u8, 227u8, 180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    previousOwner: topics.1,
                    newOwner: topics.2,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.previousOwner.clone(),
                    self.newOwner.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.previousOwner,
                );
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.newOwner,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for OwnershipTransferred {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&OwnershipTransferred> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &OwnershipTransferred) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Paused(address)` and selector `0x62e78cea01bee320cd4e420270b5ea74000d11b0c9f74754ebdbfc544b05a258`.
```solidity
event Paused(address account);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Paused {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Paused {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Paused(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                98u8, 231u8, 140u8, 234u8, 1u8, 190u8, 227u8, 32u8, 205u8, 78u8, 66u8,
                2u8, 112u8, 181u8, 234u8, 116u8, 0u8, 13u8, 17u8, 176u8, 201u8, 247u8,
                71u8, 84u8, 235u8, 219u8, 252u8, 84u8, 75u8, 5u8, 162u8, 88u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { account: data.0 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Paused {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Paused> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Paused) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleAdminChanged(bytes32,bytes32,bytes32)` and selector `0xbd79b86ffe0ab8e8776151514217cd7cacd52c909f66475c3af44e129f0b00ff`.
```solidity
event RoleAdminChanged(bytes32 indexed role, bytes32 indexed previousAdminRole, bytes32 indexed newAdminRole);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleAdminChanged {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub previousAdminRole: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub newAdminRole: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for RoleAdminChanged {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            const SIGNATURE: &'static str = "RoleAdminChanged(bytes32,bytes32,bytes32)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                189u8, 121u8, 184u8, 111u8, 254u8, 10u8, 184u8, 232u8, 119u8, 97u8, 81u8,
                81u8, 66u8, 23u8, 205u8, 124u8, 172u8, 213u8, 44u8, 144u8, 159u8, 102u8,
                71u8, 92u8, 58u8, 244u8, 78u8, 18u8, 159u8, 11u8, 0u8, 255u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    previousAdminRole: topics.2,
                    newAdminRole: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.role.clone(),
                    self.previousAdminRole.clone(),
                    self.newAdminRole.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.previousAdminRole);
                out[3usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.newAdminRole);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleAdminChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleAdminChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleAdminChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleGranted(bytes32,address,address)` and selector `0x2f8788117e7eff1d82e926ec794901d17c78024a50270940304540a733656f0d`.
```solidity
event RoleGranted(bytes32 indexed role, address indexed account, address indexed sender);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleGranted {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub sender: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for RoleGranted {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "RoleGranted(bytes32,address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                47u8, 135u8, 136u8, 17u8, 126u8, 126u8, 255u8, 29u8, 130u8, 233u8, 38u8,
                236u8, 121u8, 73u8, 1u8, 209u8, 124u8, 120u8, 2u8, 74u8, 80u8, 39u8, 9u8,
                64u8, 48u8, 69u8, 64u8, 167u8, 51u8, 101u8, 111u8, 13u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    account: topics.2,
                    sender: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.role.clone(),
                    self.account.clone(),
                    self.sender.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                out[3usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.sender,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleGranted {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleGranted> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleGranted) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `RoleRevoked(bytes32,address,address)` and selector `0xf6391f5c32d9c69d2a47ea670b442974b53935d1edc7fd64eb21e047a839171b`.
```solidity
event RoleRevoked(bytes32 indexed role, address indexed account, address indexed sender);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct RoleRevoked {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub sender: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for RoleRevoked {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "RoleRevoked(bytes32,address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                246u8, 57u8, 31u8, 92u8, 50u8, 217u8, 198u8, 157u8, 42u8, 71u8, 234u8,
                103u8, 11u8, 68u8, 41u8, 116u8, 181u8, 57u8, 53u8, 209u8, 237u8, 199u8,
                253u8, 100u8, 235u8, 33u8, 224u8, 71u8, 168u8, 57u8, 23u8, 27u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    role: topics.1,
                    account: topics.2,
                    sender: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.role.clone(),
                    self.account.clone(),
                    self.sender.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.role);
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                out[3usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.sender,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for RoleRevoked {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&RoleRevoked> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &RoleRevoked) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Undelegated(address,address,uint256)` and selector `0x4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c`.
```solidity
event Undelegated(address indexed delegator, address indexed validator, uint256 amount);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Undelegated {
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Undelegated {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Undelegated(address,address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                77u8, 16u8, 189u8, 4u8, 151u8, 117u8, 199u8, 123u8, 215u8, 242u8, 85u8,
                25u8, 90u8, 251u8, 165u8, 8u8, 128u8, 40u8, 236u8, 179u8, 199u8, 194u8,
                119u8, 211u8, 147u8, 204u8, 255u8, 121u8, 52u8, 242u8, 249u8, 44u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    delegator: topics.1,
                    validator: topics.2,
                    amount: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.delegator.clone(),
                    self.validator.clone(),
                )
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.delegator,
                );
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.validator,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Undelegated {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Undelegated> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Undelegated) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Unpaused(address)` and selector `0x5db9ee0a495bf2e6ff9c91a7834c1ba4fdd244a5e8aa4e537bd38aeae4b073aa`.
```solidity
event Unpaused(address account);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Unpaused {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Unpaused {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Unpaused(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                93u8, 185u8, 238u8, 10u8, 73u8, 91u8, 242u8, 230u8, 255u8, 156u8, 145u8,
                167u8, 131u8, 76u8, 27u8, 164u8, 253u8, 210u8, 68u8, 165u8, 232u8, 170u8,
                78u8, 83u8, 123u8, 211u8, 138u8, 234u8, 228u8, 176u8, 115u8, 170u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { account: data.0 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Unpaused {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Unpaused> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Unpaused) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Upgrade(address)` and selector `0xf78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d`.
```solidity
event Upgrade(address implementation);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Upgrade {
        #[allow(missing_docs)]
        pub implementation: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Upgrade {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Upgrade(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8,
                154u8, 22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8,
                185u8, 62u8, 237u8, 168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { implementation: data.0 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.implementation,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(),)
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Upgrade {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Upgrade> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Upgrade) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Upgraded(address)` and selector `0xbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b`.
```solidity
event Upgraded(address indexed implementation);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Upgraded {
        #[allow(missing_docs)]
        pub implementation: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Upgraded {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Upgraded(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                188u8, 124u8, 215u8, 90u8, 32u8, 238u8, 39u8, 253u8, 154u8, 222u8, 186u8,
                179u8, 32u8, 65u8, 247u8, 85u8, 33u8, 77u8, 188u8, 107u8, 255u8, 169u8,
                12u8, 192u8, 34u8, 91u8, 57u8, 218u8, 46u8, 92u8, 45u8, 59u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { implementation: topics.1 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.implementation.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.implementation,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Upgraded {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Upgraded> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Upgraded) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `ValidatorExit(address)` and selector `0xfb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd16`.
```solidity
event ValidatorExit(address indexed validator);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ValidatorExit {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ValidatorExit {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ValidatorExit(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                251u8, 36u8, 48u8, 83u8, 84u8, 200u8, 119u8, 98u8, 213u8, 87u8, 72u8,
                122u8, 228u8, 165u8, 100u8, 232u8, 208u8, 62u8, 203u8, 185u8, 169u8,
                125u8, 216u8, 175u8, 255u8, 142u8, 31u8, 111u8, 202u8, 240u8, 221u8, 22u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { validator: topics.1 }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.validator.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.validator,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ValidatorExit {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ValidatorExit> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &ValidatorExit) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive()]
    /**Event with signature `ValidatorRegistered(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16)` and selector `0xf6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f`.
```solidity
event ValidatorRegistered(address indexed account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ValidatorRegistered {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub blsVk: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVk: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub commission: u16,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ValidatorRegistered {
            type DataTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                alloy::sol_types::sol_data::Uint<16>,
            );
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ValidatorRegistered(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                246u8, 232u8, 53u8, 156u8, 87u8, 82u8, 11u8, 70u8, 150u8, 52u8, 115u8,
                107u8, 252u8, 59u8, 183u8, 236u8, 92u8, 189u8, 26u8, 11u8, 210u8, 139u8,
                16u8, 168u8, 39u8, 87u8, 147u8, 187u8, 115u8, 11u8, 121u8, 127u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    account: topics.1,
                    blsVk: data.0,
                    schnorrVk: data.1,
                    commission: data.2,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVk),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVk,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        16,
                    > as alloy_sol_types::SolType>::tokenize(&self.commission),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.account.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ValidatorRegistered {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ValidatorRegistered> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &ValidatorRegistered) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive()]
    /**Event with signature `ValidatorRegisteredV2(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16,(uint256,uint256),bytes)` and selector `0xf057d4ea81e98628653a8f90788541972078137334a92b42347eaccda80fc40a`.
```solidity
event ValidatorRegisteredV2(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK, uint16 commission, BN254.G1Point blsSig, bytes schnorrSig);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct ValidatorRegisteredV2 {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub commission: u16,
        #[allow(missing_docs)]
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrSig: alloy::sol_types::private::Bytes,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for ValidatorRegisteredV2 {
            type DataTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                alloy::sol_types::sol_data::Uint<16>,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
            );
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ValidatorRegisteredV2(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16,(uint256,uint256),bytes)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                240u8, 87u8, 212u8, 234u8, 129u8, 233u8, 134u8, 40u8, 101u8, 58u8, 143u8,
                144u8, 120u8, 133u8, 65u8, 151u8, 32u8, 120u8, 19u8, 115u8, 52u8, 169u8,
                43u8, 66u8, 52u8, 126u8, 172u8, 205u8, 168u8, 15u8, 196u8, 10u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    account: topics.1,
                    blsVK: data.0,
                    schnorrVK: data.1,
                    commission: data.2,
                    blsSig: data.3,
                    schnorrSig: data.4,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        16,
                    > as alloy_sol_types::SolType>::tokenize(&self.commission),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.blsSig),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrSig,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.account.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for ValidatorRegisteredV2 {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&ValidatorRegisteredV2> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &ValidatorRegisteredV2) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `Withdrawal(address,uint256)` and selector `0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65`.
```solidity
event Withdrawal(address indexed account, uint256 amount);
```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Withdrawal {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for Withdrawal {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Withdrawal(address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                127u8, 207u8, 83u8, 44u8, 21u8, 240u8, 166u8, 219u8, 11u8, 214u8, 208u8,
                224u8, 56u8, 190u8, 167u8, 29u8, 48u8, 216u8, 8u8, 199u8, 217u8, 140u8,
                179u8, 191u8, 114u8, 104u8, 169u8, 91u8, 245u8, 8u8, 27u8, 101u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    account: topics.1,
                    amount: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(
                        alloy_sol_types::Error::invalid_event_signature_hash(
                            Self::SIGNATURE,
                            topics.0,
                            Self::SIGNATURE_HASH,
                        ),
                    );
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.account.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.account,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for Withdrawal {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Withdrawal> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Withdrawal) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Constructor`.
```solidity
constructor();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct constructorCall {}
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<constructorCall> for UnderlyingRustTuple<'_> {
                fn from(value: constructorCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for constructorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolConstructor for constructorCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `DEFAULT_ADMIN_ROLE()` and selector `0xa217fddf`.
```solidity
function DEFAULT_ADMIN_ROLE() external view returns (bytes32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DEFAULT_ADMIN_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`DEFAULT_ADMIN_ROLE()`](DEFAULT_ADMIN_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DEFAULT_ADMIN_ROLEReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<DEFAULT_ADMIN_ROLECall>
            for UnderlyingRustTuple<'_> {
                fn from(value: DEFAULT_ADMIN_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for DEFAULT_ADMIN_ROLECall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<DEFAULT_ADMIN_ROLEReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: DEFAULT_ADMIN_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for DEFAULT_ADMIN_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for DEFAULT_ADMIN_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = DEFAULT_ADMIN_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "DEFAULT_ADMIN_ROLE()";
            const SELECTOR: [u8; 4] = [162u8, 23u8, 253u8, 223u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `PAUSER_ROLE()` and selector `0xe63ab1e9`.
```solidity
function PAUSER_ROLE() external view returns (bytes32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PAUSER_ROLECall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`PAUSER_ROLE()`](PAUSER_ROLECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PAUSER_ROLEReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<PAUSER_ROLECall> for UnderlyingRustTuple<'_> {
                fn from(value: PAUSER_ROLECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for PAUSER_ROLECall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<PAUSER_ROLEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: PAUSER_ROLEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for PAUSER_ROLEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for PAUSER_ROLECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = PAUSER_ROLEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "PAUSER_ROLE()";
            const SELECTOR: [u8; 4] = [230u8, 58u8, 177u8, 233u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `UPGRADE_INTERFACE_VERSION()` and selector `0xad3cb1cc`.
```solidity
function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UPGRADE_INTERFACE_VERSIONCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`UPGRADE_INTERFACE_VERSION()`](UPGRADE_INTERFACE_VERSIONCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UPGRADE_INTERFACE_VERSIONReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::String,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UPGRADE_INTERFACE_VERSIONCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: UPGRADE_INTERFACE_VERSIONCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for UPGRADE_INTERFACE_VERSIONCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::String,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::String,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UPGRADE_INTERFACE_VERSIONReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: UPGRADE_INTERFACE_VERSIONReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for UPGRADE_INTERFACE_VERSIONReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for UPGRADE_INTERFACE_VERSIONCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = UPGRADE_INTERFACE_VERSIONReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::String,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UPGRADE_INTERFACE_VERSION()";
            const SELECTOR: [u8; 4] = [173u8, 60u8, 177u8, 204u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive()]
    /**Function with signature `_hashBlsKey((uint256,uint256,uint256,uint256))` and selector `0x9b30a5e6`.
```solidity
function _hashBlsKey(BN254.G2Point memory blsVK) external pure returns (bytes32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _hashBlsKeyCall {
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`_hashBlsKey((uint256,uint256,uint256,uint256))`](_hashBlsKeyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _hashBlsKeyReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (BN254::G2Point,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<_hashBlsKeyCall> for UnderlyingRustTuple<'_> {
                fn from(value: _hashBlsKeyCall) -> Self {
                    (value.blsVK,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _hashBlsKeyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { blsVK: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<_hashBlsKeyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: _hashBlsKeyReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _hashBlsKeyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for _hashBlsKeyCall {
            type Parameters<'a> = (BN254::G2Point,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = _hashBlsKeyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "_hashBlsKey((uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [155u8, 48u8, 165u8, 230u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (<BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),)
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `blsKeys(bytes32)` and selector `0xb3e6ebd5`.
```solidity
function blsKeys(bytes32 blsKeyHash) external view returns (bool used);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct blsKeysCall {
        #[allow(missing_docs)]
        pub blsKeyHash: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`blsKeys(bytes32)`](blsKeysCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct blsKeysReturn {
        #[allow(missing_docs)]
        pub used: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<blsKeysCall> for UnderlyingRustTuple<'_> {
                fn from(value: blsKeysCall) -> Self {
                    (value.blsKeyHash,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for blsKeysCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { blsKeyHash: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<blsKeysReturn> for UnderlyingRustTuple<'_> {
                fn from(value: blsKeysReturn) -> Self {
                    (value.used,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for blsKeysReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { used: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for blsKeysCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = blsKeysReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "blsKeys(bytes32)";
            const SELECTOR: [u8; 4] = [179u8, 230u8, 235u8, 213u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.blsKeyHash),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `claimValidatorExit(address)` and selector `0x2140fecd`.
```solidity
function claimValidatorExit(address validator) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct claimValidatorExitCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`claimValidatorExit(address)`](claimValidatorExitCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct claimValidatorExitReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimValidatorExitCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: claimValidatorExitCall) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for claimValidatorExitCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { validator: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimValidatorExitReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: claimValidatorExitReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for claimValidatorExitReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for claimValidatorExitCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = claimValidatorExitReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "claimValidatorExit(address)";
            const SELECTOR: [u8; 4] = [33u8, 64u8, 254u8, 205u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `claimWithdrawal(address)` and selector `0xa3066aab`.
```solidity
function claimWithdrawal(address validator) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct claimWithdrawalCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`claimWithdrawal(address)`](claimWithdrawalCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct claimWithdrawalReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimWithdrawalCall> for UnderlyingRustTuple<'_> {
                fn from(value: claimWithdrawalCall) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for claimWithdrawalCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { validator: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimWithdrawalReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: claimWithdrawalReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for claimWithdrawalReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for claimWithdrawalCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = claimWithdrawalReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "claimWithdrawal(address)";
            const SELECTOR: [u8; 4] = [163u8, 6u8, 106u8, 171u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `delegate(address,uint256)` and selector `0x026e402b`.
```solidity
function delegate(address validator, uint256 amount) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegateCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`delegate(address,uint256)`](delegateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegateReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegateCall> for UnderlyingRustTuple<'_> {
                fn from(value: delegateCall) -> Self {
                    (value.validator, value.amount)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        validator: tuple.0,
                        amount: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: delegateReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegateCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegateReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "delegate(address,uint256)";
            const SELECTOR: [u8; 4] = [2u8, 110u8, 64u8, 43u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `delegations(address,address)` and selector `0xc64814dd`.
```solidity
function delegations(address validator, address delegator) external view returns (uint256 amount);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegationsCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`delegations(address,address)`](delegationsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegationsReturn {
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegationsCall> for UnderlyingRustTuple<'_> {
                fn from(value: delegationsCall) -> Self {
                    (value.validator, value.delegator)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegationsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        validator: tuple.0,
                        delegator: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegationsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: delegationsReturn) -> Self {
                    (value.amount,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegationsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { amount: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegationsCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegationsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "delegations(address,address)";
            const SELECTOR: [u8; 4] = [198u8, 72u8, 20u8, 221u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.delegator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `deregisterValidator()` and selector `0x6a911ccf`.
```solidity
function deregisterValidator() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct deregisterValidatorCall {}
    ///Container type for the return parameters of the [`deregisterValidator()`](deregisterValidatorCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct deregisterValidatorReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<deregisterValidatorCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: deregisterValidatorCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for deregisterValidatorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<deregisterValidatorReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: deregisterValidatorReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for deregisterValidatorReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for deregisterValidatorCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = deregisterValidatorReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "deregisterValidator()";
            const SELECTOR: [u8; 4] = [106u8, 145u8, 28u8, 207u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `exitEscrowPeriod()` and selector `0x9e9a8f31`.
```solidity
function exitEscrowPeriod() external view returns (uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct exitEscrowPeriodCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`exitEscrowPeriod()`](exitEscrowPeriodCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct exitEscrowPeriodReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<exitEscrowPeriodCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: exitEscrowPeriodCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for exitEscrowPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<exitEscrowPeriodReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: exitEscrowPeriodReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for exitEscrowPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for exitEscrowPeriodCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = exitEscrowPeriodReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "exitEscrowPeriod()";
            const SELECTOR: [u8; 4] = [158u8, 154u8, 143u8, 49u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getRoleAdmin(bytes32)` and selector `0x248a9ca3`.
```solidity
function getRoleAdmin(bytes32 role) external view returns (bytes32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getRoleAdminCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getRoleAdmin(bytes32)`](getRoleAdminCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getRoleAdminReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getRoleAdminCall> for UnderlyingRustTuple<'_> {
                fn from(value: getRoleAdminCall) -> Self {
                    (value.role,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getRoleAdminCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { role: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getRoleAdminReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getRoleAdminReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getRoleAdminReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getRoleAdminCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = getRoleAdminReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getRoleAdmin(bytes32)";
            const SELECTOR: [u8; 4] = [36u8, 138u8, 156u8, 163u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getVersion()` and selector `0x0d8e6e2c`.
```solidity
function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVersionCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`getVersion()`](getVersionCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVersionReturn {
        #[allow(missing_docs)]
        pub majorVersion: u8,
        #[allow(missing_docs)]
        pub minorVersion: u8,
        #[allow(missing_docs)]
        pub patchVersion: u8,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getVersionCall> for UnderlyingRustTuple<'_> {
                fn from(value: getVersionCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getVersionCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u8, u8, u8);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getVersionReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getVersionReturn) -> Self {
                    (value.majorVersion, value.minorVersion, value.patchVersion)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getVersionReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        majorVersion: tuple.0,
                        minorVersion: tuple.1,
                        patchVersion: tuple.2,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getVersionCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = getVersionReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getVersion()";
            const SELECTOR: [u8; 4] = [13u8, 142u8, 110u8, 44u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `grantRole(bytes32,address)` and selector `0x2f2ff15d`.
```solidity
function grantRole(bytes32 role, address account) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct grantRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`grantRole(bytes32,address)`](grantRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct grantRoleReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<grantRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: grantRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for grantRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<grantRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: grantRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for grantRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for grantRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = grantRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "grantRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [47u8, 47u8, 241u8, 93u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `hasRole(bytes32,address)` and selector `0x91d14854`.
```solidity
function hasRole(bytes32 role, address account) external view returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hasRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`hasRole(bytes32,address)`](hasRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct hasRoleReturn {
        #[allow(missing_docs)]
        pub _0: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<hasRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: hasRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hasRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<hasRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: hasRoleReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for hasRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for hasRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = hasRoleReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "hasRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [145u8, 209u8, 72u8, 84u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initialize(address,address,uint256,address)` and selector `0xbe203094`.
```solidity
function initialize(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod, address _timelock) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeCall {
        #[allow(missing_docs)]
        pub _tokenAddress: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _lightClientAddress: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub _timelock: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`initialize(address,address,uint256,address)`](initializeCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializeCall> for UnderlyingRustTuple<'_> {
                fn from(value: initializeCall) -> Self {
                    (
                        value._tokenAddress,
                        value._lightClientAddress,
                        value._exitEscrowPeriod,
                        value._timelock,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _tokenAddress: tuple.0,
                        _lightClientAddress: tuple.1,
                        _exitEscrowPeriod: tuple.2,
                        _timelock: tuple.3,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializeReturn> for UnderlyingRustTuple<'_> {
                fn from(value: initializeReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initialize(address,address,uint256,address)";
            const SELECTOR: [u8; 4] = [190u8, 32u8, 48u8, 148u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._tokenAddress,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._lightClientAddress,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self._exitEscrowPeriod),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._timelock,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initializeV2(address,address)` and selector `0x0840ba72`.
```solidity
function initializeV2(address pauser, address admin) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV2Call {
        #[allow(missing_docs)]
        pub pauser: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub admin: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`initializeV2(address,address)`](initializeV2Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV2Return {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializeV2Call> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV2Call) -> Self {
                    (value.pauser, value.admin)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV2Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        pauser: tuple.0,
                        admin: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializeV2Return> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV2Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeV2Call {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeV2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initializeV2(address,address)";
            const SELECTOR: [u8; 4] = [8u8, 64u8, 186u8, 114u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.pauser,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.admin,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initializedAtBlock()` and selector `0x3e9df9b5`.
```solidity
function initializedAtBlock() external view returns (uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializedAtBlockCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`initializedAtBlock()`](initializedAtBlockCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializedAtBlockReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializedAtBlockCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: initializedAtBlockCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for initializedAtBlockCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializedAtBlockReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: initializedAtBlockReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for initializedAtBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializedAtBlockCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializedAtBlockReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initializedAtBlock()";
            const SELECTOR: [u8; 4] = [62u8, 157u8, 249u8, 181u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `lightClient()` and selector `0xb5700e68`.
```solidity
function lightClient() external view returns (address);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lightClientCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`lightClient()`](lightClientCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lightClientReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<lightClientCall> for UnderlyingRustTuple<'_> {
                fn from(value: lightClientCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for lightClientCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<lightClientReturn> for UnderlyingRustTuple<'_> {
                fn from(value: lightClientReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for lightClientReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for lightClientCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = lightClientReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "lightClient()";
            const SELECTOR: [u8; 4] = [181u8, 112u8, 14u8, 104u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `owner()` and selector `0x8da5cb5b`.
```solidity
function owner() external view returns (address);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ownerCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`owner()`](ownerCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ownerReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<ownerCall> for UnderlyingRustTuple<'_> {
                fn from(value: ownerCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for ownerCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<ownerReturn> for UnderlyingRustTuple<'_> {
                fn from(value: ownerReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for ownerReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for ownerCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = ownerReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "owner()";
            const SELECTOR: [u8; 4] = [141u8, 165u8, 203u8, 91u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `pause()` and selector `0x8456cb59`.
```solidity
function pause() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct pauseCall {}
    ///Container type for the return parameters of the [`pause()`](pauseCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct pauseReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<pauseCall> for UnderlyingRustTuple<'_> {
                fn from(value: pauseCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for pauseCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<pauseReturn> for UnderlyingRustTuple<'_> {
                fn from(value: pauseReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for pauseReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for pauseCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = pauseReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "pause()";
            const SELECTOR: [u8; 4] = [132u8, 86u8, 203u8, 89u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `paused()` and selector `0x5c975abb`.
```solidity
function paused() external view returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct pausedCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`paused()`](pausedCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct pausedReturn {
        #[allow(missing_docs)]
        pub _0: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<pausedCall> for UnderlyingRustTuple<'_> {
                fn from(value: pausedCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for pausedCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<pausedReturn> for UnderlyingRustTuple<'_> {
                fn from(value: pausedReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for pausedReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for pausedCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = pausedReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "paused()";
            const SELECTOR: [u8; 4] = [92u8, 151u8, 90u8, 187u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `proxiableUUID()` and selector `0x52d1902d`.
```solidity
function proxiableUUID() external view returns (bytes32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct proxiableUUIDCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`proxiableUUID()`](proxiableUUIDCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct proxiableUUIDReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::FixedBytes<32>,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<proxiableUUIDCall> for UnderlyingRustTuple<'_> {
                fn from(value: proxiableUUIDCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for proxiableUUIDCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<proxiableUUIDReturn> for UnderlyingRustTuple<'_> {
                fn from(value: proxiableUUIDReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for proxiableUUIDReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for proxiableUUIDCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = proxiableUUIDReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "proxiableUUID()";
            const SELECTOR: [u8; 4] = [82u8, 209u8, 144u8, 45u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive()]
    /**Function with signature `registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)` and selector `0x13b9057a`.
```solidity
function registerValidator(BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory, uint16) external pure;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorCall {
        #[allow(missing_docs)]
        pub _0: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _1: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _3: u16,
    }
    ///Container type for the return parameters of the [`registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)`](registerValidatorCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Uint<16>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
                <BN254::G1Point as alloy::sol_types::SolType>::RustType,
                u16,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorCall) -> Self {
                    (value._0, value._1, value._2, value._3)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for registerValidatorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
                        _2: tuple.2,
                        _3: tuple.3,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for registerValidatorReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for registerValidatorCall {
            type Parameters<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Uint<16>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = registerValidatorReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)";
            const SELECTOR: [u8; 4] = [19u8, 185u8, 5u8, 122u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self._0),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self._2),
                    <alloy::sol_types::sol_data::Uint<
                        16,
                    > as alloy_sol_types::SolType>::tokenize(&self._3),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive()]
    /**Function with signature `registerValidatorV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes,uint16)` and selector `0x304240bf`.
```solidity
function registerValidatorV2(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, bytes memory schnorrSig, uint16 commission) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorV2Call {
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrSig: alloy::sol_types::private::Bytes,
        #[allow(missing_docs)]
        pub commission: u16,
    }
    ///Container type for the return parameters of the [`registerValidatorV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes,uint16)`](registerValidatorV2Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorV2Return {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::Uint<16>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
                <BN254::G1Point as alloy::sol_types::SolType>::RustType,
                alloy::sol_types::private::Bytes,
                u16,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorV2Call>
            for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorV2Call) -> Self {
                    (
                        value.blsVK,
                        value.schnorrVK,
                        value.blsSig,
                        value.schnorrSig,
                        value.commission,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for registerValidatorV2Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blsVK: tuple.0,
                        schnorrVK: tuple.1,
                        blsSig: tuple.2,
                        schnorrSig: tuple.3,
                        commission: tuple.4,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorV2Return>
            for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorV2Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for registerValidatorV2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for registerValidatorV2Call {
            type Parameters<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
                alloy::sol_types::sol_data::Uint<16>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = registerValidatorV2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "registerValidatorV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes,uint16)";
            const SELECTOR: [u8; 4] = [48u8, 66u8, 64u8, 191u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.blsSig),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrSig,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        16,
                    > as alloy_sol_types::SolType>::tokenize(&self.commission),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `renounceOwnership()` and selector `0x715018a6`.
```solidity
function renounceOwnership() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceOwnershipCall {}
    ///Container type for the return parameters of the [`renounceOwnership()`](renounceOwnershipCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceOwnershipReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceOwnershipCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: renounceOwnershipCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for renounceOwnershipCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceOwnershipReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: renounceOwnershipReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for renounceOwnershipReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for renounceOwnershipCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = renounceOwnershipReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "renounceOwnership()";
            const SELECTOR: [u8; 4] = [113u8, 80u8, 24u8, 166u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `renounceRole(bytes32,address)` and selector `0x36568abe`.
```solidity
function renounceRole(bytes32 role, address callerConfirmation) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub callerConfirmation: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`renounceRole(bytes32,address)`](renounceRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceRoleReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: renounceRoleCall) -> Self {
                    (value.role, value.callerConfirmation)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        callerConfirmation: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: renounceRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for renounceRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = renounceRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "renounceRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [54u8, 86u8, 138u8, 190u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.callerConfirmation,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `revokeRole(bytes32,address)` and selector `0xd547741f`.
```solidity
function revokeRole(bytes32 role, address account) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct revokeRoleCall {
        #[allow(missing_docs)]
        pub role: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`revokeRole(bytes32,address)`](revokeRoleCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct revokeRoleReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<revokeRoleCall> for UnderlyingRustTuple<'_> {
                fn from(value: revokeRoleCall) -> Self {
                    (value.role, value.account)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for revokeRoleCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        role: tuple.0,
                        account: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<revokeRoleReturn> for UnderlyingRustTuple<'_> {
                fn from(value: revokeRoleReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for revokeRoleReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for revokeRoleCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = revokeRoleReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "revokeRole(bytes32,address)";
            const SELECTOR: [u8; 4] = [213u8, 71u8, 116u8, 31u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.role),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `schnorrKeys(bytes32)` and selector `0x5f8754a6`.
```solidity
function schnorrKeys(bytes32 schnorrKey) external view returns (bool used);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct schnorrKeysCall {
        #[allow(missing_docs)]
        pub schnorrKey: alloy::sol_types::private::FixedBytes<32>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`schnorrKeys(bytes32)`](schnorrKeysCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct schnorrKeysReturn {
        #[allow(missing_docs)]
        pub used: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<32>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<schnorrKeysCall> for UnderlyingRustTuple<'_> {
                fn from(value: schnorrKeysCall) -> Self {
                    (value.schnorrKey,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for schnorrKeysCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { schnorrKey: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<schnorrKeysReturn> for UnderlyingRustTuple<'_> {
                fn from(value: schnorrKeysReturn) -> Self {
                    (value.used,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for schnorrKeysReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { used: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for schnorrKeysCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = schnorrKeysReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "schnorrKeys(bytes32)";
            const SELECTOR: [u8; 4] = [95u8, 135u8, 84u8, 166u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.schnorrKey),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `supportsInterface(bytes4)` and selector `0x01ffc9a7`.
```solidity
function supportsInterface(bytes4 interfaceId) external view returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct supportsInterfaceCall {
        #[allow(missing_docs)]
        pub interfaceId: alloy::sol_types::private::FixedBytes<4>,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`supportsInterface(bytes4)`](supportsInterfaceCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct supportsInterfaceReturn {
        #[allow(missing_docs)]
        pub _0: bool,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::FixedBytes<4>,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<supportsInterfaceCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: supportsInterfaceCall) -> Self {
                    (value.interfaceId,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for supportsInterfaceCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { interfaceId: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (bool,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<supportsInterfaceReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: supportsInterfaceReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for supportsInterfaceReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for supportsInterfaceCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::FixedBytes<4>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = supportsInterfaceReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "supportsInterface(bytes4)";
            const SELECTOR: [u8; 4] = [1u8, 255u8, 201u8, 167u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        4,
                    > as alloy_sol_types::SolType>::tokenize(&self.interfaceId),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `token()` and selector `0xfc0c546a`.
```solidity
function token() external view returns (address);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct tokenCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`token()`](tokenCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct tokenReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::Address,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<tokenCall> for UnderlyingRustTuple<'_> {
                fn from(value: tokenCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for tokenCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<tokenReturn> for UnderlyingRustTuple<'_> {
                fn from(value: tokenReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for tokenReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for tokenCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = tokenReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "token()";
            const SELECTOR: [u8; 4] = [252u8, 12u8, 84u8, 106u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `transferOwnership(address)` and selector `0xf2fde38b`.
```solidity
function transferOwnership(address newOwner) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct transferOwnershipCall {
        #[allow(missing_docs)]
        pub newOwner: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`transferOwnership(address)`](transferOwnershipCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct transferOwnershipReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<transferOwnershipCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: transferOwnershipCall) -> Self {
                    (value.newOwner,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for transferOwnershipCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { newOwner: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<transferOwnershipReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: transferOwnershipReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for transferOwnershipReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for transferOwnershipCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = transferOwnershipReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "transferOwnership(address)";
            const SELECTOR: [u8; 4] = [242u8, 253u8, 227u8, 139u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.newOwner,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `undelegate(address,uint256)` and selector `0x4d99dd16`.
```solidity
function undelegate(address validator, uint256 amount) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegateCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`undelegate(address,uint256)`](undelegateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegateReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegateCall> for UnderlyingRustTuple<'_> {
                fn from(value: undelegateCall) -> Self {
                    (value.validator, value.amount)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        validator: tuple.0,
                        amount: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: undelegateReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for undelegateCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegateReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "undelegate(address,uint256)";
            const SELECTOR: [u8; 4] = [77u8, 153u8, 221u8, 22u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `undelegations(address,address)` and selector `0xa2d78dd5`.
```solidity
function undelegations(address validator, address delegator) external view returns (uint256 amount, uint256 unlocksAt);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegationsCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`undelegations(address,address)`](undelegationsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegationsReturn {
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub unlocksAt: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Address,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegationsCall> for UnderlyingRustTuple<'_> {
                fn from(value: undelegationsCall) -> Self {
                    (value.validator, value.delegator)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegationsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        validator: tuple.0,
                        delegator: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegationsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: undelegationsReturn) -> Self {
                    (value.amount, value.unlocksAt)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegationsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        amount: tuple.0,
                        unlocksAt: tuple.1,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for undelegationsCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegationsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "undelegations(address,address)";
            const SELECTOR: [u8; 4] = [162u8, 215u8, 141u8, 213u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.delegator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `unpause()` and selector `0x3f4ba83a`.
```solidity
function unpause() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct unpauseCall {}
    ///Container type for the return parameters of the [`unpause()`](unpauseCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct unpauseReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<unpauseCall> for UnderlyingRustTuple<'_> {
                fn from(value: unpauseCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for unpauseCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<unpauseReturn> for UnderlyingRustTuple<'_> {
                fn from(value: unpauseReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for unpauseReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for unpauseCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = unpauseReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "unpause()";
            const SELECTOR: [u8; 4] = [63u8, 75u8, 168u8, 58u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                ()
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive()]
    /**Function with signature `updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))` and selector `0x5544c2f1`.
```solidity
function updateConsensusKeys(BN254.G2Point memory, EdOnBN254.EdOnBN254Point memory, BN254.G1Point memory) external pure;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateConsensusKeysCall {
        #[allow(missing_docs)]
        pub _0: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _1: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))`](updateConsensusKeysCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateConsensusKeysReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
                <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysCall) -> Self {
                    (value._0, value._1, value._2)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateConsensusKeysCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
                        _2: tuple.2,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateConsensusKeysReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateConsensusKeysCall {
            type Parameters<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateConsensusKeysReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))";
            const SELECTOR: [u8; 4] = [85u8, 68u8, 194u8, 241u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self._0),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self._2),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive()]
    /**Function with signature `updateConsensusKeysV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes)` and selector `0xac5c2ad0`.
```solidity
function updateConsensusKeysV2(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, bytes memory schnorrSig) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateConsensusKeysV2Call {
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrSig: alloy::sol_types::private::Bytes,
    }
    ///Container type for the return parameters of the [`updateConsensusKeysV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes)`](updateConsensusKeysV2Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateConsensusKeysV2Return {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
                <BN254::G1Point as alloy::sol_types::SolType>::RustType,
                alloy::sol_types::private::Bytes,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysV2Call>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysV2Call) -> Self {
                    (value.blsVK, value.schnorrVK, value.blsSig, value.schnorrSig)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateConsensusKeysV2Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blsVK: tuple.0,
                        schnorrVK: tuple.1,
                        blsSig: tuple.2,
                        schnorrSig: tuple.3,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysV2Return>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysV2Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateConsensusKeysV2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateConsensusKeysV2Call {
            type Parameters<'a> = (
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                BN254::G1Point,
                alloy::sol_types::sol_data::Bytes,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateConsensusKeysV2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "updateConsensusKeysV2((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),bytes)";
            const SELECTOR: [u8; 4] = [172u8, 92u8, 42u8, 208u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.blsSig),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrSig,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `updateExitEscrowPeriod(uint64)` and selector `0x1a20cd63`.
```solidity
function updateExitEscrowPeriod(uint64 newExitEscrowPeriod) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateExitEscrowPeriodCall {
        #[allow(missing_docs)]
        pub newExitEscrowPeriod: u64,
    }
    ///Container type for the return parameters of the [`updateExitEscrowPeriod(uint64)`](updateExitEscrowPeriodCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateExitEscrowPeriodReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u64,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateExitEscrowPeriodCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateExitEscrowPeriodCall) -> Self {
                    (value.newExitEscrowPeriod,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateExitEscrowPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newExitEscrowPeriod: tuple.0,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateExitEscrowPeriodReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateExitEscrowPeriodReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateExitEscrowPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateExitEscrowPeriodCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateExitEscrowPeriodReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "updateExitEscrowPeriod(uint64)";
            const SELECTOR: [u8; 4] = [26u8, 32u8, 205u8, 99u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.newExitEscrowPeriod),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `upgradeToAndCall(address,bytes)` and selector `0x4f1ef286`.
```solidity
function upgradeToAndCall(address newImplementation, bytes memory data) external payable;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct upgradeToAndCallCall {
        #[allow(missing_docs)]
        pub newImplementation: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub data: alloy::sol_types::private::Bytes,
    }
    ///Container type for the return parameters of the [`upgradeToAndCall(address,bytes)`](upgradeToAndCallCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct upgradeToAndCallReturn {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Bytes,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::Bytes,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<upgradeToAndCallCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: upgradeToAndCallCall) -> Self {
                    (value.newImplementation, value.data)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for upgradeToAndCallCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newImplementation: tuple.0,
                        data: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<upgradeToAndCallReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: upgradeToAndCallReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for upgradeToAndCallReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for upgradeToAndCallCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Bytes,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = upgradeToAndCallReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "upgradeToAndCall(address,bytes)";
            const SELECTOR: [u8; 4] = [79u8, 30u8, 242u8, 134u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.newImplementation,
                    ),
                    <alloy::sol_types::sol_data::Bytes as alloy_sol_types::SolType>::tokenize(
                        &self.data,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `validatorExits(address)` and selector `0xb5ecb344`.
```solidity
function validatorExits(address validator) external view returns (uint256 unlocksAt);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorExitsCall {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`validatorExits(address)`](validatorExitsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorExitsReturn {
        #[allow(missing_docs)]
        pub unlocksAt: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorExitsCall> for UnderlyingRustTuple<'_> {
                fn from(value: validatorExitsCall) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorExitsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { validator: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorExitsReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: validatorExitsReturn) -> Self {
                    (value.unlocksAt,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for validatorExitsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { unlocksAt: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for validatorExitsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = validatorExitsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "validatorExits(address)";
            const SELECTOR: [u8; 4] = [181u8, 236u8, 179u8, 68u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `validators(address)` and selector `0xfa52c7d8`.
```solidity
function validators(address account) external view returns (uint256 delegatedAmount, StakeTable.ValidatorStatus status);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`validators(address)`](validatorsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsReturn {
        #[allow(missing_docs)]
        pub delegatedAmount: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub status: <StakeTable::ValidatorStatus as alloy::sol_types::SolType>::RustType,
    }
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorsCall> for UnderlyingRustTuple<'_> {
                fn from(value: validatorsCall) -> Self {
                    (value.account,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { account: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                StakeTable::ValidatorStatus,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
                <StakeTable::ValidatorStatus as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(
                _t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>,
            ) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: validatorsReturn) -> Self {
                    (value.delegatedAmount, value.status)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        delegatedAmount: tuple.0,
                        status: tuple.1,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for validatorsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = validatorsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                StakeTable::ValidatorStatus,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "validators(address)";
            const SELECTOR: [u8; 4] = [250u8, 82u8, 199u8, 216u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    ///Container for all the [`StakeTableV2`](self) function calls.
    #[derive()]
    pub enum StakeTableV2Calls {
        #[allow(missing_docs)]
        DEFAULT_ADMIN_ROLE(DEFAULT_ADMIN_ROLECall),
        #[allow(missing_docs)]
        PAUSER_ROLE(PAUSER_ROLECall),
        #[allow(missing_docs)]
        UPGRADE_INTERFACE_VERSION(UPGRADE_INTERFACE_VERSIONCall),
        #[allow(missing_docs)]
        _hashBlsKey(_hashBlsKeyCall),
        #[allow(missing_docs)]
        blsKeys(blsKeysCall),
        #[allow(missing_docs)]
        claimValidatorExit(claimValidatorExitCall),
        #[allow(missing_docs)]
        claimWithdrawal(claimWithdrawalCall),
        #[allow(missing_docs)]
        delegate(delegateCall),
        #[allow(missing_docs)]
        delegations(delegationsCall),
        #[allow(missing_docs)]
        deregisterValidator(deregisterValidatorCall),
        #[allow(missing_docs)]
        exitEscrowPeriod(exitEscrowPeriodCall),
        #[allow(missing_docs)]
        getRoleAdmin(getRoleAdminCall),
        #[allow(missing_docs)]
        getVersion(getVersionCall),
        #[allow(missing_docs)]
        grantRole(grantRoleCall),
        #[allow(missing_docs)]
        hasRole(hasRoleCall),
        #[allow(missing_docs)]
        initialize(initializeCall),
        #[allow(missing_docs)]
        initializeV2(initializeV2Call),
        #[allow(missing_docs)]
        initializedAtBlock(initializedAtBlockCall),
        #[allow(missing_docs)]
        lightClient(lightClientCall),
        #[allow(missing_docs)]
        owner(ownerCall),
        #[allow(missing_docs)]
        pause(pauseCall),
        #[allow(missing_docs)]
        paused(pausedCall),
        #[allow(missing_docs)]
        proxiableUUID(proxiableUUIDCall),
        #[allow(missing_docs)]
        registerValidator(registerValidatorCall),
        #[allow(missing_docs)]
        registerValidatorV2(registerValidatorV2Call),
        #[allow(missing_docs)]
        renounceOwnership(renounceOwnershipCall),
        #[allow(missing_docs)]
        renounceRole(renounceRoleCall),
        #[allow(missing_docs)]
        revokeRole(revokeRoleCall),
        #[allow(missing_docs)]
        schnorrKeys(schnorrKeysCall),
        #[allow(missing_docs)]
        supportsInterface(supportsInterfaceCall),
        #[allow(missing_docs)]
        token(tokenCall),
        #[allow(missing_docs)]
        transferOwnership(transferOwnershipCall),
        #[allow(missing_docs)]
        undelegate(undelegateCall),
        #[allow(missing_docs)]
        undelegations(undelegationsCall),
        #[allow(missing_docs)]
        unpause(unpauseCall),
        #[allow(missing_docs)]
        updateConsensusKeys(updateConsensusKeysCall),
        #[allow(missing_docs)]
        updateConsensusKeysV2(updateConsensusKeysV2Call),
        #[allow(missing_docs)]
        updateExitEscrowPeriod(updateExitEscrowPeriodCall),
        #[allow(missing_docs)]
        upgradeToAndCall(upgradeToAndCallCall),
        #[allow(missing_docs)]
        validatorExits(validatorExitsCall),
        #[allow(missing_docs)]
        validators(validatorsCall),
    }
    #[automatically_derived]
    impl StakeTableV2Calls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 255u8, 201u8, 167u8],
            [2u8, 110u8, 64u8, 43u8],
            [8u8, 64u8, 186u8, 114u8],
            [13u8, 142u8, 110u8, 44u8],
            [19u8, 185u8, 5u8, 122u8],
            [26u8, 32u8, 205u8, 99u8],
            [33u8, 64u8, 254u8, 205u8],
            [36u8, 138u8, 156u8, 163u8],
            [47u8, 47u8, 241u8, 93u8],
            [48u8, 66u8, 64u8, 191u8],
            [54u8, 86u8, 138u8, 190u8],
            [62u8, 157u8, 249u8, 181u8],
            [63u8, 75u8, 168u8, 58u8],
            [77u8, 153u8, 221u8, 22u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [85u8, 68u8, 194u8, 241u8],
            [92u8, 151u8, 90u8, 187u8],
            [95u8, 135u8, 84u8, 166u8],
            [106u8, 145u8, 28u8, 207u8],
            [113u8, 80u8, 24u8, 166u8],
            [132u8, 86u8, 203u8, 89u8],
            [141u8, 165u8, 203u8, 91u8],
            [145u8, 209u8, 72u8, 84u8],
            [155u8, 48u8, 165u8, 230u8],
            [158u8, 154u8, 143u8, 49u8],
            [162u8, 23u8, 253u8, 223u8],
            [162u8, 215u8, 141u8, 213u8],
            [163u8, 6u8, 106u8, 171u8],
            [172u8, 92u8, 42u8, 208u8],
            [173u8, 60u8, 177u8, 204u8],
            [179u8, 230u8, 235u8, 213u8],
            [181u8, 112u8, 14u8, 104u8],
            [181u8, 236u8, 179u8, 68u8],
            [190u8, 32u8, 48u8, 148u8],
            [198u8, 72u8, 20u8, 221u8],
            [213u8, 71u8, 116u8, 31u8],
            [230u8, 58u8, 177u8, 233u8],
            [242u8, 253u8, 227u8, 139u8],
            [250u8, 82u8, 199u8, 216u8],
            [252u8, 12u8, 84u8, 106u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableV2Calls {
        const NAME: &'static str = "StakeTableV2Calls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 41usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::DEFAULT_ADMIN_ROLE(_) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::PAUSER_ROLE(_) => {
                    <PAUSER_ROLECall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::_hashBlsKey(_) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::blsKeys(_) => <blsKeysCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::claimValidatorExit(_) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::claimWithdrawal(_) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::delegate(_) => <delegateCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegations(_) => {
                    <delegationsCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::deregisterValidator(_) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::exitEscrowPeriod(_) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::getRoleAdmin(_) => {
                    <getRoleAdminCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::getVersion(_) => {
                    <getVersionCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::grantRole(_) => {
                    <grantRoleCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::hasRole(_) => <hasRoleCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initialize(_) => {
                    <initializeCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initializeV2(_) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initializedAtBlock(_) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::lightClient(_) => {
                    <lightClientCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::owner(_) => <ownerCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::pause(_) => <pauseCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::paused(_) => <pausedCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::proxiableUUID(_) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::registerValidator(_) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::registerValidatorV2(_) => {
                    <registerValidatorV2Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::renounceOwnership(_) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::renounceRole(_) => {
                    <renounceRoleCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::revokeRole(_) => {
                    <revokeRoleCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::schnorrKeys(_) => {
                    <schnorrKeysCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::supportsInterface(_) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::token(_) => <tokenCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::transferOwnership(_) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::undelegate(_) => {
                    <undelegateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::undelegations(_) => {
                    <undelegationsCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::unpause(_) => <unpauseCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::updateConsensusKeys(_) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::updateConsensusKeysV2(_) => {
                    <updateConsensusKeysV2Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::updateExitEscrowPeriod(_) => {
                    <updateExitEscrowPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::upgradeToAndCall(_) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::validatorExits(_) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::validators(_) => {
                    <validatorsCall as alloy_sol_types::SolCall>::SELECTOR
                }
            }
        }
        #[inline]
        fn selector_at(i: usize) -> ::core::option::Option<[u8; 4]> {
            Self::SELECTORS.get(i).copied()
        }
        #[inline]
        fn valid_selector(selector: [u8; 4]) -> bool {
            Self::SELECTORS.binary_search(&selector).is_ok()
        }
        #[inline]
        #[allow(non_snake_case)]
        fn abi_decode_raw(
            selector: [u8; 4],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
                bool,
            ) -> alloy_sol_types::Result<StakeTableV2Calls>] = &[
                {
                    fn supportsInterface(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::supportsInterface)
                    }
                    supportsInterface
                },
                {
                    fn delegate(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <delegateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::delegate)
                    }
                    delegate
                },
                {
                    fn initializeV2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <initializeV2Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::initializeV2)
                    }
                    initializeV2
                },
                {
                    fn getVersion(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::getVersion)
                    }
                    getVersion
                },
                {
                    fn registerValidator(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <registerValidatorCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::registerValidator)
                    }
                    registerValidator
                },
                {
                    fn updateExitEscrowPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <updateExitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::updateExitEscrowPeriod)
                    }
                    updateExitEscrowPeriod
                },
                {
                    fn claimValidatorExit(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::claimValidatorExit)
                    }
                    claimValidatorExit
                },
                {
                    fn getRoleAdmin(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <getRoleAdminCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::getRoleAdmin)
                    }
                    getRoleAdmin
                },
                {
                    fn grantRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <grantRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::grantRole)
                    }
                    grantRole
                },
                {
                    fn registerValidatorV2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <registerValidatorV2Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::registerValidatorV2)
                    }
                    registerValidatorV2
                },
                {
                    fn renounceRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <renounceRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::renounceRole)
                    }
                    renounceRole
                },
                {
                    fn initializedAtBlock(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::initializedAtBlock)
                    }
                    initializedAtBlock
                },
                {
                    fn unpause(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <unpauseCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::unpause)
                    }
                    unpause
                },
                {
                    fn undelegate(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <undelegateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::undelegate)
                    }
                    undelegate
                },
                {
                    fn upgradeToAndCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::upgradeToAndCall)
                    }
                    upgradeToAndCall
                },
                {
                    fn proxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::proxiableUUID)
                    }
                    proxiableUUID
                },
                {
                    fn updateConsensusKeys(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::updateConsensusKeys)
                    }
                    updateConsensusKeys
                },
                {
                    fn paused(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <pausedCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::paused)
                    }
                    paused
                },
                {
                    fn schnorrKeys(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <schnorrKeysCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::schnorrKeys)
                    }
                    schnorrKeys
                },
                {
                    fn deregisterValidator(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::deregisterValidator)
                    }
                    deregisterValidator
                },
                {
                    fn renounceOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::renounceOwnership)
                    }
                    renounceOwnership
                },
                {
                    fn pause(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <pauseCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::pause)
                    }
                    pause
                },
                {
                    fn owner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::owner)
                    }
                    owner
                },
                {
                    fn hasRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <hasRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::hasRole)
                    }
                    hasRole
                },
                {
                    fn _hashBlsKey(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::_hashBlsKey)
                    }
                    _hashBlsKey
                },
                {
                    fn exitEscrowPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::exitEscrowPeriod)
                    }
                    exitEscrowPeriod
                },
                {
                    fn DEFAULT_ADMIN_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::DEFAULT_ADMIN_ROLE)
                    }
                    DEFAULT_ADMIN_ROLE
                },
                {
                    fn undelegations(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <undelegationsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::undelegations)
                    }
                    undelegations
                },
                {
                    fn claimWithdrawal(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::claimWithdrawal)
                    }
                    claimWithdrawal
                },
                {
                    fn updateConsensusKeysV2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <updateConsensusKeysV2Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::updateConsensusKeysV2)
                    }
                    updateConsensusKeysV2
                },
                {
                    fn UPGRADE_INTERFACE_VERSION(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::UPGRADE_INTERFACE_VERSION)
                    }
                    UPGRADE_INTERFACE_VERSION
                },
                {
                    fn blsKeys(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <blsKeysCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::blsKeys)
                    }
                    blsKeys
                },
                {
                    fn lightClient(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <lightClientCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::lightClient)
                    }
                    lightClient
                },
                {
                    fn validatorExits(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <validatorExitsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::validatorExits)
                    }
                    validatorExits
                },
                {
                    fn initialize(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::initialize)
                    }
                    initialize
                },
                {
                    fn delegations(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <delegationsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::delegations)
                    }
                    delegations
                },
                {
                    fn revokeRole(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <revokeRoleCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::revokeRole)
                    }
                    revokeRole
                },
                {
                    fn PAUSER_ROLE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <PAUSER_ROLECall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::PAUSER_ROLE)
                    }
                    PAUSER_ROLE
                },
                {
                    fn transferOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::transferOwnership)
                    }
                    transferOwnership
                },
                {
                    fn validators(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <validatorsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::validators)
                    }
                    validators
                },
                {
                    fn token(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Calls> {
                        <tokenCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Calls::token)
                    }
                    token
                },
            ];
            let Ok(idx) = Self::SELECTORS.binary_search(&selector) else {
                return Err(
                    alloy_sol_types::Error::unknown_selector(
                        <Self as alloy_sol_types::SolInterface>::NAME,
                        selector,
                    ),
                );
            };
            DECODE_SHIMS[idx](data, validate)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::DEFAULT_ADMIN_ROLE(inner) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::PAUSER_ROLE(inner) => {
                    <PAUSER_ROLECall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::UPGRADE_INTERFACE_VERSION(inner) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::blsKeys(inner) => {
                    <blsKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::delegations(inner) => {
                    <delegationsCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::getRoleAdmin(inner) => {
                    <getRoleAdminCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::grantRole(inner) => {
                    <grantRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::hasRole(inner) => {
                    <hasRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::initializeV2(inner) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::lightClient(inner) => {
                    <lightClientCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::pause(inner) => {
                    <pauseCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::paused(inner) => {
                    <pausedCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::registerValidatorV2(inner) => {
                    <registerValidatorV2Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::renounceRole(inner) => {
                    <renounceRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::revokeRole(inner) => {
                    <revokeRoleCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::schnorrKeys(inner) => {
                    <schnorrKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::supportsInterface(inner) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::token(inner) => {
                    <tokenCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::undelegations(inner) => {
                    <undelegationsCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::unpause(inner) => {
                    <unpauseCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::updateConsensusKeysV2(inner) => {
                    <updateConsensusKeysV2Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::updateExitEscrowPeriod(inner) => {
                    <updateExitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::DEFAULT_ADMIN_ROLE(inner) => {
                    <DEFAULT_ADMIN_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::PAUSER_ROLE(inner) => {
                    <PAUSER_ROLECall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::UPGRADE_INTERFACE_VERSION(inner) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::blsKeys(inner) => {
                    <blsKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::delegations(inner) => {
                    <delegationsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::getRoleAdmin(inner) => {
                    <getRoleAdminCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::grantRole(inner) => {
                    <grantRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::hasRole(inner) => {
                    <hasRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::initializeV2(inner) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::lightClient(inner) => {
                    <lightClientCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::pause(inner) => {
                    <pauseCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::paused(inner) => {
                    <pausedCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::registerValidatorV2(inner) => {
                    <registerValidatorV2Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::renounceRole(inner) => {
                    <renounceRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::revokeRole(inner) => {
                    <revokeRoleCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::schnorrKeys(inner) => {
                    <schnorrKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::supportsInterface(inner) => {
                    <supportsInterfaceCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::token(inner) => {
                    <tokenCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::undelegations(inner) => {
                    <undelegationsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::unpause(inner) => {
                    <unpauseCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::updateConsensusKeysV2(inner) => {
                    <updateConsensusKeysV2Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::updateExitEscrowPeriod(inner) => {
                    <updateExitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
            }
        }
    }
    ///Container for all the [`StakeTableV2`](self) custom errors.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum StakeTableV2Errors {
        #[allow(missing_docs)]
        AccessControlBadConfirmation(AccessControlBadConfirmation),
        #[allow(missing_docs)]
        AccessControlUnauthorizedAccount(AccessControlUnauthorizedAccount),
        #[allow(missing_docs)]
        AddressEmptyCode(AddressEmptyCode),
        #[allow(missing_docs)]
        BLSSigVerificationFailed(BLSSigVerificationFailed),
        #[allow(missing_docs)]
        BlsKeyAlreadyUsed(BlsKeyAlreadyUsed),
        #[allow(missing_docs)]
        DeprecatedFunction(DeprecatedFunction),
        #[allow(missing_docs)]
        ERC1967InvalidImplementation(ERC1967InvalidImplementation),
        #[allow(missing_docs)]
        ERC1967NonPayable(ERC1967NonPayable),
        #[allow(missing_docs)]
        EnforcedPause(EnforcedPause),
        #[allow(missing_docs)]
        ExitEscrowPeriodInvalid(ExitEscrowPeriodInvalid),
        #[allow(missing_docs)]
        ExpectedPause(ExpectedPause),
        #[allow(missing_docs)]
        FailedInnerCall(FailedInnerCall),
        #[allow(missing_docs)]
        InsufficientAllowance(InsufficientAllowance),
        #[allow(missing_docs)]
        InsufficientBalance(InsufficientBalance),
        #[allow(missing_docs)]
        InvalidCommission(InvalidCommission),
        #[allow(missing_docs)]
        InvalidInitialization(InvalidInitialization),
        #[allow(missing_docs)]
        InvalidSchnorrSig(InvalidSchnorrSig),
        #[allow(missing_docs)]
        InvalidSchnorrVK(InvalidSchnorrVK),
        #[allow(missing_docs)]
        NotInitializing(NotInitializing),
        #[allow(missing_docs)]
        NothingToWithdraw(NothingToWithdraw),
        #[allow(missing_docs)]
        OwnableInvalidOwner(OwnableInvalidOwner),
        #[allow(missing_docs)]
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
        #[allow(missing_docs)]
        PrematureWithdrawal(PrematureWithdrawal),
        #[allow(missing_docs)]
        SchnorrKeyAlreadyUsed(SchnorrKeyAlreadyUsed),
        #[allow(missing_docs)]
        UUPSUnauthorizedCallContext(UUPSUnauthorizedCallContext),
        #[allow(missing_docs)]
        UUPSUnsupportedProxiableUUID(UUPSUnsupportedProxiableUUID),
        #[allow(missing_docs)]
        UndelegationAlreadyExists(UndelegationAlreadyExists),
        #[allow(missing_docs)]
        ValidatorAlreadyExited(ValidatorAlreadyExited),
        #[allow(missing_docs)]
        ValidatorAlreadyRegistered(ValidatorAlreadyRegistered),
        #[allow(missing_docs)]
        ValidatorInactive(ValidatorInactive),
        #[allow(missing_docs)]
        ValidatorNotExited(ValidatorNotExited),
        #[allow(missing_docs)]
        ZeroAddress(ZeroAddress),
        #[allow(missing_docs)]
        ZeroAmount(ZeroAmount),
    }
    #[automatically_derived]
    impl StakeTableV2Errors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 181u8, 20u8, 174u8],
            [6u8, 207u8, 67u8, 143u8],
            [12u8, 237u8, 62u8, 80u8],
            [17u8, 140u8, 218u8, 167u8],
            [20u8, 37u8, 234u8, 66u8],
            [30u8, 79u8, 189u8, 247u8],
            [31u8, 42u8, 32u8, 5u8],
            [42u8, 27u8, 45u8, 216u8],
            [66u8, 71u8, 6u8, 135u8],
            [76u8, 156u8, 140u8, 227u8],
            [80u8, 138u8, 121u8, 63u8],
            [90u8, 119u8, 67u8, 87u8],
            [102u8, 151u8, 178u8, 50u8],
            [141u8, 252u8, 32u8, 43u8],
            [146u8, 102u8, 83u8, 81u8],
            [153u8, 115u8, 247u8, 216u8],
            [153u8, 150u8, 179u8, 21u8],
            [170u8, 29u8, 73u8, 164u8],
            [174u8, 73u8, 59u8, 3u8],
            [179u8, 152u8, 151u8, 159u8],
            [181u8, 126u8, 33u8, 223u8],
            [194u8, 215u8, 248u8, 19u8],
            [208u8, 208u8, 79u8, 96u8],
            [212u8, 35u8, 164u8, 241u8],
            [215u8, 230u8, 188u8, 248u8],
            [217u8, 46u8, 35u8, 61u8],
            [217u8, 60u8, 6u8, 101u8],
            [220u8, 129u8, 219u8, 133u8],
            [224u8, 124u8, 141u8, 186u8],
            [226u8, 81u8, 125u8, 63u8],
            [234u8, 180u8, 169u8, 99u8],
            [242u8, 83u8, 20u8, 166u8],
            [249u8, 46u8, 232u8, 169u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableV2Errors {
        const NAME: &'static str = "StakeTableV2Errors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 33usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AccessControlBadConfirmation(_) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::SELECTOR
                }
                Self::AccessControlUnauthorizedAccount(_) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                }
                Self::AddressEmptyCode(_) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::SELECTOR
                }
                Self::BLSSigVerificationFailed(_) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::SELECTOR
                }
                Self::BlsKeyAlreadyUsed(_) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::SELECTOR
                }
                Self::DeprecatedFunction(_) => {
                    <DeprecatedFunction as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ERC1967InvalidImplementation(_) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ERC1967NonPayable(_) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::SELECTOR
                }
                Self::EnforcedPause(_) => {
                    <EnforcedPause as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ExitEscrowPeriodInvalid(_) => {
                    <ExitEscrowPeriodInvalid as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ExpectedPause(_) => {
                    <ExpectedPause as alloy_sol_types::SolError>::SELECTOR
                }
                Self::FailedInnerCall(_) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InsufficientAllowance(_) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InsufficientBalance(_) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidCommission(_) => {
                    <InvalidCommission as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidInitialization(_) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidSchnorrSig(_) => {
                    <InvalidSchnorrSig as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidSchnorrVK(_) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::SELECTOR
                }
                Self::NotInitializing(_) => {
                    <NotInitializing as alloy_sol_types::SolError>::SELECTOR
                }
                Self::NothingToWithdraw(_) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::SELECTOR
                }
                Self::OwnableInvalidOwner(_) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::SELECTOR
                }
                Self::OwnableUnauthorizedAccount(_) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                }
                Self::PrematureWithdrawal(_) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::SELECTOR
                }
                Self::SchnorrKeyAlreadyUsed(_) => {
                    <SchnorrKeyAlreadyUsed as alloy_sol_types::SolError>::SELECTOR
                }
                Self::UUPSUnauthorizedCallContext(_) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::SELECTOR
                }
                Self::UUPSUnsupportedProxiableUUID(_) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::SELECTOR
                }
                Self::UndelegationAlreadyExists(_) => {
                    <UndelegationAlreadyExists as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorAlreadyExited(_) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorAlreadyRegistered(_) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorInactive(_) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorNotExited(_) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ZeroAddress(_) => {
                    <ZeroAddress as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ZeroAmount(_) => {
                    <ZeroAmount as alloy_sol_types::SolError>::SELECTOR
                }
            }
        }
        #[inline]
        fn selector_at(i: usize) -> ::core::option::Option<[u8; 4]> {
            Self::SELECTORS.get(i).copied()
        }
        #[inline]
        fn valid_selector(selector: [u8; 4]) -> bool {
            Self::SELECTORS.binary_search(&selector).is_ok()
        }
        #[inline]
        #[allow(non_snake_case)]
        fn abi_decode_raw(
            selector: [u8; 4],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
                bool,
            ) -> alloy_sol_types::Result<StakeTableV2Errors>] = &[
                {
                    fn BlsKeyAlreadyUsed(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::BlsKeyAlreadyUsed)
                    }
                    BlsKeyAlreadyUsed
                },
                {
                    fn InvalidSchnorrVK(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InvalidSchnorrVK)
                    }
                    InvalidSchnorrVK
                },
                {
                    fn BLSSigVerificationFailed(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::BLSSigVerificationFailed)
                    }
                    BLSSigVerificationFailed
                },
                {
                    fn OwnableUnauthorizedAccount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::OwnableUnauthorizedAccount)
                    }
                    OwnableUnauthorizedAccount
                },
                {
                    fn FailedInnerCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn OwnableInvalidOwner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::OwnableInvalidOwner)
                    }
                    OwnableInvalidOwner
                },
                {
                    fn ZeroAmount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ZeroAmount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ZeroAmount)
                    }
                    ZeroAmount
                },
                {
                    fn InsufficientAllowance(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InsufficientAllowance as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InsufficientAllowance)
                    }
                    InsufficientAllowance
                },
                {
                    fn InvalidSchnorrSig(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InvalidSchnorrSig as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InvalidSchnorrSig)
                    }
                    InvalidSchnorrSig
                },
                {
                    fn ERC1967InvalidImplementation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ERC1967InvalidImplementation)
                    }
                    ERC1967InvalidImplementation
                },
                {
                    fn ValidatorInactive(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ValidatorInactive as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ValidatorInactive)
                    }
                    ValidatorInactive
                },
                {
                    fn PrematureWithdrawal(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <PrematureWithdrawal as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::PrematureWithdrawal)
                    }
                    PrematureWithdrawal
                },
                {
                    fn AccessControlBadConfirmation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::AccessControlBadConfirmation)
                    }
                    AccessControlBadConfirmation
                },
                {
                    fn ExpectedPause(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ExpectedPause as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ExpectedPause)
                    }
                    ExpectedPause
                },
                {
                    fn InsufficientBalance(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InsufficientBalance as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InsufficientBalance)
                    }
                    InsufficientBalance
                },
                {
                    fn ValidatorAlreadyRegistered(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ValidatorAlreadyRegistered)
                    }
                    ValidatorAlreadyRegistered
                },
                {
                    fn AddressEmptyCode(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::AddressEmptyCode)
                    }
                    AddressEmptyCode
                },
                {
                    fn UUPSUnsupportedProxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::UUPSUnsupportedProxiableUUID)
                    }
                    UUPSUnsupportedProxiableUUID
                },
                {
                    fn SchnorrKeyAlreadyUsed(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <SchnorrKeyAlreadyUsed as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::SchnorrKeyAlreadyUsed)
                    }
                    SchnorrKeyAlreadyUsed
                },
                {
                    fn ERC1967NonPayable(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ERC1967NonPayable)
                    }
                    ERC1967NonPayable
                },
                {
                    fn ExitEscrowPeriodInvalid(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ExitEscrowPeriodInvalid as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ExitEscrowPeriodInvalid)
                    }
                    ExitEscrowPeriodInvalid
                },
                {
                    fn DeprecatedFunction(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <DeprecatedFunction as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::DeprecatedFunction)
                    }
                    DeprecatedFunction
                },
                {
                    fn NothingToWithdraw(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <NothingToWithdraw as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::NothingToWithdraw)
                    }
                    NothingToWithdraw
                },
                {
                    fn UndelegationAlreadyExists(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <UndelegationAlreadyExists as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::UndelegationAlreadyExists)
                    }
                    UndelegationAlreadyExists
                },
                {
                    fn NotInitializing(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::NotInitializing)
                    }
                    NotInitializing
                },
                {
                    fn ZeroAddress(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ZeroAddress as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ZeroAddress)
                    }
                    ZeroAddress
                },
                {
                    fn EnforcedPause(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <EnforcedPause as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::EnforcedPause)
                    }
                    EnforcedPause
                },
                {
                    fn InvalidCommission(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InvalidCommission as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InvalidCommission)
                    }
                    InvalidCommission
                },
                {
                    fn UUPSUnauthorizedCallContext(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::UUPSUnauthorizedCallContext)
                    }
                    UUPSUnauthorizedCallContext
                },
                {
                    fn AccessControlUnauthorizedAccount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::AccessControlUnauthorizedAccount)
                    }
                    AccessControlUnauthorizedAccount
                },
                {
                    fn ValidatorAlreadyExited(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ValidatorAlreadyExited)
                    }
                    ValidatorAlreadyExited
                },
                {
                    fn ValidatorNotExited(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <ValidatorNotExited as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::ValidatorNotExited)
                    }
                    ValidatorNotExited
                },
                {
                    fn InvalidInitialization(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableV2Errors> {
                        <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableV2Errors::InvalidInitialization)
                    }
                    InvalidInitialization
                },
            ];
            let Ok(idx) = Self::SELECTORS.binary_search(&selector) else {
                return Err(
                    alloy_sol_types::Error::unknown_selector(
                        <Self as alloy_sol_types::SolInterface>::NAME,
                        selector,
                    ),
                );
            };
            DECODE_SHIMS[idx](data, validate)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::AccessControlBadConfirmation(inner) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::AccessControlUnauthorizedAccount(inner) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::BlsKeyAlreadyUsed(inner) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::DeprecatedFunction(inner) => {
                    <DeprecatedFunction as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::EnforcedPause(inner) => {
                    <EnforcedPause as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::ExitEscrowPeriodInvalid(inner) => {
                    <ExitEscrowPeriodInvalid as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ExpectedPause(inner) => {
                    <ExpectedPause as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidSchnorrSig(inner) => {
                    <InvalidSchnorrSig as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::SchnorrKeyAlreadyUsed(inner) => {
                    <SchnorrKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::UUPSUnauthorizedCallContext(inner) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::UUPSUnsupportedProxiableUUID(inner) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::UndelegationAlreadyExists(inner) => {
                    <UndelegationAlreadyExists as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ZeroAddress(inner) => {
                    <ZeroAddress as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::ZeroAmount(inner) => {
                    <ZeroAmount as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::AccessControlBadConfirmation(inner) => {
                    <AccessControlBadConfirmation as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::AccessControlUnauthorizedAccount(inner) => {
                    <AccessControlUnauthorizedAccount as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::BlsKeyAlreadyUsed(inner) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::DeprecatedFunction(inner) => {
                    <DeprecatedFunction as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::EnforcedPause(inner) => {
                    <EnforcedPause as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ExitEscrowPeriodInvalid(inner) => {
                    <ExitEscrowPeriodInvalid as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ExpectedPause(inner) => {
                    <ExpectedPause as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidSchnorrSig(inner) => {
                    <InvalidSchnorrSig as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::SchnorrKeyAlreadyUsed(inner) => {
                    <SchnorrKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::UUPSUnauthorizedCallContext(inner) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::UUPSUnsupportedProxiableUUID(inner) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::UndelegationAlreadyExists(inner) => {
                    <UndelegationAlreadyExists as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ZeroAddress(inner) => {
                    <ZeroAddress as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::ZeroAmount(inner) => {
                    <ZeroAmount as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
            }
        }
    }
    ///Container for all the [`StakeTableV2`](self) events.
    #[derive()]
    pub enum StakeTableV2Events {
        #[allow(missing_docs)]
        ConsensusKeysUpdated(ConsensusKeysUpdated),
        #[allow(missing_docs)]
        ConsensusKeysUpdatedV2(ConsensusKeysUpdatedV2),
        #[allow(missing_docs)]
        Delegated(Delegated),
        #[allow(missing_docs)]
        ExitEscrowPeriodUpdated(ExitEscrowPeriodUpdated),
        #[allow(missing_docs)]
        Initialized(Initialized),
        #[allow(missing_docs)]
        OwnershipTransferred(OwnershipTransferred),
        #[allow(missing_docs)]
        Paused(Paused),
        #[allow(missing_docs)]
        RoleAdminChanged(RoleAdminChanged),
        #[allow(missing_docs)]
        RoleGranted(RoleGranted),
        #[allow(missing_docs)]
        RoleRevoked(RoleRevoked),
        #[allow(missing_docs)]
        Undelegated(Undelegated),
        #[allow(missing_docs)]
        Unpaused(Unpaused),
        #[allow(missing_docs)]
        Upgrade(Upgrade),
        #[allow(missing_docs)]
        Upgraded(Upgraded),
        #[allow(missing_docs)]
        ValidatorExit(ValidatorExit),
        #[allow(missing_docs)]
        ValidatorRegistered(ValidatorRegistered),
        #[allow(missing_docs)]
        ValidatorRegisteredV2(ValidatorRegisteredV2),
        #[allow(missing_docs)]
        Withdrawal(Withdrawal),
    }
    #[automatically_derived]
    impl StakeTableV2Events {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                47u8, 135u8, 136u8, 17u8, 126u8, 126u8, 255u8, 29u8, 130u8, 233u8, 38u8,
                236u8, 121u8, 73u8, 1u8, 209u8, 124u8, 120u8, 2u8, 74u8, 80u8, 39u8, 9u8,
                64u8, 48u8, 69u8, 64u8, 167u8, 51u8, 101u8, 111u8, 13u8,
            ],
            [
                77u8, 16u8, 189u8, 4u8, 151u8, 117u8, 199u8, 123u8, 215u8, 242u8, 85u8,
                25u8, 90u8, 251u8, 165u8, 8u8, 128u8, 40u8, 236u8, 179u8, 199u8, 194u8,
                119u8, 211u8, 147u8, 204u8, 255u8, 121u8, 52u8, 242u8, 249u8, 44u8,
            ],
            [
                93u8, 185u8, 238u8, 10u8, 73u8, 91u8, 242u8, 230u8, 255u8, 156u8, 145u8,
                167u8, 131u8, 76u8, 27u8, 164u8, 253u8, 210u8, 68u8, 165u8, 232u8, 170u8,
                78u8, 83u8, 123u8, 211u8, 138u8, 234u8, 228u8, 176u8, 115u8, 170u8,
            ],
            [
                98u8, 231u8, 140u8, 234u8, 1u8, 190u8, 227u8, 32u8, 205u8, 78u8, 66u8,
                2u8, 112u8, 181u8, 234u8, 116u8, 0u8, 13u8, 17u8, 176u8, 201u8, 247u8,
                71u8, 84u8, 235u8, 219u8, 252u8, 84u8, 75u8, 5u8, 162u8, 88u8,
            ],
            [
                121u8, 62u8, 59u8, 30u8, 27u8, 205u8, 103u8, 123u8, 177u8, 25u8, 0u8,
                200u8, 49u8, 36u8, 211u8, 196u8, 76u8, 153u8, 70u8, 234u8, 141u8, 223u8,
                151u8, 138u8, 12u8, 162u8, 80u8, 176u8, 52u8, 236u8, 157u8, 222u8,
            ],
            [
                127u8, 207u8, 83u8, 44u8, 21u8, 240u8, 166u8, 219u8, 11u8, 214u8, 208u8,
                224u8, 56u8, 190u8, 167u8, 29u8, 48u8, 216u8, 8u8, 199u8, 217u8, 140u8,
                179u8, 191u8, 114u8, 104u8, 169u8, 91u8, 245u8, 8u8, 27u8, 101u8,
            ],
            [
                128u8, 216u8, 164u8, 161u8, 102u8, 51u8, 40u8, 169u8, 152u8, 212u8, 85u8,
                91u8, 162u8, 29u8, 139u8, 186u8, 110u8, 241u8, 87u8, 106u8, 140u8, 94u8,
                157u8, 39u8, 249u8, 197u8, 69u8, 241u8, 163u8, 213u8, 43u8, 29u8,
            ],
            [
                139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8,
                31u8, 208u8, 164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8,
                218u8, 175u8, 227u8, 180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
            ],
            [
                188u8, 124u8, 215u8, 90u8, 32u8, 238u8, 39u8, 253u8, 154u8, 222u8, 186u8,
                179u8, 32u8, 65u8, 247u8, 85u8, 33u8, 77u8, 188u8, 107u8, 255u8, 169u8,
                12u8, 192u8, 34u8, 91u8, 57u8, 218u8, 46u8, 92u8, 45u8, 59u8,
            ],
            [
                189u8, 121u8, 184u8, 111u8, 254u8, 10u8, 184u8, 232u8, 119u8, 97u8, 81u8,
                81u8, 66u8, 23u8, 205u8, 124u8, 172u8, 213u8, 44u8, 144u8, 159u8, 102u8,
                71u8, 92u8, 58u8, 244u8, 78u8, 18u8, 159u8, 11u8, 0u8, 255u8,
            ],
            [
                199u8, 245u8, 5u8, 178u8, 243u8, 113u8, 174u8, 33u8, 117u8, 238u8, 73u8,
                19u8, 244u8, 73u8, 158u8, 31u8, 38u8, 51u8, 167u8, 181u8, 147u8, 99u8,
                33u8, 238u8, 209u8, 205u8, 174u8, 182u8, 17u8, 81u8, 129u8, 210u8,
            ],
            [
                200u8, 197u8, 179u8, 122u8, 236u8, 127u8, 45u8, 219u8, 211u8, 161u8,
                60u8, 81u8, 54u8, 30u8, 84u8, 160u8, 168u8, 223u8, 59u8, 202u8, 37u8,
                106u8, 183u8, 88u8, 167u8, 127u8, 90u8, 215u8, 65u8, 210u8, 129u8, 229u8,
            ],
            [
                229u8, 84u8, 26u8, 107u8, 97u8, 3u8, 212u8, 250u8, 126u8, 2u8, 30u8,
                213u8, 79u8, 173u8, 57u8, 198u8, 111u8, 39u8, 167u8, 107u8, 209u8, 61u8,
                55u8, 76u8, 246u8, 36u8, 10u8, 230u8, 189u8, 11u8, 183u8, 43u8,
            ],
            [
                240u8, 87u8, 212u8, 234u8, 129u8, 233u8, 134u8, 40u8, 101u8, 58u8, 143u8,
                144u8, 120u8, 133u8, 65u8, 151u8, 32u8, 120u8, 19u8, 115u8, 52u8, 169u8,
                43u8, 66u8, 52u8, 126u8, 172u8, 205u8, 168u8, 15u8, 196u8, 10u8,
            ],
            [
                246u8, 57u8, 31u8, 92u8, 50u8, 217u8, 198u8, 157u8, 42u8, 71u8, 234u8,
                103u8, 11u8, 68u8, 41u8, 116u8, 181u8, 57u8, 53u8, 209u8, 237u8, 199u8,
                253u8, 100u8, 235u8, 33u8, 224u8, 71u8, 168u8, 57u8, 23u8, 27u8,
            ],
            [
                246u8, 232u8, 53u8, 156u8, 87u8, 82u8, 11u8, 70u8, 150u8, 52u8, 115u8,
                107u8, 252u8, 59u8, 183u8, 236u8, 92u8, 189u8, 26u8, 11u8, 210u8, 139u8,
                16u8, 168u8, 39u8, 87u8, 147u8, 187u8, 115u8, 11u8, 121u8, 127u8,
            ],
            [
                247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8,
                154u8, 22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8,
                185u8, 62u8, 237u8, 168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
            ],
            [
                251u8, 36u8, 48u8, 83u8, 84u8, 200u8, 119u8, 98u8, 213u8, 87u8, 72u8,
                122u8, 228u8, 165u8, 100u8, 232u8, 208u8, 62u8, 203u8, 185u8, 169u8,
                125u8, 216u8, 175u8, 255u8, 142u8, 31u8, 111u8, 202u8, 240u8, 221u8, 22u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for StakeTableV2Events {
        const NAME: &'static str = "StakeTableV2Events";
        const COUNT: usize = 18usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(
                    <ConsensusKeysUpdated as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <ConsensusKeysUpdated as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ConsensusKeysUpdated)
                }
                Some(
                    <ConsensusKeysUpdatedV2 as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <ConsensusKeysUpdatedV2 as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ConsensusKeysUpdatedV2)
                }
                Some(<Delegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Delegated as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Delegated)
                }
                Some(
                    <ExitEscrowPeriodUpdated as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <ExitEscrowPeriodUpdated as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ExitEscrowPeriodUpdated)
                }
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Initialized)
                }
                Some(
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::OwnershipTransferred)
                }
                Some(<Paused as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Paused as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Paused)
                }
                Some(<RoleAdminChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleAdminChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::RoleAdminChanged)
                }
                Some(<RoleGranted as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleGranted as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::RoleGranted)
                }
                Some(<RoleRevoked as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <RoleRevoked as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::RoleRevoked)
                }
                Some(<Undelegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Undelegated as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Undelegated)
                }
                Some(<Unpaused as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Unpaused as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Unpaused)
                }
                Some(<Upgrade as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgrade as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Upgrade)
                }
                Some(<Upgraded as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgraded as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Upgraded)
                }
                Some(<ValidatorExit as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ValidatorExit as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ValidatorExit)
                }
                Some(
                    <ValidatorRegistered as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <ValidatorRegistered as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ValidatorRegistered)
                }
                Some(
                    <ValidatorRegisteredV2 as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <ValidatorRegisteredV2 as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::ValidatorRegisteredV2)
                }
                Some(<Withdrawal as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Withdrawal as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Withdrawal)
                }
                _ => {
                    alloy_sol_types::private::Err(alloy_sol_types::Error::InvalidLog {
                        name: <Self as alloy_sol_types::SolEventInterface>::NAME,
                        log: alloy_sol_types::private::Box::new(
                            alloy_sol_types::private::LogData::new_unchecked(
                                topics.to_vec(),
                                data.to_vec().into(),
                            ),
                        ),
                    })
                }
            }
        }
    }
    #[automatically_derived]
    impl alloy_sol_types::private::IntoLogData for StakeTableV2Events {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::ConsensusKeysUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ConsensusKeysUpdatedV2(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Delegated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ExitEscrowPeriodUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Paused(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::RoleAdminChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::RoleGranted(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::RoleRevoked(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Unpaused(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Upgrade(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ValidatorRegistered(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ValidatorRegisteredV2(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Withdrawal(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::ConsensusKeysUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ConsensusKeysUpdatedV2(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Delegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ExitEscrowPeriodUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Paused(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::RoleAdminChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::RoleGranted(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::RoleRevoked(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Unpaused(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Upgrade(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ValidatorRegistered(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ValidatorRegisteredV2(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Withdrawal(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`StakeTableV2`](self) contract instance.

See the [wrapper's documentation](`StakeTableV2Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> StakeTableV2Instance<T, P, N> {
        StakeTableV2Instance::<T, P, N>::new(address, provider)
    }
    /**Deploys this contract using the given `provider` and constructor arguments, if any.

Returns a new instance of the contract, if the deployment was successful.

For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
    #[inline]
    pub fn deploy<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        provider: P,
    ) -> impl ::core::future::Future<
        Output = alloy_contract::Result<StakeTableV2Instance<T, P, N>>,
    > {
        StakeTableV2Instance::<T, P, N>::deploy(provider)
    }
    /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
and constructor arguments, if any.

This is a simple wrapper around creating a `RawCallBuilder` with the data set to
the bytecode concatenated with the constructor's ABI-encoded arguments.*/
    #[inline]
    pub fn deploy_builder<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(provider: P) -> alloy_contract::RawCallBuilder<T, P, N> {
        StakeTableV2Instance::<T, P, N>::deploy_builder(provider)
    }
    /**A [`StakeTableV2`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`StakeTableV2`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct StakeTableV2Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for StakeTableV2Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("StakeTableV2Instance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableV2Instance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`StakeTableV2`](self) contract instance.

See the [wrapper's documentation](`StakeTableV2Instance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            provider: P,
        ) -> Self {
            Self {
                address,
                provider,
                _network_transport: ::core::marker::PhantomData,
            }
        }
        /**Deploys this contract using the given `provider` and constructor arguments, if any.

Returns a new instance of the contract, if the deployment was successful.

For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
        #[inline]
        pub async fn deploy(
            provider: P,
        ) -> alloy_contract::Result<StakeTableV2Instance<T, P, N>> {
            let call_builder = Self::deploy_builder(provider);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
and constructor arguments, if any.

This is a simple wrapper around creating a `RawCallBuilder` with the data set to
the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(provider: P) -> alloy_contract::RawCallBuilder<T, P, N> {
            alloy_contract::RawCallBuilder::new_raw_deploy(
                provider,
                ::core::clone::Clone::clone(&BYTECODE),
            )
        }
        /// Returns a reference to the address.
        #[inline]
        pub const fn address(&self) -> &alloy_sol_types::private::Address {
            &self.address
        }
        /// Sets the address.
        #[inline]
        pub fn set_address(&mut self, address: alloy_sol_types::private::Address) {
            self.address = address;
        }
        /// Sets the address and returns `self`.
        pub fn at(mut self, address: alloy_sol_types::private::Address) -> Self {
            self.set_address(address);
            self
        }
        /// Returns a reference to the provider.
        #[inline]
        pub const fn provider(&self) -> &P {
            &self.provider
        }
    }
    impl<T, P: ::core::clone::Clone, N> StakeTableV2Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> StakeTableV2Instance<T, P, N> {
            StakeTableV2Instance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network_transport: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableV2Instance<T, P, N> {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<T, &P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
        ///Creates a new call builder for the [`DEFAULT_ADMIN_ROLE`] function.
        pub fn DEFAULT_ADMIN_ROLE(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, DEFAULT_ADMIN_ROLECall, N> {
            self.call_builder(&DEFAULT_ADMIN_ROLECall {})
        }
        ///Creates a new call builder for the [`PAUSER_ROLE`] function.
        pub fn PAUSER_ROLE(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, PAUSER_ROLECall, N> {
            self.call_builder(&PAUSER_ROLECall {})
        }
        ///Creates a new call builder for the [`UPGRADE_INTERFACE_VERSION`] function.
        pub fn UPGRADE_INTERFACE_VERSION(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, UPGRADE_INTERFACE_VERSIONCall, N> {
            self.call_builder(&UPGRADE_INTERFACE_VERSIONCall {})
        }
        ///Creates a new call builder for the [`_hashBlsKey`] function.
        pub fn _hashBlsKey(
            &self,
            blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, _hashBlsKeyCall, N> {
            self.call_builder(&_hashBlsKeyCall { blsVK })
        }
        ///Creates a new call builder for the [`blsKeys`] function.
        pub fn blsKeys(
            &self,
            blsKeyHash: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, blsKeysCall, N> {
            self.call_builder(&blsKeysCall { blsKeyHash })
        }
        ///Creates a new call builder for the [`claimValidatorExit`] function.
        pub fn claimValidatorExit(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, claimValidatorExitCall, N> {
            self.call_builder(
                &claimValidatorExitCall {
                    validator,
                },
            )
        }
        ///Creates a new call builder for the [`claimWithdrawal`] function.
        pub fn claimWithdrawal(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, claimWithdrawalCall, N> {
            self.call_builder(&claimWithdrawalCall { validator })
        }
        ///Creates a new call builder for the [`delegate`] function.
        pub fn delegate(
            &self,
            validator: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegateCall, N> {
            self.call_builder(&delegateCall { validator, amount })
        }
        ///Creates a new call builder for the [`delegations`] function.
        pub fn delegations(
            &self,
            validator: alloy::sol_types::private::Address,
            delegator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegationsCall, N> {
            self.call_builder(
                &delegationsCall {
                    validator,
                    delegator,
                },
            )
        }
        ///Creates a new call builder for the [`deregisterValidator`] function.
        pub fn deregisterValidator(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, deregisterValidatorCall, N> {
            self.call_builder(&deregisterValidatorCall {})
        }
        ///Creates a new call builder for the [`exitEscrowPeriod`] function.
        pub fn exitEscrowPeriod(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, exitEscrowPeriodCall, N> {
            self.call_builder(&exitEscrowPeriodCall {})
        }
        ///Creates a new call builder for the [`getRoleAdmin`] function.
        pub fn getRoleAdmin(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, getRoleAdminCall, N> {
            self.call_builder(&getRoleAdminCall { role })
        }
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, getVersionCall, N> {
            self.call_builder(&getVersionCall {})
        }
        ///Creates a new call builder for the [`grantRole`] function.
        pub fn grantRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, grantRoleCall, N> {
            self.call_builder(&grantRoleCall { role, account })
        }
        ///Creates a new call builder for the [`hasRole`] function.
        pub fn hasRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, hasRoleCall, N> {
            self.call_builder(&hasRoleCall { role, account })
        }
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(
            &self,
            _tokenAddress: alloy::sol_types::private::Address,
            _lightClientAddress: alloy::sol_types::private::Address,
            _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
            _timelock: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeCall, N> {
            self.call_builder(
                &initializeCall {
                    _tokenAddress,
                    _lightClientAddress,
                    _exitEscrowPeriod,
                    _timelock,
                },
            )
        }
        ///Creates a new call builder for the [`initializeV2`] function.
        pub fn initializeV2(
            &self,
            pauser: alloy::sol_types::private::Address,
            admin: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeV2Call, N> {
            self.call_builder(&initializeV2Call { pauser, admin })
        }
        ///Creates a new call builder for the [`initializedAtBlock`] function.
        pub fn initializedAtBlock(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializedAtBlockCall, N> {
            self.call_builder(&initializedAtBlockCall {})
        }
        ///Creates a new call builder for the [`lightClient`] function.
        pub fn lightClient(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, lightClientCall, N> {
            self.call_builder(&lightClientCall {})
        }
        ///Creates a new call builder for the [`owner`] function.
        pub fn owner(&self) -> alloy_contract::SolCallBuilder<T, &P, ownerCall, N> {
            self.call_builder(&ownerCall {})
        }
        ///Creates a new call builder for the [`pause`] function.
        pub fn pause(&self) -> alloy_contract::SolCallBuilder<T, &P, pauseCall, N> {
            self.call_builder(&pauseCall {})
        }
        ///Creates a new call builder for the [`paused`] function.
        pub fn paused(&self) -> alloy_contract::SolCallBuilder<T, &P, pausedCall, N> {
            self.call_builder(&pausedCall {})
        }
        ///Creates a new call builder for the [`proxiableUUID`] function.
        pub fn proxiableUUID(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, proxiableUUIDCall, N> {
            self.call_builder(&proxiableUUIDCall {})
        }
        ///Creates a new call builder for the [`registerValidator`] function.
        pub fn registerValidator(
            &self,
            _0: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            _1: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            _2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            _3: u16,
        ) -> alloy_contract::SolCallBuilder<T, &P, registerValidatorCall, N> {
            self.call_builder(
                &registerValidatorCall {
                    _0,
                    _1,
                    _2,
                    _3,
                },
            )
        }
        ///Creates a new call builder for the [`registerValidatorV2`] function.
        pub fn registerValidatorV2(
            &self,
            blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            schnorrSig: alloy::sol_types::private::Bytes,
            commission: u16,
        ) -> alloy_contract::SolCallBuilder<T, &P, registerValidatorV2Call, N> {
            self.call_builder(
                &registerValidatorV2Call {
                    blsVK,
                    schnorrVK,
                    blsSig,
                    schnorrSig,
                    commission,
                },
            )
        }
        ///Creates a new call builder for the [`renounceOwnership`] function.
        pub fn renounceOwnership(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceOwnershipCall, N> {
            self.call_builder(&renounceOwnershipCall {})
        }
        ///Creates a new call builder for the [`renounceRole`] function.
        pub fn renounceRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            callerConfirmation: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceRoleCall, N> {
            self.call_builder(
                &renounceRoleCall {
                    role,
                    callerConfirmation,
                },
            )
        }
        ///Creates a new call builder for the [`revokeRole`] function.
        pub fn revokeRole(
            &self,
            role: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, revokeRoleCall, N> {
            self.call_builder(&revokeRoleCall { role, account })
        }
        ///Creates a new call builder for the [`schnorrKeys`] function.
        pub fn schnorrKeys(
            &self,
            schnorrKey: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, schnorrKeysCall, N> {
            self.call_builder(&schnorrKeysCall { schnorrKey })
        }
        ///Creates a new call builder for the [`supportsInterface`] function.
        pub fn supportsInterface(
            &self,
            interfaceId: alloy::sol_types::private::FixedBytes<4>,
        ) -> alloy_contract::SolCallBuilder<T, &P, supportsInterfaceCall, N> {
            self.call_builder(
                &supportsInterfaceCall {
                    interfaceId,
                },
            )
        }
        ///Creates a new call builder for the [`token`] function.
        pub fn token(&self) -> alloy_contract::SolCallBuilder<T, &P, tokenCall, N> {
            self.call_builder(&tokenCall {})
        }
        ///Creates a new call builder for the [`transferOwnership`] function.
        pub fn transferOwnership(
            &self,
            newOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, transferOwnershipCall, N> {
            self.call_builder(&transferOwnershipCall { newOwner })
        }
        ///Creates a new call builder for the [`undelegate`] function.
        pub fn undelegate(
            &self,
            validator: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegateCall, N> {
            self.call_builder(
                &undelegateCall {
                    validator,
                    amount,
                },
            )
        }
        ///Creates a new call builder for the [`undelegations`] function.
        pub fn undelegations(
            &self,
            validator: alloy::sol_types::private::Address,
            delegator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegationsCall, N> {
            self.call_builder(
                &undelegationsCall {
                    validator,
                    delegator,
                },
            )
        }
        ///Creates a new call builder for the [`unpause`] function.
        pub fn unpause(&self) -> alloy_contract::SolCallBuilder<T, &P, unpauseCall, N> {
            self.call_builder(&unpauseCall {})
        }
        ///Creates a new call builder for the [`updateConsensusKeys`] function.
        pub fn updateConsensusKeys(
            &self,
            _0: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            _1: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            _2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateConsensusKeysCall, N> {
            self.call_builder(
                &updateConsensusKeysCall {
                    _0,
                    _1,
                    _2,
                },
            )
        }
        ///Creates a new call builder for the [`updateConsensusKeysV2`] function.
        pub fn updateConsensusKeysV2(
            &self,
            blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            schnorrSig: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateConsensusKeysV2Call, N> {
            self.call_builder(
                &updateConsensusKeysV2Call {
                    blsVK,
                    schnorrVK,
                    blsSig,
                    schnorrSig,
                },
            )
        }
        ///Creates a new call builder for the [`updateExitEscrowPeriod`] function.
        pub fn updateExitEscrowPeriod(
            &self,
            newExitEscrowPeriod: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateExitEscrowPeriodCall, N> {
            self.call_builder(
                &updateExitEscrowPeriodCall {
                    newExitEscrowPeriod,
                },
            )
        }
        ///Creates a new call builder for the [`upgradeToAndCall`] function.
        pub fn upgradeToAndCall(
            &self,
            newImplementation: alloy::sol_types::private::Address,
            data: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, upgradeToAndCallCall, N> {
            self.call_builder(
                &upgradeToAndCallCall {
                    newImplementation,
                    data,
                },
            )
        }
        ///Creates a new call builder for the [`validatorExits`] function.
        pub fn validatorExits(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, validatorExitsCall, N> {
            self.call_builder(&validatorExitsCall { validator })
        }
        ///Creates a new call builder for the [`validators`] function.
        pub fn validators(
            &self,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, validatorsCall, N> {
            self.call_builder(&validatorsCall { account })
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > StakeTableV2Instance<T, P, N> {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<T, &P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
        ///Creates a new event filter for the [`ConsensusKeysUpdated`] event.
        pub fn ConsensusKeysUpdated_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ConsensusKeysUpdated, N> {
            self.event_filter::<ConsensusKeysUpdated>()
        }
        ///Creates a new event filter for the [`ConsensusKeysUpdatedV2`] event.
        pub fn ConsensusKeysUpdatedV2_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ConsensusKeysUpdatedV2, N> {
            self.event_filter::<ConsensusKeysUpdatedV2>()
        }
        ///Creates a new event filter for the [`Delegated`] event.
        pub fn Delegated_filter(&self) -> alloy_contract::Event<T, &P, Delegated, N> {
            self.event_filter::<Delegated>()
        }
        ///Creates a new event filter for the [`ExitEscrowPeriodUpdated`] event.
        pub fn ExitEscrowPeriodUpdated_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ExitEscrowPeriodUpdated, N> {
            self.event_filter::<ExitEscrowPeriodUpdated>()
        }
        ///Creates a new event filter for the [`Initialized`] event.
        pub fn Initialized_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, Initialized, N> {
            self.event_filter::<Initialized>()
        }
        ///Creates a new event filter for the [`OwnershipTransferred`] event.
        pub fn OwnershipTransferred_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, OwnershipTransferred, N> {
            self.event_filter::<OwnershipTransferred>()
        }
        ///Creates a new event filter for the [`Paused`] event.
        pub fn Paused_filter(&self) -> alloy_contract::Event<T, &P, Paused, N> {
            self.event_filter::<Paused>()
        }
        ///Creates a new event filter for the [`RoleAdminChanged`] event.
        pub fn RoleAdminChanged_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, RoleAdminChanged, N> {
            self.event_filter::<RoleAdminChanged>()
        }
        ///Creates a new event filter for the [`RoleGranted`] event.
        pub fn RoleGranted_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, RoleGranted, N> {
            self.event_filter::<RoleGranted>()
        }
        ///Creates a new event filter for the [`RoleRevoked`] event.
        pub fn RoleRevoked_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, RoleRevoked, N> {
            self.event_filter::<RoleRevoked>()
        }
        ///Creates a new event filter for the [`Undelegated`] event.
        pub fn Undelegated_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, Undelegated, N> {
            self.event_filter::<Undelegated>()
        }
        ///Creates a new event filter for the [`Unpaused`] event.
        pub fn Unpaused_filter(&self) -> alloy_contract::Event<T, &P, Unpaused, N> {
            self.event_filter::<Unpaused>()
        }
        ///Creates a new event filter for the [`Upgrade`] event.
        pub fn Upgrade_filter(&self) -> alloy_contract::Event<T, &P, Upgrade, N> {
            self.event_filter::<Upgrade>()
        }
        ///Creates a new event filter for the [`Upgraded`] event.
        pub fn Upgraded_filter(&self) -> alloy_contract::Event<T, &P, Upgraded, N> {
            self.event_filter::<Upgraded>()
        }
        ///Creates a new event filter for the [`ValidatorExit`] event.
        pub fn ValidatorExit_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ValidatorExit, N> {
            self.event_filter::<ValidatorExit>()
        }
        ///Creates a new event filter for the [`ValidatorRegistered`] event.
        pub fn ValidatorRegistered_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ValidatorRegistered, N> {
            self.event_filter::<ValidatorRegistered>()
        }
        ///Creates a new event filter for the [`ValidatorRegisteredV2`] event.
        pub fn ValidatorRegisteredV2_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ValidatorRegisteredV2, N> {
            self.event_filter::<ValidatorRegisteredV2>()
        }
        ///Creates a new event filter for the [`Withdrawal`] event.
        pub fn Withdrawal_filter(&self) -> alloy_contract::Event<T, &P, Withdrawal, N> {
            self.event_filter::<Withdrawal>()
        }
    }
}
