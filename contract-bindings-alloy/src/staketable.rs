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
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BaseField(alloy::sol_types::private::primitives::aliases::U256);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<BaseField>
            for alloy::sol_types::private::primitives::aliases::U256
        {
            #[inline]
            fn stv_to_tokens(
                &self,
            ) -> <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::Token<'_>
            {
                alloy_sol_types::private::SolTypeValue::<
                    alloy::sol_types::sol_data::Uint<256>,
                >::stv_to_tokens(self)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(self)
                    .0
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
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
            pub const fn from(value: alloy::sol_types::private::primitives::aliases::U256) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into(self) -> alloy::sol_types::private::primitives::aliases::U256 {
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
            type Token<'a> =
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = Self::NAME;
            const ENCODED_SIZE: Option<usize> =
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                256,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                Self::type_check(token).is_ok()
            }
            #[inline]
            fn type_check(token: &Self::Token<'_>) -> alloy_sol_types::Result<()> {
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::type_check(
                    token,
                )
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::detokenize(
                    token,
                )
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
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::EventTopic>::encode_topic(
                    rust,
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
                Self {
                    x: tuple.0,
                    y: tuple.1,
                }
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
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encode_packed_to(
                    &tuple, out,
                )
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_packed_encoded_size(
                    &tuple,
                )
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for G1Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::detokenize(token);
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
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
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
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.x)
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.y)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.x, out);
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.y, out);
            }
            #[inline]
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, &mut out);
                alloy_sol_types::abi::token::WordToken(alloy_sol_types::private::keccak256(out))
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encode_packed_to(
                    &tuple, out,
                )
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_packed_encoded_size(
                    &tuple,
                )
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for G2Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::detokenize(token);
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
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
                alloy_sol_types::private::Vec::new()
            }
            #[inline]
            fn eip712_encode_type() -> alloy_sol_types::private::Cow<'static, str> {
                <Self as alloy_sol_types::SolStruct>::eip712_root_type()
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.x0).0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.x1).0,
                    <BaseField as alloy_sol_types::SolType>::eip712_data_word(&self.y0).0,
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
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.x0)
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.x1)
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.y0)
                    + <BaseField as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.y1)
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.x0, out);
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.x1, out);
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.y0, out);
                <BaseField as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.y1, out);
            }
            #[inline]
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, &mut out);
                alloy_sol_types::abi::token::WordToken(alloy_sol_types::private::keccak256(out))
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
        > BN254Instance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`BN254`](self) contract instance.

        See the [wrapper's documentation](`BN254Instance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
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
        > BN254Instance<T, P, N>
    {
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
        > BN254Instance<T, P, N>
    {
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
library Checkpoints {
    struct Checkpoint208 { uint48 _key; uint208 _value; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod Checkpoints {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /**```solidity
    struct Checkpoint208 { uint48 _key; uint208 _value; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct Checkpoint208 {
        #[allow(missing_docs)]
        pub _key: alloy::sol_types::private::primitives::aliases::U48,
        #[allow(missing_docs)]
        pub _value: alloy::sol_types::private::primitives::aliases::U208,
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
            alloy::sol_types::sol_data::Uint<48>,
            alloy::sol_types::sol_data::Uint<208>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U48,
            alloy::sol_types::private::primitives::aliases::U208,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<Checkpoint208> for UnderlyingRustTuple<'_> {
            fn from(value: Checkpoint208) -> Self {
                (value._key, value._value)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for Checkpoint208 {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    _key: tuple.0,
                    _value: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for Checkpoint208 {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for Checkpoint208 {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<48> as alloy_sol_types::SolType>::tokenize(
                        &self._key,
                    ),
                    <alloy::sol_types::sol_data::Uint<208> as alloy_sol_types::SolType>::tokenize(
                        &self._value,
                    ),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encode_packed_to(
                    &tuple, out,
                )
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_packed_encoded_size(
                    &tuple,
                )
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for Checkpoint208 {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolStruct for Checkpoint208 {
            const NAME: &'static str = "Checkpoint208";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed("Checkpoint208(uint48 _key,uint208 _value)")
            }
            #[inline]
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
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
                        48,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self._key)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        208,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self._value)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for Checkpoint208 {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        48,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(&rust._key)
                    + <alloy::sol_types::sol_data::Uint<
                        208,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust._value,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <alloy::sol_types::sol_data::Uint<
                    48,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust._key,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    208,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust._value,
                    out,
                );
            }
            #[inline]
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, &mut out);
                alloy_sol_types::abi::token::WordToken(alloy_sol_types::private::keccak256(out))
            }
        }
    };
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`Checkpoints`](self) contract instance.

    See the [wrapper's documentation](`CheckpointsInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> CheckpointsInstance<T, P, N> {
        CheckpointsInstance::<T, P, N>::new(address, provider)
    }
    /**A [`Checkpoints`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`Checkpoints`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct CheckpointsInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for CheckpointsInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("CheckpointsInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > CheckpointsInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`Checkpoints`](self) contract instance.

        See the [wrapper's documentation](`CheckpointsInstance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
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
    impl<T, P: ::core::clone::Clone, N> CheckpointsInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> CheckpointsInstance<T, P, N> {
            CheckpointsInstance {
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
        > CheckpointsInstance<T, P, N>
    {
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
        > CheckpointsInstance<T, P, N>
    {
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
    use alloy::sol_types as alloy_sol_types;

    use super::*;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
                Self {
                    x: tuple.0,
                    y: tuple.1,
                }
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.x,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.y,
                    ),
                )
            }
            #[inline]
            fn stv_abi_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encoded_size(&tuple)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <Self as alloy_sol_types::SolStruct>::eip712_hash_struct(self)
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_encode_packed_to(
                    &tuple, out,
                )
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                if let Some(size) = <Self as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE {
                    return size;
                }
                let tuple =
                    <UnderlyingRustTuple<'_> as ::core::convert::From<Self>>::from(self.clone());
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::abi_packed_encoded_size(
                    &tuple,
                )
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolType for EdOnBN254Point {
            type RustType = Self;
            type Token<'a> = <UnderlyingSolTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = <Self as alloy_sol_types::SolStruct>::NAME;
            const ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> =
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::valid_token(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                let tuple = <UnderlyingSolTuple<'_> as alloy_sol_types::SolType>::detokenize(token);
                <Self as ::core::convert::From<UnderlyingRustTuple<'_>>>::from(tuple)
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolStruct for EdOnBN254Point {
            const NAME: &'static str = "EdOnBN254Point";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed("EdOnBN254Point(uint256 x,uint256 y)")
            }
            #[inline]
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
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
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.x, out);
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(&rust.y, out);
            }
            #[inline]
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                let mut out = alloy_sol_types::private::Vec::new();
                <Self as alloy_sol_types::EventTopic>::encode_topic_preimage(rust, &mut out);
                alloy_sol_types::abi::token::WordToken(alloy_sol_types::private::keccak256(out))
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
            f.debug_tuple("EdOnBN254Instance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > EdOnBN254Instance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`EdOnBN254`](self) contract instance.

        See the [wrapper's documentation](`EdOnBN254Instance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
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
        > EdOnBN254Instance<T, P, N>
    {
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
        > EdOnBN254Instance<T, P, N>
    {
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

library Checkpoints {
    struct Checkpoint208 {
        uint48 _key;
        uint208 _value;
    }
}

library EdOnBN254 {
    struct EdOnBN254Point {
        uint256 x;
        uint256 y;
    }
}

interface StakeTable {
    type ValidatorStatus is uint8;

    error AddressEmptyCode(address target);
    error BLSSigVerificationFailed();
    error BlsKeyAlreadyUsed();
    error CheckpointUnorderedInsertion();
    error DelegatorAlreadyStaked();
    error ECDSAInvalidSignature();
    error ECDSAInvalidSignatureLength(uint256 length);
    error ECDSAInvalidSignatureS(bytes32 s);
    error ERC1967InvalidImplementation(address implementation);
    error ERC1967NonPayable();
    error ERC5805FutureLookup(uint256 timepoint, uint48 clock);
    error ERC6372InconsistentClock();
    error FailedInnerCall();
    error InsufficientAllowance(uint256, uint256);
    error InsufficientBalance(uint256);
    error InvalidAccountNonce(address account, uint256 currentNonce);
    error InvalidCommission();
    error InvalidInitialization();
    error InvalidSchnorrVK();
    error NotInitializing();
    error NothingToWithdraw();
    error OwnableInvalidOwner(address owner);
    error OwnableUnauthorizedAccount(address account);
    error PrematureWithdrawal();
    error SafeCastOverflowedUintDowncast(uint8 bits, uint256 value);
    error UUPSUnauthorizedCallContext();
    error UUPSUnsupportedProxiableUUID(bytes32 slot);
    error ValidatorAlreadyExited();
    error ValidatorAlreadyRegistered();
    error ValidatorCannotDelegate();
    error ValidatorInactive();
    error ValidatorNotExited();
    error VotesExpiredSignature(uint256 expiry);
    error ZeroAddress();

    event ConsensusKeysUpdated(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
    event DelegateChanged(address indexed delegator, address indexed fromDelegate, address indexed toDelegate);
    event DelegateVotesChanged(address indexed delegate, uint256 previousVotes, uint256 newVotes);
    event EIP712DomainChanged();
    event Initialized(uint64 version);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event Staked(address indexed delegator, address indexed validator, uint256 amount);
    event Undelegated(address indexed delegator, address indexed validator, uint256 amount);
    event Upgrade(address implementation);
    event Upgraded(address indexed implementation);
    event ValidatorExit(address indexed validator);
    event ValidatorRegistered(address indexed account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
    event Withdrawal(address indexed account, uint256 amount);

    constructor();

    function CLOCK_MODE() external view returns (string memory);
    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    function _hashBlsKey(BN254.G2Point memory blsVK) external pure returns (bytes32);
    function blsKeys(bytes32 blsKeyHash) external view returns (bool used);
    function checkpoints(address account, uint32 pos) external view returns (Checkpoints.Checkpoint208 memory);
    function claimValidatorExit(address validator) external;
    function claimWithdrawal(address validator) external;
    function clock() external view returns (uint48);
    function delegate(address validator, uint256 amount) external;
    function delegate(address validator) external;
    function delegateBySig(address delegatee, uint256 nonce, uint256 expiry, uint8 v, bytes32 r, bytes32 s) external;
    function delegates(address account) external view returns (address);
    function delegations(address validator, address delegator) external view returns (uint256 amount);
    function delegatorValidator(address delegator) external view returns (address validator);
    function deregisterValidator() external;
    function eip712Domain() external view returns (bytes1 fields, string memory name, string memory version, uint256 chainId, address verifyingContract, bytes32 salt, uint256[] memory extensions);
    function exitEscrowPeriod() external view returns (uint256);
    function getPastTotalSupply(uint256 timepoint) external view returns (uint256);
    function getPastVotes(address account, uint256 timepoint) external view returns (uint256);
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    function getVotes(address account) external view returns (uint256);
    function initialize(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod, address _initialOwner) external;
    function initializedAtBlock() external view returns (uint256);
    function lightClient() external view returns (address);
    function nonces(address owner) external view returns (uint256);
    function numCheckpoints(address account) external view returns (uint256);
    function owner() external view returns (address);
    function proxiableUUID() external view returns (bytes32);
    function registerValidator(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, uint16 commission) external;
    function renounceOwnership() external;
    function token() external view returns (address);
    function transferOwnership(address newOwner) external;
    function undelegate(address validator, uint256 amount) external;
    function undelegate(address validator) external;
    function undelegations(address validator, address delegator) external view returns (uint256 amount, uint256 unlocksAt);
    function updateConsensusKeys(BN254.G2Point memory newBlsVK, EdOnBN254.EdOnBN254Point memory newSchnorrVK, BN254.G1Point memory newBlsSig) external;
    function upgradeToAndCall(address newImplementation, bytes memory data) external payable;
    function validatorExits(address validator) external view returns (uint256 unlocksAt);
    function validators(address account) external view returns (uint256 delegatedAmount, ValidatorStatus status);
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
    "name": "CLOCK_MODE",
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
    "name": "checkpoints",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "pos",
        "type": "uint32",
        "internalType": "uint32"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct Checkpoints.Checkpoint208",
        "components": [
          {
            "name": "_key",
            "type": "uint48",
            "internalType": "uint48"
          },
          {
            "name": "_value",
            "type": "uint208",
            "internalType": "uint208"
          }
        ]
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
    "name": "clock",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "uint48",
        "internalType": "uint48"
      }
    ],
    "stateMutability": "view"
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
    "name": "delegate",
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
    "name": "delegateBySig",
    "inputs": [
      {
        "name": "delegatee",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "nonce",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "expiry",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "v",
        "type": "uint8",
        "internalType": "uint8"
      },
      {
        "name": "r",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "s",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "delegates",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
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
    "name": "delegatorValidator",
    "inputs": [
      {
        "name": "delegator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "validator",
        "type": "address",
        "internalType": "address"
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
    "name": "eip712Domain",
    "inputs": [],
    "outputs": [
      {
        "name": "fields",
        "type": "bytes1",
        "internalType": "bytes1"
      },
      {
        "name": "name",
        "type": "string",
        "internalType": "string"
      },
      {
        "name": "version",
        "type": "string",
        "internalType": "string"
      },
      {
        "name": "chainId",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "verifyingContract",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "salt",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "extensions",
        "type": "uint256[]",
        "internalType": "uint256[]"
      }
    ],
    "stateMutability": "view"
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
    "name": "getPastTotalSupply",
    "inputs": [
      {
        "name": "timepoint",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
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
    "name": "getPastVotes",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "timepoint",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
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
    "name": "getVotes",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
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
        "name": "_initialOwner",
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
        "internalType": "contract LightClient"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "nonces",
    "inputs": [
      {
        "name": "owner",
        "type": "address",
        "internalType": "address"
      }
    ],
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
    "name": "numCheckpoints",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      }
    ],
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
    "name": "undelegate",
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
    "name": "updateConsensusKeys",
    "inputs": [
      {
        "name": "newBlsVK",
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
        "name": "newSchnorrVK",
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
        "name": "newBlsSig",
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
    "name": "DelegateChanged",
    "inputs": [
      {
        "name": "delegator",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "fromDelegate",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "toDelegate",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "DelegateVotesChanged",
    "inputs": [
      {
        "name": "delegate",
        "type": "address",
        "indexed": true,
        "internalType": "address"
      },
      {
        "name": "previousVotes",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      },
      {
        "name": "newVotes",
        "type": "uint256",
        "indexed": false,
        "internalType": "uint256"
      }
    ],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "EIP712DomainChanged",
    "inputs": [],
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
    "name": "Staked",
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
    "name": "CheckpointUnorderedInsertion",
    "inputs": []
  },
  {
    "type": "error",
    "name": "DelegatorAlreadyStaked",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ECDSAInvalidSignature",
    "inputs": []
  },
  {
    "type": "error",
    "name": "ECDSAInvalidSignatureLength",
    "inputs": [
      {
        "name": "length",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "ECDSAInvalidSignatureS",
    "inputs": [
      {
        "name": "s",
        "type": "bytes32",
        "internalType": "bytes32"
      }
    ]
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
    "name": "ERC5805FutureLookup",
    "inputs": [
      {
        "name": "timepoint",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "clock",
        "type": "uint48",
        "internalType": "uint48"
      }
    ]
  },
  {
    "type": "error",
    "name": "ERC6372InconsistentClock",
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
    "name": "InvalidAccountNonce",
    "inputs": [
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "currentNonce",
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
    "name": "SafeCastOverflowedUintDowncast",
    "inputs": [
      {
        "name": "bits",
        "type": "uint8",
        "internalType": "uint8"
      },
      {
        "name": "value",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
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
    "name": "ValidatorCannotDelegate",
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
    "name": "VotesExpiredSignature",
    "inputs": [
      {
        "name": "expiry",
        "type": "uint256",
        "internalType": "uint256"
      }
    ]
  },
  {
    "type": "error",
    "name": "ZeroAddress",
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
pub mod StakeTable {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x60a06040523060805234801562000014575f80fd5b506200001f6200002f565b620000296200002f565b620000e3565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff1615620000805760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b0390811614620000e05780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b6080516142bf6200010a5f395f8181611b0001528181611b290152611cac01526142bf5ff3fe608060405260043610610228575f3560e01c80638e539e8c11610129578063b5ecb344116100a8578063da8be8641161006d578063da8be86414610743578063f1127ed814610762578063f2fde38b146107ad578063fa52c7d8146107cc578063fc0c546a1461080f575f80fd5b8063b5ecb34414610670578063be2030941461069b578063c3cda520146106ba578063c64814dd146106d9578063d2b750e51461070f575f80fd5b8063a2d78dd5116100ee578063a2d78dd514610572578063a3066aab146105c4578063ad3cb1cc146105e3578063b3e6ebd514610613578063b5700e6814610651575f80fd5b80638e539e8c146104d557806391ddadf4146104f45780639ab24eb01461051f5780639b30a5e61461053e5780639e9a8f311461055d575f80fd5b806352d1902d116101b55780636fcfff451161017a5780636fcfff45146103ec578063715018a61461040b5780637ecebe001461041f57806384b0196e146104725780638da5cb5b14610499575f80fd5b806352d1902d1461034f5780635544c2f114610363578063587cde1e146103825780635c19a95c146103b95780636a911ccf146103d8575f80fd5b80633a46b1a8116101fb5780633a46b1a8146102bb5780633e9df9b5146102e85780634bf5d7e9146102fc5780634d99dd161461031d5780634f1ef2861461033c575f80fd5b8063026e402b1461022c5780630d8e6e2c1461024d57806313b9057a1461027d5780632140fecd1461029c575b5f80fd5b348015610237575f80fd5b5061024b610246366004613979565b61082e565b005b348015610258575f80fd5b5060408051600181525f60208201819052918101919091526060015b60405180910390f35b348015610288575f80fd5b5061024b610297366004613a97565b61083c565b3480156102a7575f80fd5b5061024b6102b6366004613af5565b6109d5565b3480156102c6575f80fd5b506102da6102d5366004613979565b610af6565b604051908152602001610274565b3480156102f3575f80fd5b506102da5f5481565b348015610307575f80fd5b50610310610b8d565b6040516102749190613b5b565b348015610328575f80fd5b5061024b610337366004613979565b610c05565b61024b61034a366004613b6d565b610c0f565b34801561035a575f80fd5b506102da610c2a565b34801561036e575f80fd5b5061024b61037d366004613c0d565b610c45565b34801561038d575f80fd5b506103a161039c366004613af5565b610d0e565b6040516001600160a01b039091168152602001610274565b3480156103c4575f80fd5b5061024b6103d3366004613af5565b610d37565b3480156103e3575f80fd5b5061024b610dbd565b3480156103f7575f80fd5b506102da610406366004613af5565b610e3f565b348015610416575f80fd5b5061024b610e55565b34801561042a575f80fd5b506102da610439366004613af5565b6001600160a01b03165f9081527f5ab42ced628888259c08ac98db1eb0cf702fc1501344311d8b100cd1bfe4bb00602052604090205490565b34801561047d575f80fd5b50610486610e68565b6040516102749796959493929190613c51565b3480156104a4575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03166103a1565b3480156104e0575f80fd5b506102da6104ef366004613ce8565b610f11565b3480156104ff575f80fd5b50610508610f8b565b60405165ffffffffffff9091168152602001610274565b34801561052a575f80fd5b506102da610539366004613af5565b610f99565b348015610549575f80fd5b506102da610558366004613cff565b610ff7565b348015610568575f80fd5b506102da60095481565b34801561057d575f80fd5b506105af61058c366004613d19565b600760209081525f92835260408084209091529082529020805460019091015482565b60408051928352602083019190915201610274565b3480156105cf575f80fd5b5061024b6105de366004613af5565b611051565b3480156105ee575f80fd5b50610310604051806040016040528060058152602001640352e302e360dc1b81525081565b34801561061e575f80fd5b5061064161062d366004613ce8565b60046020525f908152604090205460ff1681565b6040519015158152602001610274565b34801561065c575f80fd5b506001546103a1906001600160a01b031681565b34801561067b575f80fd5b506102da61068a366004613af5565b60056020525f908152604090205481565b3480156106a6575f80fd5b5061024b6106b5366004613d4a565b611161565b3480156106c5575f80fd5b5061024b6106d4366004613d94565b61128d565b3480156106e4575f80fd5b506102da6106f3366004613d19565b600660209081525f928352604080842090915290825290205481565b34801561071a575f80fd5b506103a1610729366004613af5565b60086020525f90815260409020546001600160a01b031681565b34801561074e575f80fd5b5061024b61075d366004613af5565b611349565b34801561076d575f80fd5b5061078161077c366004613def565b61137a565b60408051825165ffffffffffff1681526020928301516001600160d01b03169281019290925201610274565b3480156107b8575f80fd5b5061024b6107c7366004613af5565b61139e565b3480156107d7575f80fd5b506108016107e6366004613af5565b60036020525f90815260409020805460019091015460ff1682565b604051610274929190613e40565b34801561081a575f80fd5b506002546103a1906001600160a01b031681565b61083882826113d8565b5050565b3361084681611639565b61084f84611686565b610858856116c1565b604080516001600160a01b03831660208201525f9101604051602081830303815290604052905061088a8185886116fd565b6127108361ffff1611156108b15760405163dc81db8560e01b815260040160405180910390fd5b600160045f6108bf89610ff7565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f81526020016001600281111561090657610906613e2c565b90526001600160a01b0383165f908152600360209081526040909120825181559082015160018083018054909160ff199091169083600281111561094c5761094c613e2c565b02179055505060408051885181526020808a015181830152828a0151928201929092526060808a015190820152875160808201529087015160a082015261ffff851660c08201526001600160a01b03841691507ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9060e0015b60405180910390a2505050505050565b6001600160a01b0381165f908152600560205260408120543391819003610a0f576040516379298a5360e11b815260040160405180910390fd5b80421015610a3057604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832093861683529290529081205490819003610a7857604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038085165f9081526006602090815260408083208785168452909152812055600254610aad91168483611792565b826001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b6582604051610ae891815260200190565b60405180910390a250505050565b5f5f8051602061423383398151915281610b0e610f8b565b90508065ffffffffffff168410610b4e57604051637669fc0f60e11b81526004810185905265ffffffffffff821660248201526044015b60405180910390fd5b610b79610b5a8561181b565b6001600160a01b0387165f908152600185016020526040902090611851565b6001600160d01b0316925050505b92915050565b6060610b97611901565b65ffffffffffff16610ba7610f8b565b65ffffffffffff1614610bcd576040516301bfc1c560e61b815260040160405180910390fd5b5060408051808201909152601d81527f6d6f64653d626c6f636b6e756d6265722666726f6d3d64656661756c74000000602082015290565b610838828261190b565b610c17611af5565b610c2082611b99565b6108388282611be0565b5f610c33611ca1565b505f8051602061429383398151915290565b33610c4f81611cea565b610c5883611686565b610c61846116c1565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610c938184876116fd565b600160045f610ca188610ff7565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550816001600160a01b03167f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d8686604051610cff929190613e70565b60405180910390a25050505050565b6001600160a01b039081165f9081525f8051602061423383398151915260205260409020541690565b6002545f906001600160a01b03166370a08231336040516001600160e01b031960e084901b1681526001600160a01b039091166004820152602401602060405180830381865afa158015610d8d573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610db19190613eaa565b905061083882826113d8565b33610dc781611cea565b6001600160a01b0381165f908152600360205260409020600101805460ff19166002179055600954610df99042613ed5565b6001600160a01b0382165f8181526005602052604080822093909355915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a250565b5f610e4982611d39565b63ffffffff1692915050565b610e5d611d88565b610e665f611de3565b565b5f60608082808083815f805160206142738339815191528054909150158015610e9357506001810154155b610ed75760405162461bcd60e51b81526020600482015260156024820152741152540dcc4c8e88155b9a5b9a5d1a585b1a5e9959605a1b6044820152606401610b45565b610edf611e53565b610ee7611f13565b604080515f80825260208201909252600f60f81b9c939b5091995046985030975095509350915050565b5f5f8051602061423383398151915281610f29610f8b565b90508065ffffffffffff168410610f6457604051637669fc0f60e11b81526004810185905265ffffffffffff82166024820152604401610b45565b610f7a610f708561181b565b6002840190611851565b6001600160d01b0316949350505050565b5f610f94611901565b905090565b6001600160a01b0381165f9081527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d01602052604081205f8051602061423383398151915290610fe790611f51565b6001600160d01b03169392505050565b5f815f0151826020015183604001518460600151604051602001611034949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f908152600760209081526040808320338085529252822054909181900361109657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320938616835292905220600101544210156110de57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320868516845290915281208181556001015560025461111991168383611792565b816001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b658260405161115491815260200190565b60405180910390a2505050565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156111a65750825b90505f8267ffffffffffffffff1660011480156111c25750303b155b9050811580156111d0575080155b156111ee5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561121857845460ff60401b1916600160401b1785555b61122186611f88565b611229611f99565b611231611fa1565b61123c8989896120a8565b831561128257845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b834211156112b157604051632341d78760e11b815260048101859052602401610b45565b604080517fe48329057bfd03d55e49b547132e39cffd9c1820ad7b9d4c5307691425d15adf60208201526001600160a01b0388169181019190915260608101869052608081018590525f9061132a906113229060a0016040516020818303038152906040528051906020012061212b565b858585612157565b90506113368187612183565b61134081886121f4565b50505050505050565b6001600160a01b0381165f90815260066020908152604080832033845290915290205461137790829061190b565b50565b604080518082019091525f808252602082015261139783836122a0565b9392505050565b6113a6611d88565b6001600160a01b0381166113cf57604051631e4fbdf760e01b81525f6004820152602401610b45565b61137781611de3565b6113e182611cea565b335f81815260036020526040902060019081015460ff16600281111561140957611409613e2c565b0361142757604051634434d19360e11b815260040160405180910390fd5b6001600160a01b038181165f90815260086020526040902054161580159061146b57506001600160a01b038181165f90815260086020526040902054811690841614155b1561148957604051630c595ddf60e01b815260040160405180910390fd5b600254604051636eb1769f60e11b81526001600160a01b0383811660048301523060248301525f92169063dd62ed3e90604401602060405180830381865afa1580156114d7573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906114fb9190613eaa565b9050828110156115285760405163054365bb60e31b81526004810182905260248101849052604401610b45565b6115315f612309565b6001600160a01b0384165f9081526003602052604081208054859290611558908490613ed5565b90915550506001600160a01b038085165f90815260066020908152604080832093861683529290529081208054859290611593908490613ed5565b90915550506001600160a01b038281165f90815260086020526040902080546001600160a01b0319168683161790556002546115d29116833086612314565b836001600160a01b0316826001600160a01b03167f5dac0c1b1112564a045ba943c9d50270893e8e826c49be8e7073adc713ab7bd78560405161161791815260200190565b60405180910390a361162884612309565b6116335f85856123b1565b50505050565b6001600160a01b0381165f9081526003602052604081206001015460ff16600281111561166857611668613e2c565b146113775760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f80825260208201526116a38282612427565b15610838576040516306cf438f60e01b815260040160405180910390fd5b60045f6116cd83610ff7565b815260208101919091526040015f205460ff16156113775760405162da8a5760e11b815260040160405180910390fd5b61170682612443565b5f60405180606001604052806024815260200161420f6024913990505f8482604051602001611736929190613ee8565b60405160208183030381529060405290505f611751826124d9565b905061176e8185611761886125c6565b61176961263d565b61270a565b61178a5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f5114161516156117dc5750823b153d17155b806116335760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610b45565b5f65ffffffffffff82111561184d576040516306dfcc6560e41b81526030600482015260248101839052604401610b45565b5090565b81545f90818160058111156118ad575f61186a846127e8565b6118749085613f16565b5f8881526020902090915081015465ffffffffffff908116908716101561189d578091506118ab565b6118a8816001613ed5565b92505b505b5f6118ba878785856128cc565b905080156118f4576118de876118d1600184613f16565b5f91825260209091200190565b54600160301b90046001600160d01b03166118f6565b5f5b979650505050505050565b5f610f944361181b565b61191482611cea565b335f8181526003602052604090206001015460029060ff168181111561193c5761193c613e2c565b0361195a5760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832093851683529290522054828110156119a357604051639266535160e01b815260048101829052602401610b45565b6119ac5f612309565b6001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906119e2908490613f16565b92505081905550604051806040016040528084815260200160095442611a089190613ed5565b90526001600160a01b038086165f81815260076020908152604080832094881683529381528382208551815594810151600190950194909455908152600390925281208054859290611a5b908490613f16565b92505081905550836001600160a01b0316826001600160a01b03167f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c85604051611aa791815260200190565b60405180910390a3808314611ac457611abf84612309565b611aea565b6001600160a01b0382165f90815260086020526040902080546001600160a01b03191690555b611633845f856123b1565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611b7b57507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611b6f5f80516020614293833981519152546001600160a01b031690565b6001600160a01b031614155b15610e665760405163703e46dd60e11b815260040160405180910390fd5b611ba1611d88565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d9060200160405180910390a150565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611c3a575060408051601f3d908101601f19168201909252611c3791810190613eaa565b60015b611c6257604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610b45565b5f805160206142938339815191528114611c9257604051632a87526960e21b815260048101829052602401610b45565b611c9c838361292b565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610e665760405163703e46dd60e11b815260040160405180910390fd5b60016001600160a01b0382165f9081526003602052604090206001015460ff166002811115611d1b57611d1b613e2c565b146113775760405163508a793f60e01b815260040160405180910390fd5b6001600160a01b0381165f9081527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d0160205260408120545f805160206142338339815191529061139790612980565b33611dba7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610e665760405163118cdaa760e01b8152336004820152602401610b45565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b7fa16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d10280546060915f8051602061427383398151915291611e9190613f29565b80601f0160208091040260200160405190810160405280929190818152602001828054611ebd90613f29565b8015611f085780601f10611edf57610100808354040283529160200191611f08565b820191905f5260205f20905b815481529060010190602001808311611eeb57829003601f168201915b505050505091505090565b7fa16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d10380546060915f8051602061427383398151915291611e9190613f29565b80545f908015611f8057611f6a836118d1600184613f16565b54600160301b90046001600160d01b0316611397565b5f9392505050565b611f906129b0565b611377816129f9565b610e666129b0565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f81158015611fe65750825b90505f8267ffffffffffffffff1660011480156120025750303b155b905081158015612010575080155b1561202e5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561205857845460ff60401b1916600160401b1785555b435f5583156120a157845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b5050505050565b6001600160a01b0383166120cf5760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b0382166120f65760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b039485166001600160a01b0319918216179091556001805493909416921691909117909155600955565b5f610b87612137612a01565b8360405161190160f01b8152600281019290925260228201526042902090565b5f805f8061216788888888612a0a565b9250925092506121778282612ad2565b50909695505050505050565b6001600160a01b0382165f9081527f5ab42ced628888259c08ac98db1eb0cf702fc1501344311d8b100cd1bfe4bb0060205260409020805460018101909155818114611c9c576040516301d4b62360e61b81526001600160a01b038416600482015260248101829052604401610b45565b5f805160206142338339815191525f61220c84610d0e565b6001600160a01b038581165f8181526020869052604080822080546001600160a01b031916898616908117909155905194955093928516927f3134e8a2e6d97e929a7e54011ea5485d7d196dd5f0ba4d4ef95803e8e3fc257f9190a4611633818461229b876001600160a01b039081165f90815260086020908152604080832054909316825260039052205490565b612b8a565b6040805180820182525f80825260208083018290526001600160a01b03861682527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d019052919091205f80516020614233833981519152906123019084612ceb565b949350505050565b3361083881836121f4565b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f51141615161561236d5750833b153d17155b806120a15760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610b45565b5f805160206142338339815191526001600160a01b0384166123e8576123e581600201612d586123e085612d63565b612d96565b50505b6001600160a01b03831661240c5761240981600201612dce6123e085612d63565b50505b61163361241885610d0e565b61242185610d0e565b84612b8a565b805182515f911480156113975750506020908101519101511490565b805160208201515f915f8051602061425383398151915291159015161561246957505050565b825160208401518260038485858609850908838283091483821084841016169350505081611c9c5760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610b45565b604080518082019091525f80825260208201525f6124f683612dd9565b90505f8051602061425383398151915260035f828485099050828061251d5761251d613f61565b8482099050828061253057612530613f61565b82820890505f8061254083612fe2565b925090505b806125a957848061255857612558613f61565b600187089550848061256c5761256c613f61565b8687099250848061257f5761257f613f61565b8684099250848061259257612592613f61565b84840892506125a083612fe2565b92509050612545565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f80825260208201528151602083015115901516156125ed575090565b6040518060400160405280835f015181526020015f80516020614253833981519152846020015161261e9190613f75565b612635905f80516020614253833981519152613f16565b905292915050565b61266460405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806127dc5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610b45565b50151595945050505050565b5f815f036127f757505f919050565b5f6001612803846130d9565b901c6001901b9050600181848161281c5761281c613f61565b048201901c9050600181848161283457612834613f61565b048201901c9050600181848161284c5761284c613f61565b048201901c9050600181848161286457612864613f61565b048201901c9050600181848161287c5761287c613f61565b048201901c9050600181848161289457612894613f61565b048201901c905060018184816128ac576128ac613f61565b048201901c9050611397818285816128c6576128c6613f61565b0461316c565b5f5b81831015612923575f6128e18484613181565b5f8781526020902090915065ffffffffffff86169082015465ffffffffffff16111561290f5780925061291d565b61291a816001613ed5565b93505b506128ce565b509392505050565b6129348261319b565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561297857611c9c82826131fe565b610838613270565b5f63ffffffff82111561184d576040516306dfcc6560e41b81526020600482015260248101839052604401610b45565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff16610e6657604051631afcd79f60e31b815260040160405180910390fd5b6113a66129b0565b5f610f9461328f565b5f80807f7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0841115612a4357505f91506003905082612ac8565b604080515f808252602082018084528a905260ff891692820192909252606081018790526080810186905260019060a0016020604051602081039080840390855afa158015612a94573d5f803e3d5ffd5b5050604051601f1901519150506001600160a01b038116612abf57505f925060019150829050612ac8565b92505f91508190505b9450945094915050565b5f826003811115612ae557612ae5613e2c565b03612aee575050565b6001826003811115612b0257612b02613e2c565b03612b205760405163f645eedf60e01b815260040160405180910390fd5b6002826003811115612b3457612b34613e2c565b03612b555760405163fce698f760e01b815260048101829052602401610b45565b6003826003811115612b6957612b69613e2c565b03610838576040516335e2f38360e21b815260048101829052602401610b45565b5f805160206142338339815191526001600160a01b0384811690841614801590612bb357505f82115b15611633576001600160a01b03841615612c57576001600160a01b0384165f90815260018201602052604081208190612bf290612dce6123e087612d63565b6001600160d01b031691506001600160d01b03169150856001600160a01b03167fdec2bacdd2f05b59de34da9b523dff8be42e5e38e818c82fdb0bae774387a7248383604051612c4c929190918252602082015260400190565b60405180910390a250505b6001600160a01b03831615611633576001600160a01b0383165f90815260018201602052604081208190612c9190612d586123e087612d63565b6001600160d01b031691506001600160d01b03169150846001600160a01b03167fdec2bacdd2f05b59de34da9b523dff8be42e5e38e818c82fdb0bae774387a72483836040516109c5929190918252602082015260400190565b604080518082019091525f8082526020820152825f018263ffffffff1681548110612d1857612d18613f88565b5f9182526020918290206040805180820190915291015465ffffffffffff81168252600160301b90046001600160d01b0316918101919091529392505050565b5f6113978284613f9c565b5f6001600160d01b0382111561184d576040516306dfcc6560e41b815260d0600482015260248101839052604401610b45565b5f80612dc1612da3610f8b565b612db9612daf88611f51565b868863ffffffff16565b879190613302565b915091505b935093915050565b5f6113978284613fbc565b5f80612de48361330f565b805190915060308114612df957612df9613fdc565b5f8167ffffffffffffffff811115612e1357612e136139a1565b6040519080825280601f01601f191660200182016040528015612e3d576020820181803683370190505b5090505f5b82811015612eac57836001612e578386613f16565b612e619190613f16565b81518110612e7157612e71613f88565b602001015160f81c60f81b828281518110612e8e57612e8e613f88565b60200101906001600160f81b03191690815f1a905350600101612e42565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b82811015612f3c578381612ee88588613f16565b612ef29190613ed5565b81518110612f0257612f02613f88565b602001015160f81c60f81b60f81c828281518110612f2257612f22613f88565b60ff90921660209283029190910190910152600101612ed4565b505f612f4782613659565b90506101005f805160206142538339815191525f612f658689613f16565b90505f5b81811015612fd2575f886001612f7f8486613f16565b612f899190613f16565b81518110612f9957612f99613f88565b016020015160f81c90508380612fb157612fb1613f61565b85870995508380612fc457612fc4613f61565b818708955050600101612f69565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f80516020614253833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f5192508361309f5760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610b45565b80600184901b11156130b8576130b58382613f16565b92505b80806130c6576130c6613f61565b8384099690961496919550909350505050565b5f80608083901c156130ed57608092831c92015b604083901c156130ff57604092831c92015b602083901c1561311157602092831c92015b601083901c1561312357601092831c92015b600883901c1561313557600892831c92015b600483901c1561314757600492831c92015b600283901c1561315957600292831c92015b600183901c15610b875760010192915050565b5f81831061317a5781611397565b5090919050565b5f61318f6002848418613ff0565b61139790848416613ed5565b806001600160a01b03163b5f036131d057604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610b45565b5f8051602061429383398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b03168460405161321a9190614003565b5f60405180830381855af49150503d805f8114613252576040519150601f19603f3d011682016040523d82523d5f602084013e613257565b606091505b50915091506132678583836136c0565b95945050505050565b3415610e665760405163b398979f60e01b815260040160405180910390fd5b5f7f8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f6132b9613715565b6132c161377d565b60408051602081019490945283019190915260608201524660808201523060a082015260c00160405160208183030381529060405280519060200120905090565b5f80612dc18585856137bf565b604080516030808252606082810190935290602090600160f91b905f9084602082018180368337019050509050808660405160200161334f929190613ee8565b6040516020818303038152906040529050808460f81b60405160200161337692919061401e565b6040516020818303038152906040529050806040516020016133989190614050565b60408051601f1981840301815290829052915061010160f01b906133c29083908390602001614070565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff811115613432576134326139a1565b6040519080825280601f01601f19166020018201604052801561345c576020820181803683370190505b5090505f8260405160200161347391815260200190565b60405160208183030381529060405290505f5b81518110156134dc578181815181106134a1576134a1613f88565b602001015160f81c60f81b8382815181106134be576134be613f88565b60200101906001600160f81b03191690815f1a905350600101613486565b505f846040516020016134f191815260200190565b60408051601f19818403018152602083019091525f80835291985091505b89811015613583575f83828151811061352a5761352a613f88565b602001015160f81c60f81b83838151811061354757613547613f88565b602001015160f81c60f81b189050888160405160200161356892919061409e565b60408051601f1981840301815291905298505060010161350f565b50868887604051602001613599939291906140cc565b604051602081830303815290604052965086805190602001209350836040516020016135c791815260200190565b60405160208183030381529060405291505f5b6135e78a60ff8d16613f16565b8110156136485782818151811061360057613600613f88565b01602001516001600160f81b0319168461361a838d613ed5565b8151811061362a5761362a613f88565b60200101906001600160f81b03191690815f1a9053506001016135da565b50919b9a5050505050505050505050565b5f80805b83518110156136b95783818151811061367857613678613f88565b602002602001015160ff16816008613690919061410c565b61369b906002614203565b6136a5919061410c565b6136af9083613ed5565b915060010161365d565b5092915050565b6060826136d5576136d082613935565b611397565b81511580156136ec57506001600160a01b0384163b155b156136b957604051639996b31560e01b81526001600160a01b0385166004820152602401610b45565b5f5f805160206142738339815191528161372d611e53565b80519091501561374557805160209091012092915050565b81548015613754579392505050565b7fc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470935050505090565b5f5f8051602061427383398151915281613795611f13565b8051909150156137ad57805160209091012092915050565b60018201548015613754579392505050565b82545f90819080156138db575f6137db876118d1600185613f16565b60408051808201909152905465ffffffffffff808216808452600160301b9092046001600160d01b03166020840152919250908716101561382f57604051632520601d60e01b815260040160405180910390fd5b805165ffffffffffff80881691160361387b5784613852886118d1600186613f16565b80546001600160d01b0392909216600160301b0265ffffffffffff9092169190911790556138cb565b6040805180820190915265ffffffffffff80881682526001600160d01b0380881660208085019182528b54600181018d555f8d81529190912094519151909216600160301b029216919091179101555b602001519250839150612dc69050565b50506040805180820190915265ffffffffffff80851682526001600160d01b0380851660208085019182528854600181018a555f8a815291822095519251909316600160301b029190931617920191909155905081612dc6565b8051156139455780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b0381168114613974575f80fd5b919050565b5f806040838503121561398a575f80fd5b6139938361395e565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f1916810167ffffffffffffffff811182821017156139de576139de6139a1565b604052919050565b5f608082840312156139f6575f80fd5b6040516080810181811067ffffffffffffffff82111715613a1957613a196139a1565b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f60408284031215613a5a575f80fd5b6040516040810181811067ffffffffffffffff82111715613a7d57613a7d6139a1565b604052823581526020928301359281019290925250919050565b5f805f806101208587031215613aab575f80fd5b613ab586866139e6565b9350613ac48660808701613a4a565b9250613ad38660c08701613a4a565b915061010085013561ffff81168114613aea575f80fd5b939692955090935050565b5f60208284031215613b05575f80fd5b6113978261395e565b5f5b83811015613b28578181015183820152602001613b10565b50505f910152565b5f8151808452613b47816020860160208601613b0e565b601f01601f19169290920160200192915050565b602081525f6113976020830184613b30565b5f8060408385031215613b7e575f80fd5b613b878361395e565b915060208084013567ffffffffffffffff80821115613ba4575f80fd5b818601915086601f830112613bb7575f80fd5b813581811115613bc957613bc96139a1565b613bdb601f8201601f191685016139b5565b91508082528784828501011115613bf0575f80fd5b80848401858401375f848284010152508093505050509250929050565b5f805f6101008486031215613c20575f80fd5b613c2a85856139e6565b9250613c398560808601613a4a565b9150613c488560c08601613a4a565b90509250925092565b60ff60f81b881681525f602060e06020840152613c7160e084018a613b30565b8381036040850152613c83818a613b30565b606085018990526001600160a01b038816608086015260a0850187905284810360c0860152855180825260208088019350909101905f5b81811015613cd657835183529284019291840191600101613cba565b50909c9b505050505050505050505050565b5f60208284031215613cf8575f80fd5b5035919050565b5f60808284031215613d0f575f80fd5b61139783836139e6565b5f8060408385031215613d2a575f80fd5b613d338361395e565b9150613d416020840161395e565b90509250929050565b5f805f8060808587031215613d5d575f80fd5b613d668561395e565b9350613d746020860161395e565b925060408501359150613d896060860161395e565b905092959194509250565b5f805f805f8060c08789031215613da9575f80fd5b613db28761395e565b95506020870135945060408701359350606087013560ff81168114613dd5575f80fd5b9598949750929560808101359460a0909101359350915050565b5f8060408385031215613e00575f80fd5b613e098361395e565b9150602083013563ffffffff81168114613e21575f80fd5b809150509250929050565b634e487b7160e01b5f52602160045260245ffd5b8281526040810160038310613e6357634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b825181526020808401518183015260408085015190830152606080850151908301528251608083015282015160a082015260c08101611397565b5f60208284031215613eba575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b80820180821115610b8757610b87613ec1565b5f8351613ef9818460208801613b0e565b835190830190613f0d818360208801613b0e565b01949350505050565b81810381811115610b8757610b87613ec1565b600181811c90821680613f3d57607f821691505b602082108103613f5b57634e487b7160e01b5f52602260045260245ffd5b50919050565b634e487b7160e01b5f52601260045260245ffd5b5f82613f8357613f83613f61565b500690565b634e487b7160e01b5f52603260045260245ffd5b6001600160d01b038181168382160190808211156136b9576136b9613ec1565b6001600160d01b038281168282160390808211156136b9576136b9613ec1565b634e487b7160e01b5f52600160045260245ffd5b5f82613ffe57613ffe613f61565b500490565b5f8251614014818460208701613b0e565b9190910192915050565b5f835161402f818460208801613b0e565b5f9201918252506001600160f81b0319919091166001820152600201919050565b5f8251614061818460208701613b0e565b5f920191825250600101919050565b5f8351614081818460208801613b0e565b6001600160f01b0319939093169190920190815260020192915050565b5f83516140af818460208801613b0e565b6001600160f81b0319939093169190920190815260010192915050565b5f84516140dd818460208901613b0e565b6001600160f81b031994909416919093019081526001600160f01b031991909116600182015260030192915050565b8082028115828204841417610b8757610b87613ec1565b600181815b8085111561415d57815f190482111561414357614143613ec1565b8085161561415057918102915b93841c9390800290614128565b509250929050565b5f8261417357506001610b87565b8161417f57505f610b87565b8160018114614195576002811461419f576141bb565b6001915050610b87565b60ff8411156141b0576141b0613ec1565b50506001821b610b87565b5060208310610133831016604e8410600b84101617156141de575081810a610b87565b6141e88383614123565b805f19048211156141fb576141fb613ec1565b029392505050565b5f611397838361416556fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d0030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47a16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d100360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15b\0\0\x14W_\x80\xFD[Pb\0\0\x1Fb\0\0/V[b\0\0)b\0\0/V[b\0\0\xE3V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15b\0\0\x80W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14b\0\0\xE0W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80QaB\xBFb\0\x01\n_9_\x81\x81a\x1B\0\x01R\x81\x81a\x1B)\x01Ra\x1C\xAC\x01RaB\xBF_\xF3\xFE`\x80`@R`\x046\x10a\x02(W_5`\xE0\x1C\x80c\x8ES\x9E\x8C\x11a\x01)W\x80c\xB5\xEC\xB3D\x11a\0\xA8W\x80c\xDA\x8B\xE8d\x11a\0mW\x80c\xDA\x8B\xE8d\x14a\x07CW\x80c\xF1\x12~\xD8\x14a\x07bW\x80c\xF2\xFD\xE3\x8B\x14a\x07\xADW\x80c\xFAR\xC7\xD8\x14a\x07\xCCW\x80c\xFC\x0CTj\x14a\x08\x0FW_\x80\xFD[\x80c\xB5\xEC\xB3D\x14a\x06pW\x80c\xBE 0\x94\x14a\x06\x9BW\x80c\xC3\xCD\xA5 \x14a\x06\xBAW\x80c\xC6H\x14\xDD\x14a\x06\xD9W\x80c\xD2\xB7P\xE5\x14a\x07\x0FW_\x80\xFD[\x80c\xA2\xD7\x8D\xD5\x11a\0\xEEW\x80c\xA2\xD7\x8D\xD5\x14a\x05rW\x80c\xA3\x06j\xAB\x14a\x05\xC4W\x80c\xAD<\xB1\xCC\x14a\x05\xE3W\x80c\xB3\xE6\xEB\xD5\x14a\x06\x13W\x80c\xB5p\x0Eh\x14a\x06QW_\x80\xFD[\x80c\x8ES\x9E\x8C\x14a\x04\xD5W\x80c\x91\xDD\xAD\xF4\x14a\x04\xF4W\x80c\x9A\xB2N\xB0\x14a\x05\x1FW\x80c\x9B0\xA5\xE6\x14a\x05>W\x80c\x9E\x9A\x8F1\x14a\x05]W_\x80\xFD[\x80cR\xD1\x90-\x11a\x01\xB5W\x80co\xCF\xFFE\x11a\x01zW\x80co\xCF\xFFE\x14a\x03\xECW\x80cqP\x18\xA6\x14a\x04\x0BW\x80c~\xCE\xBE\0\x14a\x04\x1FW\x80c\x84\xB0\x19n\x14a\x04rW\x80c\x8D\xA5\xCB[\x14a\x04\x99W_\x80\xFD[\x80cR\xD1\x90-\x14a\x03OW\x80cUD\xC2\xF1\x14a\x03cW\x80cX|\xDE\x1E\x14a\x03\x82W\x80c\\\x19\xA9\\\x14a\x03\xB9W\x80cj\x91\x1C\xCF\x14a\x03\xD8W_\x80\xFD[\x80c:F\xB1\xA8\x11a\x01\xFBW\x80c:F\xB1\xA8\x14a\x02\xBBW\x80c>\x9D\xF9\xB5\x14a\x02\xE8W\x80cK\xF5\xD7\xE9\x14a\x02\xFCW\x80cM\x99\xDD\x16\x14a\x03\x1DW\x80cO\x1E\xF2\x86\x14a\x03<W_\x80\xFD[\x80c\x02n@+\x14a\x02,W\x80c\r\x8En,\x14a\x02MW\x80c\x13\xB9\x05z\x14a\x02}W\x80c!@\xFE\xCD\x14a\x02\x9CW[_\x80\xFD[4\x80\x15a\x027W_\x80\xFD[Pa\x02Ka\x02F6`\x04a9yV[a\x08.V[\0[4\x80\x15a\x02XW_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x88W_\x80\xFD[Pa\x02Ka\x02\x976`\x04a:\x97V[a\x08<V[4\x80\x15a\x02\xA7W_\x80\xFD[Pa\x02Ka\x02\xB66`\x04a:\xF5V[a\t\xD5V[4\x80\x15a\x02\xC6W_\x80\xFD[Pa\x02\xDAa\x02\xD56`\x04a9yV[a\n\xF6V[`@Q\x90\x81R` \x01a\x02tV[4\x80\x15a\x02\xF3W_\x80\xFD[Pa\x02\xDA_T\x81V[4\x80\x15a\x03\x07W_\x80\xFD[Pa\x03\x10a\x0B\x8DV[`@Qa\x02t\x91\x90a;[V[4\x80\x15a\x03(W_\x80\xFD[Pa\x02Ka\x0376`\x04a9yV[a\x0C\x05V[a\x02Ka\x03J6`\x04a;mV[a\x0C\x0FV[4\x80\x15a\x03ZW_\x80\xFD[Pa\x02\xDAa\x0C*V[4\x80\x15a\x03nW_\x80\xFD[Pa\x02Ka\x03}6`\x04a<\rV[a\x0CEV[4\x80\x15a\x03\x8DW_\x80\xFD[Pa\x03\xA1a\x03\x9C6`\x04a:\xF5V[a\r\x0EV[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02tV[4\x80\x15a\x03\xC4W_\x80\xFD[Pa\x02Ka\x03\xD36`\x04a:\xF5V[a\r7V[4\x80\x15a\x03\xE3W_\x80\xFD[Pa\x02Ka\r\xBDV[4\x80\x15a\x03\xF7W_\x80\xFD[Pa\x02\xDAa\x04\x066`\x04a:\xF5V[a\x0E?V[4\x80\x15a\x04\x16W_\x80\xFD[Pa\x02Ka\x0EUV[4\x80\x15a\x04*W_\x80\xFD[Pa\x02\xDAa\x0496`\x04a:\xF5V[`\x01`\x01`\xA0\x1B\x03\x16_\x90\x81R\x7FZ\xB4,\xEDb\x88\x88%\x9C\x08\xAC\x98\xDB\x1E\xB0\xCFp/\xC1P\x13D1\x1D\x8B\x10\x0C\xD1\xBF\xE4\xBB\0` R`@\x90 T\x90V[4\x80\x15a\x04}W_\x80\xFD[Pa\x04\x86a\x0EhV[`@Qa\x02t\x97\x96\x95\x94\x93\x92\x91\x90a<QV[4\x80\x15a\x04\xA4W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x03\xA1V[4\x80\x15a\x04\xE0W_\x80\xFD[Pa\x02\xDAa\x04\xEF6`\x04a<\xE8V[a\x0F\x11V[4\x80\x15a\x04\xFFW_\x80\xFD[Pa\x05\x08a\x0F\x8BV[`@Qe\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02tV[4\x80\x15a\x05*W_\x80\xFD[Pa\x02\xDAa\x0596`\x04a:\xF5V[a\x0F\x99V[4\x80\x15a\x05IW_\x80\xFD[Pa\x02\xDAa\x05X6`\x04a<\xFFV[a\x0F\xF7V[4\x80\x15a\x05hW_\x80\xFD[Pa\x02\xDA`\tT\x81V[4\x80\x15a\x05}W_\x80\xFD[Pa\x05\xAFa\x05\x8C6`\x04a=\x19V[`\x07` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 \x80T`\x01\x90\x91\x01T\x82V[`@\x80Q\x92\x83R` \x83\x01\x91\x90\x91R\x01a\x02tV[4\x80\x15a\x05\xCFW_\x80\xFD[Pa\x02Ka\x05\xDE6`\x04a:\xF5V[a\x10QV[4\x80\x15a\x05\xEEW_\x80\xFD[Pa\x03\x10`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[4\x80\x15a\x06\x1EW_\x80\xFD[Pa\x06Aa\x06-6`\x04a<\xE8V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01a\x02tV[4\x80\x15a\x06\\W_\x80\xFD[P`\x01Ta\x03\xA1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x06{W_\x80\xFD[Pa\x02\xDAa\x06\x8A6`\x04a:\xF5V[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x06\xA6W_\x80\xFD[Pa\x02Ka\x06\xB56`\x04a=JV[a\x11aV[4\x80\x15a\x06\xC5W_\x80\xFD[Pa\x02Ka\x06\xD46`\x04a=\x94V[a\x12\x8DV[4\x80\x15a\x06\xE4W_\x80\xFD[Pa\x02\xDAa\x06\xF36`\x04a=\x19V[`\x06` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 T\x81V[4\x80\x15a\x07\x1AW_\x80\xFD[Pa\x03\xA1a\x07)6`\x04a:\xF5V[`\x08` R_\x90\x81R`@\x90 T`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x07NW_\x80\xFD[Pa\x02Ka\x07]6`\x04a:\xF5V[a\x13IV[4\x80\x15a\x07mW_\x80\xFD[Pa\x07\x81a\x07|6`\x04a=\xEFV[a\x13zV[`@\x80Q\x82Qe\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81R` \x92\x83\x01Q`\x01`\x01`\xD0\x1B\x03\x16\x92\x81\x01\x92\x90\x92R\x01a\x02tV[4\x80\x15a\x07\xB8W_\x80\xFD[Pa\x02Ka\x07\xC76`\x04a:\xF5V[a\x13\x9EV[4\x80\x15a\x07\xD7W_\x80\xFD[Pa\x08\x01a\x07\xE66`\x04a:\xF5V[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x02t\x92\x91\x90a>@V[4\x80\x15a\x08\x1AW_\x80\xFD[P`\x02Ta\x03\xA1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x088\x82\x82a\x13\xD8V[PPV[3a\x08F\x81a\x169V[a\x08O\x84a\x16\x86V[a\x08X\x85a\x16\xC1V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x08\x8A\x81\x85\x88a\x16\xFDV[a'\x10\x83a\xFF\xFF\x16\x11\x15a\x08\xB1W`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x08\xBF\x89a\x0F\xF7V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\t\x06Wa\t\x06a>,V[\x90R`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\tLWa\tLa>,V[\x02\x17\x90UPP`@\x80Q\x88Q\x81R` \x80\x8A\x01Q\x81\x83\x01R\x82\x8A\x01Q\x92\x82\x01\x92\x90\x92R``\x80\x8A\x01Q\x90\x82\x01R\x87Q`\x80\x82\x01R\x90\x87\x01Q`\xA0\x82\x01Ra\xFF\xFF\x85\x16`\xC0\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16\x91P\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90`\xE0\x01[`@Q\x80\x91\x03\x90\xA2PPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T3\x91\x81\x90\x03a\n\x0FW`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\n0W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 T\x90\x81\x90\x03a\nxW`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x87\x85\x16\x84R\x90\x91R\x81 U`\x02Ta\n\xAD\x91\x16\x84\x83a\x17\x92V[\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\n\xE8\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPPV[__\x80Q` aB3\x839\x81Q\x91R\x81a\x0B\x0Ea\x0F\x8BV[\x90P\x80e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x84\x10a\x0BNW`@Qcvi\xFC\x0F`\xE1\x1B\x81R`\x04\x81\x01\x85\x90Re\xFF\xFF\xFF\xFF\xFF\xFF\x82\x16`$\x82\x01R`D\x01[`@Q\x80\x91\x03\x90\xFD[a\x0Bya\x0BZ\x85a\x18\x1BV[`\x01`\x01`\xA0\x1B\x03\x87\x16_\x90\x81R`\x01\x85\x01` R`@\x90 \x90a\x18QV[`\x01`\x01`\xD0\x1B\x03\x16\x92PPP[\x92\x91PPV[``a\x0B\x97a\x19\x01V[e\xFF\xFF\xFF\xFF\xFF\xFF\x16a\x0B\xA7a\x0F\x8BV[e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x14a\x0B\xCDW`@Qc\x01\xBF\xC1\xC5`\xE6\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`@\x80Q\x80\x82\x01\x90\x91R`\x1D\x81R\x7Fmode=blocknumber&from=default\0\0\0` \x82\x01R\x90V[a\x088\x82\x82a\x19\x0BV[a\x0C\x17a\x1A\xF5V[a\x0C \x82a\x1B\x99V[a\x088\x82\x82a\x1B\xE0V[_a\x0C3a\x1C\xA1V[P_\x80Q` aB\x93\x839\x81Q\x91R\x90V[3a\x0CO\x81a\x1C\xEAV[a\x0CX\x83a\x16\x86V[a\x0Ca\x84a\x16\xC1V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0C\x93\x81\x84\x87a\x16\xFDV[`\x01`\x04_a\x0C\xA1\x88a\x0F\xF7V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D\x86\x86`@Qa\x0C\xFF\x92\x91\x90a>pV[`@Q\x80\x91\x03\x90\xA2PPPPPV[`\x01`\x01`\xA0\x1B\x03\x90\x81\x16_\x90\x81R_\x80Q` aB3\x839\x81Q\x91R` R`@\x90 T\x16\x90V[`\x02T_\x90`\x01`\x01`\xA0\x1B\x03\x16cp\xA0\x8213`@Q`\x01`\x01`\xE0\x1B\x03\x19`\xE0\x84\x90\x1B\x16\x81R`\x01`\x01`\xA0\x1B\x03\x90\x91\x16`\x04\x82\x01R`$\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\r\x8DW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\r\xB1\x91\x90a>\xAAV[\x90Pa\x088\x82\x82a\x13\xD8V[3a\r\xC7\x81a\x1C\xEAV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\tTa\r\xF9\x90Ba>\xD5V[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R`\x05` R`@\x80\x82 \x93\x90\x93U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2PV[_a\x0EI\x82a\x1D9V[c\xFF\xFF\xFF\xFF\x16\x92\x91PPV[a\x0E]a\x1D\x88V[a\x0Ef_a\x1D\xE3V[V[_``\x80\x82\x80\x80\x83\x81_\x80Q` aBs\x839\x81Q\x91R\x80T\x90\x91P\x15\x80\x15a\x0E\x93WP`\x01\x81\x01T\x15[a\x0E\xD7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x15`$\x82\x01Rt\x11RT\r\xCCL\x8E\x88\x15[\x9A[\x9A]\x1AX[\x1A^\x99Y`Z\x1B`D\x82\x01R`d\x01a\x0BEV[a\x0E\xDFa\x1ESV[a\x0E\xE7a\x1F\x13V[`@\x80Q_\x80\x82R` \x82\x01\x90\x92R`\x0F`\xF8\x1B\x9C\x93\x9BP\x91\x99PF\x98P0\x97P\x95P\x93P\x91PPV[__\x80Q` aB3\x839\x81Q\x91R\x81a\x0F)a\x0F\x8BV[\x90P\x80e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x84\x10a\x0FdW`@Qcvi\xFC\x0F`\xE1\x1B\x81R`\x04\x81\x01\x85\x90Re\xFF\xFF\xFF\xFF\xFF\xFF\x82\x16`$\x82\x01R`D\x01a\x0BEV[a\x0Fza\x0Fp\x85a\x18\x1BV[`\x02\x84\x01\x90a\x18QV[`\x01`\x01`\xD0\x1B\x03\x16\x94\x93PPPPV[_a\x0F\x94a\x19\x01V[\x90P\x90V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01` R`@\x81 _\x80Q` aB3\x839\x81Q\x91R\x90a\x0F\xE7\x90a\x1FQV[`\x01`\x01`\xD0\x1B\x03\x16\x93\x92PPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x104\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 T\x90\x91\x81\x90\x03a\x10\x96W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R `\x01\x01TB\x10\x15a\x10\xDEW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x86\x85\x16\x84R\x90\x91R\x81 \x81\x81U`\x01\x01U`\x02Ta\x11\x19\x91\x16\x83\x83a\x17\x92V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x11T\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x11\xA6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x11\xC2WP0;\x15[\x90P\x81\x15\x80\x15a\x11\xD0WP\x80\x15[\x15a\x11\xEEW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x12\x18W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x12!\x86a\x1F\x88V[a\x12)a\x1F\x99V[a\x121a\x1F\xA1V[a\x12<\x89\x89\x89a \xA8V[\x83\x15a\x12\x82W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[\x83B\x11\x15a\x12\xB1W`@Qc#A\xD7\x87`\xE1\x1B\x81R`\x04\x81\x01\x85\x90R`$\x01a\x0BEV[`@\x80Q\x7F\xE4\x83)\x05{\xFD\x03\xD5^I\xB5G\x13.9\xCF\xFD\x9C\x18 \xAD{\x9DLS\x07i\x14%\xD1Z\xDF` \x82\x01R`\x01`\x01`\xA0\x1B\x03\x88\x16\x91\x81\x01\x91\x90\x91R``\x81\x01\x86\x90R`\x80\x81\x01\x85\x90R_\x90a\x13*\x90a\x13\"\x90`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 a!+V[\x85\x85\x85a!WV[\x90Pa\x136\x81\x87a!\x83V[a\x13@\x81\x88a!\xF4V[PPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 Ta\x13w\x90\x82\x90a\x19\x0BV[PV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x13\x97\x83\x83a\"\xA0V[\x93\x92PPPV[a\x13\xA6a\x1D\x88V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x13\xCFW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x0BEV[a\x13w\x81a\x1D\xE3V[a\x13\xE1\x82a\x1C\xEAV[3_\x81\x81R`\x03` R`@\x90 `\x01\x90\x81\x01T`\xFF\x16`\x02\x81\x11\x15a\x14\tWa\x14\ta>,V[\x03a\x14'W`@QcD4\xD1\x93`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x81\x16_\x90\x81R`\x08` R`@\x90 T\x16\x15\x80\x15\x90a\x14kWP`\x01`\x01`\xA0\x1B\x03\x81\x81\x16_\x90\x81R`\x08` R`@\x90 T\x81\x16\x90\x84\x16\x14\x15[\x15a\x14\x89W`@Qc\x0CY]\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R0`$\x83\x01R_\x92\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x14\xD7W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x14\xFB\x91\x90a>\xAAV[\x90P\x82\x81\x10\x15a\x15(W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x84\x90R`D\x01a\x0BEV[a\x151_a#\tV[`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x03` R`@\x81 \x80T\x85\x92\x90a\x15X\x90\x84\x90a>\xD5V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x15\x93\x90\x84\x90a>\xD5V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x82\x81\x16_\x90\x81R`\x08` R`@\x90 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x86\x83\x16\x17\x90U`\x02Ta\x15\xD2\x91\x16\x830\x86a#\x14V[\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F]\xAC\x0C\x1B\x11\x12VJ\x04[\xA9C\xC9\xD5\x02p\x89>\x8E\x82lI\xBE\x8Eps\xAD\xC7\x13\xAB{\xD7\x85`@Qa\x16\x17\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3a\x16(\x84a#\tV[a\x163_\x85\x85a#\xB1V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x16hWa\x16ha>,V[\x14a\x13wW`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16\xA3\x82\x82a$'V[\x15a\x088W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x16\xCD\x83a\x0F\xF7V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x13wW`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x17\x06\x82a$CV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01aB\x0F`$\x919\x90P_\x84\x82`@Q` \x01a\x176\x92\x91\x90a>\xE8V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x17Q\x82a$\xD9V[\x90Pa\x17n\x81\x85a\x17a\x88a%\xC6V[a\x17ia&=V[a'\nV[a\x17\x8AW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x17\xDCWP\x82;\x15=\x17\x15[\x80a\x163W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x0BEV[_e\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R`0`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[P\x90V[\x81T_\x90\x81\x81`\x05\x81\x11\x15a\x18\xADW_a\x18j\x84a'\xE8V[a\x18t\x90\x85a?\x16V[_\x88\x81R` \x90 \x90\x91P\x81\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x90\x81\x16\x90\x87\x16\x10\x15a\x18\x9DW\x80\x91Pa\x18\xABV[a\x18\xA8\x81`\x01a>\xD5V[\x92P[P[_a\x18\xBA\x87\x87\x85\x85a(\xCCV[\x90P\x80\x15a\x18\xF4Wa\x18\xDE\x87a\x18\xD1`\x01\x84a?\x16V[_\x91\x82R` \x90\x91 \x01\x90V[T`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16a\x18\xF6V[_[\x97\x96PPPPPPPV[_a\x0F\x94Ca\x18\x1BV[a\x19\x14\x82a\x1C\xEAV[3_\x81\x81R`\x03` R`@\x90 `\x01\x01T`\x02\x90`\xFF\x16\x81\x81\x11\x15a\x19<Wa\x19<a>,V[\x03a\x19ZW`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x82\x81\x10\x15a\x19\xA3W`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[a\x19\xAC_a#\tV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x19\xE2\x90\x84\x90a?\x16V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x84\x81R` \x01`\tTBa\x1A\x08\x91\x90a>\xD5V[\x90R`\x01`\x01`\xA0\x1B\x03\x80\x86\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 \x94\x88\x16\x83R\x93\x81R\x83\x82 \x85Q\x81U\x94\x81\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x90\x81R`\x03\x90\x92R\x81 \x80T\x85\x92\x90a\x1A[\x90\x84\x90a?\x16V[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x85`@Qa\x1A\xA7\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3\x80\x83\x14a\x1A\xC4Wa\x1A\xBF\x84a#\tV[a\x1A\xEAV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x08` R`@\x90 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U[a\x163\x84_\x85a#\xB1V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1B{WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1Bo_\x80Q` aB\x93\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0EfW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1B\xA1a\x1D\x88V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01`@Q\x80\x91\x03\x90\xA1PV[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1C:WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1C7\x91\x81\x01\x90a>\xAAV[`\x01[a\x1CbW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x0BEV[_\x80Q` aB\x93\x839\x81Q\x91R\x81\x14a\x1C\x92W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[a\x1C\x9C\x83\x83a)+V[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0EfW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x1D\x1BWa\x1D\x1Ba>,V[\x14a\x13wW`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01` R`@\x81 T_\x80Q` aB3\x839\x81Q\x91R\x90a\x13\x97\x90a)\x80V[3a\x1D\xBA\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0EfW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x0BEV[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[\x7F\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x02\x80T``\x91_\x80Q` aBs\x839\x81Q\x91R\x91a\x1E\x91\x90a?)V[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x1E\xBD\x90a?)V[\x80\x15a\x1F\x08W\x80`\x1F\x10a\x1E\xDFWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x1F\x08V[\x82\x01\x91\x90_R` _ \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x1E\xEBW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x91PP\x90V[\x7F\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x03\x80T``\x91_\x80Q` aBs\x839\x81Q\x91R\x91a\x1E\x91\x90a?)V[\x80T_\x90\x80\x15a\x1F\x80Wa\x1Fj\x83a\x18\xD1`\x01\x84a?\x16V[T`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16a\x13\x97V[_\x93\x92PPPV[a\x1F\x90a)\xB0V[a\x13w\x81a)\xF9V[a\x0Efa)\xB0V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x1F\xE6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a \x02WP0;\x15[\x90P\x81\x15\x80\x15a \x10WP\x80\x15[\x15a .W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a XW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a \xA1W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a \xCFW`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a \xF6W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x94\x85\x16`\x01`\x01`\xA0\x1B\x03\x19\x91\x82\x16\x17\x90\x91U`\x01\x80T\x93\x90\x94\x16\x92\x16\x91\x90\x91\x17\x90\x91U`\tUV[_a\x0B\x87a!7a*\x01V[\x83`@Qa\x19\x01`\xF0\x1B\x81R`\x02\x81\x01\x92\x90\x92R`\"\x82\x01R`B\x90 \x90V[_\x80_\x80a!g\x88\x88\x88\x88a*\nV[\x92P\x92P\x92Pa!w\x82\x82a*\xD2V[P\x90\x96\x95PPPPPPV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R\x7FZ\xB4,\xEDb\x88\x88%\x9C\x08\xAC\x98\xDB\x1E\xB0\xCFp/\xC1P\x13D1\x1D\x8B\x10\x0C\xD1\xBF\xE4\xBB\0` R`@\x90 \x80T`\x01\x81\x01\x90\x91U\x81\x81\x14a\x1C\x9CW`@Qc\x01\xD4\xB6#`\xE6\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R_a\"\x0C\x84a\r\x0EV[`\x01`\x01`\xA0\x1B\x03\x85\x81\x16_\x81\x81R` \x86\x90R`@\x80\x82 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x89\x86\x16\x90\x81\x17\x90\x91U\x90Q\x94\x95P\x93\x92\x85\x16\x92\x7F14\xE8\xA2\xE6\xD9~\x92\x9A~T\x01\x1E\xA5H]}\x19m\xD5\xF0\xBAMN\xF9X\x03\xE8\xE3\xFC%\x7F\x91\x90\xA4a\x163\x81\x84a\"\x9B\x87`\x01`\x01`\xA0\x1B\x03\x90\x81\x16_\x90\x81R`\x08` \x90\x81R`@\x80\x83 T\x90\x93\x16\x82R`\x03\x90R T\x90V[a+\x8AV[`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x80\x83\x01\x82\x90R`\x01`\x01`\xA0\x1B\x03\x86\x16\x82R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01\x90R\x91\x90\x91 _\x80Q` aB3\x839\x81Q\x91R\x90a#\x01\x90\x84a,\xEBV[\x94\x93PPPPV[3a\x088\x81\x83a!\xF4V[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a#mWP\x83;\x15=\x17\x15[\x80a \xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R`\x01`\x01`\xA0\x1B\x03\x84\x16a#\xE8Wa#\xE5\x81`\x02\x01a-Xa#\xE0\x85a-cV[a-\x96V[PP[`\x01`\x01`\xA0\x1B\x03\x83\x16a$\x0CWa$\t\x81`\x02\x01a-\xCEa#\xE0\x85a-cV[PP[a\x163a$\x18\x85a\r\x0EV[a$!\x85a\r\x0EV[\x84a+\x8AV[\x80Q\x82Q_\x91\x14\x80\x15a\x13\x97WPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` aBS\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a$iWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x1C\x9CW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a$\xF6\x83a-\xD9V[\x90P_\x80Q` aBS\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a%\x1DWa%\x1Da?aV[\x84\x82\t\x90P\x82\x80a%0Wa%0a?aV[\x82\x82\x08\x90P_\x80a%@\x83a/\xE2V[\x92P\x90P[\x80a%\xA9W\x84\x80a%XWa%Xa?aV[`\x01\x87\x08\x95P\x84\x80a%lWa%la?aV[\x86\x87\t\x92P\x84\x80a%\x7FWa%\x7Fa?aV[\x86\x84\t\x92P\x84\x80a%\x92Wa%\x92a?aV[\x84\x84\x08\x92Pa%\xA0\x83a/\xE2V[\x92P\x90Pa%EV[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a%\xEDWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` aBS\x839\x81Q\x91R\x84` \x01Qa&\x1E\x91\x90a?uV[a&5\x90_\x80Q` aBS\x839\x81Q\x91Ra?\x16V[\x90R\x92\x91PPV[a&d`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a'\xDCW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[P\x15\x15\x95\x94PPPPPV[_\x81_\x03a'\xF7WP_\x91\x90PV[_`\x01a(\x03\x84a0\xD9V[\x90\x1C`\x01\x90\x1B\x90P`\x01\x81\x84\x81a(\x1CWa(\x1Ca?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(4Wa(4a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(LWa(La?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(dWa(da?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(|Wa(|a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(\x94Wa(\x94a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(\xACWa(\xACa?aV[\x04\x82\x01\x90\x1C\x90Pa\x13\x97\x81\x82\x85\x81a(\xC6Wa(\xC6a?aV[\x04a1lV[_[\x81\x83\x10\x15a)#W_a(\xE1\x84\x84a1\x81V[_\x87\x81R` \x90 \x90\x91Pe\xFF\xFF\xFF\xFF\xFF\xFF\x86\x16\x90\x82\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x16\x11\x15a)\x0FW\x80\x92Pa)\x1DV[a)\x1A\x81`\x01a>\xD5V[\x93P[Pa(\xCEV[P\x93\x92PPPV[a)4\x82a1\x9BV[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a)xWa\x1C\x9C\x82\x82a1\xFEV[a\x088a2pV[_c\xFF\xFF\xFF\xFF\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R` `\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x0EfW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x13\xA6a)\xB0V[_a\x0F\x94a2\x8FV[_\x80\x80\x7F\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF]WnsW\xA4P\x1D\xDF\xE9/Fh\x1B \xA0\x84\x11\x15a*CWP_\x91P`\x03\x90P\x82a*\xC8V[`@\x80Q_\x80\x82R` \x82\x01\x80\x84R\x8A\x90R`\xFF\x89\x16\x92\x82\x01\x92\x90\x92R``\x81\x01\x87\x90R`\x80\x81\x01\x86\x90R`\x01\x90`\xA0\x01` `@Q` \x81\x03\x90\x80\x84\x03\x90\x85Z\xFA\x15\x80\x15a*\x94W=_\x80>=_\xFD[PP`@Q`\x1F\x19\x01Q\x91PP`\x01`\x01`\xA0\x1B\x03\x81\x16a*\xBFWP_\x92P`\x01\x91P\x82\x90Pa*\xC8V[\x92P_\x91P\x81\x90P[\x94P\x94P\x94\x91PPV[_\x82`\x03\x81\x11\x15a*\xE5Wa*\xE5a>,V[\x03a*\xEEWPPV[`\x01\x82`\x03\x81\x11\x15a+\x02Wa+\x02a>,V[\x03a+ W`@Qc\xF6E\xEE\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x82`\x03\x81\x11\x15a+4Wa+4a>,V[\x03a+UW`@Qc\xFC\xE6\x98\xF7`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[`\x03\x82`\x03\x81\x11\x15a+iWa+ia>,V[\x03a\x088W`@Qc5\xE2\xF3\x83`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x90\x84\x16\x14\x80\x15\x90a+\xB3WP_\x82\x11[\x15a\x163W`\x01`\x01`\xA0\x1B\x03\x84\x16\x15a,WW`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x01\x82\x01` R`@\x81 \x81\x90a+\xF2\x90a-\xCEa#\xE0\x87a-cV[`\x01`\x01`\xD0\x1B\x03\x16\x91P`\x01`\x01`\xD0\x1B\x03\x16\x91P\x85`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDE\xC2\xBA\xCD\xD2\xF0[Y\xDE4\xDA\x9BR=\xFF\x8B\xE4.^8\xE8\x18\xC8/\xDB\x0B\xAEwC\x87\xA7$\x83\x83`@Qa,L\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[`@Q\x80\x91\x03\x90\xA2PP[`\x01`\x01`\xA0\x1B\x03\x83\x16\x15a\x163W`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x01\x82\x01` R`@\x81 \x81\x90a,\x91\x90a-Xa#\xE0\x87a-cV[`\x01`\x01`\xD0\x1B\x03\x16\x91P`\x01`\x01`\xD0\x1B\x03\x16\x91P\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDE\xC2\xBA\xCD\xD2\xF0[Y\xDE4\xDA\x9BR=\xFF\x8B\xE4.^8\xE8\x18\xC8/\xDB\x0B\xAEwC\x87\xA7$\x83\x83`@Qa\t\xC5\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x82_\x01\x82c\xFF\xFF\xFF\xFF\x16\x81T\x81\x10a-\x18Wa-\x18a?\x88V[_\x91\x82R` \x91\x82\x90 `@\x80Q\x80\x82\x01\x90\x91R\x91\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x82R`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16\x91\x81\x01\x91\x90\x91R\x93\x92PPPV[_a\x13\x97\x82\x84a?\x9CV[_`\x01`\x01`\xD0\x1B\x03\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R`\xD0`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[_\x80a-\xC1a-\xA3a\x0F\x8BV[a-\xB9a-\xAF\x88a\x1FQV[\x86\x88c\xFF\xFF\xFF\xFF\x16V[\x87\x91\x90a3\x02V[\x91P\x91P[\x93P\x93\x91PPV[_a\x13\x97\x82\x84a?\xBCV[_\x80a-\xE4\x83a3\x0FV[\x80Q\x90\x91P`0\x81\x14a-\xF9Wa-\xF9a?\xDCV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a.\x13Wa.\x13a9\xA1V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a.=W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a.\xACW\x83`\x01a.W\x83\x86a?\x16V[a.a\x91\x90a?\x16V[\x81Q\x81\x10a.qWa.qa?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a.\x8EWa.\x8Ea?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a.BV[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a/<W\x83\x81a.\xE8\x85\x88a?\x16V[a.\xF2\x91\x90a>\xD5V[\x81Q\x81\x10a/\x02Wa/\x02a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a/\"Wa/\"a?\x88V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a.\xD4V[P_a/G\x82a6YV[\x90Pa\x01\0_\x80Q` aBS\x839\x81Q\x91R_a/e\x86\x89a?\x16V[\x90P_[\x81\x81\x10\x15a/\xD2W_\x88`\x01a/\x7F\x84\x86a?\x16V[a/\x89\x91\x90a?\x16V[\x81Q\x81\x10a/\x99Wa/\x99a?\x88V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a/\xB1Wa/\xB1a?aV[\x85\x87\t\x95P\x83\x80a/\xC4Wa/\xC4a?aV[\x81\x87\x08\x95PP`\x01\x01a/iV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` aBS\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a0\x9FW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[\x80`\x01\x84\x90\x1B\x11\x15a0\xB8Wa0\xB5\x83\x82a?\x16V[\x92P[\x80\x80a0\xC6Wa0\xC6a?aV[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[_\x80`\x80\x83\x90\x1C\x15a0\xEDW`\x80\x92\x83\x1C\x92\x01[`@\x83\x90\x1C\x15a0\xFFW`@\x92\x83\x1C\x92\x01[` \x83\x90\x1C\x15a1\x11W` \x92\x83\x1C\x92\x01[`\x10\x83\x90\x1C\x15a1#W`\x10\x92\x83\x1C\x92\x01[`\x08\x83\x90\x1C\x15a15W`\x08\x92\x83\x1C\x92\x01[`\x04\x83\x90\x1C\x15a1GW`\x04\x92\x83\x1C\x92\x01[`\x02\x83\x90\x1C\x15a1YW`\x02\x92\x83\x1C\x92\x01[`\x01\x83\x90\x1C\x15a\x0B\x87W`\x01\x01\x92\x91PPV[_\x81\x83\x10a1zW\x81a\x13\x97V[P\x90\x91\x90PV[_a1\x8F`\x02\x84\x84\x18a?\xF0V[a\x13\x97\x90\x84\x84\x16a>\xD5V[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a1\xD0W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x0BEV[_\x80Q` aB\x93\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa2\x1A\x91\x90a@\x03V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a2RW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a2WV[``\x91P[P\x91P\x91Pa2g\x85\x83\x83a6\xC0V[\x95\x94PPPPPV[4\x15a\x0EfW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x7F\x8Bs\xC3\xC6\x9B\xB8\xFE=Q.\xCCL\xF7Y\xCCy#\x9F{\x17\x9B\x0F\xFA\xCA\xA9\xA7]R+9@\x0Fa2\xB9a7\x15V[a2\xC1a7}V[`@\x80Q` \x81\x01\x94\x90\x94R\x83\x01\x91\x90\x91R``\x82\x01RF`\x80\x82\x01R0`\xA0\x82\x01R`\xC0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x90V[_\x80a-\xC1\x85\x85\x85a7\xBFV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a3O\x92\x91\x90a>\xE8V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a3v\x92\x91\x90a@\x1EV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a3\x98\x91\x90a@PV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a3\xC2\x90\x83\x90\x83\x90` \x01a@pV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a42Wa42a9\xA1V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a4\\W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a4s\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a4\xDCW\x81\x81\x81Q\x81\x10a4\xA1Wa4\xA1a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a4\xBEWa4\xBEa?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a4\x86V[P_\x84`@Q` \x01a4\xF1\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a5\x83W_\x83\x82\x81Q\x81\x10a5*Wa5*a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a5GWa5Ga?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a5h\x92\x91\x90a@\x9EV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a5\x0FV[P\x86\x88\x87`@Q` \x01a5\x99\x93\x92\x91\x90a@\xCCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a5\xC7\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a5\xE7\x8A`\xFF\x8D\x16a?\x16V[\x81\x10\x15a6HW\x82\x81\x81Q\x81\x10a6\0Wa6\0a?\x88V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a6\x1A\x83\x8Da>\xD5V[\x81Q\x81\x10a6*Wa6*a?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a5\xDAV[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a6\xB9W\x83\x81\x81Q\x81\x10a6xWa6xa?\x88V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a6\x90\x91\x90aA\x0CV[a6\x9B\x90`\x02aB\x03V[a6\xA5\x91\x90aA\x0CV[a6\xAF\x90\x83a>\xD5V[\x91P`\x01\x01a6]V[P\x92\x91PPV[``\x82a6\xD5Wa6\xD0\x82a95V[a\x13\x97V[\x81Q\x15\x80\x15a6\xECWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a6\xB9W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x0BEV[__\x80Q` aBs\x839\x81Q\x91R\x81a7-a\x1ESV[\x80Q\x90\x91P\x15a7EW\x80Q` \x90\x91\x01 \x92\x91PPV[\x81T\x80\x15a7TW\x93\x92PPPV[\x7F\xC5\xD2F\x01\x86\xF7#<\x92~}\xB2\xDC\xC7\x03\xC0\xE5\0\xB6S\xCA\x82';{\xFA\xD8\x04]\x85\xA4p\x93PPPP\x90V[__\x80Q` aBs\x839\x81Q\x91R\x81a7\x95a\x1F\x13V[\x80Q\x90\x91P\x15a7\xADW\x80Q` \x90\x91\x01 \x92\x91PPV[`\x01\x82\x01T\x80\x15a7TW\x93\x92PPPV[\x82T_\x90\x81\x90\x80\x15a8\xDBW_a7\xDB\x87a\x18\xD1`\x01\x85a?\x16V[`@\x80Q\x80\x82\x01\x90\x91R\x90Te\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x16\x80\x84R`\x01`0\x1B\x90\x92\x04`\x01`\x01`\xD0\x1B\x03\x16` \x84\x01R\x91\x92P\x90\x87\x16\x10\x15a8/W`@Qc% `\x1D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Qe\xFF\xFF\xFF\xFF\xFF\xFF\x80\x88\x16\x91\x16\x03a8{W\x84a8R\x88a\x18\xD1`\x01\x86a?\x16V[\x80T`\x01`\x01`\xD0\x1B\x03\x92\x90\x92\x16`\x01`0\x1B\x02e\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x16\x91\x90\x91\x17\x90Ua8\xCBV[`@\x80Q\x80\x82\x01\x90\x91Re\xFF\xFF\xFF\xFF\xFF\xFF\x80\x88\x16\x82R`\x01`\x01`\xD0\x1B\x03\x80\x88\x16` \x80\x85\x01\x91\x82R\x8BT`\x01\x81\x01\x8DU_\x8D\x81R\x91\x90\x91 \x94Q\x91Q\x90\x92\x16`\x01`0\x1B\x02\x92\x16\x91\x90\x91\x17\x91\x01U[` \x01Q\x92P\x83\x91Pa-\xC6\x90PV[PP`@\x80Q\x80\x82\x01\x90\x91Re\xFF\xFF\xFF\xFF\xFF\xFF\x80\x85\x16\x82R`\x01`\x01`\xD0\x1B\x03\x80\x85\x16` \x80\x85\x01\x91\x82R\x88T`\x01\x81\x01\x8AU_\x8A\x81R\x91\x82 \x95Q\x92Q\x90\x93\x16`\x01`0\x1B\x02\x91\x90\x93\x16\x17\x92\x01\x91\x90\x91U\x90P\x81a-\xC6V[\x80Q\x15a9EW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a9tW_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a9\x8AW_\x80\xFD[a9\x93\x83a9^V[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a9\xDEWa9\xDEa9\xA1V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a9\xF6W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a:\x19Wa:\x19a9\xA1V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a:ZW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a:}Wa:}a9\xA1V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a:\xABW_\x80\xFD[a:\xB5\x86\x86a9\xE6V[\x93Pa:\xC4\x86`\x80\x87\x01a:JV[\x92Pa:\xD3\x86`\xC0\x87\x01a:JV[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a:\xEAW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a;\x05W_\x80\xFD[a\x13\x97\x82a9^V[_[\x83\x81\x10\x15a;(W\x81\x81\x01Q\x83\x82\x01R` \x01a;\x10V[PP_\x91\x01RV[_\x81Q\x80\x84Ra;G\x81` \x86\x01` \x86\x01a;\x0EV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R_a\x13\x97` \x83\x01\x84a;0V[_\x80`@\x83\x85\x03\x12\x15a;~W_\x80\xFD[a;\x87\x83a9^V[\x91P` \x80\x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a;\xA4W_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a;\xB7W_\x80\xFD[\x815\x81\x81\x11\x15a;\xC9Wa;\xC9a9\xA1V[a;\xDB`\x1F\x82\x01`\x1F\x19\x16\x85\x01a9\xB5V[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a;\xF0W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[_\x80_a\x01\0\x84\x86\x03\x12\x15a< W_\x80\xFD[a<*\x85\x85a9\xE6V[\x92Pa<9\x85`\x80\x86\x01a:JV[\x91Pa<H\x85`\xC0\x86\x01a:JV[\x90P\x92P\x92P\x92V[`\xFF`\xF8\x1B\x88\x16\x81R_` `\xE0` \x84\x01Ra<q`\xE0\x84\x01\x8Aa;0V[\x83\x81\x03`@\x85\x01Ra<\x83\x81\x8Aa;0V[``\x85\x01\x89\x90R`\x01`\x01`\xA0\x1B\x03\x88\x16`\x80\x86\x01R`\xA0\x85\x01\x87\x90R\x84\x81\x03`\xC0\x86\x01R\x85Q\x80\x82R` \x80\x88\x01\x93P\x90\x91\x01\x90_[\x81\x81\x10\x15a<\xD6W\x83Q\x83R\x92\x84\x01\x92\x91\x84\x01\x91`\x01\x01a<\xBAV[P\x90\x9C\x9BPPPPPPPPPPPPV[_` \x82\x84\x03\x12\x15a<\xF8W_\x80\xFD[P5\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a=\x0FW_\x80\xFD[a\x13\x97\x83\x83a9\xE6V[_\x80`@\x83\x85\x03\x12\x15a=*W_\x80\xFD[a=3\x83a9^V[\x91Pa=A` \x84\x01a9^V[\x90P\x92P\x92\x90PV[_\x80_\x80`\x80\x85\x87\x03\x12\x15a=]W_\x80\xFD[a=f\x85a9^V[\x93Pa=t` \x86\x01a9^V[\x92P`@\x85\x015\x91Pa=\x89``\x86\x01a9^V[\x90P\x92\x95\x91\x94P\x92PV[_\x80_\x80_\x80`\xC0\x87\x89\x03\x12\x15a=\xA9W_\x80\xFD[a=\xB2\x87a9^V[\x95P` \x87\x015\x94P`@\x87\x015\x93P``\x87\x015`\xFF\x81\x16\x81\x14a=\xD5W_\x80\xFD[\x95\x98\x94\x97P\x92\x95`\x80\x81\x015\x94`\xA0\x90\x91\x015\x93P\x91PPV[_\x80`@\x83\x85\x03\x12\x15a>\0W_\x80\xFD[a>\t\x83a9^V[\x91P` \x83\x015c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a>!W_\x80\xFD[\x80\x91PP\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a>cWcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R`@\x80\x85\x01Q\x90\x83\x01R``\x80\x85\x01Q\x90\x83\x01R\x82Q`\x80\x83\x01R\x82\x01Q`\xA0\x82\x01R`\xC0\x81\x01a\x13\x97V[_` \x82\x84\x03\x12\x15a>\xBAW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0B\x87Wa\x0B\x87a>\xC1V[_\x83Qa>\xF9\x81\x84` \x88\x01a;\x0EV[\x83Q\x90\x83\x01\x90a?\r\x81\x83` \x88\x01a;\x0EV[\x01\x94\x93PPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0B\x87Wa\x0B\x87a>\xC1V[`\x01\x81\x81\x1C\x90\x82\x16\x80a?=W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a?[WcNH{q`\xE0\x1B_R`\"`\x04R`$_\xFD[P\x91\x90PV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a?\x83Wa?\x83a?aV[P\x06\x90V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[`\x01`\x01`\xD0\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x80\x82\x11\x15a6\xB9Wa6\xB9a>\xC1V[`\x01`\x01`\xD0\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15a6\xB9Wa6\xB9a>\xC1V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[_\x82a?\xFEWa?\xFEa?aV[P\x04\x90V[_\x82Qa@\x14\x81\x84` \x87\x01a;\x0EV[\x91\x90\x91\x01\x92\x91PPV[_\x83Qa@/\x81\x84` \x88\x01a;\x0EV[_\x92\x01\x91\x82RP`\x01`\x01`\xF8\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x02\x01\x91\x90PV[_\x82Qa@a\x81\x84` \x87\x01a;\x0EV[_\x92\x01\x91\x82RP`\x01\x01\x91\x90PV[_\x83Qa@\x81\x81\x84` \x88\x01a;\x0EV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x02\x01\x92\x91PPV[_\x83Qa@\xAF\x81\x84` \x88\x01a;\x0EV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x01\x01\x92\x91PPV[_\x84Qa@\xDD\x81\x84` \x89\x01a;\x0EV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x91\x90\x93\x01\x90\x81R`\x01`\x01`\xF0\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x03\x01\x92\x91PPV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0B\x87Wa\x0B\x87a>\xC1V[`\x01\x81\x81[\x80\x85\x11\x15aA]W\x81_\x19\x04\x82\x11\x15aACWaACa>\xC1V[\x80\x85\x16\x15aAPW\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90aA(V[P\x92P\x92\x90PV[_\x82aAsWP`\x01a\x0B\x87V[\x81aA\x7FWP_a\x0B\x87V[\x81`\x01\x81\x14aA\x95W`\x02\x81\x14aA\x9FWaA\xBBV[`\x01\x91PPa\x0B\x87V[`\xFF\x84\x11\x15aA\xB0WaA\xB0a>\xC1V[PP`\x01\x82\x1Ba\x0B\x87V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15aA\xDEWP\x81\x81\na\x0B\x87V[aA\xE8\x83\x83aA#V[\x80_\x19\x04\x82\x11\x15aA\xFBWaA\xFBa>\xC1V[\x02\x93\x92PPPV[_a\x13\x97\x83\x83aAeV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x000dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x006\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405260043610610228575f3560e01c80638e539e8c11610129578063b5ecb344116100a8578063da8be8641161006d578063da8be86414610743578063f1127ed814610762578063f2fde38b146107ad578063fa52c7d8146107cc578063fc0c546a1461080f575f80fd5b8063b5ecb34414610670578063be2030941461069b578063c3cda520146106ba578063c64814dd146106d9578063d2b750e51461070f575f80fd5b8063a2d78dd5116100ee578063a2d78dd514610572578063a3066aab146105c4578063ad3cb1cc146105e3578063b3e6ebd514610613578063b5700e6814610651575f80fd5b80638e539e8c146104d557806391ddadf4146104f45780639ab24eb01461051f5780639b30a5e61461053e5780639e9a8f311461055d575f80fd5b806352d1902d116101b55780636fcfff451161017a5780636fcfff45146103ec578063715018a61461040b5780637ecebe001461041f57806384b0196e146104725780638da5cb5b14610499575f80fd5b806352d1902d1461034f5780635544c2f114610363578063587cde1e146103825780635c19a95c146103b95780636a911ccf146103d8575f80fd5b80633a46b1a8116101fb5780633a46b1a8146102bb5780633e9df9b5146102e85780634bf5d7e9146102fc5780634d99dd161461031d5780634f1ef2861461033c575f80fd5b8063026e402b1461022c5780630d8e6e2c1461024d57806313b9057a1461027d5780632140fecd1461029c575b5f80fd5b348015610237575f80fd5b5061024b610246366004613979565b61082e565b005b348015610258575f80fd5b5060408051600181525f60208201819052918101919091526060015b60405180910390f35b348015610288575f80fd5b5061024b610297366004613a97565b61083c565b3480156102a7575f80fd5b5061024b6102b6366004613af5565b6109d5565b3480156102c6575f80fd5b506102da6102d5366004613979565b610af6565b604051908152602001610274565b3480156102f3575f80fd5b506102da5f5481565b348015610307575f80fd5b50610310610b8d565b6040516102749190613b5b565b348015610328575f80fd5b5061024b610337366004613979565b610c05565b61024b61034a366004613b6d565b610c0f565b34801561035a575f80fd5b506102da610c2a565b34801561036e575f80fd5b5061024b61037d366004613c0d565b610c45565b34801561038d575f80fd5b506103a161039c366004613af5565b610d0e565b6040516001600160a01b039091168152602001610274565b3480156103c4575f80fd5b5061024b6103d3366004613af5565b610d37565b3480156103e3575f80fd5b5061024b610dbd565b3480156103f7575f80fd5b506102da610406366004613af5565b610e3f565b348015610416575f80fd5b5061024b610e55565b34801561042a575f80fd5b506102da610439366004613af5565b6001600160a01b03165f9081527f5ab42ced628888259c08ac98db1eb0cf702fc1501344311d8b100cd1bfe4bb00602052604090205490565b34801561047d575f80fd5b50610486610e68565b6040516102749796959493929190613c51565b3480156104a4575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03166103a1565b3480156104e0575f80fd5b506102da6104ef366004613ce8565b610f11565b3480156104ff575f80fd5b50610508610f8b565b60405165ffffffffffff9091168152602001610274565b34801561052a575f80fd5b506102da610539366004613af5565b610f99565b348015610549575f80fd5b506102da610558366004613cff565b610ff7565b348015610568575f80fd5b506102da60095481565b34801561057d575f80fd5b506105af61058c366004613d19565b600760209081525f92835260408084209091529082529020805460019091015482565b60408051928352602083019190915201610274565b3480156105cf575f80fd5b5061024b6105de366004613af5565b611051565b3480156105ee575f80fd5b50610310604051806040016040528060058152602001640352e302e360dc1b81525081565b34801561061e575f80fd5b5061064161062d366004613ce8565b60046020525f908152604090205460ff1681565b6040519015158152602001610274565b34801561065c575f80fd5b506001546103a1906001600160a01b031681565b34801561067b575f80fd5b506102da61068a366004613af5565b60056020525f908152604090205481565b3480156106a6575f80fd5b5061024b6106b5366004613d4a565b611161565b3480156106c5575f80fd5b5061024b6106d4366004613d94565b61128d565b3480156106e4575f80fd5b506102da6106f3366004613d19565b600660209081525f928352604080842090915290825290205481565b34801561071a575f80fd5b506103a1610729366004613af5565b60086020525f90815260409020546001600160a01b031681565b34801561074e575f80fd5b5061024b61075d366004613af5565b611349565b34801561076d575f80fd5b5061078161077c366004613def565b61137a565b60408051825165ffffffffffff1681526020928301516001600160d01b03169281019290925201610274565b3480156107b8575f80fd5b5061024b6107c7366004613af5565b61139e565b3480156107d7575f80fd5b506108016107e6366004613af5565b60036020525f90815260409020805460019091015460ff1682565b604051610274929190613e40565b34801561081a575f80fd5b506002546103a1906001600160a01b031681565b61083882826113d8565b5050565b3361084681611639565b61084f84611686565b610858856116c1565b604080516001600160a01b03831660208201525f9101604051602081830303815290604052905061088a8185886116fd565b6127108361ffff1611156108b15760405163dc81db8560e01b815260040160405180910390fd5b600160045f6108bf89610ff7565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f81526020016001600281111561090657610906613e2c565b90526001600160a01b0383165f908152600360209081526040909120825181559082015160018083018054909160ff199091169083600281111561094c5761094c613e2c565b02179055505060408051885181526020808a015181830152828a0151928201929092526060808a015190820152875160808201529087015160a082015261ffff851660c08201526001600160a01b03841691507ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9060e0015b60405180910390a2505050505050565b6001600160a01b0381165f908152600560205260408120543391819003610a0f576040516379298a5360e11b815260040160405180910390fd5b80421015610a3057604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832093861683529290529081205490819003610a7857604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038085165f9081526006602090815260408083208785168452909152812055600254610aad91168483611792565b826001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b6582604051610ae891815260200190565b60405180910390a250505050565b5f5f8051602061423383398151915281610b0e610f8b565b90508065ffffffffffff168410610b4e57604051637669fc0f60e11b81526004810185905265ffffffffffff821660248201526044015b60405180910390fd5b610b79610b5a8561181b565b6001600160a01b0387165f908152600185016020526040902090611851565b6001600160d01b0316925050505b92915050565b6060610b97611901565b65ffffffffffff16610ba7610f8b565b65ffffffffffff1614610bcd576040516301bfc1c560e61b815260040160405180910390fd5b5060408051808201909152601d81527f6d6f64653d626c6f636b6e756d6265722666726f6d3d64656661756c74000000602082015290565b610838828261190b565b610c17611af5565b610c2082611b99565b6108388282611be0565b5f610c33611ca1565b505f8051602061429383398151915290565b33610c4f81611cea565b610c5883611686565b610c61846116c1565b604080516001600160a01b03831660208201525f91016040516020818303038152906040529050610c938184876116fd565b600160045f610ca188610ff7565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550816001600160a01b03167f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d8686604051610cff929190613e70565b60405180910390a25050505050565b6001600160a01b039081165f9081525f8051602061423383398151915260205260409020541690565b6002545f906001600160a01b03166370a08231336040516001600160e01b031960e084901b1681526001600160a01b039091166004820152602401602060405180830381865afa158015610d8d573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610db19190613eaa565b905061083882826113d8565b33610dc781611cea565b6001600160a01b0381165f908152600360205260409020600101805460ff19166002179055600954610df99042613ed5565b6001600160a01b0382165f8181526005602052604080822093909355915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a250565b5f610e4982611d39565b63ffffffff1692915050565b610e5d611d88565b610e665f611de3565b565b5f60608082808083815f805160206142738339815191528054909150158015610e9357506001810154155b610ed75760405162461bcd60e51b81526020600482015260156024820152741152540dcc4c8e88155b9a5b9a5d1a585b1a5e9959605a1b6044820152606401610b45565b610edf611e53565b610ee7611f13565b604080515f80825260208201909252600f60f81b9c939b5091995046985030975095509350915050565b5f5f8051602061423383398151915281610f29610f8b565b90508065ffffffffffff168410610f6457604051637669fc0f60e11b81526004810185905265ffffffffffff82166024820152604401610b45565b610f7a610f708561181b565b6002840190611851565b6001600160d01b0316949350505050565b5f610f94611901565b905090565b6001600160a01b0381165f9081527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d01602052604081205f8051602061423383398151915290610fe790611f51565b6001600160d01b03169392505050565b5f815f0151826020015183604001518460600151604051602001611034949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f908152600760209081526040808320338085529252822054909181900361109657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320938616835292905220600101544210156110de57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038084165f908152600760209081526040808320868516845290915281208181556001015560025461111991168383611792565b816001600160a01b03167f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b658260405161115491815260200190565b60405180910390a2505050565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156111a65750825b90505f8267ffffffffffffffff1660011480156111c25750303b155b9050811580156111d0575080155b156111ee5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561121857845460ff60401b1916600160401b1785555b61122186611f88565b611229611f99565b611231611fa1565b61123c8989896120a8565b831561128257845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b834211156112b157604051632341d78760e11b815260048101859052602401610b45565b604080517fe48329057bfd03d55e49b547132e39cffd9c1820ad7b9d4c5307691425d15adf60208201526001600160a01b0388169181019190915260608101869052608081018590525f9061132a906113229060a0016040516020818303038152906040528051906020012061212b565b858585612157565b90506113368187612183565b61134081886121f4565b50505050505050565b6001600160a01b0381165f90815260066020908152604080832033845290915290205461137790829061190b565b50565b604080518082019091525f808252602082015261139783836122a0565b9392505050565b6113a6611d88565b6001600160a01b0381166113cf57604051631e4fbdf760e01b81525f6004820152602401610b45565b61137781611de3565b6113e182611cea565b335f81815260036020526040902060019081015460ff16600281111561140957611409613e2c565b0361142757604051634434d19360e11b815260040160405180910390fd5b6001600160a01b038181165f90815260086020526040902054161580159061146b57506001600160a01b038181165f90815260086020526040902054811690841614155b1561148957604051630c595ddf60e01b815260040160405180910390fd5b600254604051636eb1769f60e11b81526001600160a01b0383811660048301523060248301525f92169063dd62ed3e90604401602060405180830381865afa1580156114d7573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906114fb9190613eaa565b9050828110156115285760405163054365bb60e31b81526004810182905260248101849052604401610b45565b6115315f612309565b6001600160a01b0384165f9081526003602052604081208054859290611558908490613ed5565b90915550506001600160a01b038085165f90815260066020908152604080832093861683529290529081208054859290611593908490613ed5565b90915550506001600160a01b038281165f90815260086020526040902080546001600160a01b0319168683161790556002546115d29116833086612314565b836001600160a01b0316826001600160a01b03167f5dac0c1b1112564a045ba943c9d50270893e8e826c49be8e7073adc713ab7bd78560405161161791815260200190565b60405180910390a361162884612309565b6116335f85856123b1565b50505050565b6001600160a01b0381165f9081526003602052604081206001015460ff16600281111561166857611668613e2c565b146113775760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f80825260208201526116a38282612427565b15610838576040516306cf438f60e01b815260040160405180910390fd5b60045f6116cd83610ff7565b815260208101919091526040015f205460ff16156113775760405162da8a5760e11b815260040160405180910390fd5b61170682612443565b5f60405180606001604052806024815260200161420f6024913990505f8482604051602001611736929190613ee8565b60405160208183030381529060405290505f611751826124d9565b905061176e8185611761886125c6565b61176961263d565b61270a565b61178a5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f5114161516156117dc5750823b153d17155b806116335760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610b45565b5f65ffffffffffff82111561184d576040516306dfcc6560e41b81526030600482015260248101839052604401610b45565b5090565b81545f90818160058111156118ad575f61186a846127e8565b6118749085613f16565b5f8881526020902090915081015465ffffffffffff908116908716101561189d578091506118ab565b6118a8816001613ed5565b92505b505b5f6118ba878785856128cc565b905080156118f4576118de876118d1600184613f16565b5f91825260209091200190565b54600160301b90046001600160d01b03166118f6565b5f5b979650505050505050565b5f610f944361181b565b61191482611cea565b335f8181526003602052604090206001015460029060ff168181111561193c5761193c613e2c565b0361195a5760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832093851683529290522054828110156119a357604051639266535160e01b815260048101829052602401610b45565b6119ac5f612309565b6001600160a01b038085165f908152600660209081526040808320938616835292905290812080548592906119e2908490613f16565b92505081905550604051806040016040528084815260200160095442611a089190613ed5565b90526001600160a01b038086165f81815260076020908152604080832094881683529381528382208551815594810151600190950194909455908152600390925281208054859290611a5b908490613f16565b92505081905550836001600160a01b0316826001600160a01b03167f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c85604051611aa791815260200190565b60405180910390a3808314611ac457611abf84612309565b611aea565b6001600160a01b0382165f90815260086020526040902080546001600160a01b03191690555b611633845f856123b1565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611b7b57507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611b6f5f80516020614293833981519152546001600160a01b031690565b6001600160a01b031614155b15610e665760405163703e46dd60e11b815260040160405180910390fd5b611ba1611d88565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d9060200160405180910390a150565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611c3a575060408051601f3d908101601f19168201909252611c3791810190613eaa565b60015b611c6257604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610b45565b5f805160206142938339815191528114611c9257604051632a87526960e21b815260048101829052602401610b45565b611c9c838361292b565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610e665760405163703e46dd60e11b815260040160405180910390fd5b60016001600160a01b0382165f9081526003602052604090206001015460ff166002811115611d1b57611d1b613e2c565b146113775760405163508a793f60e01b815260040160405180910390fd5b6001600160a01b0381165f9081527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d0160205260408120545f805160206142338339815191529061139790612980565b33611dba7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610e665760405163118cdaa760e01b8152336004820152602401610b45565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b7fa16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d10280546060915f8051602061427383398151915291611e9190613f29565b80601f0160208091040260200160405190810160405280929190818152602001828054611ebd90613f29565b8015611f085780601f10611edf57610100808354040283529160200191611f08565b820191905f5260205f20905b815481529060010190602001808311611eeb57829003601f168201915b505050505091505090565b7fa16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d10380546060915f8051602061427383398151915291611e9190613f29565b80545f908015611f8057611f6a836118d1600184613f16565b54600160301b90046001600160d01b0316611397565b5f9392505050565b611f906129b0565b611377816129f9565b610e666129b0565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f81158015611fe65750825b90505f8267ffffffffffffffff1660011480156120025750303b155b905081158015612010575080155b1561202e5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561205857845460ff60401b1916600160401b1785555b435f5583156120a157845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b5050505050565b6001600160a01b0383166120cf5760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b0382166120f65760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b039485166001600160a01b0319918216179091556001805493909416921691909117909155600955565b5f610b87612137612a01565b8360405161190160f01b8152600281019290925260228201526042902090565b5f805f8061216788888888612a0a565b9250925092506121778282612ad2565b50909695505050505050565b6001600160a01b0382165f9081527f5ab42ced628888259c08ac98db1eb0cf702fc1501344311d8b100cd1bfe4bb0060205260409020805460018101909155818114611c9c576040516301d4b62360e61b81526001600160a01b038416600482015260248101829052604401610b45565b5f805160206142338339815191525f61220c84610d0e565b6001600160a01b038581165f8181526020869052604080822080546001600160a01b031916898616908117909155905194955093928516927f3134e8a2e6d97e929a7e54011ea5485d7d196dd5f0ba4d4ef95803e8e3fc257f9190a4611633818461229b876001600160a01b039081165f90815260086020908152604080832054909316825260039052205490565b612b8a565b6040805180820182525f80825260208083018290526001600160a01b03861682527fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d019052919091205f80516020614233833981519152906123019084612ceb565b949350505050565b3361083881836121f4565b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f51141615161561236d5750833b153d17155b806120a15760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610b45565b5f805160206142338339815191526001600160a01b0384166123e8576123e581600201612d586123e085612d63565b612d96565b50505b6001600160a01b03831661240c5761240981600201612dce6123e085612d63565b50505b61163361241885610d0e565b61242185610d0e565b84612b8a565b805182515f911480156113975750506020908101519101511490565b805160208201515f915f8051602061425383398151915291159015161561246957505050565b825160208401518260038485858609850908838283091483821084841016169350505081611c9c5760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610b45565b604080518082019091525f80825260208201525f6124f683612dd9565b90505f8051602061425383398151915260035f828485099050828061251d5761251d613f61565b8482099050828061253057612530613f61565b82820890505f8061254083612fe2565b925090505b806125a957848061255857612558613f61565b600187089550848061256c5761256c613f61565b8687099250848061257f5761257f613f61565b8684099250848061259257612592613f61565b84840892506125a083612fe2565b92509050612545565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f80825260208201528151602083015115901516156125ed575090565b6040518060400160405280835f015181526020015f80516020614253833981519152846020015161261e9190613f75565b612635905f80516020614253833981519152613f16565b905292915050565b61266460405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806127dc5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610b45565b50151595945050505050565b5f815f036127f757505f919050565b5f6001612803846130d9565b901c6001901b9050600181848161281c5761281c613f61565b048201901c9050600181848161283457612834613f61565b048201901c9050600181848161284c5761284c613f61565b048201901c9050600181848161286457612864613f61565b048201901c9050600181848161287c5761287c613f61565b048201901c9050600181848161289457612894613f61565b048201901c905060018184816128ac576128ac613f61565b048201901c9050611397818285816128c6576128c6613f61565b0461316c565b5f5b81831015612923575f6128e18484613181565b5f8781526020902090915065ffffffffffff86169082015465ffffffffffff16111561290f5780925061291d565b61291a816001613ed5565b93505b506128ce565b509392505050565b6129348261319b565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561297857611c9c82826131fe565b610838613270565b5f63ffffffff82111561184d576040516306dfcc6560e41b81526020600482015260248101839052604401610b45565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff16610e6657604051631afcd79f60e31b815260040160405180910390fd5b6113a66129b0565b5f610f9461328f565b5f80807f7fffffffffffffffffffffffffffffff5d576e7357a4501ddfe92f46681b20a0841115612a4357505f91506003905082612ac8565b604080515f808252602082018084528a905260ff891692820192909252606081018790526080810186905260019060a0016020604051602081039080840390855afa158015612a94573d5f803e3d5ffd5b5050604051601f1901519150506001600160a01b038116612abf57505f925060019150829050612ac8565b92505f91508190505b9450945094915050565b5f826003811115612ae557612ae5613e2c565b03612aee575050565b6001826003811115612b0257612b02613e2c565b03612b205760405163f645eedf60e01b815260040160405180910390fd5b6002826003811115612b3457612b34613e2c565b03612b555760405163fce698f760e01b815260048101829052602401610b45565b6003826003811115612b6957612b69613e2c565b03610838576040516335e2f38360e21b815260048101829052602401610b45565b5f805160206142338339815191526001600160a01b0384811690841614801590612bb357505f82115b15611633576001600160a01b03841615612c57576001600160a01b0384165f90815260018201602052604081208190612bf290612dce6123e087612d63565b6001600160d01b031691506001600160d01b03169150856001600160a01b03167fdec2bacdd2f05b59de34da9b523dff8be42e5e38e818c82fdb0bae774387a7248383604051612c4c929190918252602082015260400190565b60405180910390a250505b6001600160a01b03831615611633576001600160a01b0383165f90815260018201602052604081208190612c9190612d586123e087612d63565b6001600160d01b031691506001600160d01b03169150846001600160a01b03167fdec2bacdd2f05b59de34da9b523dff8be42e5e38e818c82fdb0bae774387a72483836040516109c5929190918252602082015260400190565b604080518082019091525f8082526020820152825f018263ffffffff1681548110612d1857612d18613f88565b5f9182526020918290206040805180820190915291015465ffffffffffff81168252600160301b90046001600160d01b0316918101919091529392505050565b5f6113978284613f9c565b5f6001600160d01b0382111561184d576040516306dfcc6560e41b815260d0600482015260248101839052604401610b45565b5f80612dc1612da3610f8b565b612db9612daf88611f51565b868863ffffffff16565b879190613302565b915091505b935093915050565b5f6113978284613fbc565b5f80612de48361330f565b805190915060308114612df957612df9613fdc565b5f8167ffffffffffffffff811115612e1357612e136139a1565b6040519080825280601f01601f191660200182016040528015612e3d576020820181803683370190505b5090505f5b82811015612eac57836001612e578386613f16565b612e619190613f16565b81518110612e7157612e71613f88565b602001015160f81c60f81b828281518110612e8e57612e8e613f88565b60200101906001600160f81b03191690815f1a905350600101612e42565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b82811015612f3c578381612ee88588613f16565b612ef29190613ed5565b81518110612f0257612f02613f88565b602001015160f81c60f81b60f81c828281518110612f2257612f22613f88565b60ff90921660209283029190910190910152600101612ed4565b505f612f4782613659565b90506101005f805160206142538339815191525f612f658689613f16565b90505f5b81811015612fd2575f886001612f7f8486613f16565b612f899190613f16565b81518110612f9957612f99613f88565b016020015160f81c90508380612fb157612fb1613f61565b85870995508380612fc457612fc4613f61565b818708955050600101612f69565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f80516020614253833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f5192508361309f5760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610b45565b80600184901b11156130b8576130b58382613f16565b92505b80806130c6576130c6613f61565b8384099690961496919550909350505050565b5f80608083901c156130ed57608092831c92015b604083901c156130ff57604092831c92015b602083901c1561311157602092831c92015b601083901c1561312357601092831c92015b600883901c1561313557600892831c92015b600483901c1561314757600492831c92015b600283901c1561315957600292831c92015b600183901c15610b875760010192915050565b5f81831061317a5781611397565b5090919050565b5f61318f6002848418613ff0565b61139790848416613ed5565b806001600160a01b03163b5f036131d057604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610b45565b5f8051602061429383398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b03168460405161321a9190614003565b5f60405180830381855af49150503d805f8114613252576040519150601f19603f3d011682016040523d82523d5f602084013e613257565b606091505b50915091506132678583836136c0565b95945050505050565b3415610e665760405163b398979f60e01b815260040160405180910390fd5b5f7f8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f6132b9613715565b6132c161377d565b60408051602081019490945283019190915260608201524660808201523060a082015260c00160405160208183030381529060405280519060200120905090565b5f80612dc18585856137bf565b604080516030808252606082810190935290602090600160f91b905f9084602082018180368337019050509050808660405160200161334f929190613ee8565b6040516020818303038152906040529050808460f81b60405160200161337692919061401e565b6040516020818303038152906040529050806040516020016133989190614050565b60408051601f1981840301815290829052915061010160f01b906133c29083908390602001614070565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff811115613432576134326139a1565b6040519080825280601f01601f19166020018201604052801561345c576020820181803683370190505b5090505f8260405160200161347391815260200190565b60405160208183030381529060405290505f5b81518110156134dc578181815181106134a1576134a1613f88565b602001015160f81c60f81b8382815181106134be576134be613f88565b60200101906001600160f81b03191690815f1a905350600101613486565b505f846040516020016134f191815260200190565b60408051601f19818403018152602083019091525f80835291985091505b89811015613583575f83828151811061352a5761352a613f88565b602001015160f81c60f81b83838151811061354757613547613f88565b602001015160f81c60f81b189050888160405160200161356892919061409e565b60408051601f1981840301815291905298505060010161350f565b50868887604051602001613599939291906140cc565b604051602081830303815290604052965086805190602001209350836040516020016135c791815260200190565b60405160208183030381529060405291505f5b6135e78a60ff8d16613f16565b8110156136485782818151811061360057613600613f88565b01602001516001600160f81b0319168461361a838d613ed5565b8151811061362a5761362a613f88565b60200101906001600160f81b03191690815f1a9053506001016135da565b50919b9a5050505050505050505050565b5f80805b83518110156136b95783818151811061367857613678613f88565b602002602001015160ff16816008613690919061410c565b61369b906002614203565b6136a5919061410c565b6136af9083613ed5565b915060010161365d565b5092915050565b6060826136d5576136d082613935565b611397565b81511580156136ec57506001600160a01b0384163b155b156136b957604051639996b31560e01b81526001600160a01b0385166004820152602401610b45565b5f5f805160206142738339815191528161372d611e53565b80519091501561374557805160209091012092915050565b81548015613754579392505050565b7fc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470935050505090565b5f5f8051602061427383398151915281613795611f13565b8051909150156137ad57805160209091012092915050565b60018201548015613754579392505050565b82545f90819080156138db575f6137db876118d1600185613f16565b60408051808201909152905465ffffffffffff808216808452600160301b9092046001600160d01b03166020840152919250908716101561382f57604051632520601d60e01b815260040160405180910390fd5b805165ffffffffffff80881691160361387b5784613852886118d1600186613f16565b80546001600160d01b0392909216600160301b0265ffffffffffff9092169190911790556138cb565b6040805180820190915265ffffffffffff80881682526001600160d01b0380881660208085019182528b54600181018d555f8d81529190912094519151909216600160301b029216919091179101555b602001519250839150612dc69050565b50506040805180820190915265ffffffffffff80851682526001600160d01b0380851660208085019182528854600181018a555f8a815291822095519251909316600160301b029190931617920191909155905081612dc6565b8051156139455780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b0381168114613974575f80fd5b919050565b5f806040838503121561398a575f80fd5b6139938361395e565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f1916810167ffffffffffffffff811182821017156139de576139de6139a1565b604052919050565b5f608082840312156139f6575f80fd5b6040516080810181811067ffffffffffffffff82111715613a1957613a196139a1565b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f60408284031215613a5a575f80fd5b6040516040810181811067ffffffffffffffff82111715613a7d57613a7d6139a1565b604052823581526020928301359281019290925250919050565b5f805f806101208587031215613aab575f80fd5b613ab586866139e6565b9350613ac48660808701613a4a565b9250613ad38660c08701613a4a565b915061010085013561ffff81168114613aea575f80fd5b939692955090935050565b5f60208284031215613b05575f80fd5b6113978261395e565b5f5b83811015613b28578181015183820152602001613b10565b50505f910152565b5f8151808452613b47816020860160208601613b0e565b601f01601f19169290920160200192915050565b602081525f6113976020830184613b30565b5f8060408385031215613b7e575f80fd5b613b878361395e565b915060208084013567ffffffffffffffff80821115613ba4575f80fd5b818601915086601f830112613bb7575f80fd5b813581811115613bc957613bc96139a1565b613bdb601f8201601f191685016139b5565b91508082528784828501011115613bf0575f80fd5b80848401858401375f848284010152508093505050509250929050565b5f805f6101008486031215613c20575f80fd5b613c2a85856139e6565b9250613c398560808601613a4a565b9150613c488560c08601613a4a565b90509250925092565b60ff60f81b881681525f602060e06020840152613c7160e084018a613b30565b8381036040850152613c83818a613b30565b606085018990526001600160a01b038816608086015260a0850187905284810360c0860152855180825260208088019350909101905f5b81811015613cd657835183529284019291840191600101613cba565b50909c9b505050505050505050505050565b5f60208284031215613cf8575f80fd5b5035919050565b5f60808284031215613d0f575f80fd5b61139783836139e6565b5f8060408385031215613d2a575f80fd5b613d338361395e565b9150613d416020840161395e565b90509250929050565b5f805f8060808587031215613d5d575f80fd5b613d668561395e565b9350613d746020860161395e565b925060408501359150613d896060860161395e565b905092959194509250565b5f805f805f8060c08789031215613da9575f80fd5b613db28761395e565b95506020870135945060408701359350606087013560ff81168114613dd5575f80fd5b9598949750929560808101359460a0909101359350915050565b5f8060408385031215613e00575f80fd5b613e098361395e565b9150602083013563ffffffff81168114613e21575f80fd5b809150509250929050565b634e487b7160e01b5f52602160045260245ffd5b8281526040810160038310613e6357634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b825181526020808401518183015260408085015190830152606080850151908301528251608083015282015160a082015260c08101611397565b5f60208284031215613eba575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b80820180821115610b8757610b87613ec1565b5f8351613ef9818460208801613b0e565b835190830190613f0d818360208801613b0e565b01949350505050565b81810381811115610b8757610b87613ec1565b600181811c90821680613f3d57607f821691505b602082108103613f5b57634e487b7160e01b5f52602260045260245ffd5b50919050565b634e487b7160e01b5f52601260045260245ffd5b5f82613f8357613f83613f61565b500690565b634e487b7160e01b5f52603260045260245ffd5b6001600160d01b038181168382160190808211156136b9576136b9613ec1565b6001600160d01b038281168282160390808211156136b9576136b9613ec1565b634e487b7160e01b5f52600160045260245ffd5b5f82613ffe57613ffe613f61565b500490565b5f8251614014818460208701613b0e565b9190910192915050565b5f835161402f818460208801613b0e565b5f9201918252506001600160f81b0319919091166001820152600201919050565b5f8251614061818460208701613b0e565b5f920191825250600101919050565b5f8351614081818460208801613b0e565b6001600160f01b0319939093169190920190815260020192915050565b5f83516140af818460208801613b0e565b6001600160f81b0319939093169190920190815260010192915050565b5f84516140dd818460208901613b0e565b6001600160f81b031994909416919093019081526001600160f01b031991909116600182015260030192915050565b8082028115828204841417610b8757610b87613ec1565b600181815b8085111561415d57815f190482111561414357614143613ec1565b8085161561415057918102915b93841c9390800290614128565b509250929050565b5f8261417357506001610b87565b8161417f57505f610b87565b8160018114614195576002811461419f576141bb565b6001915050610b87565b60ff8411156141b0576141b0613ec1565b50506001821b610b87565b5060208310610133831016604e8410600b84101617156141de575081810a610b87565b6141e88383614123565b805f19048211156141fb576141fb613ec1565b029392505050565b5f611397838361416556fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5fe8b26c30fad74198956032a3533d903385d56dd795af560196f9c78d4af40d0030644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47a16a46d94261c7517cc8ff89f61c0ce93598e3c849801011dee649a6a557d100360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x02(W_5`\xE0\x1C\x80c\x8ES\x9E\x8C\x11a\x01)W\x80c\xB5\xEC\xB3D\x11a\0\xA8W\x80c\xDA\x8B\xE8d\x11a\0mW\x80c\xDA\x8B\xE8d\x14a\x07CW\x80c\xF1\x12~\xD8\x14a\x07bW\x80c\xF2\xFD\xE3\x8B\x14a\x07\xADW\x80c\xFAR\xC7\xD8\x14a\x07\xCCW\x80c\xFC\x0CTj\x14a\x08\x0FW_\x80\xFD[\x80c\xB5\xEC\xB3D\x14a\x06pW\x80c\xBE 0\x94\x14a\x06\x9BW\x80c\xC3\xCD\xA5 \x14a\x06\xBAW\x80c\xC6H\x14\xDD\x14a\x06\xD9W\x80c\xD2\xB7P\xE5\x14a\x07\x0FW_\x80\xFD[\x80c\xA2\xD7\x8D\xD5\x11a\0\xEEW\x80c\xA2\xD7\x8D\xD5\x14a\x05rW\x80c\xA3\x06j\xAB\x14a\x05\xC4W\x80c\xAD<\xB1\xCC\x14a\x05\xE3W\x80c\xB3\xE6\xEB\xD5\x14a\x06\x13W\x80c\xB5p\x0Eh\x14a\x06QW_\x80\xFD[\x80c\x8ES\x9E\x8C\x14a\x04\xD5W\x80c\x91\xDD\xAD\xF4\x14a\x04\xF4W\x80c\x9A\xB2N\xB0\x14a\x05\x1FW\x80c\x9B0\xA5\xE6\x14a\x05>W\x80c\x9E\x9A\x8F1\x14a\x05]W_\x80\xFD[\x80cR\xD1\x90-\x11a\x01\xB5W\x80co\xCF\xFFE\x11a\x01zW\x80co\xCF\xFFE\x14a\x03\xECW\x80cqP\x18\xA6\x14a\x04\x0BW\x80c~\xCE\xBE\0\x14a\x04\x1FW\x80c\x84\xB0\x19n\x14a\x04rW\x80c\x8D\xA5\xCB[\x14a\x04\x99W_\x80\xFD[\x80cR\xD1\x90-\x14a\x03OW\x80cUD\xC2\xF1\x14a\x03cW\x80cX|\xDE\x1E\x14a\x03\x82W\x80c\\\x19\xA9\\\x14a\x03\xB9W\x80cj\x91\x1C\xCF\x14a\x03\xD8W_\x80\xFD[\x80c:F\xB1\xA8\x11a\x01\xFBW\x80c:F\xB1\xA8\x14a\x02\xBBW\x80c>\x9D\xF9\xB5\x14a\x02\xE8W\x80cK\xF5\xD7\xE9\x14a\x02\xFCW\x80cM\x99\xDD\x16\x14a\x03\x1DW\x80cO\x1E\xF2\x86\x14a\x03<W_\x80\xFD[\x80c\x02n@+\x14a\x02,W\x80c\r\x8En,\x14a\x02MW\x80c\x13\xB9\x05z\x14a\x02}W\x80c!@\xFE\xCD\x14a\x02\x9CW[_\x80\xFD[4\x80\x15a\x027W_\x80\xFD[Pa\x02Ka\x02F6`\x04a9yV[a\x08.V[\0[4\x80\x15a\x02XW_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\x88W_\x80\xFD[Pa\x02Ka\x02\x976`\x04a:\x97V[a\x08<V[4\x80\x15a\x02\xA7W_\x80\xFD[Pa\x02Ka\x02\xB66`\x04a:\xF5V[a\t\xD5V[4\x80\x15a\x02\xC6W_\x80\xFD[Pa\x02\xDAa\x02\xD56`\x04a9yV[a\n\xF6V[`@Q\x90\x81R` \x01a\x02tV[4\x80\x15a\x02\xF3W_\x80\xFD[Pa\x02\xDA_T\x81V[4\x80\x15a\x03\x07W_\x80\xFD[Pa\x03\x10a\x0B\x8DV[`@Qa\x02t\x91\x90a;[V[4\x80\x15a\x03(W_\x80\xFD[Pa\x02Ka\x0376`\x04a9yV[a\x0C\x05V[a\x02Ka\x03J6`\x04a;mV[a\x0C\x0FV[4\x80\x15a\x03ZW_\x80\xFD[Pa\x02\xDAa\x0C*V[4\x80\x15a\x03nW_\x80\xFD[Pa\x02Ka\x03}6`\x04a<\rV[a\x0CEV[4\x80\x15a\x03\x8DW_\x80\xFD[Pa\x03\xA1a\x03\x9C6`\x04a:\xF5V[a\r\x0EV[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02tV[4\x80\x15a\x03\xC4W_\x80\xFD[Pa\x02Ka\x03\xD36`\x04a:\xF5V[a\r7V[4\x80\x15a\x03\xE3W_\x80\xFD[Pa\x02Ka\r\xBDV[4\x80\x15a\x03\xF7W_\x80\xFD[Pa\x02\xDAa\x04\x066`\x04a:\xF5V[a\x0E?V[4\x80\x15a\x04\x16W_\x80\xFD[Pa\x02Ka\x0EUV[4\x80\x15a\x04*W_\x80\xFD[Pa\x02\xDAa\x0496`\x04a:\xF5V[`\x01`\x01`\xA0\x1B\x03\x16_\x90\x81R\x7FZ\xB4,\xEDb\x88\x88%\x9C\x08\xAC\x98\xDB\x1E\xB0\xCFp/\xC1P\x13D1\x1D\x8B\x10\x0C\xD1\xBF\xE4\xBB\0` R`@\x90 T\x90V[4\x80\x15a\x04}W_\x80\xFD[Pa\x04\x86a\x0EhV[`@Qa\x02t\x97\x96\x95\x94\x93\x92\x91\x90a<QV[4\x80\x15a\x04\xA4W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x03\xA1V[4\x80\x15a\x04\xE0W_\x80\xFD[Pa\x02\xDAa\x04\xEF6`\x04a<\xE8V[a\x0F\x11V[4\x80\x15a\x04\xFFW_\x80\xFD[Pa\x05\x08a\x0F\x8BV[`@Qe\xFF\xFF\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02tV[4\x80\x15a\x05*W_\x80\xFD[Pa\x02\xDAa\x0596`\x04a:\xF5V[a\x0F\x99V[4\x80\x15a\x05IW_\x80\xFD[Pa\x02\xDAa\x05X6`\x04a<\xFFV[a\x0F\xF7V[4\x80\x15a\x05hW_\x80\xFD[Pa\x02\xDA`\tT\x81V[4\x80\x15a\x05}W_\x80\xFD[Pa\x05\xAFa\x05\x8C6`\x04a=\x19V[`\x07` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 \x80T`\x01\x90\x91\x01T\x82V[`@\x80Q\x92\x83R` \x83\x01\x91\x90\x91R\x01a\x02tV[4\x80\x15a\x05\xCFW_\x80\xFD[Pa\x02Ka\x05\xDE6`\x04a:\xF5V[a\x10QV[4\x80\x15a\x05\xEEW_\x80\xFD[Pa\x03\x10`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[4\x80\x15a\x06\x1EW_\x80\xFD[Pa\x06Aa\x06-6`\x04a<\xE8V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01a\x02tV[4\x80\x15a\x06\\W_\x80\xFD[P`\x01Ta\x03\xA1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x06{W_\x80\xFD[Pa\x02\xDAa\x06\x8A6`\x04a:\xF5V[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x06\xA6W_\x80\xFD[Pa\x02Ka\x06\xB56`\x04a=JV[a\x11aV[4\x80\x15a\x06\xC5W_\x80\xFD[Pa\x02Ka\x06\xD46`\x04a=\x94V[a\x12\x8DV[4\x80\x15a\x06\xE4W_\x80\xFD[Pa\x02\xDAa\x06\xF36`\x04a=\x19V[`\x06` \x90\x81R_\x92\x83R`@\x80\x84 \x90\x91R\x90\x82R\x90 T\x81V[4\x80\x15a\x07\x1AW_\x80\xFD[Pa\x03\xA1a\x07)6`\x04a:\xF5V[`\x08` R_\x90\x81R`@\x90 T`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x07NW_\x80\xFD[Pa\x02Ka\x07]6`\x04a:\xF5V[a\x13IV[4\x80\x15a\x07mW_\x80\xFD[Pa\x07\x81a\x07|6`\x04a=\xEFV[a\x13zV[`@\x80Q\x82Qe\xFF\xFF\xFF\xFF\xFF\xFF\x16\x81R` \x92\x83\x01Q`\x01`\x01`\xD0\x1B\x03\x16\x92\x81\x01\x92\x90\x92R\x01a\x02tV[4\x80\x15a\x07\xB8W_\x80\xFD[Pa\x02Ka\x07\xC76`\x04a:\xF5V[a\x13\x9EV[4\x80\x15a\x07\xD7W_\x80\xFD[Pa\x08\x01a\x07\xE66`\x04a:\xF5V[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x02t\x92\x91\x90a>@V[4\x80\x15a\x08\x1AW_\x80\xFD[P`\x02Ta\x03\xA1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x088\x82\x82a\x13\xD8V[PPV[3a\x08F\x81a\x169V[a\x08O\x84a\x16\x86V[a\x08X\x85a\x16\xC1V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x08\x8A\x81\x85\x88a\x16\xFDV[a'\x10\x83a\xFF\xFF\x16\x11\x15a\x08\xB1W`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x08\xBF\x89a\x0F\xF7V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\t\x06Wa\t\x06a>,V[\x90R`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\tLWa\tLa>,V[\x02\x17\x90UPP`@\x80Q\x88Q\x81R` \x80\x8A\x01Q\x81\x83\x01R\x82\x8A\x01Q\x92\x82\x01\x92\x90\x92R``\x80\x8A\x01Q\x90\x82\x01R\x87Q`\x80\x82\x01R\x90\x87\x01Q`\xA0\x82\x01Ra\xFF\xFF\x85\x16`\xC0\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16\x91P\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90`\xE0\x01[`@Q\x80\x91\x03\x90\xA2PPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T3\x91\x81\x90\x03a\n\x0FW`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\n0W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 T\x90\x81\x90\x03a\nxW`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x87\x85\x16\x84R\x90\x91R\x81 U`\x02Ta\n\xAD\x91\x16\x84\x83a\x17\x92V[\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\n\xE8\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPPV[__\x80Q` aB3\x839\x81Q\x91R\x81a\x0B\x0Ea\x0F\x8BV[\x90P\x80e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x84\x10a\x0BNW`@Qcvi\xFC\x0F`\xE1\x1B\x81R`\x04\x81\x01\x85\x90Re\xFF\xFF\xFF\xFF\xFF\xFF\x82\x16`$\x82\x01R`D\x01[`@Q\x80\x91\x03\x90\xFD[a\x0Bya\x0BZ\x85a\x18\x1BV[`\x01`\x01`\xA0\x1B\x03\x87\x16_\x90\x81R`\x01\x85\x01` R`@\x90 \x90a\x18QV[`\x01`\x01`\xD0\x1B\x03\x16\x92PPP[\x92\x91PPV[``a\x0B\x97a\x19\x01V[e\xFF\xFF\xFF\xFF\xFF\xFF\x16a\x0B\xA7a\x0F\x8BV[e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x14a\x0B\xCDW`@Qc\x01\xBF\xC1\xC5`\xE6\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[P`@\x80Q\x80\x82\x01\x90\x91R`\x1D\x81R\x7Fmode=blocknumber&from=default\0\0\0` \x82\x01R\x90V[a\x088\x82\x82a\x19\x0BV[a\x0C\x17a\x1A\xF5V[a\x0C \x82a\x1B\x99V[a\x088\x82\x82a\x1B\xE0V[_a\x0C3a\x1C\xA1V[P_\x80Q` aB\x93\x839\x81Q\x91R\x90V[3a\x0CO\x81a\x1C\xEAV[a\x0CX\x83a\x16\x86V[a\x0Ca\x84a\x16\xC1V[`@\x80Q`\x01`\x01`\xA0\x1B\x03\x83\x16` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x0C\x93\x81\x84\x87a\x16\xFDV[`\x01`\x04_a\x0C\xA1\x88a\x0F\xF7V[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D\x86\x86`@Qa\x0C\xFF\x92\x91\x90a>pV[`@Q\x80\x91\x03\x90\xA2PPPPPV[`\x01`\x01`\xA0\x1B\x03\x90\x81\x16_\x90\x81R_\x80Q` aB3\x839\x81Q\x91R` R`@\x90 T\x16\x90V[`\x02T_\x90`\x01`\x01`\xA0\x1B\x03\x16cp\xA0\x8213`@Q`\x01`\x01`\xE0\x1B\x03\x19`\xE0\x84\x90\x1B\x16\x81R`\x01`\x01`\xA0\x1B\x03\x90\x91\x16`\x04\x82\x01R`$\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\r\x8DW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\r\xB1\x91\x90a>\xAAV[\x90Pa\x088\x82\x82a\x13\xD8V[3a\r\xC7\x81a\x1C\xEAV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\tTa\r\xF9\x90Ba>\xD5V[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x81\x81R`\x05` R`@\x80\x82 \x93\x90\x93U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2PV[_a\x0EI\x82a\x1D9V[c\xFF\xFF\xFF\xFF\x16\x92\x91PPV[a\x0E]a\x1D\x88V[a\x0Ef_a\x1D\xE3V[V[_``\x80\x82\x80\x80\x83\x81_\x80Q` aBs\x839\x81Q\x91R\x80T\x90\x91P\x15\x80\x15a\x0E\x93WP`\x01\x81\x01T\x15[a\x0E\xD7W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x15`$\x82\x01Rt\x11RT\r\xCCL\x8E\x88\x15[\x9A[\x9A]\x1AX[\x1A^\x99Y`Z\x1B`D\x82\x01R`d\x01a\x0BEV[a\x0E\xDFa\x1ESV[a\x0E\xE7a\x1F\x13V[`@\x80Q_\x80\x82R` \x82\x01\x90\x92R`\x0F`\xF8\x1B\x9C\x93\x9BP\x91\x99PF\x98P0\x97P\x95P\x93P\x91PPV[__\x80Q` aB3\x839\x81Q\x91R\x81a\x0F)a\x0F\x8BV[\x90P\x80e\xFF\xFF\xFF\xFF\xFF\xFF\x16\x84\x10a\x0FdW`@Qcvi\xFC\x0F`\xE1\x1B\x81R`\x04\x81\x01\x85\x90Re\xFF\xFF\xFF\xFF\xFF\xFF\x82\x16`$\x82\x01R`D\x01a\x0BEV[a\x0Fza\x0Fp\x85a\x18\x1BV[`\x02\x84\x01\x90a\x18QV[`\x01`\x01`\xD0\x1B\x03\x16\x94\x93PPPPV[_a\x0F\x94a\x19\x01V[\x90P\x90V[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01` R`@\x81 _\x80Q` aB3\x839\x81Q\x91R\x90a\x0F\xE7\x90a\x1FQV[`\x01`\x01`\xD0\x1B\x03\x16\x93\x92PPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x104\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 T\x90\x91\x81\x90\x03a\x10\x96W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R `\x01\x01TB\x10\x15a\x10\xDEW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 \x86\x85\x16\x84R\x90\x91R\x81 \x81\x81U`\x01\x01U`\x02Ta\x11\x19\x91\x16\x83\x83a\x17\x92V[\x81`\x01`\x01`\xA0\x1B\x03\x16\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x82`@Qa\x11T\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA2PPPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x11\xA6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x11\xC2WP0;\x15[\x90P\x81\x15\x80\x15a\x11\xD0WP\x80\x15[\x15a\x11\xEEW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x12\x18W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x12!\x86a\x1F\x88V[a\x12)a\x1F\x99V[a\x121a\x1F\xA1V[a\x12<\x89\x89\x89a \xA8V[\x83\x15a\x12\x82W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[\x83B\x11\x15a\x12\xB1W`@Qc#A\xD7\x87`\xE1\x1B\x81R`\x04\x81\x01\x85\x90R`$\x01a\x0BEV[`@\x80Q\x7F\xE4\x83)\x05{\xFD\x03\xD5^I\xB5G\x13.9\xCF\xFD\x9C\x18 \xAD{\x9DLS\x07i\x14%\xD1Z\xDF` \x82\x01R`\x01`\x01`\xA0\x1B\x03\x88\x16\x91\x81\x01\x91\x90\x91R``\x81\x01\x86\x90R`\x80\x81\x01\x85\x90R_\x90a\x13*\x90a\x13\"\x90`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 a!+V[\x85\x85\x85a!WV[\x90Pa\x136\x81\x87a!\x83V[a\x13@\x81\x88a!\xF4V[PPPPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 Ta\x13w\x90\x82\x90a\x19\x0BV[PV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x13\x97\x83\x83a\"\xA0V[\x93\x92PPPV[a\x13\xA6a\x1D\x88V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x13\xCFW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x0BEV[a\x13w\x81a\x1D\xE3V[a\x13\xE1\x82a\x1C\xEAV[3_\x81\x81R`\x03` R`@\x90 `\x01\x90\x81\x01T`\xFF\x16`\x02\x81\x11\x15a\x14\tWa\x14\ta>,V[\x03a\x14'W`@QcD4\xD1\x93`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x81\x16_\x90\x81R`\x08` R`@\x90 T\x16\x15\x80\x15\x90a\x14kWP`\x01`\x01`\xA0\x1B\x03\x81\x81\x16_\x90\x81R`\x08` R`@\x90 T\x81\x16\x90\x84\x16\x14\x15[\x15a\x14\x89W`@Qc\x0CY]\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x04\x83\x01R0`$\x83\x01R_\x92\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x14\xD7W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x14\xFB\x91\x90a>\xAAV[\x90P\x82\x81\x10\x15a\x15(W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x84\x90R`D\x01a\x0BEV[a\x151_a#\tV[`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x03` R`@\x81 \x80T\x85\x92\x90a\x15X\x90\x84\x90a>\xD5V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x15\x93\x90\x84\x90a>\xD5V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x82\x81\x16_\x90\x81R`\x08` R`@\x90 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x86\x83\x16\x17\x90U`\x02Ta\x15\xD2\x91\x16\x830\x86a#\x14V[\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7F]\xAC\x0C\x1B\x11\x12VJ\x04[\xA9C\xC9\xD5\x02p\x89>\x8E\x82lI\xBE\x8Eps\xAD\xC7\x13\xAB{\xD7\x85`@Qa\x16\x17\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3a\x16(\x84a#\tV[a\x163_\x85\x85a#\xB1V[PPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x16hWa\x16ha>,V[\x14a\x13wW`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16\xA3\x82\x82a$'V[\x15a\x088W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x16\xCD\x83a\x0F\xF7V[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x13wW`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x17\x06\x82a$CV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01aB\x0F`$\x919\x90P_\x84\x82`@Q` \x01a\x176\x92\x91\x90a>\xE8V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x17Q\x82a$\xD9V[\x90Pa\x17n\x81\x85a\x17a\x88a%\xC6V[a\x17ia&=V[a'\nV[a\x17\x8AW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x17\xDCWP\x82;\x15=\x17\x15[\x80a\x163W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x0BEV[_e\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R`0`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[P\x90V[\x81T_\x90\x81\x81`\x05\x81\x11\x15a\x18\xADW_a\x18j\x84a'\xE8V[a\x18t\x90\x85a?\x16V[_\x88\x81R` \x90 \x90\x91P\x81\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x90\x81\x16\x90\x87\x16\x10\x15a\x18\x9DW\x80\x91Pa\x18\xABV[a\x18\xA8\x81`\x01a>\xD5V[\x92P[P[_a\x18\xBA\x87\x87\x85\x85a(\xCCV[\x90P\x80\x15a\x18\xF4Wa\x18\xDE\x87a\x18\xD1`\x01\x84a?\x16V[_\x91\x82R` \x90\x91 \x01\x90V[T`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16a\x18\xF6V[_[\x97\x96PPPPPPPV[_a\x0F\x94Ca\x18\x1BV[a\x19\x14\x82a\x1C\xEAV[3_\x81\x81R`\x03` R`@\x90 `\x01\x01T`\x02\x90`\xFF\x16\x81\x81\x11\x15a\x19<Wa\x19<a>,V[\x03a\x19ZW`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x85\x16\x83R\x92\x90R T\x82\x81\x10\x15a\x19\xA3W`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[a\x19\xAC_a#\tV[`\x01`\x01`\xA0\x1B\x03\x80\x85\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 \x93\x86\x16\x83R\x92\x90R\x90\x81 \x80T\x85\x92\x90a\x19\xE2\x90\x84\x90a?\x16V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x84\x81R` \x01`\tTBa\x1A\x08\x91\x90a>\xD5V[\x90R`\x01`\x01`\xA0\x1B\x03\x80\x86\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 \x94\x88\x16\x83R\x93\x81R\x83\x82 \x85Q\x81U\x94\x81\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x90\x81R`\x03\x90\x92R\x81 \x80T\x85\x92\x90a\x1A[\x90\x84\x90a?\x16V[\x92PP\x81\x90UP\x83`\x01`\x01`\xA0\x1B\x03\x16\x82`\x01`\x01`\xA0\x1B\x03\x16\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x85`@Qa\x1A\xA7\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3\x80\x83\x14a\x1A\xC4Wa\x1A\xBF\x84a#\tV[a\x1A\xEAV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x08` R`@\x90 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U[a\x163\x84_\x85a#\xB1V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1B{WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1Bo_\x80Q` aB\x93\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0EfW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1B\xA1a\x1D\x88V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01`@Q\x80\x91\x03\x90\xA1PV[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1C:WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1C7\x91\x81\x01\x90a>\xAAV[`\x01[a\x1CbW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x0BEV[_\x80Q` aB\x93\x839\x81Q\x91R\x81\x14a\x1C\x92W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[a\x1C\x9C\x83\x83a)+V[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0EfW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x1D\x1BWa\x1D\x1Ba>,V[\x14a\x13wW`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01` R`@\x81 T_\x80Q` aB3\x839\x81Q\x91R\x90a\x13\x97\x90a)\x80V[3a\x1D\xBA\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0EfW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x0BEV[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[\x7F\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x02\x80T``\x91_\x80Q` aBs\x839\x81Q\x91R\x91a\x1E\x91\x90a?)V[\x80`\x1F\x01` \x80\x91\x04\x02` \x01`@Q\x90\x81\x01`@R\x80\x92\x91\x90\x81\x81R` \x01\x82\x80Ta\x1E\xBD\x90a?)V[\x80\x15a\x1F\x08W\x80`\x1F\x10a\x1E\xDFWa\x01\0\x80\x83T\x04\x02\x83R\x91` \x01\x91a\x1F\x08V[\x82\x01\x91\x90_R` _ \x90[\x81T\x81R\x90`\x01\x01\x90` \x01\x80\x83\x11a\x1E\xEBW\x82\x90\x03`\x1F\x16\x82\x01\x91[PPPPP\x91PP\x90V[\x7F\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x03\x80T``\x91_\x80Q` aBs\x839\x81Q\x91R\x91a\x1E\x91\x90a?)V[\x80T_\x90\x80\x15a\x1F\x80Wa\x1Fj\x83a\x18\xD1`\x01\x84a?\x16V[T`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16a\x13\x97V[_\x93\x92PPPV[a\x1F\x90a)\xB0V[a\x13w\x81a)\xF9V[a\x0Efa)\xB0V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x1F\xE6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a \x02WP0;\x15[\x90P\x81\x15\x80\x15a \x10WP\x80\x15[\x15a .W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a XW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a \xA1W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a \xCFW`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a \xF6W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x94\x85\x16`\x01`\x01`\xA0\x1B\x03\x19\x91\x82\x16\x17\x90\x91U`\x01\x80T\x93\x90\x94\x16\x92\x16\x91\x90\x91\x17\x90\x91U`\tUV[_a\x0B\x87a!7a*\x01V[\x83`@Qa\x19\x01`\xF0\x1B\x81R`\x02\x81\x01\x92\x90\x92R`\"\x82\x01R`B\x90 \x90V[_\x80_\x80a!g\x88\x88\x88\x88a*\nV[\x92P\x92P\x92Pa!w\x82\x82a*\xD2V[P\x90\x96\x95PPPPPPV[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R\x7FZ\xB4,\xEDb\x88\x88%\x9C\x08\xAC\x98\xDB\x1E\xB0\xCFp/\xC1P\x13D1\x1D\x8B\x10\x0C\xD1\xBF\xE4\xBB\0` R`@\x90 \x80T`\x01\x81\x01\x90\x91U\x81\x81\x14a\x1C\x9CW`@Qc\x01\xD4\xB6#`\xE6\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R`$\x81\x01\x82\x90R`D\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R_a\"\x0C\x84a\r\x0EV[`\x01`\x01`\xA0\x1B\x03\x85\x81\x16_\x81\x81R` \x86\x90R`@\x80\x82 \x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x89\x86\x16\x90\x81\x17\x90\x91U\x90Q\x94\x95P\x93\x92\x85\x16\x92\x7F14\xE8\xA2\xE6\xD9~\x92\x9A~T\x01\x1E\xA5H]}\x19m\xD5\xF0\xBAMN\xF9X\x03\xE8\xE3\xFC%\x7F\x91\x90\xA4a\x163\x81\x84a\"\x9B\x87`\x01`\x01`\xA0\x1B\x03\x90\x81\x16_\x90\x81R`\x08` \x90\x81R`@\x80\x83 T\x90\x93\x16\x82R`\x03\x90R T\x90V[a+\x8AV[`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x80\x83\x01\x82\x90R`\x01`\x01`\xA0\x1B\x03\x86\x16\x82R\x7F\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x01\x90R\x91\x90\x91 _\x80Q` aB3\x839\x81Q\x91R\x90a#\x01\x90\x84a,\xEBV[\x94\x93PPPPV[3a\x088\x81\x83a!\xF4V[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a#mWP\x83;\x15=\x17\x15[\x80a \xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R`\x01`\x01`\xA0\x1B\x03\x84\x16a#\xE8Wa#\xE5\x81`\x02\x01a-Xa#\xE0\x85a-cV[a-\x96V[PP[`\x01`\x01`\xA0\x1B\x03\x83\x16a$\x0CWa$\t\x81`\x02\x01a-\xCEa#\xE0\x85a-cV[PP[a\x163a$\x18\x85a\r\x0EV[a$!\x85a\r\x0EV[\x84a+\x8AV[\x80Q\x82Q_\x91\x14\x80\x15a\x13\x97WPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` aBS\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a$iWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x1C\x9CW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a$\xF6\x83a-\xD9V[\x90P_\x80Q` aBS\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a%\x1DWa%\x1Da?aV[\x84\x82\t\x90P\x82\x80a%0Wa%0a?aV[\x82\x82\x08\x90P_\x80a%@\x83a/\xE2V[\x92P\x90P[\x80a%\xA9W\x84\x80a%XWa%Xa?aV[`\x01\x87\x08\x95P\x84\x80a%lWa%la?aV[\x86\x87\t\x92P\x84\x80a%\x7FWa%\x7Fa?aV[\x86\x84\t\x92P\x84\x80a%\x92Wa%\x92a?aV[\x84\x84\x08\x92Pa%\xA0\x83a/\xE2V[\x92P\x90Pa%EV[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a%\xEDWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` aBS\x839\x81Q\x91R\x84` \x01Qa&\x1E\x91\x90a?uV[a&5\x90_\x80Q` aBS\x839\x81Q\x91Ra?\x16V[\x90R\x92\x91PPV[a&d`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a'\xDCW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[P\x15\x15\x95\x94PPPPPV[_\x81_\x03a'\xF7WP_\x91\x90PV[_`\x01a(\x03\x84a0\xD9V[\x90\x1C`\x01\x90\x1B\x90P`\x01\x81\x84\x81a(\x1CWa(\x1Ca?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(4Wa(4a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(LWa(La?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(dWa(da?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(|Wa(|a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(\x94Wa(\x94a?aV[\x04\x82\x01\x90\x1C\x90P`\x01\x81\x84\x81a(\xACWa(\xACa?aV[\x04\x82\x01\x90\x1C\x90Pa\x13\x97\x81\x82\x85\x81a(\xC6Wa(\xC6a?aV[\x04a1lV[_[\x81\x83\x10\x15a)#W_a(\xE1\x84\x84a1\x81V[_\x87\x81R` \x90 \x90\x91Pe\xFF\xFF\xFF\xFF\xFF\xFF\x86\x16\x90\x82\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x16\x11\x15a)\x0FW\x80\x92Pa)\x1DV[a)\x1A\x81`\x01a>\xD5V[\x93P[Pa(\xCEV[P\x93\x92PPPV[a)4\x82a1\x9BV[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a)xWa\x1C\x9C\x82\x82a1\xFEV[a\x088a2pV[_c\xFF\xFF\xFF\xFF\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R` `\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x0EfW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x13\xA6a)\xB0V[_a\x0F\x94a2\x8FV[_\x80\x80\x7F\x7F\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF]WnsW\xA4P\x1D\xDF\xE9/Fh\x1B \xA0\x84\x11\x15a*CWP_\x91P`\x03\x90P\x82a*\xC8V[`@\x80Q_\x80\x82R` \x82\x01\x80\x84R\x8A\x90R`\xFF\x89\x16\x92\x82\x01\x92\x90\x92R``\x81\x01\x87\x90R`\x80\x81\x01\x86\x90R`\x01\x90`\xA0\x01` `@Q` \x81\x03\x90\x80\x84\x03\x90\x85Z\xFA\x15\x80\x15a*\x94W=_\x80>=_\xFD[PP`@Q`\x1F\x19\x01Q\x91PP`\x01`\x01`\xA0\x1B\x03\x81\x16a*\xBFWP_\x92P`\x01\x91P\x82\x90Pa*\xC8V[\x92P_\x91P\x81\x90P[\x94P\x94P\x94\x91PPV[_\x82`\x03\x81\x11\x15a*\xE5Wa*\xE5a>,V[\x03a*\xEEWPPV[`\x01\x82`\x03\x81\x11\x15a+\x02Wa+\x02a>,V[\x03a+ W`@Qc\xF6E\xEE\xDF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x82`\x03\x81\x11\x15a+4Wa+4a>,V[\x03a+UW`@Qc\xFC\xE6\x98\xF7`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[`\x03\x82`\x03\x81\x11\x15a+iWa+ia>,V[\x03a\x088W`@Qc5\xE2\xF3\x83`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0BEV[_\x80Q` aB3\x839\x81Q\x91R`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x90\x84\x16\x14\x80\x15\x90a+\xB3WP_\x82\x11[\x15a\x163W`\x01`\x01`\xA0\x1B\x03\x84\x16\x15a,WW`\x01`\x01`\xA0\x1B\x03\x84\x16_\x90\x81R`\x01\x82\x01` R`@\x81 \x81\x90a+\xF2\x90a-\xCEa#\xE0\x87a-cV[`\x01`\x01`\xD0\x1B\x03\x16\x91P`\x01`\x01`\xD0\x1B\x03\x16\x91P\x85`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDE\xC2\xBA\xCD\xD2\xF0[Y\xDE4\xDA\x9BR=\xFF\x8B\xE4.^8\xE8\x18\xC8/\xDB\x0B\xAEwC\x87\xA7$\x83\x83`@Qa,L\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[`@Q\x80\x91\x03\x90\xA2PP[`\x01`\x01`\xA0\x1B\x03\x83\x16\x15a\x163W`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x01\x82\x01` R`@\x81 \x81\x90a,\x91\x90a-Xa#\xE0\x87a-cV[`\x01`\x01`\xD0\x1B\x03\x16\x91P`\x01`\x01`\xD0\x1B\x03\x16\x91P\x84`\x01`\x01`\xA0\x1B\x03\x16\x7F\xDE\xC2\xBA\xCD\xD2\xF0[Y\xDE4\xDA\x9BR=\xFF\x8B\xE4.^8\xE8\x18\xC8/\xDB\x0B\xAEwC\x87\xA7$\x83\x83`@Qa\t\xC5\x92\x91\x90\x91\x82R` \x82\x01R`@\x01\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x82_\x01\x82c\xFF\xFF\xFF\xFF\x16\x81T\x81\x10a-\x18Wa-\x18a?\x88V[_\x91\x82R` \x91\x82\x90 `@\x80Q\x80\x82\x01\x90\x91R\x91\x01Te\xFF\xFF\xFF\xFF\xFF\xFF\x81\x16\x82R`\x01`0\x1B\x90\x04`\x01`\x01`\xD0\x1B\x03\x16\x91\x81\x01\x91\x90\x91R\x93\x92PPPV[_a\x13\x97\x82\x84a?\x9CV[_`\x01`\x01`\xD0\x1B\x03\x82\x11\x15a\x18MW`@Qc\x06\xDF\xCCe`\xE4\x1B\x81R`\xD0`\x04\x82\x01R`$\x81\x01\x83\x90R`D\x01a\x0BEV[_\x80a-\xC1a-\xA3a\x0F\x8BV[a-\xB9a-\xAF\x88a\x1FQV[\x86\x88c\xFF\xFF\xFF\xFF\x16V[\x87\x91\x90a3\x02V[\x91P\x91P[\x93P\x93\x91PPV[_a\x13\x97\x82\x84a?\xBCV[_\x80a-\xE4\x83a3\x0FV[\x80Q\x90\x91P`0\x81\x14a-\xF9Wa-\xF9a?\xDCV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a.\x13Wa.\x13a9\xA1V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a.=W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a.\xACW\x83`\x01a.W\x83\x86a?\x16V[a.a\x91\x90a?\x16V[\x81Q\x81\x10a.qWa.qa?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a.\x8EWa.\x8Ea?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a.BV[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a/<W\x83\x81a.\xE8\x85\x88a?\x16V[a.\xF2\x91\x90a>\xD5V[\x81Q\x81\x10a/\x02Wa/\x02a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a/\"Wa/\"a?\x88V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a.\xD4V[P_a/G\x82a6YV[\x90Pa\x01\0_\x80Q` aBS\x839\x81Q\x91R_a/e\x86\x89a?\x16V[\x90P_[\x81\x81\x10\x15a/\xD2W_\x88`\x01a/\x7F\x84\x86a?\x16V[a/\x89\x91\x90a?\x16V[\x81Q\x81\x10a/\x99Wa/\x99a?\x88V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a/\xB1Wa/\xB1a?aV[\x85\x87\t\x95P\x83\x80a/\xC4Wa/\xC4a?aV[\x81\x87\x08\x95PP`\x01\x01a/iV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` aBS\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a0\x9FW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x0BEV[\x80`\x01\x84\x90\x1B\x11\x15a0\xB8Wa0\xB5\x83\x82a?\x16V[\x92P[\x80\x80a0\xC6Wa0\xC6a?aV[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[_\x80`\x80\x83\x90\x1C\x15a0\xEDW`\x80\x92\x83\x1C\x92\x01[`@\x83\x90\x1C\x15a0\xFFW`@\x92\x83\x1C\x92\x01[` \x83\x90\x1C\x15a1\x11W` \x92\x83\x1C\x92\x01[`\x10\x83\x90\x1C\x15a1#W`\x10\x92\x83\x1C\x92\x01[`\x08\x83\x90\x1C\x15a15W`\x08\x92\x83\x1C\x92\x01[`\x04\x83\x90\x1C\x15a1GW`\x04\x92\x83\x1C\x92\x01[`\x02\x83\x90\x1C\x15a1YW`\x02\x92\x83\x1C\x92\x01[`\x01\x83\x90\x1C\x15a\x0B\x87W`\x01\x01\x92\x91PPV[_\x81\x83\x10a1zW\x81a\x13\x97V[P\x90\x91\x90PV[_a1\x8F`\x02\x84\x84\x18a?\xF0V[a\x13\x97\x90\x84\x84\x16a>\xD5V[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a1\xD0W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x0BEV[_\x80Q` aB\x93\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa2\x1A\x91\x90a@\x03V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a2RW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a2WV[``\x91P[P\x91P\x91Pa2g\x85\x83\x83a6\xC0V[\x95\x94PPPPPV[4\x15a\x0EfW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x7F\x8Bs\xC3\xC6\x9B\xB8\xFE=Q.\xCCL\xF7Y\xCCy#\x9F{\x17\x9B\x0F\xFA\xCA\xA9\xA7]R+9@\x0Fa2\xB9a7\x15V[a2\xC1a7}V[`@\x80Q` \x81\x01\x94\x90\x94R\x83\x01\x91\x90\x91R``\x82\x01RF`\x80\x82\x01R0`\xA0\x82\x01R`\xC0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x90V[_\x80a-\xC1\x85\x85\x85a7\xBFV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a3O\x92\x91\x90a>\xE8V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a3v\x92\x91\x90a@\x1EV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a3\x98\x91\x90a@PV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a3\xC2\x90\x83\x90\x83\x90` \x01a@pV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a42Wa42a9\xA1V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a4\\W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a4s\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a4\xDCW\x81\x81\x81Q\x81\x10a4\xA1Wa4\xA1a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a4\xBEWa4\xBEa?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a4\x86V[P_\x84`@Q` \x01a4\xF1\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a5\x83W_\x83\x82\x81Q\x81\x10a5*Wa5*a?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a5GWa5Ga?\x88V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a5h\x92\x91\x90a@\x9EV[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a5\x0FV[P\x86\x88\x87`@Q` \x01a5\x99\x93\x92\x91\x90a@\xCCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a5\xC7\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a5\xE7\x8A`\xFF\x8D\x16a?\x16V[\x81\x10\x15a6HW\x82\x81\x81Q\x81\x10a6\0Wa6\0a?\x88V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a6\x1A\x83\x8Da>\xD5V[\x81Q\x81\x10a6*Wa6*a?\x88V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a5\xDAV[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a6\xB9W\x83\x81\x81Q\x81\x10a6xWa6xa?\x88V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a6\x90\x91\x90aA\x0CV[a6\x9B\x90`\x02aB\x03V[a6\xA5\x91\x90aA\x0CV[a6\xAF\x90\x83a>\xD5V[\x91P`\x01\x01a6]V[P\x92\x91PPV[``\x82a6\xD5Wa6\xD0\x82a95V[a\x13\x97V[\x81Q\x15\x80\x15a6\xECWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a6\xB9W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x0BEV[__\x80Q` aBs\x839\x81Q\x91R\x81a7-a\x1ESV[\x80Q\x90\x91P\x15a7EW\x80Q` \x90\x91\x01 \x92\x91PPV[\x81T\x80\x15a7TW\x93\x92PPPV[\x7F\xC5\xD2F\x01\x86\xF7#<\x92~}\xB2\xDC\xC7\x03\xC0\xE5\0\xB6S\xCA\x82';{\xFA\xD8\x04]\x85\xA4p\x93PPPP\x90V[__\x80Q` aBs\x839\x81Q\x91R\x81a7\x95a\x1F\x13V[\x80Q\x90\x91P\x15a7\xADW\x80Q` \x90\x91\x01 \x92\x91PPV[`\x01\x82\x01T\x80\x15a7TW\x93\x92PPPV[\x82T_\x90\x81\x90\x80\x15a8\xDBW_a7\xDB\x87a\x18\xD1`\x01\x85a?\x16V[`@\x80Q\x80\x82\x01\x90\x91R\x90Te\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x16\x80\x84R`\x01`0\x1B\x90\x92\x04`\x01`\x01`\xD0\x1B\x03\x16` \x84\x01R\x91\x92P\x90\x87\x16\x10\x15a8/W`@Qc% `\x1D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Qe\xFF\xFF\xFF\xFF\xFF\xFF\x80\x88\x16\x91\x16\x03a8{W\x84a8R\x88a\x18\xD1`\x01\x86a?\x16V[\x80T`\x01`\x01`\xD0\x1B\x03\x92\x90\x92\x16`\x01`0\x1B\x02e\xFF\xFF\xFF\xFF\xFF\xFF\x90\x92\x16\x91\x90\x91\x17\x90Ua8\xCBV[`@\x80Q\x80\x82\x01\x90\x91Re\xFF\xFF\xFF\xFF\xFF\xFF\x80\x88\x16\x82R`\x01`\x01`\xD0\x1B\x03\x80\x88\x16` \x80\x85\x01\x91\x82R\x8BT`\x01\x81\x01\x8DU_\x8D\x81R\x91\x90\x91 \x94Q\x91Q\x90\x92\x16`\x01`0\x1B\x02\x92\x16\x91\x90\x91\x17\x91\x01U[` \x01Q\x92P\x83\x91Pa-\xC6\x90PV[PP`@\x80Q\x80\x82\x01\x90\x91Re\xFF\xFF\xFF\xFF\xFF\xFF\x80\x85\x16\x82R`\x01`\x01`\xD0\x1B\x03\x80\x85\x16` \x80\x85\x01\x91\x82R\x88T`\x01\x81\x01\x8AU_\x8A\x81R\x91\x82 \x95Q\x92Q\x90\x93\x16`\x01`0\x1B\x02\x91\x90\x93\x16\x17\x92\x01\x91\x90\x91U\x90P\x81a-\xC6V[\x80Q\x15a9EW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a9tW_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a9\x8AW_\x80\xFD[a9\x93\x83a9^V[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a9\xDEWa9\xDEa9\xA1V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a9\xF6W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a:\x19Wa:\x19a9\xA1V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a:ZW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a:}Wa:}a9\xA1V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a:\xABW_\x80\xFD[a:\xB5\x86\x86a9\xE6V[\x93Pa:\xC4\x86`\x80\x87\x01a:JV[\x92Pa:\xD3\x86`\xC0\x87\x01a:JV[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a:\xEAW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a;\x05W_\x80\xFD[a\x13\x97\x82a9^V[_[\x83\x81\x10\x15a;(W\x81\x81\x01Q\x83\x82\x01R` \x01a;\x10V[PP_\x91\x01RV[_\x81Q\x80\x84Ra;G\x81` \x86\x01` \x86\x01a;\x0EV[`\x1F\x01`\x1F\x19\x16\x92\x90\x92\x01` \x01\x92\x91PPV[` \x81R_a\x13\x97` \x83\x01\x84a;0V[_\x80`@\x83\x85\x03\x12\x15a;~W_\x80\xFD[a;\x87\x83a9^V[\x91P` \x80\x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a;\xA4W_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a;\xB7W_\x80\xFD[\x815\x81\x81\x11\x15a;\xC9Wa;\xC9a9\xA1V[a;\xDB`\x1F\x82\x01`\x1F\x19\x16\x85\x01a9\xB5V[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a;\xF0W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[_\x80_a\x01\0\x84\x86\x03\x12\x15a< W_\x80\xFD[a<*\x85\x85a9\xE6V[\x92Pa<9\x85`\x80\x86\x01a:JV[\x91Pa<H\x85`\xC0\x86\x01a:JV[\x90P\x92P\x92P\x92V[`\xFF`\xF8\x1B\x88\x16\x81R_` `\xE0` \x84\x01Ra<q`\xE0\x84\x01\x8Aa;0V[\x83\x81\x03`@\x85\x01Ra<\x83\x81\x8Aa;0V[``\x85\x01\x89\x90R`\x01`\x01`\xA0\x1B\x03\x88\x16`\x80\x86\x01R`\xA0\x85\x01\x87\x90R\x84\x81\x03`\xC0\x86\x01R\x85Q\x80\x82R` \x80\x88\x01\x93P\x90\x91\x01\x90_[\x81\x81\x10\x15a<\xD6W\x83Q\x83R\x92\x84\x01\x92\x91\x84\x01\x91`\x01\x01a<\xBAV[P\x90\x9C\x9BPPPPPPPPPPPPV[_` \x82\x84\x03\x12\x15a<\xF8W_\x80\xFD[P5\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a=\x0FW_\x80\xFD[a\x13\x97\x83\x83a9\xE6V[_\x80`@\x83\x85\x03\x12\x15a=*W_\x80\xFD[a=3\x83a9^V[\x91Pa=A` \x84\x01a9^V[\x90P\x92P\x92\x90PV[_\x80_\x80`\x80\x85\x87\x03\x12\x15a=]W_\x80\xFD[a=f\x85a9^V[\x93Pa=t` \x86\x01a9^V[\x92P`@\x85\x015\x91Pa=\x89``\x86\x01a9^V[\x90P\x92\x95\x91\x94P\x92PV[_\x80_\x80_\x80`\xC0\x87\x89\x03\x12\x15a=\xA9W_\x80\xFD[a=\xB2\x87a9^V[\x95P` \x87\x015\x94P`@\x87\x015\x93P``\x87\x015`\xFF\x81\x16\x81\x14a=\xD5W_\x80\xFD[\x95\x98\x94\x97P\x92\x95`\x80\x81\x015\x94`\xA0\x90\x91\x015\x93P\x91PPV[_\x80`@\x83\x85\x03\x12\x15a>\0W_\x80\xFD[a>\t\x83a9^V[\x91P` \x83\x015c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a>!W_\x80\xFD[\x80\x91PP\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a>cWcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R`@\x80\x85\x01Q\x90\x83\x01R``\x80\x85\x01Q\x90\x83\x01R\x82Q`\x80\x83\x01R\x82\x01Q`\xA0\x82\x01R`\xC0\x81\x01a\x13\x97V[_` \x82\x84\x03\x12\x15a>\xBAW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0B\x87Wa\x0B\x87a>\xC1V[_\x83Qa>\xF9\x81\x84` \x88\x01a;\x0EV[\x83Q\x90\x83\x01\x90a?\r\x81\x83` \x88\x01a;\x0EV[\x01\x94\x93PPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0B\x87Wa\x0B\x87a>\xC1V[`\x01\x81\x81\x1C\x90\x82\x16\x80a?=W`\x7F\x82\x16\x91P[` \x82\x10\x81\x03a?[WcNH{q`\xE0\x1B_R`\"`\x04R`$_\xFD[P\x91\x90PV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a?\x83Wa?\x83a?aV[P\x06\x90V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[`\x01`\x01`\xD0\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x80\x82\x11\x15a6\xB9Wa6\xB9a>\xC1V[`\x01`\x01`\xD0\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15a6\xB9Wa6\xB9a>\xC1V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[_\x82a?\xFEWa?\xFEa?aV[P\x04\x90V[_\x82Qa@\x14\x81\x84` \x87\x01a;\x0EV[\x91\x90\x91\x01\x92\x91PPV[_\x83Qa@/\x81\x84` \x88\x01a;\x0EV[_\x92\x01\x91\x82RP`\x01`\x01`\xF8\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x02\x01\x91\x90PV[_\x82Qa@a\x81\x84` \x87\x01a;\x0EV[_\x92\x01\x91\x82RP`\x01\x01\x91\x90PV[_\x83Qa@\x81\x81\x84` \x88\x01a;\x0EV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x02\x01\x92\x91PPV[_\x83Qa@\xAF\x81\x84` \x88\x01a;\x0EV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x01\x01\x92\x91PPV[_\x84Qa@\xDD\x81\x84` \x89\x01a;\x0EV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x91\x90\x93\x01\x90\x81R`\x01`\x01`\xF0\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x03\x01\x92\x91PPV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0B\x87Wa\x0B\x87a>\xC1V[`\x01\x81\x81[\x80\x85\x11\x15aA]W\x81_\x19\x04\x82\x11\x15aACWaACa>\xC1V[\x80\x85\x16\x15aAPW\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90aA(V[P\x92P\x92\x90PV[_\x82aAsWP`\x01a\x0B\x87V[\x81aA\x7FWP_a\x0B\x87V[\x81`\x01\x81\x14aA\x95W`\x02\x81\x14aA\x9FWaA\xBBV[`\x01\x91PPa\x0B\x87V[`\xFF\x84\x11\x15aA\xB0WaA\xB0a>\xC1V[PP`\x01\x82\x1Ba\x0B\x87V[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15aA\xDEWP\x81\x81\na\x0B\x87V[aA\xE8\x83\x83aA#V[\x80_\x19\x04\x82\x11\x15aA\xFBWaA\xFBa>\xC1V[\x02\x93\x92PPPV[_a\x13\x97\x83\x83aAeV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_\xE8\xB2l0\xFA\xD7A\x98\x95`2\xA3S=\x903\x85\xD5m\xD7\x95\xAFV\x01\x96\xF9\xC7\x8DJ\xF4\r\x000dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1jF\xD9Ba\xC7Q|\xC8\xFF\x89\xF6\x1C\x0C\xE95\x98\xE3\xC8I\x80\x10\x11\xDE\xE6I\xA6\xA5W\xD1\x006\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
    );
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
            ) -> <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::Token<'_>
            {
                alloy_sol_types::private::SolTypeValue::<
                    alloy::sol_types::sol_data::Uint<8>,
                >::stv_to_tokens(self)
            }
            #[inline]
            fn stv_eip712_data_word(&self) -> alloy_sol_types::Word {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(self).0
            }
            #[inline]
            fn stv_abi_encode_packed_to(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
                <alloy::sol_types::sol_data::Uint<
                    8,
                > as alloy_sol_types::SolType>::abi_encode_packed_to(self, out)
            }
            #[inline]
            fn stv_abi_packed_encoded_size(&self) -> usize {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::abi_encoded_size(
                    self,
                )
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
            type Token<'a> =
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::Token<'a>;
            const SOL_NAME: &'static str = Self::NAME;
            const ENCODED_SIZE: Option<usize> =
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::ENCODED_SIZE;
            const PACKED_ENCODED_SIZE: Option<usize> = <alloy::sol_types::sol_data::Uint<
                8,
            > as alloy_sol_types::SolType>::PACKED_ENCODED_SIZE;
            #[inline]
            fn valid_token(token: &Self::Token<'_>) -> bool {
                Self::type_check(token).is_ok()
            }
            #[inline]
            fn type_check(token: &Self::Token<'_>) -> alloy_sol_types::Result<()> {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::type_check(token)
            }
            #[inline]
            fn detokenize(token: Self::Token<'_>) -> Self::RustType {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::detokenize(token)
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
            fn encode_topic(rust: &Self::RustType) -> alloy_sol_types::abi::token::WordToken {
                <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::EventTopic>::encode_topic(
                    rust,
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<BLSSigVerificationFailed> for UnderlyingRustTuple<'_> {
            fn from(value: BLSSigVerificationFailed) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for BLSSigVerificationFailed {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for BLSSigVerificationFailed {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `CheckpointUnorderedInsertion()` and selector `0x2520601d`.
    ```solidity
    error CheckpointUnorderedInsertion();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct CheckpointUnorderedInsertion {}
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<CheckpointUnorderedInsertion> for UnderlyingRustTuple<'_> {
            fn from(value: CheckpointUnorderedInsertion) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for CheckpointUnorderedInsertion {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for CheckpointUnorderedInsertion {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "CheckpointUnorderedInsertion()";
            const SELECTOR: [u8; 4] = [37u8, 32u8, 96u8, 29u8];
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
    /**Custom error with signature `DelegatorAlreadyStaked()` and selector `0x0c595ddf`.
    ```solidity
    error DelegatorAlreadyStaked();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DelegatorAlreadyStaked {}
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<DelegatorAlreadyStaked> for UnderlyingRustTuple<'_> {
            fn from(value: DelegatorAlreadyStaked) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for DelegatorAlreadyStaked {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for DelegatorAlreadyStaked {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "DelegatorAlreadyStaked()";
            const SELECTOR: [u8; 4] = [12u8, 89u8, 93u8, 223u8];
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
    /**Custom error with signature `ECDSAInvalidSignature()` and selector `0xf645eedf`.
    ```solidity
    error ECDSAInvalidSignature();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ECDSAInvalidSignature {}
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ECDSAInvalidSignature> for UnderlyingRustTuple<'_> {
            fn from(value: ECDSAInvalidSignature) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ECDSAInvalidSignature {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ECDSAInvalidSignature {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ECDSAInvalidSignature()";
            const SELECTOR: [u8; 4] = [246u8, 69u8, 238u8, 223u8];
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
    /**Custom error with signature `ECDSAInvalidSignatureLength(uint256)` and selector `0xfce698f7`.
    ```solidity
    error ECDSAInvalidSignatureLength(uint256 length);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ECDSAInvalidSignatureLength {
        #[allow(missing_docs)]
        pub length: alloy::sol_types::private::primitives::aliases::U256,
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
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ECDSAInvalidSignatureLength> for UnderlyingRustTuple<'_> {
            fn from(value: ECDSAInvalidSignatureLength) -> Self {
                (value.length,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ECDSAInvalidSignatureLength {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { length: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ECDSAInvalidSignatureLength {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ECDSAInvalidSignatureLength(uint256)";
            const SELECTOR: [u8; 4] = [252u8, 230u8, 152u8, 247u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.length,
                    ),
                )
            }
        }
    };
    /**Custom error with signature `ECDSAInvalidSignatureS(bytes32)` and selector `0xd78bce0c`.
    ```solidity
    error ECDSAInvalidSignatureS(bytes32 s);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ECDSAInvalidSignatureS {
        #[allow(missing_docs)]
        pub s: alloy::sol_types::private::FixedBytes<32>,
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ECDSAInvalidSignatureS> for UnderlyingRustTuple<'_> {
            fn from(value: ECDSAInvalidSignatureS) -> Self {
                (value.s,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ECDSAInvalidSignatureS {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { s: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ECDSAInvalidSignatureS {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ECDSAInvalidSignatureS(bytes32)";
            const SELECTOR: [u8; 4] = [215u8, 139u8, 206u8, 12u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.s),
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ERC1967InvalidImplementation> for UnderlyingRustTuple<'_> {
            fn from(value: ERC1967InvalidImplementation) -> Self {
                (value.implementation,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ERC1967InvalidImplementation {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    implementation: tuple.0,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ERC1967InvalidImplementation {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `ERC5805FutureLookup(uint256,uint48)` and selector `0xecd3f81e`.
    ```solidity
    error ERC5805FutureLookup(uint256 timepoint, uint48 clock);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ERC5805FutureLookup {
        #[allow(missing_docs)]
        pub timepoint: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub clock: alloy::sol_types::private::primitives::aliases::U48,
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
            alloy::sol_types::sol_data::Uint<48>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            alloy::sol_types::private::primitives::aliases::U48,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ERC5805FutureLookup> for UnderlyingRustTuple<'_> {
            fn from(value: ERC5805FutureLookup) -> Self {
                (value.timepoint, value.clock)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ERC5805FutureLookup {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    timepoint: tuple.0,
                    clock: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ERC5805FutureLookup {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ERC5805FutureLookup(uint256,uint48)";
            const SELECTOR: [u8; 4] = [236u8, 211u8, 248u8, 30u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.timepoint,
                    ),
                    <alloy::sol_types::sol_data::Uint<48> as alloy_sol_types::SolType>::tokenize(
                        &self.clock,
                    ),
                )
            }
        }
    };
    /**Custom error with signature `ERC6372InconsistentClock()` and selector `0x6ff07140`.
    ```solidity
    error ERC6372InconsistentClock();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ERC6372InconsistentClock {}
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ERC6372InconsistentClock> for UnderlyingRustTuple<'_> {
            fn from(value: ERC6372InconsistentClock) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ERC6372InconsistentClock {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ERC6372InconsistentClock {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ERC6372InconsistentClock()";
            const SELECTOR: [u8; 4] = [111u8, 240u8, 113u8, 64u8];
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
                Self {
                    _0: tuple.0,
                    _1: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InsufficientAllowance {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._0,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                )
            }
        }
    };
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
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._0,
                    ),
                )
            }
        }
    };
    /**Custom error with signature `InvalidAccountNonce(address,uint256)` and selector `0x752d88c0`.
    ```solidity
    error InvalidAccountNonce(address account, uint256 currentNonce);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidAccountNonce {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub currentNonce: alloy::sol_types::private::primitives::aliases::U256,
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
            alloy::sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::Address,
            alloy::sol_types::private::primitives::aliases::U256,
        );
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<InvalidAccountNonce> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidAccountNonce) -> Self {
                (value.account, value.currentNonce)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidAccountNonce {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    account: tuple.0,
                    currentNonce: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidAccountNonce {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidAccountNonce(address,uint256)";
            const SELECTOR: [u8; 4] = [117u8, 45u8, 136u8, 192u8];
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.currentNonce,
                    ),
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<OwnableUnauthorizedAccount> for UnderlyingRustTuple<'_> {
            fn from(value: OwnableUnauthorizedAccount) -> Self {
                (value.account,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for OwnableUnauthorizedAccount {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { account: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for OwnableUnauthorizedAccount {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `SafeCastOverflowedUintDowncast(uint8,uint256)` and selector `0x6dfcc650`.
    ```solidity
    error SafeCastOverflowedUintDowncast(uint8 bits, uint256 value);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct SafeCastOverflowedUintDowncast {
        #[allow(missing_docs)]
        pub bits: u8,
        #[allow(missing_docs)]
        pub value: alloy::sol_types::private::primitives::aliases::U256,
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
            alloy::sol_types::sol_data::Uint<8>,
            alloy::sol_types::sol_data::Uint<256>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (u8, alloy::sol_types::private::primitives::aliases::U256);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<SafeCastOverflowedUintDowncast> for UnderlyingRustTuple<'_> {
            fn from(value: SafeCastOverflowedUintDowncast) -> Self {
                (value.bits, value.value)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for SafeCastOverflowedUintDowncast {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    bits: tuple.0,
                    value: tuple.1,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for SafeCastOverflowedUintDowncast {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "SafeCastOverflowedUintDowncast(uint8,uint256)";
            const SELECTOR: [u8; 4] = [109u8, 252u8, 198u8, 80u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(
                        &self.bits,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.value,
                    ),
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UUPSUnauthorizedCallContext> for UnderlyingRustTuple<'_> {
            fn from(value: UUPSUnauthorizedCallContext) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for UUPSUnauthorizedCallContext {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UUPSUnauthorizedCallContext {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UUPSUnsupportedProxiableUUID> for UnderlyingRustTuple<'_> {
            fn from(value: UUPSUnsupportedProxiableUUID) -> Self {
                (value.slot,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for UUPSUnsupportedProxiableUUID {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { slot: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UUPSUnsupportedProxiableUUID {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorAlreadyRegistered> for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorAlreadyRegistered) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ValidatorAlreadyRegistered {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorAlreadyRegistered {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `ValidatorCannotDelegate()` and selector `0x8869a326`.
    ```solidity
    error ValidatorCannotDelegate();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ValidatorCannotDelegate {}
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<ValidatorCannotDelegate> for UnderlyingRustTuple<'_> {
            fn from(value: ValidatorCannotDelegate) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ValidatorCannotDelegate {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ValidatorCannotDelegate {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ValidatorCannotDelegate()";
            const SELECTOR: [u8; 4] = [136u8, 105u8, 163u8, 38u8];
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Custom error with signature `VotesExpiredSignature(uint256)` and selector `0x4683af0e`.
    ```solidity
    error VotesExpiredSignature(uint256 expiry);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct VotesExpiredSignature {
        #[allow(missing_docs)]
        pub expiry: alloy::sol_types::private::primitives::aliases::U256,
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
        type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
        #[cfg(test)]
        #[allow(dead_code, unreachable_patterns)]
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<VotesExpiredSignature> for UnderlyingRustTuple<'_> {
            fn from(value: VotesExpiredSignature) -> Self {
                (value.expiry,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for VotesExpiredSignature {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { expiry: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for VotesExpiredSignature {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "VotesExpiredSignature(uint256)";
            const SELECTOR: [u8; 4] = [70u8, 131u8, 175u8, 14u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.expiry,
                    ),
                )
            }
        }
    };
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
        fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
            match _t {
                alloy_sol_types::private::AssertTypeEq::<
                    <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str =
                "ConsensusKeysUpdated(address,(uint256,uint256,uint256,uint256),(uint256,uint256))";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    128u8, 216u8, 164u8, 161u8, 102u8, 51u8, 40u8, 169u8, 152u8, 212u8, 85u8, 91u8,
                    162u8, 29u8, 139u8, 186u8, 110u8, 241u8, 87u8, 106u8, 140u8, 94u8, 157u8, 39u8,
                    249u8, 197u8, 69u8, 241u8, 163u8, 213u8, 43u8, 29u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
    /**Event with signature `DelegateChanged(address,address,address)` and selector `0x3134e8a2e6d97e929a7e54011ea5485d7d196dd5f0ba4d4ef95803e8e3fc257f`.
    ```solidity
    event DelegateChanged(address indexed delegator, address indexed fromDelegate, address indexed toDelegate);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct DelegateChanged {
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub fromDelegate: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub toDelegate: alloy::sol_types::private::Address,
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
        impl alloy_sol_types::SolEvent for DelegateChanged {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "DelegateChanged(address,address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    49u8, 52u8, 232u8, 162u8, 230u8, 217u8, 126u8, 146u8, 154u8, 126u8, 84u8, 1u8,
                    30u8, 165u8, 72u8, 93u8, 125u8, 25u8, 109u8, 213u8, 240u8, 186u8, 77u8, 78u8,
                    249u8, 88u8, 3u8, 232u8, 227u8, 252u8, 37u8, 127u8,
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
                    fromDelegate: topics.2,
                    toDelegate: topics.3,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                    self.delegator.clone(),
                    self.fromDelegate.clone(),
                    self.toDelegate.clone(),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.delegator,
                );
                out[2usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.fromDelegate,
                );
                out[3usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.toDelegate,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for DelegateChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&DelegateChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &DelegateChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Event with signature `DelegateVotesChanged(address,uint256,uint256)` and selector `0xdec2bacdd2f05b59de34da9b523dff8be42e5e38e818c82fdb0bae774387a724`.
    ```solidity
    event DelegateVotesChanged(address indexed delegate, uint256 previousVotes, uint256 newVotes);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct DelegateVotesChanged {
        #[allow(missing_docs)]
        pub delegate: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub previousVotes: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub newVotes: alloy::sol_types::private::primitives::aliases::U256,
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
        impl alloy_sol_types::SolEvent for DelegateVotesChanged {
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "DelegateVotesChanged(address,uint256,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    222u8, 194u8, 186u8, 205u8, 210u8, 240u8, 91u8, 89u8, 222u8, 52u8, 218u8,
                    155u8, 82u8, 61u8, 255u8, 139u8, 228u8, 46u8, 94u8, 56u8, 232u8, 24u8, 200u8,
                    47u8, 219u8, 11u8, 174u8, 119u8, 67u8, 135u8, 167u8, 36u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    delegate: topics.1,
                    previousVotes: data.0,
                    newVotes: data.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.previousVotes,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.newVotes,
                    ),
                )
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (Self::SIGNATURE_HASH.into(), self.delegate.clone())
            }
            #[inline]
            fn encode_topics_raw(
                &self,
                out: &mut [alloy_sol_types::abi::token::WordToken],
            ) -> alloy_sol_types::Result<()> {
                if out.len() < <Self::TopicList as alloy_sol_types::TopicList>::COUNT {
                    return Err(alloy_sol_types::Error::Overrun);
                }
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                out[1usize] = <alloy::sol_types::sol_data::Address as alloy_sol_types::EventTopic>::encode_topic(
                    &self.delegate,
                );
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for DelegateVotesChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&DelegateVotesChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &DelegateVotesChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Event with signature `EIP712DomainChanged()` and selector `0x0a6387c9ea3628b88a633bb4f3b151770f70085117a15f9bf3787cda53f13d31`.
    ```solidity
    event EIP712DomainChanged();
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct EIP712DomainChanged {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for EIP712DomainChanged {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "EIP712DomainChanged()";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    10u8, 99u8, 135u8, 201u8, 234u8, 54u8, 40u8, 184u8, 138u8, 99u8, 59u8, 180u8,
                    243u8, 177u8, 81u8, 119u8, 15u8, 112u8, 8u8, 81u8, 23u8, 161u8, 95u8, 155u8,
                    243u8, 120u8, 124u8, 218u8, 83u8, 241u8, 61u8, 49u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {}
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                ()
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for EIP712DomainChanged {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&EIP712DomainChanged> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &EIP712DomainChanged) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Initialized(uint64)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    199u8, 245u8, 5u8, 178u8, 243u8, 113u8, 174u8, 33u8, 117u8, 238u8, 73u8, 19u8,
                    244u8, 73u8, 158u8, 31u8, 38u8, 51u8, 167u8, 181u8, 147u8, 99u8, 33u8, 238u8,
                    209u8, 205u8, 174u8, 182u8, 17u8, 81u8, 129u8, 210u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.version,
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "OwnershipTransferred(address,address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8, 31u8,
                    208u8, 164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8, 218u8,
                    175u8, 227u8, 180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
    /**Event with signature `Staked(address,address,uint256)` and selector `0x5dac0c1b1112564a045ba943c9d50270893e8e826c49be8e7073adc713ab7bd7`.
    ```solidity
    event Staked(address indexed delegator, address indexed validator, uint256 amount);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct Staked {
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
        impl alloy_sol_types::SolEvent for Staked {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Staked(address,address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    93u8, 172u8, 12u8, 27u8, 17u8, 18u8, 86u8, 74u8, 4u8, 91u8, 169u8, 67u8, 201u8,
                    213u8, 2u8, 112u8, 137u8, 62u8, 142u8, 130u8, 108u8, 73u8, 190u8, 142u8, 112u8,
                    115u8, 173u8, 199u8, 19u8, 171u8, 123u8, 215u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
                    ),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
        impl alloy_sol_types::private::IntoLogData for Staked {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&Staked> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &Staked) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Undelegated(address,address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    77u8, 16u8, 189u8, 4u8, 151u8, 117u8, 199u8, 123u8, 215u8, 242u8, 85u8, 25u8,
                    90u8, 251u8, 165u8, 8u8, 128u8, 40u8, 236u8, 179u8, 199u8, 194u8, 119u8, 211u8,
                    147u8, 204u8, 255u8, 121u8, 52u8, 242u8, 249u8, 44u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
                    ),
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "Upgrade(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8, 154u8,
                    22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8, 185u8, 62u8,
                    237u8, 168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    implementation: data.0,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Upgraded(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    188u8, 124u8, 215u8, 90u8, 32u8, 238u8, 39u8, 253u8, 154u8, 222u8, 186u8,
                    179u8, 32u8, 65u8, 247u8, 85u8, 33u8, 77u8, 188u8, 107u8, 255u8, 169u8, 12u8,
                    192u8, 34u8, 91u8, 57u8, 218u8, 46u8, 92u8, 45u8, 59u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    implementation: topics.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ValidatorExit(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    251u8, 36u8, 48u8, 83u8, 84u8, 200u8, 119u8, 98u8, 213u8, 87u8, 72u8, 122u8,
                    228u8, 165u8, 100u8, 232u8, 208u8, 62u8, 203u8, 185u8, 169u8, 125u8, 216u8,
                    175u8, 255u8, 142u8, 31u8, 111u8, 202u8, 240u8, 221u8, 22u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    validator: topics.1,
                }
            }
            #[inline]
            fn check_signature(
                topics: &<Self::TopicList as alloy_sol_types::SolType>::RustType,
            ) -> alloy_sol_types::Result<()> {
                if topics.0 != Self::SIGNATURE_HASH {
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "ValidatorRegistered(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    246u8, 232u8, 53u8, 156u8, 87u8, 82u8, 11u8, 70u8, 150u8, 52u8, 115u8, 107u8,
                    252u8, 59u8, 183u8, 236u8, 92u8, 189u8, 26u8, 11u8, 210u8, 139u8, 16u8, 168u8,
                    39u8, 87u8, 147u8, 187u8, 115u8, 11u8, 121u8, 127u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
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
                    <alloy::sol_types::sol_data::Uint<16> as alloy_sol_types::SolType>::tokenize(
                        &self.commission,
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Withdrawal(address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    127u8, 207u8, 83u8, 44u8, 21u8, 240u8, 166u8, 219u8, 11u8, 214u8, 208u8, 224u8,
                    56u8, 190u8, 167u8, 29u8, 48u8, 216u8, 8u8, 199u8, 217u8, 140u8, 179u8, 191u8,
                    114u8, 104u8, 169u8, 91u8, 245u8, 8u8, 27u8, 101u8,
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
                    return Err(alloy_sol_types::Error::invalid_event_signature_hash(
                        Self::SIGNATURE,
                        topics.0,
                        Self::SIGNATURE_HASH,
                    ));
                }
                Ok(())
            }
            #[inline]
            fn tokenize_body(&self) -> Self::DataToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(Self::SIGNATURE_HASH);
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `CLOCK_MODE()` and selector `0x4bf5d7e9`.
    ```solidity
    function CLOCK_MODE() external view returns (string memory);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct CLOCK_MODECall {}
    ///Container type for the return parameters of the [`CLOCK_MODE()`](CLOCK_MODECall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct CLOCK_MODEReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<CLOCK_MODECall> for UnderlyingRustTuple<'_> {
                fn from(value: CLOCK_MODECall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for CLOCK_MODECall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<CLOCK_MODEReturn> for UnderlyingRustTuple<'_> {
                fn from(value: CLOCK_MODEReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for CLOCK_MODEReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for CLOCK_MODECall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = CLOCK_MODEReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::String,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "CLOCK_MODE()";
            const SELECTOR: [u8; 4] = [75u8, 245u8, 215u8, 233u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `UPGRADE_INTERFACE_VERSION()` and selector `0xad3cb1cc`.
    ```solidity
    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UPGRADE_INTERFACE_VERSIONCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UPGRADE_INTERFACE_VERSIONCall> for UnderlyingRustTuple<'_> {
                fn from(value: UPGRADE_INTERFACE_VERSIONCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for UPGRADE_INTERFACE_VERSIONCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UPGRADE_INTERFACE_VERSIONReturn> for UnderlyingRustTuple<'_> {
                fn from(value: UPGRADE_INTERFACE_VERSIONReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for UPGRADE_INTERFACE_VERSIONReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for UPGRADE_INTERFACE_VERSIONCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = UPGRADE_INTERFACE_VERSIONReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::String,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            type UnderlyingRustTuple<'a> =
                (<BN254::G2Point as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = _hashBlsKeyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                (<BN254::G2Point as alloy_sol_types::SolType>::tokenize(
                    &self.blsVK,
                ),)
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
                    Self {
                        blsKeyHash: tuple.0,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = blsKeysReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `checkpoints(address,uint32)` and selector `0xf1127ed8`.
    ```solidity
    function checkpoints(address account, uint32 pos) external view returns (Checkpoints.Checkpoint208 memory);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct checkpointsCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub pos: u32,
    }
    ///Container type for the return parameters of the [`checkpoints(address,uint32)`](checkpointsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct checkpointsReturn {
        #[allow(missing_docs)]
        pub _0: <Checkpoints::Checkpoint208 as alloy::sol_types::SolType>::RustType,
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
                alloy::sol_types::sol_data::Uint<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address, u32);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<checkpointsCall> for UnderlyingRustTuple<'_> {
                fn from(value: checkpointsCall) -> Self {
                    (value.account, value.pos)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for checkpointsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        account: tuple.0,
                        pos: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (Checkpoints::Checkpoint208,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<Checkpoints::Checkpoint208 as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<checkpointsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: checkpointsReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for checkpointsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for checkpointsCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = checkpointsReturn;
            type ReturnTuple<'a> = (Checkpoints::Checkpoint208,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "checkpoints(address,uint32)";
            const SELECTOR: [u8; 4] = [241u8, 18u8, 126u8, 216u8];
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
                    <alloy::sol_types::sol_data::Uint<32> as alloy_sol_types::SolType>::tokenize(
                        &self.pos,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimValidatorExitCall> for UnderlyingRustTuple<'_> {
                fn from(value: claimValidatorExitCall) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for claimValidatorExitCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimValidatorExitReturn> for UnderlyingRustTuple<'_> {
                fn from(value: claimValidatorExitReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for claimValidatorExitReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for claimValidatorExitCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = claimValidatorExitReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<claimWithdrawalReturn> for UnderlyingRustTuple<'_> {
                fn from(value: claimWithdrawalReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for claimWithdrawalReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for claimWithdrawalCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = claimWithdrawalReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `clock()` and selector `0x91ddadf4`.
    ```solidity
    function clock() external view returns (uint48);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct clockCall {}
    ///Container type for the return parameters of the [`clock()`](clockCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct clockReturn {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U48,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<clockCall> for UnderlyingRustTuple<'_> {
                fn from(value: clockCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for clockCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<48>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U48,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<clockReturn> for UnderlyingRustTuple<'_> {
                fn from(value: clockReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for clockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for clockCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = clockReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<48>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "clock()";
            const SELECTOR: [u8; 4] = [145u8, 221u8, 173u8, 244u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `delegate(address,uint256)` and selector `0x026e402b`.
    ```solidity
    function delegate(address validator, uint256 amount) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegate_0Call {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`delegate(address,uint256)`](delegate_0Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegate_0Return {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegate_0Call> for UnderlyingRustTuple<'_> {
                fn from(value: delegate_0Call) -> Self {
                    (value.validator, value.amount)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegate_0Call {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegate_0Return> for UnderlyingRustTuple<'_> {
                fn from(value: delegate_0Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegate_0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegate_0Call {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegate_0Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `delegate(address)` and selector `0x5c19a95c`.
    ```solidity
    function delegate(address validator) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegate_1Call {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`delegate(address)`](delegate_1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegate_1Return {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegate_1Call> for UnderlyingRustTuple<'_> {
                fn from(value: delegate_1Call) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegate_1Call {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegate_1Return> for UnderlyingRustTuple<'_> {
                fn from(value: delegate_1Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegate_1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegate_1Call {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegate_1Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "delegate(address)";
            const SELECTOR: [u8; 4] = [92u8, 25u8, 169u8, 92u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `delegateBySig(address,uint256,uint256,uint8,bytes32,bytes32)` and selector `0xc3cda520`.
    ```solidity
    function delegateBySig(address delegatee, uint256 nonce, uint256 expiry, uint8 v, bytes32 r, bytes32 s) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegateBySigCall {
        #[allow(missing_docs)]
        pub delegatee: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub nonce: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub expiry: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub v: u8,
        #[allow(missing_docs)]
        pub r: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub s: alloy::sol_types::private::FixedBytes<32>,
    }
    ///Container type for the return parameters of the [`delegateBySig(address,uint256,uint256,uint8,bytes32,bytes32)`](delegateBySigCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegateBySigReturn {}
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
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::primitives::aliases::U256,
                u8,
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::FixedBytes<32>,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegateBySigCall> for UnderlyingRustTuple<'_> {
                fn from(value: delegateBySigCall) -> Self {
                    (
                        value.delegatee,
                        value.nonce,
                        value.expiry,
                        value.v,
                        value.r,
                        value.s,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegateBySigCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        delegatee: tuple.0,
                        nonce: tuple.1,
                        expiry: tuple.2,
                        v: tuple.3,
                        r: tuple.4,
                        s: tuple.5,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegateBySigReturn> for UnderlyingRustTuple<'_> {
                fn from(value: delegateBySigReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegateBySigReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegateBySigCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::FixedBytes<32>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegateBySigReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "delegateBySig(address,uint256,uint256,uint8,bytes32,bytes32)";
            const SELECTOR: [u8; 4] = [195u8, 205u8, 165u8, 32u8];
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
                        &self.delegatee,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.nonce),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.expiry),
                    <alloy::sol_types::sol_data::Uint<
                        8,
                    > as alloy_sol_types::SolType>::tokenize(&self.v),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.r),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.s),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `delegates(address)` and selector `0x587cde1e`.
    ```solidity
    function delegates(address account) external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegatesCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`delegates(address)`](delegatesCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegatesReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegatesCall> for UnderlyingRustTuple<'_> {
                fn from(value: delegatesCall) -> Self {
                    (value.account,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegatesCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { account: tuple.0 }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegatesReturn> for UnderlyingRustTuple<'_> {
                fn from(value: delegatesReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegatesReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegatesCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegatesReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "delegates(address)";
            const SELECTOR: [u8; 4] = [88u8, 124u8, 222u8, 30u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegationsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `delegatorValidator(address)` and selector `0xd2b750e5`.
    ```solidity
    function delegatorValidator(address delegator) external view returns (address validator);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegatorValidatorCall {
        #[allow(missing_docs)]
        pub delegator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`delegatorValidator(address)`](delegatorValidatorCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct delegatorValidatorReturn {
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
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegatorValidatorCall> for UnderlyingRustTuple<'_> {
                fn from(value: delegatorValidatorCall) -> Self {
                    (value.delegator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegatorValidatorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { delegator: tuple.0 }
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<delegatorValidatorReturn> for UnderlyingRustTuple<'_> {
                fn from(value: delegatorValidatorReturn) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for delegatorValidatorReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { validator: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for delegatorValidatorCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegatorValidatorReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "delegatorValidator(address)";
            const SELECTOR: [u8; 4] = [210u8, 183u8, 80u8, 229u8];
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
                        &self.delegator,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<deregisterValidatorCall> for UnderlyingRustTuple<'_> {
                fn from(value: deregisterValidatorCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for deregisterValidatorCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<deregisterValidatorReturn> for UnderlyingRustTuple<'_> {
                fn from(value: deregisterValidatorReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for deregisterValidatorReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for deregisterValidatorCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = deregisterValidatorReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `eip712Domain()` and selector `0x84b0196e`.
    ```solidity
    function eip712Domain() external view returns (bytes1 fields, string memory name, string memory version, uint256 chainId, address verifyingContract, bytes32 salt, uint256[] memory extensions);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct eip712DomainCall {}
    ///Container type for the return parameters of the [`eip712Domain()`](eip712DomainCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct eip712DomainReturn {
        #[allow(missing_docs)]
        pub fields: alloy::sol_types::private::FixedBytes<1>,
        #[allow(missing_docs)]
        pub name: alloy::sol_types::private::String,
        #[allow(missing_docs)]
        pub version: alloy::sol_types::private::String,
        #[allow(missing_docs)]
        pub chainId: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub verifyingContract: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub salt: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub extensions:
            alloy::sol_types::private::Vec<alloy::sol_types::private::primitives::aliases::U256>,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<eip712DomainCall> for UnderlyingRustTuple<'_> {
                fn from(value: eip712DomainCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for eip712DomainCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<1>,
                alloy::sol_types::sol_data::String,
                alloy::sol_types::sol_data::String,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<1>,
                alloy::sol_types::private::String,
                alloy::sol_types::private::String,
                alloy::sol_types::private::primitives::aliases::U256,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Vec<
                    alloy::sol_types::private::primitives::aliases::U256,
                >,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<eip712DomainReturn> for UnderlyingRustTuple<'_> {
                fn from(value: eip712DomainReturn) -> Self {
                    (
                        value.fields,
                        value.name,
                        value.version,
                        value.chainId,
                        value.verifyingContract,
                        value.salt,
                        value.extensions,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for eip712DomainReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        fields: tuple.0,
                        name: tuple.1,
                        version: tuple.2,
                        chainId: tuple.3,
                        verifyingContract: tuple.4,
                        salt: tuple.5,
                        extensions: tuple.6,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for eip712DomainCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = eip712DomainReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<1>,
                alloy::sol_types::sol_data::String,
                alloy::sol_types::sol_data::String,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Array<alloy::sol_types::sol_data::Uint<256>>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "eip712Domain()";
            const SELECTOR: [u8; 4] = [132u8, 176u8, 25u8, 110u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `exitEscrowPeriod()` and selector `0x9e9a8f31`.
    ```solidity
    function exitEscrowPeriod() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct exitEscrowPeriodCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<exitEscrowPeriodCall> for UnderlyingRustTuple<'_> {
                fn from(value: exitEscrowPeriodCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for exitEscrowPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<exitEscrowPeriodReturn> for UnderlyingRustTuple<'_> {
                fn from(value: exitEscrowPeriodReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for exitEscrowPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for exitEscrowPeriodCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = exitEscrowPeriodReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `getPastTotalSupply(uint256)` and selector `0x8e539e8c`.
    ```solidity
    function getPastTotalSupply(uint256 timepoint) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getPastTotalSupplyCall {
        #[allow(missing_docs)]
        pub timepoint: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`getPastTotalSupply(uint256)`](getPastTotalSupplyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getPastTotalSupplyReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getPastTotalSupplyCall> for UnderlyingRustTuple<'_> {
                fn from(value: getPastTotalSupplyCall) -> Self {
                    (value.timepoint,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getPastTotalSupplyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { timepoint: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getPastTotalSupplyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getPastTotalSupplyReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getPastTotalSupplyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getPastTotalSupplyCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getPastTotalSupplyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getPastTotalSupply(uint256)";
            const SELECTOR: [u8; 4] = [142u8, 83u8, 158u8, 140u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.timepoint,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `getPastVotes(address,uint256)` and selector `0x3a46b1a8`.
    ```solidity
    function getPastVotes(address account, uint256 timepoint) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getPastVotesCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub timepoint: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`getPastVotes(address,uint256)`](getPastVotesCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getPastVotesReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getPastVotesCall> for UnderlyingRustTuple<'_> {
                fn from(value: getPastVotesCall) -> Self {
                    (value.account, value.timepoint)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getPastVotesCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        account: tuple.0,
                        timepoint: tuple.1,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getPastVotesReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getPastVotesReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getPastVotesReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getPastVotesCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getPastVotesReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getPastVotes(address,uint256)";
            const SELECTOR: [u8; 4] = [58u8, 70u8, 177u8, 168u8];
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.timepoint,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `getVersion()` and selector `0x0d8e6e2c`.
    ```solidity
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVersionCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getVersionReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
                alloy::sol_types::sol_data::Uint<8>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `getVotes(address)` and selector `0x9ab24eb0`.
    ```solidity
    function getVotes(address account) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVotesCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`getVotes(address)`](getVotesCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVotesReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getVotesCall> for UnderlyingRustTuple<'_> {
                fn from(value: getVotesCall) -> Self {
                    (value.account,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getVotesCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { account: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<getVotesReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getVotesReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getVotesReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getVotesCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getVotesReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getVotes(address)";
            const SELECTOR: [u8; 4] = [154u8, 178u8, 78u8, 176u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `initialize(address,address,uint256,address)` and selector `0xbe203094`.
    ```solidity
    function initialize(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod, address _initialOwner) external;
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
        pub _initialOwner: alloy::sol_types::private::Address,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
                        value._initialOwner,
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
                        _initialOwner: tuple.3,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._exitEscrowPeriod,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._initialOwner,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `initializedAtBlock()` and selector `0x3e9df9b5`.
    ```solidity
    function initializedAtBlock() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializedAtBlockCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializedAtBlockCall> for UnderlyingRustTuple<'_> {
                fn from(value: initializedAtBlockCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializedAtBlockCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializedAtBlockReturn> for UnderlyingRustTuple<'_> {
                fn from(value: initializedAtBlockReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializedAtBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializedAtBlockCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializedAtBlockReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `lightClient()` and selector `0xb5700e68`.
    ```solidity
    function lightClient() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lightClientCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = lightClientReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `nonces(address)` and selector `0x7ecebe00`.
    ```solidity
    function nonces(address owner) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct noncesCall {
        #[allow(missing_docs)]
        pub owner: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`nonces(address)`](noncesCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct noncesReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<noncesCall> for UnderlyingRustTuple<'_> {
                fn from(value: noncesCall) -> Self {
                    (value.owner,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for noncesCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { owner: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<noncesReturn> for UnderlyingRustTuple<'_> {
                fn from(value: noncesReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for noncesReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for noncesCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = noncesReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "nonces(address)";
            const SELECTOR: [u8; 4] = [126u8, 206u8, 190u8, 0u8];
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
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `numCheckpoints(address)` and selector `0x6fcfff45`.
    ```solidity
    function numCheckpoints(address account) external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct numCheckpointsCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`numCheckpoints(address)`](numCheckpointsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct numCheckpointsReturn {
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Address,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::Address,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<numCheckpointsCall> for UnderlyingRustTuple<'_> {
                fn from(value: numCheckpointsCall) -> Self {
                    (value.account,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for numCheckpointsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { account: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<numCheckpointsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: numCheckpointsReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for numCheckpointsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for numCheckpointsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = numCheckpointsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "numCheckpoints(address)";
            const SELECTOR: [u8; 4] = [111u8, 207u8, 255u8, 69u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `owner()` and selector `0x8da5cb5b`.
    ```solidity
    function owner() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ownerCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = ownerReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `proxiableUUID()` and selector `0x52d1902d`.
    ```solidity
    function proxiableUUID() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct proxiableUUIDCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = proxiableUUIDReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::FixedBytes<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)` and selector `0x13b9057a`.
    ```solidity
    function registerValidator(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, uint16 commission) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorCall {
        #[allow(missing_docs)]
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub commission: u16,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorCall> for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorCall) -> Self {
                    (value.blsVK, value.schnorrVK, value.blsSig, value.commission)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for registerValidatorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blsVK: tuple.0,
                        schnorrVK: tuple.1,
                        blsSig: tuple.2,
                        commission: tuple.3,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<registerValidatorReturn> for UnderlyingRustTuple<'_> {
                fn from(value: registerValidatorReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for registerValidatorReturn {
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = registerValidatorReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.blsSig),
                    <alloy::sol_types::sol_data::Uint<16> as alloy_sol_types::SolType>::tokenize(
                        &self.commission,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceOwnershipCall> for UnderlyingRustTuple<'_> {
                fn from(value: renounceOwnershipCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceOwnershipCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<renounceOwnershipReturn> for UnderlyingRustTuple<'_> {
                fn from(value: renounceOwnershipReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for renounceOwnershipReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for renounceOwnershipCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = renounceOwnershipReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `token()` and selector `0xfc0c546a`.
    ```solidity
    function token() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct tokenCall {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = tokenReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<transferOwnershipCall> for UnderlyingRustTuple<'_> {
                fn from(value: transferOwnershipCall) -> Self {
                    (value.newOwner,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for transferOwnershipCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<transferOwnershipReturn> for UnderlyingRustTuple<'_> {
                fn from(value: transferOwnershipReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for transferOwnershipReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for transferOwnershipCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = transferOwnershipReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `undelegate(address,uint256)` and selector `0x4d99dd16`.
    ```solidity
    function undelegate(address validator, uint256 amount) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegate_0Call {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`undelegate(address,uint256)`](undelegate_0Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegate_0Return {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegate_0Call> for UnderlyingRustTuple<'_> {
                fn from(value: undelegate_0Call) -> Self {
                    (value.validator, value.amount)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegate_0Call {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegate_0Return> for UnderlyingRustTuple<'_> {
                fn from(value: undelegate_0Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegate_0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for undelegate_0Call {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegate_0Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `undelegate(address)` and selector `0xda8be864`.
    ```solidity
    function undelegate(address validator) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegate_1Call {
        #[allow(missing_docs)]
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`undelegate(address)`](undelegate_1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct undelegate_1Return {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegate_1Call> for UnderlyingRustTuple<'_> {
                fn from(value: undelegate_1Call) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegate_1Call {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<undelegate_1Return> for UnderlyingRustTuple<'_> {
                fn from(value: undelegate_1Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for undelegate_1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for undelegate_1Call {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegate_1Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "undelegate(address)";
            const SELECTOR: [u8; 4] = [218u8, 139u8, 232u8, 100u8];
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegationsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))` and selector `0x5544c2f1`.
    ```solidity
    function updateConsensusKeys(BN254.G2Point memory newBlsVK, EdOnBN254.EdOnBN254Point memory newSchnorrVK, BN254.G1Point memory newBlsSig) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateConsensusKeysCall {
        #[allow(missing_docs)]
        pub newBlsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub newSchnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub newBlsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
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
            type UnderlyingSolTuple<'a> =
                (BN254::G2Point, EdOnBN254::EdOnBN254Point, BN254::G1Point);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
                <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysCall> for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysCall) -> Self {
                    (value.newBlsVK, value.newSchnorrVK, value.newBlsSig)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateConsensusKeysCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newBlsVK: tuple.0,
                        newSchnorrVK: tuple.1,
                        newBlsSig: tuple.2,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateConsensusKeysReturn> for UnderlyingRustTuple<'_> {
                fn from(value: updateConsensusKeysReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateConsensusKeysReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateConsensusKeysCall {
            type Parameters<'a> = (BN254::G2Point, EdOnBN254::EdOnBN254Point, BN254::G1Point);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateConsensusKeysReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.newBlsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.newSchnorrVK,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.newBlsSig),
                )
            }
            #[inline]
            fn abi_decode_returns(
                data: &[u8],
                validate: bool,
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<upgradeToAndCallCall> for UnderlyingRustTuple<'_> {
                fn from(value: upgradeToAndCallCall) -> Self {
                    (value.newImplementation, value.data)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for upgradeToAndCallCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<upgradeToAndCallReturn> for UnderlyingRustTuple<'_> {
                fn from(value: upgradeToAndCallReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for upgradeToAndCallReturn {
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = upgradeToAndCallReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorExitsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: validatorExitsReturn) -> Self {
                    (value.unlocksAt,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorExitsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { unlocksAt: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for validatorExitsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = validatorExitsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    /**Function with signature `validators(address)` and selector `0xfa52c7d8`.
    ```solidity
    function validators(address account) external view returns (uint256 delegatedAmount, ValidatorStatus status);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsCall {
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`validators(address)`](validatorsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsReturn {
        #[allow(missing_docs)]
        pub delegatedAmount: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub status: <ValidatorStatus as alloy::sol_types::SolType>::RustType,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<256>, ValidatorStatus);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
                <ValidatorStatus as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = validatorsReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>, ValidatorStatus);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(
                    data, validate,
                )
                .map(Into::into)
            }
        }
    };
    ///Container for all the [`StakeTable`](self) function calls.
    pub enum StakeTableCalls {
        #[allow(missing_docs)]
        CLOCK_MODE(CLOCK_MODECall),
        #[allow(missing_docs)]
        UPGRADE_INTERFACE_VERSION(UPGRADE_INTERFACE_VERSIONCall),
        #[allow(missing_docs)]
        _hashBlsKey(_hashBlsKeyCall),
        #[allow(missing_docs)]
        blsKeys(blsKeysCall),
        #[allow(missing_docs)]
        checkpoints(checkpointsCall),
        #[allow(missing_docs)]
        claimValidatorExit(claimValidatorExitCall),
        #[allow(missing_docs)]
        claimWithdrawal(claimWithdrawalCall),
        #[allow(missing_docs)]
        clock(clockCall),
        #[allow(missing_docs)]
        delegate_0(delegate_0Call),
        #[allow(missing_docs)]
        delegate_1(delegate_1Call),
        #[allow(missing_docs)]
        delegateBySig(delegateBySigCall),
        #[allow(missing_docs)]
        delegates(delegatesCall),
        #[allow(missing_docs)]
        delegations(delegationsCall),
        #[allow(missing_docs)]
        delegatorValidator(delegatorValidatorCall),
        #[allow(missing_docs)]
        deregisterValidator(deregisterValidatorCall),
        #[allow(missing_docs)]
        eip712Domain(eip712DomainCall),
        #[allow(missing_docs)]
        exitEscrowPeriod(exitEscrowPeriodCall),
        #[allow(missing_docs)]
        getPastTotalSupply(getPastTotalSupplyCall),
        #[allow(missing_docs)]
        getPastVotes(getPastVotesCall),
        #[allow(missing_docs)]
        getVersion(getVersionCall),
        #[allow(missing_docs)]
        getVotes(getVotesCall),
        #[allow(missing_docs)]
        initialize(initializeCall),
        #[allow(missing_docs)]
        initializedAtBlock(initializedAtBlockCall),
        #[allow(missing_docs)]
        lightClient(lightClientCall),
        #[allow(missing_docs)]
        nonces(noncesCall),
        #[allow(missing_docs)]
        numCheckpoints(numCheckpointsCall),
        #[allow(missing_docs)]
        owner(ownerCall),
        #[allow(missing_docs)]
        proxiableUUID(proxiableUUIDCall),
        #[allow(missing_docs)]
        registerValidator(registerValidatorCall),
        #[allow(missing_docs)]
        renounceOwnership(renounceOwnershipCall),
        #[allow(missing_docs)]
        token(tokenCall),
        #[allow(missing_docs)]
        transferOwnership(transferOwnershipCall),
        #[allow(missing_docs)]
        undelegate_0(undelegate_0Call),
        #[allow(missing_docs)]
        undelegate_1(undelegate_1Call),
        #[allow(missing_docs)]
        undelegations(undelegationsCall),
        #[allow(missing_docs)]
        updateConsensusKeys(updateConsensusKeysCall),
        #[allow(missing_docs)]
        upgradeToAndCall(upgradeToAndCallCall),
        #[allow(missing_docs)]
        validatorExits(validatorExitsCall),
        #[allow(missing_docs)]
        validators(validatorsCall),
    }
    #[automatically_derived]
    impl StakeTableCalls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [2u8, 110u8, 64u8, 43u8],
            [13u8, 142u8, 110u8, 44u8],
            [19u8, 185u8, 5u8, 122u8],
            [33u8, 64u8, 254u8, 205u8],
            [58u8, 70u8, 177u8, 168u8],
            [62u8, 157u8, 249u8, 181u8],
            [75u8, 245u8, 215u8, 233u8],
            [77u8, 153u8, 221u8, 22u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [85u8, 68u8, 194u8, 241u8],
            [88u8, 124u8, 222u8, 30u8],
            [92u8, 25u8, 169u8, 92u8],
            [106u8, 145u8, 28u8, 207u8],
            [111u8, 207u8, 255u8, 69u8],
            [113u8, 80u8, 24u8, 166u8],
            [126u8, 206u8, 190u8, 0u8],
            [132u8, 176u8, 25u8, 110u8],
            [141u8, 165u8, 203u8, 91u8],
            [142u8, 83u8, 158u8, 140u8],
            [145u8, 221u8, 173u8, 244u8],
            [154u8, 178u8, 78u8, 176u8],
            [155u8, 48u8, 165u8, 230u8],
            [158u8, 154u8, 143u8, 49u8],
            [162u8, 215u8, 141u8, 213u8],
            [163u8, 6u8, 106u8, 171u8],
            [173u8, 60u8, 177u8, 204u8],
            [179u8, 230u8, 235u8, 213u8],
            [181u8, 112u8, 14u8, 104u8],
            [181u8, 236u8, 179u8, 68u8],
            [190u8, 32u8, 48u8, 148u8],
            [195u8, 205u8, 165u8, 32u8],
            [198u8, 72u8, 20u8, 221u8],
            [210u8, 183u8, 80u8, 229u8],
            [218u8, 139u8, 232u8, 100u8],
            [241u8, 18u8, 126u8, 216u8],
            [242u8, 253u8, 227u8, 139u8],
            [250u8, 82u8, 199u8, 216u8],
            [252u8, 12u8, 84u8, 106u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableCalls {
        const NAME: &'static str = "StakeTableCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 39usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::CLOCK_MODE(_) => <CLOCK_MODECall as alloy_sol_types::SolCall>::SELECTOR,
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::_hashBlsKey(_) => <_hashBlsKeyCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::blsKeys(_) => <blsKeysCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::checkpoints(_) => <checkpointsCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::claimValidatorExit(_) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::claimWithdrawal(_) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::clock(_) => <clockCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegate_0(_) => <delegate_0Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegate_1(_) => <delegate_1Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegateBySig(_) => <delegateBySigCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegates(_) => <delegatesCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegations(_) => <delegationsCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::delegatorValidator(_) => {
                    <delegatorValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::deregisterValidator(_) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::eip712Domain(_) => <eip712DomainCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::exitEscrowPeriod(_) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getPastTotalSupply(_) => {
                    <getPastTotalSupplyCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getPastVotes(_) => <getPastVotesCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getVersion(_) => <getVersionCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getVotes(_) => <getVotesCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initialize(_) => <initializeCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initializedAtBlock(_) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::lightClient(_) => <lightClientCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::nonces(_) => <noncesCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::numCheckpoints(_) => {
                    <numCheckpointsCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::owner(_) => <ownerCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::proxiableUUID(_) => <proxiableUUIDCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::registerValidator(_) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::renounceOwnership(_) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::token(_) => <tokenCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::transferOwnership(_) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::undelegate_0(_) => <undelegate_0Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::undelegate_1(_) => <undelegate_1Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::undelegations(_) => <undelegationsCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::updateConsensusKeys(_) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::upgradeToAndCall(_) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::validatorExits(_) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::validators(_) => <validatorsCall as alloy_sol_types::SolCall>::SELECTOR,
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<StakeTableCalls>] = &[
                {
                    fn delegate_0(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegate_0Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::delegate_0)
                    }
                    delegate_0
                },
                {
                    fn getVersion(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::getVersion)
                    }
                    getVersion
                },
                {
                    fn registerValidator(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <registerValidatorCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::registerValidator)
                    }
                    registerValidator
                },
                {
                    fn claimValidatorExit(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::claimValidatorExit)
                    }
                    claimValidatorExit
                },
                {
                    fn getPastVotes(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <getPastVotesCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::getPastVotes)
                    }
                    getPastVotes
                },
                {
                    fn initializedAtBlock(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::initializedAtBlock)
                    }
                    initializedAtBlock
                },
                {
                    fn CLOCK_MODE(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <CLOCK_MODECall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::CLOCK_MODE)
                    }
                    CLOCK_MODE
                },
                {
                    fn undelegate_0(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <undelegate_0Call as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::undelegate_0)
                    }
                    undelegate_0
                },
                {
                    fn upgradeToAndCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::upgradeToAndCall)
                    }
                    upgradeToAndCall
                },
                {
                    fn proxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::proxiableUUID)
                    }
                    proxiableUUID
                },
                {
                    fn updateConsensusKeys(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::updateConsensusKeys)
                    }
                    updateConsensusKeys
                },
                {
                    fn delegates(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegatesCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::delegates)
                    }
                    delegates
                },
                {
                    fn delegate_1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegate_1Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::delegate_1)
                    }
                    delegate_1
                },
                {
                    fn deregisterValidator(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::deregisterValidator)
                    }
                    deregisterValidator
                },
                {
                    fn numCheckpoints(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <numCheckpointsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::numCheckpoints)
                    }
                    numCheckpoints
                },
                {
                    fn renounceOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::renounceOwnership)
                    }
                    renounceOwnership
                },
                {
                    fn nonces(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <noncesCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::nonces)
                    }
                    nonces
                },
                {
                    fn eip712Domain(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <eip712DomainCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::eip712Domain)
                    }
                    eip712Domain
                },
                {
                    fn owner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::owner)
                    }
                    owner
                },
                {
                    fn getPastTotalSupply(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <getPastTotalSupplyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::getPastTotalSupply)
                    }
                    getPastTotalSupply
                },
                {
                    fn clock(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <clockCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::clock)
                    }
                    clock
                },
                {
                    fn getVotes(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <getVotesCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::getVotes)
                    }
                    getVotes
                },
                {
                    fn _hashBlsKey(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::_hashBlsKey)
                    }
                    _hashBlsKey
                },
                {
                    fn exitEscrowPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::exitEscrowPeriod)
                    }
                    exitEscrowPeriod
                },
                {
                    fn undelegations(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <undelegationsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::undelegations)
                    }
                    undelegations
                },
                {
                    fn claimWithdrawal(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::claimWithdrawal)
                    }
                    claimWithdrawal
                },
                {
                    fn UPGRADE_INTERFACE_VERSION(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::UPGRADE_INTERFACE_VERSION)
                    }
                    UPGRADE_INTERFACE_VERSION
                },
                {
                    fn blsKeys(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <blsKeysCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::blsKeys)
                    }
                    blsKeys
                },
                {
                    fn lightClient(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <lightClientCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::lightClient)
                    }
                    lightClient
                },
                {
                    fn validatorExits(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <validatorExitsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::validatorExits)
                    }
                    validatorExits
                },
                {
                    fn initialize(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::initialize)
                    }
                    initialize
                },
                {
                    fn delegateBySig(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegateBySigCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::delegateBySig)
                    }
                    delegateBySig
                },
                {
                    fn delegations(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegationsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::delegations)
                    }
                    delegations
                },
                {
                    fn delegatorValidator(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegatorValidatorCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::delegatorValidator)
                    }
                    delegatorValidator
                },
                {
                    fn undelegate_1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <undelegate_1Call as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::undelegate_1)
                    }
                    undelegate_1
                },
                {
                    fn checkpoints(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <checkpointsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::checkpoints)
                    }
                    checkpoints
                },
                {
                    fn transferOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::transferOwnership)
                    }
                    transferOwnership
                },
                {
                    fn validators(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <validatorsCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::validators)
                    }
                    validators
                },
                {
                    fn token(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <tokenCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::token)
                    }
                    token
                },
            ];
            let Ok(idx) = Self::SELECTORS.binary_search(&selector) else {
                return Err(alloy_sol_types::Error::unknown_selector(
                    <Self as alloy_sol_types::SolInterface>::NAME,
                    selector,
                ));
            };
            DECODE_SHIMS[idx](data, validate)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::CLOCK_MODE(inner) => {
                    <CLOCK_MODECall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::UPGRADE_INTERFACE_VERSION(inner) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                },
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::blsKeys(inner) => {
                    <blsKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::checkpoints(inner) => {
                    <checkpointsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::clock(inner) => {
                    <clockCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegate_0(inner) => {
                    <delegate_0Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegate_1(inner) => {
                    <delegate_1Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegateBySig(inner) => {
                    <delegateBySigCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegates(inner) => {
                    <delegatesCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegations(inner) => {
                    <delegationsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegatorValidator(inner) => {
                    <delegatorValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::eip712Domain(inner) => {
                    <eip712DomainCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getPastTotalSupply(inner) => {
                    <getPastTotalSupplyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getPastVotes(inner) => {
                    <getPastVotesCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getVotes(inner) => {
                    <getVotesCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::lightClient(inner) => {
                    <lightClientCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::nonces(inner) => {
                    <noncesCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::numCheckpoints(inner) => {
                    <numCheckpointsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::token(inner) => {
                    <tokenCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::undelegate_0(inner) => {
                    <undelegate_0Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::undelegate_1(inner) => {
                    <undelegate_1Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::undelegations(inner) => {
                    <undelegationsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::CLOCK_MODE(inner) => {
                    <CLOCK_MODECall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::UPGRADE_INTERFACE_VERSION(inner) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::blsKeys(inner) => {
                    <blsKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::checkpoints(inner) => {
                    <checkpointsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::clock(inner) => {
                    <clockCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegate_0(inner) => {
                    <delegate_0Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegate_1(inner) => {
                    <delegate_1Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegateBySig(inner) => {
                    <delegateBySigCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegates(inner) => {
                    <delegatesCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegations(inner) => {
                    <delegationsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegatorValidator(inner) => {
                    <delegatorValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::eip712Domain(inner) => {
                    <eip712DomainCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getPastTotalSupply(inner) => {
                    <getPastTotalSupplyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getPastVotes(inner) => {
                    <getPastVotesCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getVotes(inner) => {
                    <getVotesCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::lightClient(inner) => {
                    <lightClientCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::nonces(inner) => {
                    <noncesCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::numCheckpoints(inner) => {
                    <numCheckpointsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::token(inner) => {
                    <tokenCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::undelegate_0(inner) => {
                    <undelegate_0Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::undelegate_1(inner) => {
                    <undelegate_1Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::undelegations(inner) => {
                    <undelegationsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`StakeTable`](self) custom errors.
    pub enum StakeTableErrors {
        #[allow(missing_docs)]
        AddressEmptyCode(AddressEmptyCode),
        #[allow(missing_docs)]
        BLSSigVerificationFailed(BLSSigVerificationFailed),
        #[allow(missing_docs)]
        BlsKeyAlreadyUsed(BlsKeyAlreadyUsed),
        #[allow(missing_docs)]
        CheckpointUnorderedInsertion(CheckpointUnorderedInsertion),
        #[allow(missing_docs)]
        DelegatorAlreadyStaked(DelegatorAlreadyStaked),
        #[allow(missing_docs)]
        ECDSAInvalidSignature(ECDSAInvalidSignature),
        #[allow(missing_docs)]
        ECDSAInvalidSignatureLength(ECDSAInvalidSignatureLength),
        #[allow(missing_docs)]
        ECDSAInvalidSignatureS(ECDSAInvalidSignatureS),
        #[allow(missing_docs)]
        ERC1967InvalidImplementation(ERC1967InvalidImplementation),
        #[allow(missing_docs)]
        ERC1967NonPayable(ERC1967NonPayable),
        #[allow(missing_docs)]
        ERC5805FutureLookup(ERC5805FutureLookup),
        #[allow(missing_docs)]
        ERC6372InconsistentClock(ERC6372InconsistentClock),
        #[allow(missing_docs)]
        FailedInnerCall(FailedInnerCall),
        #[allow(missing_docs)]
        InsufficientAllowance(InsufficientAllowance),
        #[allow(missing_docs)]
        InsufficientBalance(InsufficientBalance),
        #[allow(missing_docs)]
        InvalidAccountNonce(InvalidAccountNonce),
        #[allow(missing_docs)]
        InvalidCommission(InvalidCommission),
        #[allow(missing_docs)]
        InvalidInitialization(InvalidInitialization),
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
        SafeCastOverflowedUintDowncast(SafeCastOverflowedUintDowncast),
        #[allow(missing_docs)]
        UUPSUnauthorizedCallContext(UUPSUnauthorizedCallContext),
        #[allow(missing_docs)]
        UUPSUnsupportedProxiableUUID(UUPSUnsupportedProxiableUUID),
        #[allow(missing_docs)]
        ValidatorAlreadyExited(ValidatorAlreadyExited),
        #[allow(missing_docs)]
        ValidatorAlreadyRegistered(ValidatorAlreadyRegistered),
        #[allow(missing_docs)]
        ValidatorCannotDelegate(ValidatorCannotDelegate),
        #[allow(missing_docs)]
        ValidatorInactive(ValidatorInactive),
        #[allow(missing_docs)]
        ValidatorNotExited(ValidatorNotExited),
        #[allow(missing_docs)]
        VotesExpiredSignature(VotesExpiredSignature),
        #[allow(missing_docs)]
        ZeroAddress(ZeroAddress),
    }
    #[automatically_derived]
    impl StakeTableErrors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 181u8, 20u8, 174u8],
            [6u8, 207u8, 67u8, 143u8],
            [12u8, 89u8, 93u8, 223u8],
            [12u8, 237u8, 62u8, 80u8],
            [17u8, 140u8, 218u8, 167u8],
            [20u8, 37u8, 234u8, 66u8],
            [30u8, 79u8, 189u8, 247u8],
            [37u8, 32u8, 96u8, 29u8],
            [42u8, 27u8, 45u8, 216u8],
            [70u8, 131u8, 175u8, 14u8],
            [76u8, 156u8, 140u8, 227u8],
            [80u8, 138u8, 121u8, 63u8],
            [90u8, 119u8, 67u8, 87u8],
            [109u8, 252u8, 198u8, 80u8],
            [111u8, 240u8, 113u8, 64u8],
            [117u8, 45u8, 136u8, 192u8],
            [136u8, 105u8, 163u8, 38u8],
            [146u8, 102u8, 83u8, 81u8],
            [153u8, 115u8, 247u8, 216u8],
            [153u8, 150u8, 179u8, 21u8],
            [170u8, 29u8, 73u8, 164u8],
            [179u8, 152u8, 151u8, 159u8],
            [208u8, 208u8, 79u8, 96u8],
            [215u8, 139u8, 206u8, 12u8],
            [215u8, 230u8, 188u8, 248u8],
            [217u8, 46u8, 35u8, 61u8],
            [220u8, 129u8, 219u8, 133u8],
            [224u8, 124u8, 141u8, 186u8],
            [234u8, 180u8, 169u8, 99u8],
            [236u8, 211u8, 248u8, 30u8],
            [242u8, 83u8, 20u8, 166u8],
            [246u8, 69u8, 238u8, 223u8],
            [249u8, 46u8, 232u8, 169u8],
            [252u8, 230u8, 152u8, 247u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableErrors {
        const NAME: &'static str = "StakeTableErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 34usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AddressEmptyCode(_) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::SELECTOR
                },
                Self::BLSSigVerificationFailed(_) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::SELECTOR
                },
                Self::BlsKeyAlreadyUsed(_) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::SELECTOR
                },
                Self::CheckpointUnorderedInsertion(_) => {
                    <CheckpointUnorderedInsertion as alloy_sol_types::SolError>::SELECTOR
                },
                Self::DelegatorAlreadyStaked(_) => {
                    <DelegatorAlreadyStaked as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ECDSAInvalidSignature(_) => {
                    <ECDSAInvalidSignature as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ECDSAInvalidSignatureLength(_) => {
                    <ECDSAInvalidSignatureLength as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ECDSAInvalidSignatureS(_) => {
                    <ECDSAInvalidSignatureS as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ERC1967InvalidImplementation(_) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ERC1967NonPayable(_) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ERC5805FutureLookup(_) => {
                    <ERC5805FutureLookup as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ERC6372InconsistentClock(_) => {
                    <ERC6372InconsistentClock as alloy_sol_types::SolError>::SELECTOR
                },
                Self::FailedInnerCall(_) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InsufficientAllowance(_) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InsufficientBalance(_) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidAccountNonce(_) => {
                    <InvalidAccountNonce as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidCommission(_) => {
                    <InvalidCommission as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidInitialization(_) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidSchnorrVK(_) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::SELECTOR
                },
                Self::NotInitializing(_) => {
                    <NotInitializing as alloy_sol_types::SolError>::SELECTOR
                },
                Self::NothingToWithdraw(_) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::SELECTOR
                },
                Self::OwnableInvalidOwner(_) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::SELECTOR
                },
                Self::OwnableUnauthorizedAccount(_) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                },
                Self::PrematureWithdrawal(_) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::SELECTOR
                },
                Self::SafeCastOverflowedUintDowncast(_) => {
                    <SafeCastOverflowedUintDowncast as alloy_sol_types::SolError>::SELECTOR
                },
                Self::UUPSUnauthorizedCallContext(_) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::SELECTOR
                },
                Self::UUPSUnsupportedProxiableUUID(_) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorAlreadyExited(_) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorAlreadyRegistered(_) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorCannotDelegate(_) => {
                    <ValidatorCannotDelegate as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorInactive(_) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorNotExited(_) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::SELECTOR
                },
                Self::VotesExpiredSignature(_) => {
                    <VotesExpiredSignature as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ZeroAddress(_) => <ZeroAddress as alloy_sol_types::SolError>::SELECTOR,
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<StakeTableErrors>] =
                &[
                    {
                        fn BlsKeyAlreadyUsed(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::BlsKeyAlreadyUsed)
                        }
                        BlsKeyAlreadyUsed
                    },
                    {
                        fn InvalidSchnorrVK(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InvalidSchnorrVK)
                        }
                        InvalidSchnorrVK
                    },
                    {
                        fn DelegatorAlreadyStaked(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <DelegatorAlreadyStaked as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::DelegatorAlreadyStaked)
                        }
                        DelegatorAlreadyStaked
                    },
                    {
                        fn BLSSigVerificationFailed(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::BLSSigVerificationFailed)
                        }
                        BLSSigVerificationFailed
                    },
                    {
                        fn OwnableUnauthorizedAccount(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::OwnableUnauthorizedAccount)
                        }
                        OwnableUnauthorizedAccount
                    },
                    {
                        fn FailedInnerCall(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::FailedInnerCall)
                        }
                        FailedInnerCall
                    },
                    {
                        fn OwnableInvalidOwner(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::OwnableInvalidOwner)
                        }
                        OwnableInvalidOwner
                    },
                    {
                        fn CheckpointUnorderedInsertion(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <CheckpointUnorderedInsertion as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::CheckpointUnorderedInsertion)
                        }
                        CheckpointUnorderedInsertion
                    },
                    {
                        fn InsufficientAllowance(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InsufficientAllowance as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InsufficientAllowance)
                        }
                        InsufficientAllowance
                    },
                    {
                        fn VotesExpiredSignature(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <VotesExpiredSignature as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::VotesExpiredSignature)
                        }
                        VotesExpiredSignature
                    },
                    {
                        fn ERC1967InvalidImplementation(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::ERC1967InvalidImplementation)
                        }
                        ERC1967InvalidImplementation
                    },
                    {
                        fn ValidatorInactive(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ValidatorInactive as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ValidatorInactive)
                        }
                        ValidatorInactive
                    },
                    {
                        fn PrematureWithdrawal(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <PrematureWithdrawal as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::PrematureWithdrawal)
                        }
                        PrematureWithdrawal
                    },
                    {
                        fn SafeCastOverflowedUintDowncast(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <SafeCastOverflowedUintDowncast as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::SafeCastOverflowedUintDowncast)
                        }
                        SafeCastOverflowedUintDowncast
                    },
                    {
                        fn ERC6372InconsistentClock(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ERC6372InconsistentClock as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ERC6372InconsistentClock)
                        }
                        ERC6372InconsistentClock
                    },
                    {
                        fn InvalidAccountNonce(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InvalidAccountNonce as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InvalidAccountNonce)
                        }
                        InvalidAccountNonce
                    },
                    {
                        fn ValidatorCannotDelegate(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ValidatorCannotDelegate as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ValidatorCannotDelegate)
                        }
                        ValidatorCannotDelegate
                    },
                    {
                        fn InsufficientBalance(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InsufficientBalance as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InsufficientBalance)
                        }
                        InsufficientBalance
                    },
                    {
                        fn ValidatorAlreadyRegistered(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::ValidatorAlreadyRegistered)
                        }
                        ValidatorAlreadyRegistered
                    },
                    {
                        fn AddressEmptyCode(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::AddressEmptyCode)
                        }
                        AddressEmptyCode
                    },
                    {
                        fn UUPSUnsupportedProxiableUUID(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::UUPSUnsupportedProxiableUUID)
                        }
                        UUPSUnsupportedProxiableUUID
                    },
                    {
                        fn ERC1967NonPayable(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ERC1967NonPayable)
                        }
                        ERC1967NonPayable
                    },
                    {
                        fn NothingToWithdraw(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <NothingToWithdraw as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::NothingToWithdraw)
                        }
                        NothingToWithdraw
                    },
                    {
                        fn ECDSAInvalidSignatureS(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ECDSAInvalidSignatureS as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ECDSAInvalidSignatureS)
                        }
                        ECDSAInvalidSignatureS
                    },
                    {
                        fn NotInitializing(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::NotInitializing)
                        }
                        NotInitializing
                    },
                    {
                        fn ZeroAddress(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ZeroAddress as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ZeroAddress)
                        }
                        ZeroAddress
                    },
                    {
                        fn InvalidCommission(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InvalidCommission as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InvalidCommission)
                        }
                        InvalidCommission
                    },
                    {
                        fn UUPSUnauthorizedCallContext(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::UUPSUnauthorizedCallContext)
                        }
                        UUPSUnauthorizedCallContext
                    },
                    {
                        fn ValidatorAlreadyExited(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ValidatorAlreadyExited)
                        }
                        ValidatorAlreadyExited
                    },
                    {
                        fn ERC5805FutureLookup(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ERC5805FutureLookup as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ERC5805FutureLookup)
                        }
                        ERC5805FutureLookup
                    },
                    {
                        fn ValidatorNotExited(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ValidatorNotExited as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ValidatorNotExited)
                        }
                        ValidatorNotExited
                    },
                    {
                        fn ECDSAInvalidSignature(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ECDSAInvalidSignature as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::ECDSAInvalidSignature)
                        }
                        ECDSAInvalidSignature
                    },
                    {
                        fn InvalidInitialization(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::InvalidInitialization)
                        }
                        InvalidInitialization
                    },
                    {
                        fn ECDSAInvalidSignatureLength(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <ECDSAInvalidSignatureLength as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(StakeTableErrors::ECDSAInvalidSignatureLength)
                        }
                        ECDSAInvalidSignatureLength
                    },
                ];
            let Ok(idx) = Self::SELECTORS.binary_search(&selector) else {
                return Err(alloy_sol_types::Error::unknown_selector(
                    <Self as alloy_sol_types::SolInterface>::NAME,
                    selector,
                ));
            };
            DECODE_SHIMS[idx](data, validate)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::BlsKeyAlreadyUsed(inner) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::CheckpointUnorderedInsertion(inner) => {
                    <CheckpointUnorderedInsertion as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::DelegatorAlreadyStaked(inner) => {
                    <DelegatorAlreadyStaked as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ECDSAInvalidSignature(inner) => {
                    <ECDSAInvalidSignature as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ECDSAInvalidSignatureLength(inner) => {
                    <ECDSAInvalidSignatureLength as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::ECDSAInvalidSignatureS(inner) => {
                    <ECDSAInvalidSignatureS as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ERC5805FutureLookup(inner) => {
                    <ERC5805FutureLookup as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ERC6372InconsistentClock(inner) => {
                    <ERC6372InconsistentClock as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InvalidAccountNonce(inner) => {
                    <InvalidAccountNonce as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::SafeCastOverflowedUintDowncast(inner) => {
                    <SafeCastOverflowedUintDowncast as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::UUPSUnauthorizedCallContext(inner) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::UUPSUnsupportedProxiableUUID(inner) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::ValidatorCannotDelegate(inner) => {
                    <ValidatorCannotDelegate as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::VotesExpiredSignature(inner) => {
                    <VotesExpiredSignature as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ZeroAddress(inner) => {
                    <ZeroAddress as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::BlsKeyAlreadyUsed(inner) => {
                    <BlsKeyAlreadyUsed as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::CheckpointUnorderedInsertion(inner) => {
                    <CheckpointUnorderedInsertion as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::DelegatorAlreadyStaked(inner) => {
                    <DelegatorAlreadyStaked as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ECDSAInvalidSignature(inner) => {
                    <ECDSAInvalidSignature as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ECDSAInvalidSignatureLength(inner) => {
                    <ECDSAInvalidSignatureLength as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ECDSAInvalidSignatureS(inner) => {
                    <ECDSAInvalidSignatureS as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ERC5805FutureLookup(inner) => {
                    <ERC5805FutureLookup as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ERC6372InconsistentClock(inner) => {
                    <ERC6372InconsistentClock as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InvalidAccountNonce(inner) => {
                    <InvalidAccountNonce as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::SafeCastOverflowedUintDowncast(inner) => {
                    <SafeCastOverflowedUintDowncast as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::UUPSUnauthorizedCallContext(inner) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::UUPSUnsupportedProxiableUUID(inner) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ValidatorCannotDelegate(inner) => {
                    <ValidatorCannotDelegate as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::VotesExpiredSignature(inner) => {
                    <VotesExpiredSignature as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ZeroAddress(inner) => {
                    <ZeroAddress as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`StakeTable`](self) events.
    pub enum StakeTableEvents {
        #[allow(missing_docs)]
        ConsensusKeysUpdated(ConsensusKeysUpdated),
        #[allow(missing_docs)]
        DelegateChanged(DelegateChanged),
        #[allow(missing_docs)]
        DelegateVotesChanged(DelegateVotesChanged),
        #[allow(missing_docs)]
        EIP712DomainChanged(EIP712DomainChanged),
        #[allow(missing_docs)]
        Initialized(Initialized),
        #[allow(missing_docs)]
        OwnershipTransferred(OwnershipTransferred),
        #[allow(missing_docs)]
        Staked(Staked),
        #[allow(missing_docs)]
        Undelegated(Undelegated),
        #[allow(missing_docs)]
        Upgrade(Upgrade),
        #[allow(missing_docs)]
        Upgraded(Upgraded),
        #[allow(missing_docs)]
        ValidatorExit(ValidatorExit),
        #[allow(missing_docs)]
        ValidatorRegistered(ValidatorRegistered),
        #[allow(missing_docs)]
        Withdrawal(Withdrawal),
    }
    #[automatically_derived]
    impl StakeTableEvents {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                10u8, 99u8, 135u8, 201u8, 234u8, 54u8, 40u8, 184u8, 138u8, 99u8, 59u8, 180u8,
                243u8, 177u8, 81u8, 119u8, 15u8, 112u8, 8u8, 81u8, 23u8, 161u8, 95u8, 155u8, 243u8,
                120u8, 124u8, 218u8, 83u8, 241u8, 61u8, 49u8,
            ],
            [
                49u8, 52u8, 232u8, 162u8, 230u8, 217u8, 126u8, 146u8, 154u8, 126u8, 84u8, 1u8,
                30u8, 165u8, 72u8, 93u8, 125u8, 25u8, 109u8, 213u8, 240u8, 186u8, 77u8, 78u8,
                249u8, 88u8, 3u8, 232u8, 227u8, 252u8, 37u8, 127u8,
            ],
            [
                77u8, 16u8, 189u8, 4u8, 151u8, 117u8, 199u8, 123u8, 215u8, 242u8, 85u8, 25u8, 90u8,
                251u8, 165u8, 8u8, 128u8, 40u8, 236u8, 179u8, 199u8, 194u8, 119u8, 211u8, 147u8,
                204u8, 255u8, 121u8, 52u8, 242u8, 249u8, 44u8,
            ],
            [
                93u8, 172u8, 12u8, 27u8, 17u8, 18u8, 86u8, 74u8, 4u8, 91u8, 169u8, 67u8, 201u8,
                213u8, 2u8, 112u8, 137u8, 62u8, 142u8, 130u8, 108u8, 73u8, 190u8, 142u8, 112u8,
                115u8, 173u8, 199u8, 19u8, 171u8, 123u8, 215u8,
            ],
            [
                127u8, 207u8, 83u8, 44u8, 21u8, 240u8, 166u8, 219u8, 11u8, 214u8, 208u8, 224u8,
                56u8, 190u8, 167u8, 29u8, 48u8, 216u8, 8u8, 199u8, 217u8, 140u8, 179u8, 191u8,
                114u8, 104u8, 169u8, 91u8, 245u8, 8u8, 27u8, 101u8,
            ],
            [
                128u8, 216u8, 164u8, 161u8, 102u8, 51u8, 40u8, 169u8, 152u8, 212u8, 85u8, 91u8,
                162u8, 29u8, 139u8, 186u8, 110u8, 241u8, 87u8, 106u8, 140u8, 94u8, 157u8, 39u8,
                249u8, 197u8, 69u8, 241u8, 163u8, 213u8, 43u8, 29u8,
            ],
            [
                139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8, 31u8, 208u8,
                164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8, 218u8, 175u8, 227u8,
                180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
            ],
            [
                188u8, 124u8, 215u8, 90u8, 32u8, 238u8, 39u8, 253u8, 154u8, 222u8, 186u8, 179u8,
                32u8, 65u8, 247u8, 85u8, 33u8, 77u8, 188u8, 107u8, 255u8, 169u8, 12u8, 192u8, 34u8,
                91u8, 57u8, 218u8, 46u8, 92u8, 45u8, 59u8,
            ],
            [
                199u8, 245u8, 5u8, 178u8, 243u8, 113u8, 174u8, 33u8, 117u8, 238u8, 73u8, 19u8,
                244u8, 73u8, 158u8, 31u8, 38u8, 51u8, 167u8, 181u8, 147u8, 99u8, 33u8, 238u8,
                209u8, 205u8, 174u8, 182u8, 17u8, 81u8, 129u8, 210u8,
            ],
            [
                222u8, 194u8, 186u8, 205u8, 210u8, 240u8, 91u8, 89u8, 222u8, 52u8, 218u8, 155u8,
                82u8, 61u8, 255u8, 139u8, 228u8, 46u8, 94u8, 56u8, 232u8, 24u8, 200u8, 47u8, 219u8,
                11u8, 174u8, 119u8, 67u8, 135u8, 167u8, 36u8,
            ],
            [
                246u8, 232u8, 53u8, 156u8, 87u8, 82u8, 11u8, 70u8, 150u8, 52u8, 115u8, 107u8,
                252u8, 59u8, 183u8, 236u8, 92u8, 189u8, 26u8, 11u8, 210u8, 139u8, 16u8, 168u8,
                39u8, 87u8, 147u8, 187u8, 115u8, 11u8, 121u8, 127u8,
            ],
            [
                247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8, 154u8,
                22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8, 185u8, 62u8, 237u8,
                168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
            ],
            [
                251u8, 36u8, 48u8, 83u8, 84u8, 200u8, 119u8, 98u8, 213u8, 87u8, 72u8, 122u8, 228u8,
                165u8, 100u8, 232u8, 208u8, 62u8, 203u8, 185u8, 169u8, 125u8, 216u8, 175u8, 255u8,
                142u8, 31u8, 111u8, 202u8, 240u8, 221u8, 22u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for StakeTableEvents {
        const NAME: &'static str = "StakeTableEvents";
        const COUNT: usize = 13usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<ConsensusKeysUpdated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ConsensusKeysUpdated as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::ConsensusKeysUpdated)
                },
                Some(<DelegateChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <DelegateChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::DelegateChanged)
                },
                Some(<DelegateVotesChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <DelegateVotesChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::DelegateVotesChanged)
                },
                Some(<EIP712DomainChanged as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <EIP712DomainChanged as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::EIP712DomainChanged)
                },
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Initialized)
                },
                Some(<OwnershipTransferred as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::OwnershipTransferred)
                },
                Some(<Staked as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Staked as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Staked)
                },
                Some(<Undelegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Undelegated as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Undelegated)
                },
                Some(<Upgrade as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgrade as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Upgrade)
                },
                Some(<Upgraded as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgraded as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Upgraded)
                },
                Some(<ValidatorExit as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ValidatorExit as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::ValidatorExit)
                },
                Some(<ValidatorRegistered as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ValidatorRegistered as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::ValidatorRegistered)
                },
                Some(<Withdrawal as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Withdrawal as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Withdrawal)
                },
                _ => alloy_sol_types::private::Err(alloy_sol_types::Error::InvalidLog {
                    name: <Self as alloy_sol_types::SolEventInterface>::NAME,
                    log: alloy_sol_types::private::Box::new(
                        alloy_sol_types::private::LogData::new_unchecked(
                            topics.to_vec(),
                            data.to_vec().into(),
                        ),
                    ),
                }),
            }
        }
    }
    #[automatically_derived]
    impl alloy_sol_types::private::IntoLogData for StakeTableEvents {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::ConsensusKeysUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::DelegateChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::DelegateVotesChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::EIP712DomainChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Staked(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Upgrade(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::Upgraded(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::ValidatorRegistered(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Withdrawal(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::ConsensusKeysUpdated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::DelegateChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::DelegateVotesChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::EIP712DomainChanged(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Staked(inner) => alloy_sol_types::private::IntoLogData::into_log_data(inner),
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Upgrade(inner) => alloy_sol_types::private::IntoLogData::into_log_data(inner),
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::ValidatorRegistered(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Withdrawal(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
            }
        }
    }
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
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<StakeTableInstance<T, P, N>>>
    {
        StakeTableInstance::<T, P, N>::deploy(provider)
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
    >(
        provider: P,
    ) -> alloy_contract::RawCallBuilder<T, P, N> {
        StakeTableInstance::<T, P, N>::deploy_builder(provider)
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
            f.debug_tuple("StakeTableInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > StakeTableInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`StakeTable`](self) contract instance.

        See the [wrapper's documentation](`StakeTableInstance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
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
        pub async fn deploy(provider: P) -> alloy_contract::Result<StakeTableInstance<T, P, N>> {
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
        > StakeTableInstance<T, P, N>
    {
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
        ///Creates a new call builder for the [`CLOCK_MODE`] function.
        pub fn CLOCK_MODE(&self) -> alloy_contract::SolCallBuilder<T, &P, CLOCK_MODECall, N> {
            self.call_builder(&CLOCK_MODECall {})
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
        ///Creates a new call builder for the [`checkpoints`] function.
        pub fn checkpoints(
            &self,
            account: alloy::sol_types::private::Address,
            pos: u32,
        ) -> alloy_contract::SolCallBuilder<T, &P, checkpointsCall, N> {
            self.call_builder(&checkpointsCall { account, pos })
        }
        ///Creates a new call builder for the [`claimValidatorExit`] function.
        pub fn claimValidatorExit(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, claimValidatorExitCall, N> {
            self.call_builder(&claimValidatorExitCall { validator })
        }
        ///Creates a new call builder for the [`claimWithdrawal`] function.
        pub fn claimWithdrawal(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, claimWithdrawalCall, N> {
            self.call_builder(&claimWithdrawalCall { validator })
        }
        ///Creates a new call builder for the [`clock`] function.
        pub fn clock(&self) -> alloy_contract::SolCallBuilder<T, &P, clockCall, N> {
            self.call_builder(&clockCall {})
        }
        ///Creates a new call builder for the [`delegate_0`] function.
        pub fn delegate_0(
            &self,
            validator: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegate_0Call, N> {
            self.call_builder(&delegate_0Call { validator, amount })
        }
        ///Creates a new call builder for the [`delegate_1`] function.
        pub fn delegate_1(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegate_1Call, N> {
            self.call_builder(&delegate_1Call { validator })
        }
        ///Creates a new call builder for the [`delegateBySig`] function.
        pub fn delegateBySig(
            &self,
            delegatee: alloy::sol_types::private::Address,
            nonce: alloy::sol_types::private::primitives::aliases::U256,
            expiry: alloy::sol_types::private::primitives::aliases::U256,
            v: u8,
            r: alloy::sol_types::private::FixedBytes<32>,
            s: alloy::sol_types::private::FixedBytes<32>,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegateBySigCall, N> {
            self.call_builder(&delegateBySigCall {
                delegatee,
                nonce,
                expiry,
                v,
                r,
                s,
            })
        }
        ///Creates a new call builder for the [`delegates`] function.
        pub fn delegates(
            &self,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegatesCall, N> {
            self.call_builder(&delegatesCall { account })
        }
        ///Creates a new call builder for the [`delegations`] function.
        pub fn delegations(
            &self,
            validator: alloy::sol_types::private::Address,
            delegator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegationsCall, N> {
            self.call_builder(&delegationsCall {
                validator,
                delegator,
            })
        }
        ///Creates a new call builder for the [`delegatorValidator`] function.
        pub fn delegatorValidator(
            &self,
            delegator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, delegatorValidatorCall, N> {
            self.call_builder(&delegatorValidatorCall { delegator })
        }
        ///Creates a new call builder for the [`deregisterValidator`] function.
        pub fn deregisterValidator(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, deregisterValidatorCall, N> {
            self.call_builder(&deregisterValidatorCall {})
        }
        ///Creates a new call builder for the [`eip712Domain`] function.
        pub fn eip712Domain(&self) -> alloy_contract::SolCallBuilder<T, &P, eip712DomainCall, N> {
            self.call_builder(&eip712DomainCall {})
        }
        ///Creates a new call builder for the [`exitEscrowPeriod`] function.
        pub fn exitEscrowPeriod(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, exitEscrowPeriodCall, N> {
            self.call_builder(&exitEscrowPeriodCall {})
        }
        ///Creates a new call builder for the [`getPastTotalSupply`] function.
        pub fn getPastTotalSupply(
            &self,
            timepoint: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, getPastTotalSupplyCall, N> {
            self.call_builder(&getPastTotalSupplyCall { timepoint })
        }
        ///Creates a new call builder for the [`getPastVotes`] function.
        pub fn getPastVotes(
            &self,
            account: alloy::sol_types::private::Address,
            timepoint: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, getPastVotesCall, N> {
            self.call_builder(&getPastVotesCall { account, timepoint })
        }
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(&self) -> alloy_contract::SolCallBuilder<T, &P, getVersionCall, N> {
            self.call_builder(&getVersionCall {})
        }
        ///Creates a new call builder for the [`getVotes`] function.
        pub fn getVotes(
            &self,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, getVotesCall, N> {
            self.call_builder(&getVotesCall { account })
        }
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(
            &self,
            _tokenAddress: alloy::sol_types::private::Address,
            _lightClientAddress: alloy::sol_types::private::Address,
            _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
            _initialOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeCall, N> {
            self.call_builder(&initializeCall {
                _tokenAddress,
                _lightClientAddress,
                _exitEscrowPeriod,
                _initialOwner,
            })
        }
        ///Creates a new call builder for the [`initializedAtBlock`] function.
        pub fn initializedAtBlock(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializedAtBlockCall, N> {
            self.call_builder(&initializedAtBlockCall {})
        }
        ///Creates a new call builder for the [`lightClient`] function.
        pub fn lightClient(&self) -> alloy_contract::SolCallBuilder<T, &P, lightClientCall, N> {
            self.call_builder(&lightClientCall {})
        }
        ///Creates a new call builder for the [`nonces`] function.
        pub fn nonces(
            &self,
            owner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, noncesCall, N> {
            self.call_builder(&noncesCall { owner })
        }
        ///Creates a new call builder for the [`numCheckpoints`] function.
        pub fn numCheckpoints(
            &self,
            account: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, numCheckpointsCall, N> {
            self.call_builder(&numCheckpointsCall { account })
        }
        ///Creates a new call builder for the [`owner`] function.
        pub fn owner(&self) -> alloy_contract::SolCallBuilder<T, &P, ownerCall, N> {
            self.call_builder(&ownerCall {})
        }
        ///Creates a new call builder for the [`proxiableUUID`] function.
        pub fn proxiableUUID(&self) -> alloy_contract::SolCallBuilder<T, &P, proxiableUUIDCall, N> {
            self.call_builder(&proxiableUUIDCall {})
        }
        ///Creates a new call builder for the [`registerValidator`] function.
        pub fn registerValidator(
            &self,
            blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            commission: u16,
        ) -> alloy_contract::SolCallBuilder<T, &P, registerValidatorCall, N> {
            self.call_builder(&registerValidatorCall {
                blsVK,
                schnorrVK,
                blsSig,
                commission,
            })
        }
        ///Creates a new call builder for the [`renounceOwnership`] function.
        pub fn renounceOwnership(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceOwnershipCall, N> {
            self.call_builder(&renounceOwnershipCall {})
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
        ///Creates a new call builder for the [`undelegate_0`] function.
        pub fn undelegate_0(
            &self,
            validator: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegate_0Call, N> {
            self.call_builder(&undelegate_0Call { validator, amount })
        }
        ///Creates a new call builder for the [`undelegate_1`] function.
        pub fn undelegate_1(
            &self,
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegate_1Call, N> {
            self.call_builder(&undelegate_1Call { validator })
        }
        ///Creates a new call builder for the [`undelegations`] function.
        pub fn undelegations(
            &self,
            validator: alloy::sol_types::private::Address,
            delegator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegationsCall, N> {
            self.call_builder(&undelegationsCall {
                validator,
                delegator,
            })
        }
        ///Creates a new call builder for the [`updateConsensusKeys`] function.
        pub fn updateConsensusKeys(
            &self,
            newBlsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            newSchnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
            newBlsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateConsensusKeysCall, N> {
            self.call_builder(&updateConsensusKeysCall {
                newBlsVK,
                newSchnorrVK,
                newBlsSig,
            })
        }
        ///Creates a new call builder for the [`upgradeToAndCall`] function.
        pub fn upgradeToAndCall(
            &self,
            newImplementation: alloy::sol_types::private::Address,
            data: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<T, &P, upgradeToAndCallCall, N> {
            self.call_builder(&upgradeToAndCallCall {
                newImplementation,
                data,
            })
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
        > StakeTableInstance<T, P, N>
    {
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
        ///Creates a new event filter for the [`DelegateChanged`] event.
        pub fn DelegateChanged_filter(&self) -> alloy_contract::Event<T, &P, DelegateChanged, N> {
            self.event_filter::<DelegateChanged>()
        }
        ///Creates a new event filter for the [`DelegateVotesChanged`] event.
        pub fn DelegateVotesChanged_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, DelegateVotesChanged, N> {
            self.event_filter::<DelegateVotesChanged>()
        }
        ///Creates a new event filter for the [`EIP712DomainChanged`] event.
        pub fn EIP712DomainChanged_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, EIP712DomainChanged, N> {
            self.event_filter::<EIP712DomainChanged>()
        }
        ///Creates a new event filter for the [`Initialized`] event.
        pub fn Initialized_filter(&self) -> alloy_contract::Event<T, &P, Initialized, N> {
            self.event_filter::<Initialized>()
        }
        ///Creates a new event filter for the [`OwnershipTransferred`] event.
        pub fn OwnershipTransferred_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, OwnershipTransferred, N> {
            self.event_filter::<OwnershipTransferred>()
        }
        ///Creates a new event filter for the [`Staked`] event.
        pub fn Staked_filter(&self) -> alloy_contract::Event<T, &P, Staked, N> {
            self.event_filter::<Staked>()
        }
        ///Creates a new event filter for the [`Undelegated`] event.
        pub fn Undelegated_filter(&self) -> alloy_contract::Event<T, &P, Undelegated, N> {
            self.event_filter::<Undelegated>()
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
        pub fn ValidatorExit_filter(&self) -> alloy_contract::Event<T, &P, ValidatorExit, N> {
            self.event_filter::<ValidatorExit>()
        }
        ///Creates a new event filter for the [`ValidatorRegistered`] event.
        pub fn ValidatorRegistered_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, ValidatorRegistered, N> {
            self.event_filter::<ValidatorRegistered>()
        }
        ///Creates a new event filter for the [`Withdrawal`] event.
        pub fn Withdrawal_filter(&self) -> alloy_contract::Event<T, &P, Withdrawal, N> {
            self.event_filter::<Withdrawal>()
        }
    }
}
