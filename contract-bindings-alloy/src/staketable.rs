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
    error ERC1967InvalidImplementation(address implementation);
    error ERC1967NonPayable();
    error FailedInnerCall();
    error InsufficientAllowance(uint256, uint256);
    error InsufficientBalance(uint256);
    error InvalidCommission();
    error InvalidInitialization();
    error InvalidSchnorrVK();
    error NotInitializing();
    error NothingToWithdraw();
    error OwnableInvalidOwner(address owner);
    error OwnableUnauthorizedAccount(address account);
    error PrematureWithdrawal();
    error UUPSUnauthorizedCallContext();
    error UUPSUnsupportedProxiableUUID(bytes32 slot);
    error ValidatorAlreadyExited();
    error ValidatorAlreadyRegistered();
    error ValidatorInactive();
    error ValidatorNotExited();
    error ZeroAddress();

    event ConsensusKeysUpdated(address indexed account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
    event Delegated(address indexed delegator, address indexed validator, uint256 amount);
    event Initialized(uint64 version);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event Undelegated(address indexed delegator, address indexed validator, uint256 amount);
    event Upgrade(address implementation);
    event Upgraded(address indexed implementation);
    event ValidatorExit(address indexed validator);
    event ValidatorRegistered(address indexed account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
    event Withdrawal(address indexed account, uint256 amount);

    constructor();

    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    function _hashBlsKey(BN254.G2Point memory blsVK) external pure returns (bytes32);
    function blsKeys(bytes32 blsKeyHash) external view returns (bool used);
    function claimValidatorExit(address validator) external;
    function claimWithdrawal(address validator) external;
    function delegate(address validator, uint256 amount) external;
    function deregisterValidator() external;
    function exitEscrowPeriod() external view returns (uint256);
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    function initialize(address _tokenAddress, address _lightClientAddress, uint256 _exitEscrowPeriod, address _initialOwner) external;
    function initializedAtBlock() external view returns (uint256);
    function lightClient() external view returns (address);
    function owner() external view returns (address);
    function proxiableUUID() external view returns (bytes32);
    function registerValidator(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, uint16 commission) external;
    function renounceOwnership() external;
    function token() external view returns (address);
    function transferOwnership(address newOwner) external;
    function undelegate(address validator, uint256 amount) external;
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
    ///0x60a06040523060805234801562000014575f80fd5b506200001f6200002f565b620000296200002f565b620000e3565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff1615620000805760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b0390811614620000e05780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b6080516129176200010a5f395f81816110ee01528181611117015261129a01526129175ff3fe60806040526004361061013c575f3560e01c80638da5cb5b116100b3578063b5700e681161006d578063b5700e681461039c578063b5ecb344146103bb578063be203094146103e6578063f2fde38b14610405578063fa52c7d814610424578063fc0c546a14610467575f80fd5b80638da5cb5b1461027e5780639b30a5e6146102ce5780639e9a8f31146102ed578063a3066aab14610302578063ad3cb1cc14610321578063b3e6ebd51461035e575f80fd5b80634d99dd16116101045780634d99dd16146101f15780634f1ef2861461021057806352d1902d146102235780635544c2f1146102375780636a911ccf14610256578063715018a61461026a575f80fd5b8063026e402b146101405780630d8e6e2c1461016157806313b9057a146101915780632140fecd146101b05780633e9df9b5146101cf575b5f80fd5b34801561014b575f80fd5b5061015f61015a3660046121fb565b610486565b005b34801561016c575f80fd5b5060408051600181525f60208201819052918101919091526060015b60405180910390f35b34801561019c575f80fd5b5061015f6101ab366004612319565b6105f2565b3480156101bb575f80fd5b5061015f6101ca366004612377565b610768565b3480156101da575f80fd5b506101e35f5481565b604051908152602001610188565b3480156101fc575f80fd5b5061015f61020b3660046121fb565b610876565b61015f61021e366004612390565b6109cd565b34801561022e575f80fd5b506101e36109ec565b348015610242575f80fd5b5061015f610251366004612430565b610a07565b348015610261575f80fd5b5061015f610ac5565b348015610275575f80fd5b5061015f610b33565b348015610289575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03165b6040516001600160a01b039091168152602001610188565b3480156102d9575f80fd5b506101e36102e8366004612474565b610b46565b3480156102f8575f80fd5b506101e360085481565b34801561030d575f80fd5b5061015f61031c366004612377565b610ba0565b34801561032c575f80fd5b50610351604051806040016040528060058152602001640352e302e360dc1b81525081565b60405161018891906124b0565b348015610369575f80fd5b5061038c6103783660046124e2565b60046020525f908152604090205460ff1681565b6040519015158152602001610188565b3480156103a7575f80fd5b506001546102b6906001600160a01b031681565b3480156103c6575f80fd5b506101e36103d5366004612377565b60056020525f908152604090205481565b3480156103f1575f80fd5b5061015f6104003660046124f9565b610c9f565b348015610410575f80fd5b5061015f61041f366004612377565b610dcb565b34801561042f575f80fd5b5061045961043e366004612377565b60036020525f90815260409020805460019091015460ff1682565b604051610188929190612557565b348015610472575f80fd5b506002546102b6906001600160a01b031681565b61048f82610e08565b600254604051636eb1769f60e11b81523360048201523060248201525f916001600160a01b03169063dd62ed3e90604401602060405180830381865afa1580156104db573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906104ff9190612587565b9050818110156105315760405163054365bb60e31b815260048101829052602481018390526044015b60405180910390fd5b6001600160a01b0383165f90815260036020526040812080548492906105589084906125b2565b90915550506001600160a01b0383165f9081526006602090815260408083203384529091528120805484929061058f9084906125b2565b90915550506002546105ac906001600160a01b0316333085610e57565b6040518281526001600160a01b0384169033907fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b906020015b60405180910390a3505050565b6105fb33610efb565b61060483610f48565b61060d84610f83565b604080513360208201525f91016040516020818303038152906040529050610636818487610fbf565b6127108261ffff16111561065d5760405163dc81db8560e01b815260040160405180910390fd5b600160045f61066b88610b46565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f8152602001600160028111156106b2576106b2612543565b9052335f908152600360209081526040909120825181559082015160018083018054909160ff19909116908360028111156106ef576106ef612543565b02179055505060408051875181526020808901518183015288830151828401526060808a0151908301528751608083015287015160a082015261ffff851660c082015290513392507ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9181900360e00190a25050505050565b6001600160a01b0381165f90815260056020526040812054908190036107a1576040516379298a5360e11b815260040160405180910390fd5b804210156107c257604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0382165f9081526006602090815260408083203384529091528120549081900361080657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832033808552925282209190915560025461083c92169083611054565b60405181815233907f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659060200160405180910390a2505050565b61087f82610e08565b6002335f9081526003602052604090206001015460ff1660028111156108a7576108a7612543565b036108c55760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b0382165f9081526006602090815260408083203384529091529020548181101561090c57604051639266535160e01b815260048101829052602401610528565b6001600160a01b0383165f9081526006602090815260408083203384529091528120805484929061093e9084906125c5565b9250508190555060405180604001604052808381526020016008544261096491906125b2565b90526001600160a01b0384165f818152600760209081526040808320338085529083529281902085518155948201516001909501949094559251858152919290917f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c91016105e5565b6109d56110e3565b6109de82611187565b6109e882826111ce565b5050565b5f6109f561128f565b505f805160206128eb83398151915290565b610a1033610e08565b610a1982610f48565b610a2283610f83565b604080513360208201525f91016040516020818303038152906040529050610a4b818386610fbf565b600160045f610a5987610b46565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550336001600160a01b03167f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d8585604051610ab79291906125d8565b60405180910390a250505050565b610ace33610e08565b335f908152600360205260409020600101805460ff19166002179055600854610af790426125b2565b335f8181526005602052604080822093909355915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a2565b610b3b6112d8565b610b445f611333565b565b5f815f0151826020015183604001518460600151604051602001610b83949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f90815260076020908152604080832033845290915281205490819003610be457604051630686827b60e51b815260040160405180910390fd5b6001600160a01b0382165f908152600760209081526040808320338452909152902060010154421015610c2a57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038083165f908152600760209081526040808320338085529252822082815560010191909155600254610c6692169083611054565b60405181815233907f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659060200160405180910390a25050565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f81158015610ce45750825b90505f8267ffffffffffffffff166001148015610d005750303b155b905081158015610d0e575080155b15610d2c5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610d5657845460ff60401b1916600160401b1785555b610d5f866113a3565b610d676113b4565b610d6f6113bc565b610d7a8989896114c2565b8315610dc057845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b610dd36112d8565b6001600160a01b038116610dfc57604051631e4fbdf760e01b81525f6004820152602401610528565b610e0581611333565b50565b60016001600160a01b0382165f9081526003602052604090206001015460ff166002811115610e3957610e39612543565b14610e055760405163508a793f60e01b815260040160405180910390fd5b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f511416151615610eb05750833b153d17155b80610ef45760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610528565b5050505050565b6001600160a01b0381165f9081526003602052604081206001015460ff166002811115610f2a57610f2a612543565b14610e055760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f8082526020820152610f658282611545565b156109e8576040516306cf438f60e01b815260040160405180910390fd5b60045f610f8f83610b46565b815260208101919091526040015f205460ff1615610e055760405162da8a5760e11b815260040160405180910390fd5b610fc882611568565b5f6040518060600160405280602481526020016128a76024913990505f8482604051602001610ff8929190612612565b60405160208183030381529060405290505f611013826115fe565b90506110308185611023886116eb565b61102b611762565b61182f565b61104c5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f51141615161561109e5750823b153d17155b806110dd5760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610528565b50505050565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061116957507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b031661115d5f805160206128eb833981519152546001600160a01b031690565b6001600160a01b031614155b15610b445760405163703e46dd60e11b815260040160405180910390fd5b61118f6112d8565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d9060200160405180910390a150565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611228575060408051601f3d908101601f1916820190925261122591810190612587565b60015b61125057604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610528565b5f805160206128eb833981519152811461128057604051632a87526960e21b815260048101829052602401610528565b61128a838361190d565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610b445760405163703e46dd60e11b815260040160405180910390fd5b3361130a7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610b445760405163118cdaa760e01b8152336004820152602401610528565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b6113ab611962565b610e05816119ab565b610b44611962565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156114015750825b90505f8267ffffffffffffffff16600114801561141d5750303b155b90508115801561142b575080155b156114495760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561147357845460ff60401b1916600160401b1785555b435f558315610ef457845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050505050565b6001600160a01b0383166114e95760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b0382166115105760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b039485166001600160a01b0319918216179091556001805493909416921691909117909155600855565b805182515f9114801561155f575081602001518360200151145b90505b92915050565b805160208201515f915f805160206128cb83398151915291159015161561158e57505050565b82516020840151826003848585860985090883828309148382108484101616935050508161128a5760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610528565b604080518082019091525f80825260208201525f61161b836119b3565b90505f805160206128cb83398151915260035f828485099050828061164257611642612640565b8482099050828061165557611655612640565b82820890505f8061166583611bbc565b925090505b806116ce57848061167d5761167d612640565b600187089550848061169157611691612640565b868709925084806116a4576116a4612640565b868409925084806116b7576116b7612640565b84840892506116c583611bbc565b9250905061166a565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f8082526020820152815160208301511590151615611712575090565b6040518060400160405280835f015181526020015f805160206128cb83398151915284602001516117439190612654565b61175a905f805160206128cb8339815191526125c5565b905292915050565b61178960405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806119015760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610528565b50151595945050505050565b61191682611cb3565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561195a5761128a8282611d16565b6109e8611d88565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff16610b4457604051631afcd79f60e31b815260040160405180910390fd5b610dd3611962565b5f806119be83611da7565b8051909150603081146119d3576119d3612673565b5f8167ffffffffffffffff8111156119ed576119ed612223565b6040519080825280601f01601f191660200182016040528015611a17576020820181803683370190505b5090505f5b82811015611a8657836001611a3183866125c5565b611a3b91906125c5565b81518110611a4b57611a4b612687565b602001015160f81c60f81b828281518110611a6857611a68612687565b60200101906001600160f81b03191690815f1a905350600101611a1c565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b82811015611b16578381611ac285886125c5565b611acc91906125b2565b81518110611adc57611adc612687565b602001015160f81c60f81b60f81c828281518110611afc57611afc612687565b60ff90921660209283029190910190910152600101611aae565b505f611b21826120f1565b90506101005f805160206128cb8339815191525f611b3f86896125c5565b90505f5b81811015611bac575f886001611b5984866125c5565b611b6391906125c5565b81518110611b7357611b73612687565b016020015160f81c90508380611b8b57611b8b612640565b85870995508380611b9e57611b9e612640565b818708955050600101611b43565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f805160206128cb833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f51925083611c795760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610528565b80600184901b1115611c9257611c8f83826125c5565b92505b8080611ca057611ca0612640565b8384099690961496919550909350505050565b806001600160a01b03163b5f03611ce857604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610528565b5f805160206128eb83398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b031684604051611d32919061269b565b5f60405180830381855af49150503d805f8114611d6a576040519150601f19603f3d011682016040523d82523d5f602084013e611d6f565b606091505b5091509150611d7f858383612158565b95945050505050565b3415610b445760405163b398979f60e01b815260040160405180910390fd5b604080516030808252606082810190935290602090600160f91b905f90846020820181803683370190505090508086604051602001611de7929190612612565b6040516020818303038152906040529050808460f81b604051602001611e0e9291906126b6565b604051602081830303815290604052905080604051602001611e3091906126e8565b60408051601f1981840301815290829052915061010160f01b90611e5a9083908390602001612708565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff811115611eca57611eca612223565b6040519080825280601f01601f191660200182016040528015611ef4576020820181803683370190505b5090505f82604051602001611f0b91815260200190565b60405160208183030381529060405290505f5b8151811015611f7457818181518110611f3957611f39612687565b602001015160f81c60f81b838281518110611f5657611f56612687565b60200101906001600160f81b03191690815f1a905350600101611f1e565b505f84604051602001611f8991815260200190565b60408051601f19818403018152602083019091525f80835291985091505b8981101561201b575f838281518110611fc257611fc2612687565b602001015160f81c60f81b838381518110611fdf57611fdf612687565b602001015160f81c60f81b1890508881604051602001612000929190612736565b60408051601f19818403018152919052985050600101611fa7565b5086888760405160200161203193929190612764565b6040516020818303038152906040529650868051906020012093508360405160200161205f91815260200190565b60405160208183030381529060405291505f5b61207f8a60ff8d166125c5565b8110156120e05782818151811061209857612098612687565b01602001516001600160f81b031916846120b2838d6125b2565b815181106120c2576120c2612687565b60200101906001600160f81b03191690815f1a905350600101612072565b50919b9a5050505050505050505050565b5f80805b83518110156121515783818151811061211057612110612687565b602002602001015160ff1681600861212891906127a4565b61213390600261289b565b61213d91906127a4565b61214790836125b2565b91506001016120f5565b5092915050565b60608261216d57612168826121b7565b6121b0565b815115801561218457506001600160a01b0384163b155b156121ad57604051639996b31560e01b81526001600160a01b0385166004820152602401610528565b50805b9392505050565b8051156121c75780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b03811681146121f6575f80fd5b919050565b5f806040838503121561220c575f80fd5b612215836121e0565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f1916810167ffffffffffffffff8111828210171561226057612260612223565b604052919050565b5f60808284031215612278575f80fd5b6040516080810181811067ffffffffffffffff8211171561229b5761229b612223565b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f604082840312156122dc575f80fd5b6040516040810181811067ffffffffffffffff821117156122ff576122ff612223565b604052823581526020928301359281019290925250919050565b5f805f80610120858703121561232d575f80fd5b6123378686612268565b935061234686608087016122cc565b92506123558660c087016122cc565b915061010085013561ffff8116811461236c575f80fd5b939692955090935050565b5f60208284031215612387575f80fd5b61155f826121e0565b5f80604083850312156123a1575f80fd5b6123aa836121e0565b915060208084013567ffffffffffffffff808211156123c7575f80fd5b818601915086601f8301126123da575f80fd5b8135818111156123ec576123ec612223565b6123fe601f8201601f19168501612237565b91508082528784828501011115612413575f80fd5b80848401858401375f848284010152508093505050509250929050565b5f805f6101008486031215612443575f80fd5b61244d8585612268565b925061245c85608086016122cc565b915061246b8560c086016122cc565b90509250925092565b5f60808284031215612484575f80fd5b61155f8383612268565b5f5b838110156124a8578181015183820152602001612490565b50505f910152565b602081525f82518060208401526124ce81604085016020870161248e565b601f01601f19169190910160400192915050565b5f602082840312156124f2575f80fd5b5035919050565b5f805f806080858703121561250c575f80fd5b612515856121e0565b9350612523602086016121e0565b925060408501359150612538606086016121e0565b905092959194509250565b634e487b7160e01b5f52602160045260245ffd5b828152604081016003831061257a57634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b5f60208284031215612597575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b808201808211156115625761156261259e565b818103818111156115625761156261259e565b825181526020808401518183015260408085015190830152606080850151908301528251608083015282015160a082015260c081016121b0565b5f835161262381846020880161248e565b83519083019061263781836020880161248e565b01949350505050565b634e487b7160e01b5f52601260045260245ffd5b5f8261266e57634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f82516126ac81846020870161248e565b9190910192915050565b5f83516126c781846020880161248e565b5f9201918252506001600160f81b0319919091166001820152600201919050565b5f82516126f981846020870161248e565b5f920191825250600101919050565b5f835161271981846020880161248e565b6001600160f01b0319939093169190920190815260020192915050565b5f835161274781846020880161248e565b6001600160f81b0319939093169190920190815260010192915050565b5f845161277581846020890161248e565b6001600160f81b031994909416919093019081526001600160f01b031991909116600182015260030192915050565b80820281158282048414176115625761156261259e565b600181815b808511156127f557815f19048211156127db576127db61259e565b808516156127e857918102915b93841c93908002906127c0565b509250929050565b5f8261280b57506001611562565b8161281757505f611562565b816001811461282d576002811461283757612853565b6001915050611562565b60ff8411156128485761284861259e565b50506001821b611562565b5060208310610133831016604e8410600b8410161715612876575081810a611562565b61288083836127bb565b805f19048211156128935761289361259e565b029392505050565b5f61155f83836127fd56fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15b\0\0\x14W_\x80\xFD[Pb\0\0\x1Fb\0\0/V[b\0\0)b\0\0/V[b\0\0\xE3V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15b\0\0\x80W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14b\0\0\xE0W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80Qa)\x17b\0\x01\n_9_\x81\x81a\x10\xEE\x01R\x81\x81a\x11\x17\x01Ra\x12\x9A\x01Ra)\x17_\xF3\xFE`\x80`@R`\x046\x10a\x01<W_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xB3W\x80c\xB5p\x0Eh\x11a\0mW\x80c\xB5p\x0Eh\x14a\x03\x9CW\x80c\xB5\xEC\xB3D\x14a\x03\xBBW\x80c\xBE 0\x94\x14a\x03\xE6W\x80c\xF2\xFD\xE3\x8B\x14a\x04\x05W\x80c\xFAR\xC7\xD8\x14a\x04$W\x80c\xFC\x0CTj\x14a\x04gW_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x02~W\x80c\x9B0\xA5\xE6\x14a\x02\xCEW\x80c\x9E\x9A\x8F1\x14a\x02\xEDW\x80c\xA3\x06j\xAB\x14a\x03\x02W\x80c\xAD<\xB1\xCC\x14a\x03!W\x80c\xB3\xE6\xEB\xD5\x14a\x03^W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\x01\x04W\x80cM\x99\xDD\x16\x14a\x01\xF1W\x80cO\x1E\xF2\x86\x14a\x02\x10W\x80cR\xD1\x90-\x14a\x02#W\x80cUD\xC2\xF1\x14a\x027W\x80cj\x91\x1C\xCF\x14a\x02VW\x80cqP\x18\xA6\x14a\x02jW_\x80\xFD[\x80c\x02n@+\x14a\x01@W\x80c\r\x8En,\x14a\x01aW\x80c\x13\xB9\x05z\x14a\x01\x91W\x80c!@\xFE\xCD\x14a\x01\xB0W\x80c>\x9D\xF9\xB5\x14a\x01\xCFW[_\x80\xFD[4\x80\x15a\x01KW_\x80\xFD[Pa\x01_a\x01Z6`\x04a!\xFBV[a\x04\x86V[\0[4\x80\x15a\x01lW_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x9CW_\x80\xFD[Pa\x01_a\x01\xAB6`\x04a#\x19V[a\x05\xF2V[4\x80\x15a\x01\xBBW_\x80\xFD[Pa\x01_a\x01\xCA6`\x04a#wV[a\x07hV[4\x80\x15a\x01\xDAW_\x80\xFD[Pa\x01\xE3_T\x81V[`@Q\x90\x81R` \x01a\x01\x88V[4\x80\x15a\x01\xFCW_\x80\xFD[Pa\x01_a\x02\x0B6`\x04a!\xFBV[a\x08vV[a\x01_a\x02\x1E6`\x04a#\x90V[a\t\xCDV[4\x80\x15a\x02.W_\x80\xFD[Pa\x01\xE3a\t\xECV[4\x80\x15a\x02BW_\x80\xFD[Pa\x01_a\x02Q6`\x04a$0V[a\n\x07V[4\x80\x15a\x02aW_\x80\xFD[Pa\x01_a\n\xC5V[4\x80\x15a\x02uW_\x80\xFD[Pa\x01_a\x0B3V[4\x80\x15a\x02\x89W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x88V[4\x80\x15a\x02\xD9W_\x80\xFD[Pa\x01\xE3a\x02\xE86`\x04a$tV[a\x0BFV[4\x80\x15a\x02\xF8W_\x80\xFD[Pa\x01\xE3`\x08T\x81V[4\x80\x15a\x03\rW_\x80\xFD[Pa\x01_a\x03\x1C6`\x04a#wV[a\x0B\xA0V[4\x80\x15a\x03,W_\x80\xFD[Pa\x03Q`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x01\x88\x91\x90a$\xB0V[4\x80\x15a\x03iW_\x80\xFD[Pa\x03\x8Ca\x03x6`\x04a$\xE2V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01a\x01\x88V[4\x80\x15a\x03\xA7W_\x80\xFD[P`\x01Ta\x02\xB6\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x03\xC6W_\x80\xFD[Pa\x01\xE3a\x03\xD56`\x04a#wV[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x03\xF1W_\x80\xFD[Pa\x01_a\x04\x006`\x04a$\xF9V[a\x0C\x9FV[4\x80\x15a\x04\x10W_\x80\xFD[Pa\x01_a\x04\x1F6`\x04a#wV[a\r\xCBV[4\x80\x15a\x04/W_\x80\xFD[Pa\x04Ya\x04>6`\x04a#wV[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x01\x88\x92\x91\x90a%WV[4\x80\x15a\x04rW_\x80\xFD[P`\x02Ta\x02\xB6\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x04\x8F\x82a\x0E\x08V[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\xDBW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x04\xFF\x91\x90a%\x87V[\x90P\x81\x81\x10\x15a\x051W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` R`@\x81 \x80T\x84\x92\x90a\x05X\x90\x84\x90a%\xB2V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x05\x8F\x90\x84\x90a%\xB2V[\x90\x91UPP`\x02Ta\x05\xAC\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0EWV[`@Q\x82\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16\x903\x90\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90` \x01[`@Q\x80\x91\x03\x90\xA3PPPV[a\x05\xFB3a\x0E\xFBV[a\x06\x04\x83a\x0FHV[a\x06\r\x84a\x0F\x83V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x066\x81\x84\x87a\x0F\xBFV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x06]W`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x06k\x88a\x0BFV[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\x06\xB2Wa\x06\xB2a%CV[\x90R3_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\x06\xEFWa\x06\xEFa%CV[\x02\x17\x90UPP`@\x80Q\x87Q\x81R` \x80\x89\x01Q\x81\x83\x01R\x88\x83\x01Q\x82\x84\x01R``\x80\x8A\x01Q\x90\x83\x01R\x87Q`\x80\x83\x01R\x87\x01Q`\xA0\x82\x01Ra\xFF\xFF\x85\x16`\xC0\x82\x01R\x90Q3\x92P\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x91\x81\x90\x03`\xE0\x01\x90\xA2PPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T\x90\x81\x90\x03a\x07\xA1W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x07\xC2W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x08\x06W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x02Ta\x08<\x92\x16\x90\x83a\x10TV[`@Q\x81\x81R3\x90\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x90` \x01`@Q\x80\x91\x03\x90\xA2PPPV[a\x08\x7F\x82a\x0E\x08V[`\x023_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x08\xA7Wa\x08\xA7a%CV[\x03a\x08\xC5W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\t\x0CW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x05(V[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\t>\x90\x84\x90a%\xC5V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x08TBa\td\x91\x90a%\xB2V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x92Q\x85\x81R\x91\x92\x90\x91\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x91\x01a\x05\xE5V[a\t\xD5a\x10\xE3V[a\t\xDE\x82a\x11\x87V[a\t\xE8\x82\x82a\x11\xCEV[PPV[_a\t\xF5a\x12\x8FV[P_\x80Q` a(\xEB\x839\x81Q\x91R\x90V[a\n\x103a\x0E\x08V[a\n\x19\x82a\x0FHV[a\n\"\x83a\x0F\x83V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\nK\x81\x83\x86a\x0F\xBFV[`\x01`\x04_a\nY\x87a\x0BFV[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D\x85\x85`@Qa\n\xB7\x92\x91\x90a%\xD8V[`@Q\x80\x91\x03\x90\xA2PPPPV[a\n\xCE3a\x0E\x08V[3_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\x08Ta\n\xF7\x90Ba%\xB2V[3_\x81\x81R`\x05` R`@\x80\x82 \x93\x90\x93U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2V[a\x0B;a\x12\xD8V[a\x0BD_a\x133V[V[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x0B\x83\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x0B\xE4W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\x0C*W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x02Ta\x0Cf\x92\x16\x90\x83a\x10TV[`@Q\x81\x81R3\x90\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x90` \x01`@Q\x80\x91\x03\x90\xA2PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x0C\xE4WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\r\0WP0;\x15[\x90P\x81\x15\x80\x15a\r\x0EWP\x80\x15[\x15a\r,W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\rVW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\r_\x86a\x13\xA3V[a\rga\x13\xB4V[a\roa\x13\xBCV[a\rz\x89\x89\x89a\x14\xC2V[\x83\x15a\r\xC0W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[a\r\xD3a\x12\xD8V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\r\xFCW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x05(V[a\x0E\x05\x81a\x133V[PV[`\x01`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x0E9Wa\x0E9a%CV[\x14a\x0E\x05W`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x0E\xB0WP\x83;\x15=\x17\x15[\x80a\x0E\xF4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x05(V[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x0F*Wa\x0F*a%CV[\x14a\x0E\x05W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0Fe\x82\x82a\x15EV[\x15a\t\xE8W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x0F\x8F\x83a\x0BFV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x0E\x05W`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0F\xC8\x82a\x15hV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a(\xA7`$\x919\x90P_\x84\x82`@Q` \x01a\x0F\xF8\x92\x91\x90a&\x12V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x10\x13\x82a\x15\xFEV[\x90Pa\x100\x81\x85a\x10#\x88a\x16\xEBV[a\x10+a\x17bV[a\x18/V[a\x10LW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x10\x9EWP\x82;\x15=\x17\x15[\x80a\x10\xDDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x05(V[PPPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x11iWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x11]_\x80Q` a(\xEB\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0BDW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11\x8Fa\x12\xD8V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01`@Q\x80\x91\x03\x90\xA1PV[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x12(WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x12%\x91\x81\x01\x90a%\x87V[`\x01[a\x12PW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x05(V[_\x80Q` a(\xEB\x839\x81Q\x91R\x81\x14a\x12\x80W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x05(V[a\x12\x8A\x83\x83a\x19\rV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0BDW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[3a\x13\n\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0BDW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x05(V[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x13\xABa\x19bV[a\x0E\x05\x81a\x19\xABV[a\x0BDa\x19bV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x14\x01WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x14\x1DWP0;\x15[\x90P\x81\x15\x80\x15a\x14+WP\x80\x15[\x15a\x14IW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x14sW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a\x0E\xF4W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x14\xE9W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x15\x10W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x94\x85\x16`\x01`\x01`\xA0\x1B\x03\x19\x91\x82\x16\x17\x90\x91U`\x01\x80T\x93\x90\x94\x16\x92\x16\x91\x90\x91\x17\x90\x91U`\x08UV[\x80Q\x82Q_\x91\x14\x80\x15a\x15_WP\x81` \x01Q\x83` \x01Q\x14[\x90P[\x92\x91PPV[\x80Q` \x82\x01Q_\x91_\x80Q` a(\xCB\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x15\x8EWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x12\x8AW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x05(V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x16\x1B\x83a\x19\xB3V[\x90P_\x80Q` a(\xCB\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x16BWa\x16Ba&@V[\x84\x82\t\x90P\x82\x80a\x16UWa\x16Ua&@V[\x82\x82\x08\x90P_\x80a\x16e\x83a\x1B\xBCV[\x92P\x90P[\x80a\x16\xCEW\x84\x80a\x16}Wa\x16}a&@V[`\x01\x87\x08\x95P\x84\x80a\x16\x91Wa\x16\x91a&@V[\x86\x87\t\x92P\x84\x80a\x16\xA4Wa\x16\xA4a&@V[\x86\x84\t\x92P\x84\x80a\x16\xB7Wa\x16\xB7a&@V[\x84\x84\x08\x92Pa\x16\xC5\x83a\x1B\xBCV[\x92P\x90Pa\x16jV[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x17\x12WP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a(\xCB\x839\x81Q\x91R\x84` \x01Qa\x17C\x91\x90a&TV[a\x17Z\x90_\x80Q` a(\xCB\x839\x81Q\x91Ra%\xC5V[\x90R\x92\x91PPV[a\x17\x89`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x19\x01W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x05(V[P\x15\x15\x95\x94PPPPPV[a\x19\x16\x82a\x1C\xB3V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a\x19ZWa\x12\x8A\x82\x82a\x1D\x16V[a\t\xE8a\x1D\x88V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x0BDW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\r\xD3a\x19bV[_\x80a\x19\xBE\x83a\x1D\xA7V[\x80Q\x90\x91P`0\x81\x14a\x19\xD3Wa\x19\xD3a&sV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x19\xEDWa\x19\xEDa\"#V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x1A\x17W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x1A\x86W\x83`\x01a\x1A1\x83\x86a%\xC5V[a\x1A;\x91\x90a%\xC5V[\x81Q\x81\x10a\x1AKWa\x1AKa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x1AhWa\x1Aha&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x1A\x1CV[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x1B\x16W\x83\x81a\x1A\xC2\x85\x88a%\xC5V[a\x1A\xCC\x91\x90a%\xB2V[\x81Q\x81\x10a\x1A\xDCWa\x1A\xDCa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x1A\xFCWa\x1A\xFCa&\x87V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x1A\xAEV[P_a\x1B!\x82a \xF1V[\x90Pa\x01\0_\x80Q` a(\xCB\x839\x81Q\x91R_a\x1B?\x86\x89a%\xC5V[\x90P_[\x81\x81\x10\x15a\x1B\xACW_\x88`\x01a\x1BY\x84\x86a%\xC5V[a\x1Bc\x91\x90a%\xC5V[\x81Q\x81\x10a\x1BsWa\x1Bsa&\x87V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x1B\x8BWa\x1B\x8Ba&@V[\x85\x87\t\x95P\x83\x80a\x1B\x9EWa\x1B\x9Ea&@V[\x81\x87\x08\x95PP`\x01\x01a\x1BCV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a(\xCB\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x1CyW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x05(V[\x80`\x01\x84\x90\x1B\x11\x15a\x1C\x92Wa\x1C\x8F\x83\x82a%\xC5V[\x92P[\x80\x80a\x1C\xA0Wa\x1C\xA0a&@V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a\x1C\xE8W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x05(V[_\x80Q` a(\xEB\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa\x1D2\x91\x90a&\x9BV[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a\x1DjW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x1DoV[``\x91P[P\x91P\x91Pa\x1D\x7F\x85\x83\x83a!XV[\x95\x94PPPPPV[4\x15a\x0BDW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x1D\xE7\x92\x91\x90a&\x12V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x1E\x0E\x92\x91\x90a&\xB6V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x1E0\x91\x90a&\xE8V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x1EZ\x90\x83\x90\x83\x90` \x01a'\x08V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x1E\xCAWa\x1E\xCAa\"#V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x1E\xF4W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x1F\x0B\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x1FtW\x81\x81\x81Q\x81\x10a\x1F9Wa\x1F9a&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x1FVWa\x1FVa&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x1F\x1EV[P_\x84`@Q` \x01a\x1F\x89\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a \x1BW_\x83\x82\x81Q\x81\x10a\x1F\xC2Wa\x1F\xC2a&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x1F\xDFWa\x1F\xDFa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a \0\x92\x91\x90a'6V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x1F\xA7V[P\x86\x88\x87`@Q` \x01a 1\x93\x92\x91\x90a'dV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a _\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a \x7F\x8A`\xFF\x8D\x16a%\xC5V[\x81\x10\x15a \xE0W\x82\x81\x81Q\x81\x10a \x98Wa \x98a&\x87V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a \xB2\x83\x8Da%\xB2V[\x81Q\x81\x10a \xC2Wa \xC2a&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a rV[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a!QW\x83\x81\x81Q\x81\x10a!\x10Wa!\x10a&\x87V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a!(\x91\x90a'\xA4V[a!3\x90`\x02a(\x9BV[a!=\x91\x90a'\xA4V[a!G\x90\x83a%\xB2V[\x91P`\x01\x01a \xF5V[P\x92\x91PPV[``\x82a!mWa!h\x82a!\xB7V[a!\xB0V[\x81Q\x15\x80\x15a!\x84WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a!\xADW`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x05(V[P\x80[\x93\x92PPPV[\x80Q\x15a!\xC7W\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a!\xF6W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\"\x0CW_\x80\xFD[a\"\x15\x83a!\xE0V[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"`Wa\"`a\"#V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\"xW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\x9BWa\"\x9Ba\"#V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\"\xDCW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\xFFWa\"\xFFa\"#V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a#-W_\x80\xFD[a#7\x86\x86a\"hV[\x93Pa#F\x86`\x80\x87\x01a\"\xCCV[\x92Pa#U\x86`\xC0\x87\x01a\"\xCCV[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a#lW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a#\x87W_\x80\xFD[a\x15_\x82a!\xE0V[_\x80`@\x83\x85\x03\x12\x15a#\xA1W_\x80\xFD[a#\xAA\x83a!\xE0V[\x91P` \x80\x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a#\xC7W_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a#\xDAW_\x80\xFD[\x815\x81\x81\x11\x15a#\xECWa#\xECa\"#V[a#\xFE`\x1F\x82\x01`\x1F\x19\x16\x85\x01a\"7V[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a$\x13W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[_\x80_a\x01\0\x84\x86\x03\x12\x15a$CW_\x80\xFD[a$M\x85\x85a\"hV[\x92Pa$\\\x85`\x80\x86\x01a\"\xCCV[\x91Pa$k\x85`\xC0\x86\x01a\"\xCCV[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a$\x84W_\x80\xFD[a\x15_\x83\x83a\"hV[_[\x83\x81\x10\x15a$\xA8W\x81\x81\x01Q\x83\x82\x01R` \x01a$\x90V[PP_\x91\x01RV[` \x81R_\x82Q\x80` \x84\x01Ra$\xCE\x81`@\x85\x01` \x87\x01a$\x8EV[`\x1F\x01`\x1F\x19\x16\x91\x90\x91\x01`@\x01\x92\x91PPV[_` \x82\x84\x03\x12\x15a$\xF2W_\x80\xFD[P5\x91\x90PV[_\x80_\x80`\x80\x85\x87\x03\x12\x15a%\x0CW_\x80\xFD[a%\x15\x85a!\xE0V[\x93Pa%#` \x86\x01a!\xE0V[\x92P`@\x85\x015\x91Pa%8``\x86\x01a!\xE0V[\x90P\x92\x95\x91\x94P\x92PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a%zWcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15a%\x97W_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x15bWa\x15ba%\x9EV[\x81\x81\x03\x81\x81\x11\x15a\x15bWa\x15ba%\x9EV[\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R`@\x80\x85\x01Q\x90\x83\x01R``\x80\x85\x01Q\x90\x83\x01R\x82Q`\x80\x83\x01R\x82\x01Q`\xA0\x82\x01R`\xC0\x81\x01a!\xB0V[_\x83Qa&#\x81\x84` \x88\x01a$\x8EV[\x83Q\x90\x83\x01\x90a&7\x81\x83` \x88\x01a$\x8EV[\x01\x94\x93PPPPV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a&nWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_\x82Qa&\xAC\x81\x84` \x87\x01a$\x8EV[\x91\x90\x91\x01\x92\x91PPV[_\x83Qa&\xC7\x81\x84` \x88\x01a$\x8EV[_\x92\x01\x91\x82RP`\x01`\x01`\xF8\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x02\x01\x91\x90PV[_\x82Qa&\xF9\x81\x84` \x87\x01a$\x8EV[_\x92\x01\x91\x82RP`\x01\x01\x91\x90PV[_\x83Qa'\x19\x81\x84` \x88\x01a$\x8EV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x02\x01\x92\x91PPV[_\x83Qa'G\x81\x84` \x88\x01a$\x8EV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x01\x01\x92\x91PPV[_\x84Qa'u\x81\x84` \x89\x01a$\x8EV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x91\x90\x93\x01\x90\x81R`\x01`\x01`\xF0\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x03\x01\x92\x91PPV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x15bWa\x15ba%\x9EV[`\x01\x81\x81[\x80\x85\x11\x15a'\xF5W\x81_\x19\x04\x82\x11\x15a'\xDBWa'\xDBa%\x9EV[\x80\x85\x16\x15a'\xE8W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a'\xC0V[P\x92P\x92\x90PV[_\x82a(\x0BWP`\x01a\x15bV[\x81a(\x17WP_a\x15bV[\x81`\x01\x81\x14a(-W`\x02\x81\x14a(7Wa(SV[`\x01\x91PPa\x15bV[`\xFF\x84\x11\x15a(HWa(Ha%\x9EV[PP`\x01\x82\x1Ba\x15bV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a(vWP\x81\x81\na\x15bV[a(\x80\x83\x83a'\xBBV[\x80_\x19\x04\x82\x11\x15a(\x93Wa(\x93a%\x9EV[\x02\x93\x92PPPV[_a\x15_\x83\x83a'\xFDV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x60806040526004361061013c575f3560e01c80638da5cb5b116100b3578063b5700e681161006d578063b5700e681461039c578063b5ecb344146103bb578063be203094146103e6578063f2fde38b14610405578063fa52c7d814610424578063fc0c546a14610467575f80fd5b80638da5cb5b1461027e5780639b30a5e6146102ce5780639e9a8f31146102ed578063a3066aab14610302578063ad3cb1cc14610321578063b3e6ebd51461035e575f80fd5b80634d99dd16116101045780634d99dd16146101f15780634f1ef2861461021057806352d1902d146102235780635544c2f1146102375780636a911ccf14610256578063715018a61461026a575f80fd5b8063026e402b146101405780630d8e6e2c1461016157806313b9057a146101915780632140fecd146101b05780633e9df9b5146101cf575b5f80fd5b34801561014b575f80fd5b5061015f61015a3660046121fb565b610486565b005b34801561016c575f80fd5b5060408051600181525f60208201819052918101919091526060015b60405180910390f35b34801561019c575f80fd5b5061015f6101ab366004612319565b6105f2565b3480156101bb575f80fd5b5061015f6101ca366004612377565b610768565b3480156101da575f80fd5b506101e35f5481565b604051908152602001610188565b3480156101fc575f80fd5b5061015f61020b3660046121fb565b610876565b61015f61021e366004612390565b6109cd565b34801561022e575f80fd5b506101e36109ec565b348015610242575f80fd5b5061015f610251366004612430565b610a07565b348015610261575f80fd5b5061015f610ac5565b348015610275575f80fd5b5061015f610b33565b348015610289575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03165b6040516001600160a01b039091168152602001610188565b3480156102d9575f80fd5b506101e36102e8366004612474565b610b46565b3480156102f8575f80fd5b506101e360085481565b34801561030d575f80fd5b5061015f61031c366004612377565b610ba0565b34801561032c575f80fd5b50610351604051806040016040528060058152602001640352e302e360dc1b81525081565b60405161018891906124b0565b348015610369575f80fd5b5061038c6103783660046124e2565b60046020525f908152604090205460ff1681565b6040519015158152602001610188565b3480156103a7575f80fd5b506001546102b6906001600160a01b031681565b3480156103c6575f80fd5b506101e36103d5366004612377565b60056020525f908152604090205481565b3480156103f1575f80fd5b5061015f6104003660046124f9565b610c9f565b348015610410575f80fd5b5061015f61041f366004612377565b610dcb565b34801561042f575f80fd5b5061045961043e366004612377565b60036020525f90815260409020805460019091015460ff1682565b604051610188929190612557565b348015610472575f80fd5b506002546102b6906001600160a01b031681565b61048f82610e08565b600254604051636eb1769f60e11b81523360048201523060248201525f916001600160a01b03169063dd62ed3e90604401602060405180830381865afa1580156104db573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906104ff9190612587565b9050818110156105315760405163054365bb60e31b815260048101829052602481018390526044015b60405180910390fd5b6001600160a01b0383165f90815260036020526040812080548492906105589084906125b2565b90915550506001600160a01b0383165f9081526006602090815260408083203384529091528120805484929061058f9084906125b2565b90915550506002546105ac906001600160a01b0316333085610e57565b6040518281526001600160a01b0384169033907fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b906020015b60405180910390a3505050565b6105fb33610efb565b61060483610f48565b61060d84610f83565b604080513360208201525f91016040516020818303038152906040529050610636818487610fbf565b6127108261ffff16111561065d5760405163dc81db8560e01b815260040160405180910390fd5b600160045f61066b88610b46565b81526020019081526020015f205f6101000a81548160ff02191690831515021790555060405180604001604052805f8152602001600160028111156106b2576106b2612543565b9052335f908152600360209081526040909120825181559082015160018083018054909160ff19909116908360028111156106ef576106ef612543565b02179055505060408051875181526020808901518183015288830151828401526060808a0151908301528751608083015287015160a082015261ffff851660c082015290513392507ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9181900360e00190a25050505050565b6001600160a01b0381165f90815260056020526040812054908190036107a1576040516379298a5360e11b815260040160405180910390fd5b804210156107c257604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0382165f9081526006602090815260408083203384529091528120549081900361080657604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f90815260066020908152604080832033808552925282209190915560025461083c92169083611054565b60405181815233907f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659060200160405180910390a2505050565b61087f82610e08565b6002335f9081526003602052604090206001015460ff1660028111156108a7576108a7612543565b036108c55760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b0382165f9081526006602090815260408083203384529091529020548181101561090c57604051639266535160e01b815260048101829052602401610528565b6001600160a01b0383165f9081526006602090815260408083203384529091528120805484929061093e9084906125c5565b9250508190555060405180604001604052808381526020016008544261096491906125b2565b90526001600160a01b0384165f818152600760209081526040808320338085529083529281902085518155948201516001909501949094559251858152919290917f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c91016105e5565b6109d56110e3565b6109de82611187565b6109e882826111ce565b5050565b5f6109f561128f565b505f805160206128eb83398151915290565b610a1033610e08565b610a1982610f48565b610a2283610f83565b604080513360208201525f91016040516020818303038152906040529050610a4b818386610fbf565b600160045f610a5987610b46565b81526020019081526020015f205f6101000a81548160ff021916908315150217905550336001600160a01b03167f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d8585604051610ab79291906125d8565b60405180910390a250505050565b610ace33610e08565b335f908152600360205260409020600101805460ff19166002179055600854610af790426125b2565b335f8181526005602052604080822093909355915190917ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd1691a2565b610b3b6112d8565b610b445f611333565b565b5f815f0151826020015183604001518460600151604051602001610b83949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f90815260076020908152604080832033845290915281205490819003610be457604051630686827b60e51b815260040160405180910390fd5b6001600160a01b0382165f908152600760209081526040808320338452909152902060010154421015610c2a57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b038083165f908152600760209081526040808320338085529252822082815560010191909155600254610c6692169083611054565b60405181815233907f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659060200160405180910390a25050565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f81158015610ce45750825b90505f8267ffffffffffffffff166001148015610d005750303b155b905081158015610d0e575080155b15610d2c5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610d5657845460ff60401b1916600160401b1785555b610d5f866113a3565b610d676113b4565b610d6f6113bc565b610d7a8989896114c2565b8315610dc057845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b610dd36112d8565b6001600160a01b038116610dfc57604051631e4fbdf760e01b81525f6004820152602401610528565b610e0581611333565b50565b60016001600160a01b0382165f9081526003602052604090206001015460ff166002811115610e3957610e39612543565b14610e055760405163508a793f60e01b815260040160405180910390fd5b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af191505080601f3d1160015f511416151615610eb05750833b153d17155b80610ef45760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610528565b5050505050565b6001600160a01b0381165f9081526003602052604081206001015460ff166002811115610f2a57610f2a612543565b14610e055760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f8082526020820152610f658282611545565b156109e8576040516306cf438f60e01b815260040160405180910390fd5b60045f610f8f83610b46565b815260208101919091526040015f205460ff1615610e055760405162da8a5760e11b815260040160405180910390fd5b610fc882611568565b5f6040518060600160405280602481526020016128a76024913990505f8482604051602001610ff8929190612612565b60405160208183030381529060405290505f611013826115fe565b90506110308185611023886116eb565b61102b611762565b61182f565b61104c5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af191505080601f3d1160015f51141615161561109e5750823b153d17155b806110dd5760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610528565b50505050565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061116957507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b031661115d5f805160206128eb833981519152546001600160a01b031690565b6001600160a01b031614155b15610b445760405163703e46dd60e11b815260040160405180910390fd5b61118f6112d8565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d9060200160405180910390a150565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611228575060408051601f3d908101601f1916820190925261122591810190612587565b60015b61125057604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610528565b5f805160206128eb833981519152811461128057604051632a87526960e21b815260048101829052602401610528565b61128a838361190d565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610b445760405163703e46dd60e11b815260040160405180910390fd5b3361130a7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610b445760405163118cdaa760e01b8152336004820152602401610528565b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b6113ab611962565b610e05816119ab565b610b44611962565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156114015750825b90505f8267ffffffffffffffff16600114801561141d5750303b155b90508115801561142b575080155b156114495760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561147357845460ff60401b1916600160401b1785555b435f558315610ef457845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050505050565b6001600160a01b0383166114e95760405163d92e233d60e01b815260040160405180910390fd5b6001600160a01b0382166115105760405163d92e233d60e01b815260040160405180910390fd5b600280546001600160a01b039485166001600160a01b0319918216179091556001805493909416921691909117909155600855565b805182515f9114801561155f575081602001518360200151145b90505b92915050565b805160208201515f915f805160206128cb83398151915291159015161561158e57505050565b82516020840151826003848585860985090883828309148382108484101616935050508161128a5760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610528565b604080518082019091525f80825260208201525f61161b836119b3565b90505f805160206128cb83398151915260035f828485099050828061164257611642612640565b8482099050828061165557611655612640565b82820890505f8061166583611bbc565b925090505b806116ce57848061167d5761167d612640565b600187089550848061169157611691612640565b868709925084806116a4576116a4612640565b868409925084806116b7576116b7612640565b84840892506116c583611bbc565b9250905061166a565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f8082526020820152815160208301511590151615611712575090565b6040518060400160405280835f015181526020015f805160206128cb83398151915284602001516117439190612654565b61175a905f805160206128cb8339815191526125c5565b905292915050565b61178960405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806119015760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610528565b50151595945050505050565b61191682611cb3565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561195a5761128a8282611d16565b6109e8611d88565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff16610b4457604051631afcd79f60e31b815260040160405180910390fd5b610dd3611962565b5f806119be83611da7565b8051909150603081146119d3576119d3612673565b5f8167ffffffffffffffff8111156119ed576119ed612223565b6040519080825280601f01601f191660200182016040528015611a17576020820181803683370190505b5090505f5b82811015611a8657836001611a3183866125c5565b611a3b91906125c5565b81518110611a4b57611a4b612687565b602001015160f81c60f81b828281518110611a6857611a68612687565b60200101906001600160f81b03191690815f1a905350600101611a1c565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b82811015611b16578381611ac285886125c5565b611acc91906125b2565b81518110611adc57611adc612687565b602001015160f81c60f81b60f81c828281518110611afc57611afc612687565b60ff90921660209283029190910190910152600101611aae565b505f611b21826120f1565b90506101005f805160206128cb8339815191525f611b3f86896125c5565b90505f5b81811015611bac575f886001611b5984866125c5565b611b6391906125c5565b81518110611b7357611b73612687565b016020015160f81c90508380611b8b57611b8b612640565b85870995508380611b9e57611b9e612640565b818708955050600101611b43565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f805160206128cb833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f51925083611c795760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610528565b80600184901b1115611c9257611c8f83826125c5565b92505b8080611ca057611ca0612640565b8384099690961496919550909350505050565b806001600160a01b03163b5f03611ce857604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610528565b5f805160206128eb83398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b031684604051611d32919061269b565b5f60405180830381855af49150503d805f8114611d6a576040519150601f19603f3d011682016040523d82523d5f602084013e611d6f565b606091505b5091509150611d7f858383612158565b95945050505050565b3415610b445760405163b398979f60e01b815260040160405180910390fd5b604080516030808252606082810190935290602090600160f91b905f90846020820181803683370190505090508086604051602001611de7929190612612565b6040516020818303038152906040529050808460f81b604051602001611e0e9291906126b6565b604051602081830303815290604052905080604051602001611e3091906126e8565b60408051601f1981840301815290829052915061010160f01b90611e5a9083908390602001612708565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff811115611eca57611eca612223565b6040519080825280601f01601f191660200182016040528015611ef4576020820181803683370190505b5090505f82604051602001611f0b91815260200190565b60405160208183030381529060405290505f5b8151811015611f7457818181518110611f3957611f39612687565b602001015160f81c60f81b838281518110611f5657611f56612687565b60200101906001600160f81b03191690815f1a905350600101611f1e565b505f84604051602001611f8991815260200190565b60408051601f19818403018152602083019091525f80835291985091505b8981101561201b575f838281518110611fc257611fc2612687565b602001015160f81c60f81b838381518110611fdf57611fdf612687565b602001015160f81c60f81b1890508881604051602001612000929190612736565b60408051601f19818403018152919052985050600101611fa7565b5086888760405160200161203193929190612764565b6040516020818303038152906040529650868051906020012093508360405160200161205f91815260200190565b60405160208183030381529060405291505f5b61207f8a60ff8d166125c5565b8110156120e05782818151811061209857612098612687565b01602001516001600160f81b031916846120b2838d6125b2565b815181106120c2576120c2612687565b60200101906001600160f81b03191690815f1a905350600101612072565b50919b9a5050505050505050505050565b5f80805b83518110156121515783818151811061211057612110612687565b602002602001015160ff1681600861212891906127a4565b61213390600261289b565b61213d91906127a4565b61214790836125b2565b91506001016120f5565b5092915050565b60608261216d57612168826121b7565b6121b0565b815115801561218457506001600160a01b0384163b155b156121ad57604051639996b31560e01b81526001600160a01b0385166004820152602401610528565b50805b9392505050565b8051156121c75780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b80356001600160a01b03811681146121f6575f80fd5b919050565b5f806040838503121561220c575f80fd5b612215836121e0565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b604051601f8201601f1916810167ffffffffffffffff8111828210171561226057612260612223565b604052919050565b5f60808284031215612278575f80fd5b6040516080810181811067ffffffffffffffff8211171561229b5761229b612223565b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f604082840312156122dc575f80fd5b6040516040810181811067ffffffffffffffff821117156122ff576122ff612223565b604052823581526020928301359281019290925250919050565b5f805f80610120858703121561232d575f80fd5b6123378686612268565b935061234686608087016122cc565b92506123558660c087016122cc565b915061010085013561ffff8116811461236c575f80fd5b939692955090935050565b5f60208284031215612387575f80fd5b61155f826121e0565b5f80604083850312156123a1575f80fd5b6123aa836121e0565b915060208084013567ffffffffffffffff808211156123c7575f80fd5b818601915086601f8301126123da575f80fd5b8135818111156123ec576123ec612223565b6123fe601f8201601f19168501612237565b91508082528784828501011115612413575f80fd5b80848401858401375f848284010152508093505050509250929050565b5f805f6101008486031215612443575f80fd5b61244d8585612268565b925061245c85608086016122cc565b915061246b8560c086016122cc565b90509250925092565b5f60808284031215612484575f80fd5b61155f8383612268565b5f5b838110156124a8578181015183820152602001612490565b50505f910152565b602081525f82518060208401526124ce81604085016020870161248e565b601f01601f19169190910160400192915050565b5f602082840312156124f2575f80fd5b5035919050565b5f805f806080858703121561250c575f80fd5b612515856121e0565b9350612523602086016121e0565b925060408501359150612538606086016121e0565b905092959194509250565b634e487b7160e01b5f52602160045260245ffd5b828152604081016003831061257a57634e487b7160e01b5f52602160045260245ffd5b8260208301529392505050565b5f60208284031215612597575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b808201808211156115625761156261259e565b818103818111156115625761156261259e565b825181526020808401518183015260408085015190830152606080850151908301528251608083015282015160a082015260c081016121b0565b5f835161262381846020880161248e565b83519083019061263781836020880161248e565b01949350505050565b634e487b7160e01b5f52601260045260245ffd5b5f8261266e57634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f82516126ac81846020870161248e565b9190910192915050565b5f83516126c781846020880161248e565b5f9201918252506001600160f81b0319919091166001820152600201919050565b5f82516126f981846020870161248e565b5f920191825250600101919050565b5f835161271981846020880161248e565b6001600160f01b0319939093169190920190815260020192915050565b5f835161274781846020880161248e565b6001600160f81b0319939093169190920190815260010192915050565b5f845161277581846020890161248e565b6001600160f81b031994909416919093019081526001600160f01b031991909116600182015260030192915050565b80820281158282048414176115625761156261259e565b600181815b808511156127f557815f19048211156127db576127db61259e565b808516156127e857918102915b93841c93908002906127c0565b509250929050565b5f8261280b57506001611562565b8161281757505f611562565b816001811461282d576002811461283757612853565b6001915050611562565b60ff8411156128485761284861259e565b50506001821b611562565b5060208310610133831016604e8410600b8410161715612876575081810a611562565b61288083836127bb565b805f19048211156128935761289361259e565b029392505050565b5f61155f83836127fd56fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x01<W_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xB3W\x80c\xB5p\x0Eh\x11a\0mW\x80c\xB5p\x0Eh\x14a\x03\x9CW\x80c\xB5\xEC\xB3D\x14a\x03\xBBW\x80c\xBE 0\x94\x14a\x03\xE6W\x80c\xF2\xFD\xE3\x8B\x14a\x04\x05W\x80c\xFAR\xC7\xD8\x14a\x04$W\x80c\xFC\x0CTj\x14a\x04gW_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x02~W\x80c\x9B0\xA5\xE6\x14a\x02\xCEW\x80c\x9E\x9A\x8F1\x14a\x02\xEDW\x80c\xA3\x06j\xAB\x14a\x03\x02W\x80c\xAD<\xB1\xCC\x14a\x03!W\x80c\xB3\xE6\xEB\xD5\x14a\x03^W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\x01\x04W\x80cM\x99\xDD\x16\x14a\x01\xF1W\x80cO\x1E\xF2\x86\x14a\x02\x10W\x80cR\xD1\x90-\x14a\x02#W\x80cUD\xC2\xF1\x14a\x027W\x80cj\x91\x1C\xCF\x14a\x02VW\x80cqP\x18\xA6\x14a\x02jW_\x80\xFD[\x80c\x02n@+\x14a\x01@W\x80c\r\x8En,\x14a\x01aW\x80c\x13\xB9\x05z\x14a\x01\x91W\x80c!@\xFE\xCD\x14a\x01\xB0W\x80c>\x9D\xF9\xB5\x14a\x01\xCFW[_\x80\xFD[4\x80\x15a\x01KW_\x80\xFD[Pa\x01_a\x01Z6`\x04a!\xFBV[a\x04\x86V[\0[4\x80\x15a\x01lW_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\x9CW_\x80\xFD[Pa\x01_a\x01\xAB6`\x04a#\x19V[a\x05\xF2V[4\x80\x15a\x01\xBBW_\x80\xFD[Pa\x01_a\x01\xCA6`\x04a#wV[a\x07hV[4\x80\x15a\x01\xDAW_\x80\xFD[Pa\x01\xE3_T\x81V[`@Q\x90\x81R` \x01a\x01\x88V[4\x80\x15a\x01\xFCW_\x80\xFD[Pa\x01_a\x02\x0B6`\x04a!\xFBV[a\x08vV[a\x01_a\x02\x1E6`\x04a#\x90V[a\t\xCDV[4\x80\x15a\x02.W_\x80\xFD[Pa\x01\xE3a\t\xECV[4\x80\x15a\x02BW_\x80\xFD[Pa\x01_a\x02Q6`\x04a$0V[a\n\x07V[4\x80\x15a\x02aW_\x80\xFD[Pa\x01_a\n\xC5V[4\x80\x15a\x02uW_\x80\xFD[Pa\x01_a\x0B3V[4\x80\x15a\x02\x89W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\x88V[4\x80\x15a\x02\xD9W_\x80\xFD[Pa\x01\xE3a\x02\xE86`\x04a$tV[a\x0BFV[4\x80\x15a\x02\xF8W_\x80\xFD[Pa\x01\xE3`\x08T\x81V[4\x80\x15a\x03\rW_\x80\xFD[Pa\x01_a\x03\x1C6`\x04a#wV[a\x0B\xA0V[4\x80\x15a\x03,W_\x80\xFD[Pa\x03Q`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x01\x88\x91\x90a$\xB0V[4\x80\x15a\x03iW_\x80\xFD[Pa\x03\x8Ca\x03x6`\x04a$\xE2V[`\x04` R_\x90\x81R`@\x90 T`\xFF\x16\x81V[`@Q\x90\x15\x15\x81R` \x01a\x01\x88V[4\x80\x15a\x03\xA7W_\x80\xFD[P`\x01Ta\x02\xB6\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[4\x80\x15a\x03\xC6W_\x80\xFD[Pa\x01\xE3a\x03\xD56`\x04a#wV[`\x05` R_\x90\x81R`@\x90 T\x81V[4\x80\x15a\x03\xF1W_\x80\xFD[Pa\x01_a\x04\x006`\x04a$\xF9V[a\x0C\x9FV[4\x80\x15a\x04\x10W_\x80\xFD[Pa\x01_a\x04\x1F6`\x04a#wV[a\r\xCBV[4\x80\x15a\x04/W_\x80\xFD[Pa\x04Ya\x04>6`\x04a#wV[`\x03` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x16\x82V[`@Qa\x01\x88\x92\x91\x90a%WV[4\x80\x15a\x04rW_\x80\xFD[P`\x02Ta\x02\xB6\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x04\x8F\x82a\x0E\x08V[`\x02T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x04\xDBW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x04\xFF\x91\x90a%\x87V[\x90P\x81\x81\x10\x15a\x051W`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x03` R`@\x81 \x80T\x84\x92\x90a\x05X\x90\x84\x90a%\xB2V[\x90\x91UPP`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x05\x8F\x90\x84\x90a%\xB2V[\x90\x91UPP`\x02Ta\x05\xAC\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0EWV[`@Q\x82\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16\x903\x90\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90` \x01[`@Q\x80\x91\x03\x90\xA3PPPV[a\x05\xFB3a\x0E\xFBV[a\x06\x04\x83a\x0FHV[a\x06\r\x84a\x0F\x83V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x066\x81\x84\x87a\x0F\xBFV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x06]W`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x04_a\x06k\x88a\x0BFV[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP`@Q\x80`@\x01`@R\x80_\x81R` \x01`\x01`\x02\x81\x11\x15a\x06\xB2Wa\x06\xB2a%CV[\x90R3_\x90\x81R`\x03` \x90\x81R`@\x90\x91 \x82Q\x81U\x90\x82\x01Q`\x01\x80\x83\x01\x80T\x90\x91`\xFF\x19\x90\x91\x16\x90\x83`\x02\x81\x11\x15a\x06\xEFWa\x06\xEFa%CV[\x02\x17\x90UPP`@\x80Q\x87Q\x81R` \x80\x89\x01Q\x81\x83\x01R\x88\x83\x01Q\x82\x84\x01R``\x80\x8A\x01Q\x90\x83\x01R\x87Q`\x80\x83\x01R\x87\x01Q`\xA0\x82\x01Ra\xFF\xFF\x85\x16`\xC0\x82\x01R\x90Q3\x92P\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x91\x81\x90\x03`\xE0\x01\x90\xA2PPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` R`@\x81 T\x90\x81\x90\x03a\x07\xA1W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x07\xC2W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x08\x06W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x02Ta\x08<\x92\x16\x90\x83a\x10TV[`@Q\x81\x81R3\x90\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x90` \x01`@Q\x80\x91\x03\x90\xA2PPPV[a\x08\x7F\x82a\x0E\x08V[`\x023_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x08\xA7Wa\x08\xA7a%CV[\x03a\x08\xC5W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\t\x0CW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x05(V[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x06` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\t>\x90\x84\x90a%\xC5V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x08TBa\td\x91\x90a%\xB2V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x92Q\x85\x81R\x91\x92\x90\x91\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x91\x01a\x05\xE5V[a\t\xD5a\x10\xE3V[a\t\xDE\x82a\x11\x87V[a\t\xE8\x82\x82a\x11\xCEV[PPV[_a\t\xF5a\x12\x8FV[P_\x80Q` a(\xEB\x839\x81Q\x91R\x90V[a\n\x103a\x0E\x08V[a\n\x19\x82a\x0FHV[a\n\"\x83a\x0F\x83V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\nK\x81\x83\x86a\x0F\xBFV[`\x01`\x04_a\nY\x87a\x0BFV[\x81R` \x01\x90\x81R` \x01_ _a\x01\0\n\x81T\x81`\xFF\x02\x19\x16\x90\x83\x15\x15\x02\x17\x90UP3`\x01`\x01`\xA0\x1B\x03\x16\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D\x85\x85`@Qa\n\xB7\x92\x91\x90a%\xD8V[`@Q\x80\x91\x03\x90\xA2PPPPV[a\n\xCE3a\x0E\x08V[3_\x90\x81R`\x03` R`@\x90 `\x01\x01\x80T`\xFF\x19\x16`\x02\x17\x90U`\x08Ta\n\xF7\x90Ba%\xB2V[3_\x81\x81R`\x05` R`@\x80\x82 \x93\x90\x93U\x91Q\x90\x91\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\xA2V[a\x0B;a\x12\xD8V[a\x0BD_a\x133V[V[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x0B\x83\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x0B\xE4W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\x0C*W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x07` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x02Ta\x0Cf\x92\x16\x90\x83a\x10TV[`@Q\x81\x81R3\x90\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x90` \x01`@Q\x80\x91\x03\x90\xA2PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x0C\xE4WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\r\0WP0;\x15[\x90P\x81\x15\x80\x15a\r\x0EWP\x80\x15[\x15a\r,W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\rVW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\r_\x86a\x13\xA3V[a\rga\x13\xB4V[a\roa\x13\xBCV[a\rz\x89\x89\x89a\x14\xC2V[\x83\x15a\r\xC0W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[a\r\xD3a\x12\xD8V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\r\xFCW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x05(V[a\x0E\x05\x81a\x133V[PV[`\x01`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x03` R`@\x90 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x0E9Wa\x0E9a%CV[\x14a\x0E\x05W`@QcP\x8Ay?`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x0E\xB0WP\x83;\x15=\x17\x15[\x80a\x0E\xF4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x05(V[PPPPPV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x81 `\x01\x01T`\xFF\x16`\x02\x81\x11\x15a\x0F*Wa\x0F*a%CV[\x14a\x0E\x05W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0Fe\x82\x82a\x15EV[\x15a\t\xE8W`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x04_a\x0F\x8F\x83a\x0BFV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x15a\x0E\x05W`@Qb\xDA\x8AW`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0F\xC8\x82a\x15hV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a(\xA7`$\x919\x90P_\x84\x82`@Q` \x01a\x0F\xF8\x92\x91\x90a&\x12V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\x10\x13\x82a\x15\xFEV[\x90Pa\x100\x81\x85a\x10#\x88a\x16\xEBV[a\x10+a\x17bV[a\x18/V[a\x10LW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1\x91PP\x80`\x1F=\x11`\x01_Q\x14\x16\x15\x16\x15a\x10\x9EWP\x82;\x15=\x17\x15[\x80a\x10\xDDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x05(V[PPPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x11iWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x11]_\x80Q` a(\xEB\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0BDW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11\x8Fa\x12\xD8V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01`@Q\x80\x91\x03\x90\xA1PV[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x12(WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x12%\x91\x81\x01\x90a%\x87V[`\x01[a\x12PW`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x05(V[_\x80Q` a(\xEB\x839\x81Q\x91R\x81\x14a\x12\x80W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x05(V[a\x12\x8A\x83\x83a\x19\rV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0BDW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[3a\x13\n\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0BDW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x05(V[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x13\xABa\x19bV[a\x0E\x05\x81a\x19\xABV[a\x0BDa\x19bV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x14\x01WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x14\x1DWP0;\x15[\x90P\x81\x15\x80\x15a\x14+WP\x80\x15[\x15a\x14IW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x14sW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C_U\x83\x15a\x0E\xF4W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPPV[`\x01`\x01`\xA0\x1B\x03\x83\x16a\x14\xE9W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16a\x15\x10W`@Qc\xD9.#=`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x02\x80T`\x01`\x01`\xA0\x1B\x03\x94\x85\x16`\x01`\x01`\xA0\x1B\x03\x19\x91\x82\x16\x17\x90\x91U`\x01\x80T\x93\x90\x94\x16\x92\x16\x91\x90\x91\x17\x90\x91U`\x08UV[\x80Q\x82Q_\x91\x14\x80\x15a\x15_WP\x81` \x01Q\x83` \x01Q\x14[\x90P[\x92\x91PPV[\x80Q` \x82\x01Q_\x91_\x80Q` a(\xCB\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x15\x8EWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x12\x8AW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x05(V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x16\x1B\x83a\x19\xB3V[\x90P_\x80Q` a(\xCB\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x16BWa\x16Ba&@V[\x84\x82\t\x90P\x82\x80a\x16UWa\x16Ua&@V[\x82\x82\x08\x90P_\x80a\x16e\x83a\x1B\xBCV[\x92P\x90P[\x80a\x16\xCEW\x84\x80a\x16}Wa\x16}a&@V[`\x01\x87\x08\x95P\x84\x80a\x16\x91Wa\x16\x91a&@V[\x86\x87\t\x92P\x84\x80a\x16\xA4Wa\x16\xA4a&@V[\x86\x84\t\x92P\x84\x80a\x16\xB7Wa\x16\xB7a&@V[\x84\x84\x08\x92Pa\x16\xC5\x83a\x1B\xBCV[\x92P\x90Pa\x16jV[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x17\x12WP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a(\xCB\x839\x81Q\x91R\x84` \x01Qa\x17C\x91\x90a&TV[a\x17Z\x90_\x80Q` a(\xCB\x839\x81Q\x91Ra%\xC5V[\x90R\x92\x91PPV[a\x17\x89`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x19\x01W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x05(V[P\x15\x15\x95\x94PPPPPV[a\x19\x16\x82a\x1C\xB3V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a\x19ZWa\x12\x8A\x82\x82a\x1D\x16V[a\t\xE8a\x1D\x88V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x0BDW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\r\xD3a\x19bV[_\x80a\x19\xBE\x83a\x1D\xA7V[\x80Q\x90\x91P`0\x81\x14a\x19\xD3Wa\x19\xD3a&sV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x19\xEDWa\x19\xEDa\"#V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x1A\x17W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x1A\x86W\x83`\x01a\x1A1\x83\x86a%\xC5V[a\x1A;\x91\x90a%\xC5V[\x81Q\x81\x10a\x1AKWa\x1AKa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x1AhWa\x1Aha&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x1A\x1CV[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x1B\x16W\x83\x81a\x1A\xC2\x85\x88a%\xC5V[a\x1A\xCC\x91\x90a%\xB2V[\x81Q\x81\x10a\x1A\xDCWa\x1A\xDCa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x1A\xFCWa\x1A\xFCa&\x87V[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x1A\xAEV[P_a\x1B!\x82a \xF1V[\x90Pa\x01\0_\x80Q` a(\xCB\x839\x81Q\x91R_a\x1B?\x86\x89a%\xC5V[\x90P_[\x81\x81\x10\x15a\x1B\xACW_\x88`\x01a\x1BY\x84\x86a%\xC5V[a\x1Bc\x91\x90a%\xC5V[\x81Q\x81\x10a\x1BsWa\x1Bsa&\x87V[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x1B\x8BWa\x1B\x8Ba&@V[\x85\x87\t\x95P\x83\x80a\x1B\x9EWa\x1B\x9Ea&@V[\x81\x87\x08\x95PP`\x01\x01a\x1BCV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a(\xCB\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x1CyW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x05(V[\x80`\x01\x84\x90\x1B\x11\x15a\x1C\x92Wa\x1C\x8F\x83\x82a%\xC5V[\x92P[\x80\x80a\x1C\xA0Wa\x1C\xA0a&@V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a\x1C\xE8W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x05(V[_\x80Q` a(\xEB\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa\x1D2\x91\x90a&\x9BV[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a\x1DjW`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x1DoV[``\x91P[P\x91P\x91Pa\x1D\x7F\x85\x83\x83a!XV[\x95\x94PPPPPV[4\x15a\x0BDW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x1D\xE7\x92\x91\x90a&\x12V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x1E\x0E\x92\x91\x90a&\xB6V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x1E0\x91\x90a&\xE8V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x1EZ\x90\x83\x90\x83\x90` \x01a'\x08V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x1E\xCAWa\x1E\xCAa\"#V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x1E\xF4W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x1F\x0B\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x1FtW\x81\x81\x81Q\x81\x10a\x1F9Wa\x1F9a&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x1FVWa\x1FVa&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x1F\x1EV[P_\x84`@Q` \x01a\x1F\x89\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a \x1BW_\x83\x82\x81Q\x81\x10a\x1F\xC2Wa\x1F\xC2a&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x1F\xDFWa\x1F\xDFa&\x87V[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a \0\x92\x91\x90a'6V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x1F\xA7V[P\x86\x88\x87`@Q` \x01a 1\x93\x92\x91\x90a'dV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a _\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a \x7F\x8A`\xFF\x8D\x16a%\xC5V[\x81\x10\x15a \xE0W\x82\x81\x81Q\x81\x10a \x98Wa \x98a&\x87V[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a \xB2\x83\x8Da%\xB2V[\x81Q\x81\x10a \xC2Wa \xC2a&\x87V[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a rV[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a!QW\x83\x81\x81Q\x81\x10a!\x10Wa!\x10a&\x87V[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a!(\x91\x90a'\xA4V[a!3\x90`\x02a(\x9BV[a!=\x91\x90a'\xA4V[a!G\x90\x83a%\xB2V[\x91P`\x01\x01a \xF5V[P\x92\x91PPV[``\x82a!mWa!h\x82a!\xB7V[a!\xB0V[\x81Q\x15\x80\x15a!\x84WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a!\xADW`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x05(V[P\x80[\x93\x92PPPV[\x80Q\x15a!\xC7W\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a!\xF6W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\"\x0CW_\x80\xFD[a\"\x15\x83a!\xE0V[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"`Wa\"`a\"#V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\"xW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\x9BWa\"\x9Ba\"#V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\"\xDCW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\xFFWa\"\xFFa\"#V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a#-W_\x80\xFD[a#7\x86\x86a\"hV[\x93Pa#F\x86`\x80\x87\x01a\"\xCCV[\x92Pa#U\x86`\xC0\x87\x01a\"\xCCV[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a#lW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a#\x87W_\x80\xFD[a\x15_\x82a!\xE0V[_\x80`@\x83\x85\x03\x12\x15a#\xA1W_\x80\xFD[a#\xAA\x83a!\xE0V[\x91P` \x80\x84\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a#\xC7W_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a#\xDAW_\x80\xFD[\x815\x81\x81\x11\x15a#\xECWa#\xECa\"#V[a#\xFE`\x1F\x82\x01`\x1F\x19\x16\x85\x01a\"7V[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a$\x13W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[_\x80_a\x01\0\x84\x86\x03\x12\x15a$CW_\x80\xFD[a$M\x85\x85a\"hV[\x92Pa$\\\x85`\x80\x86\x01a\"\xCCV[\x91Pa$k\x85`\xC0\x86\x01a\"\xCCV[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a$\x84W_\x80\xFD[a\x15_\x83\x83a\"hV[_[\x83\x81\x10\x15a$\xA8W\x81\x81\x01Q\x83\x82\x01R` \x01a$\x90V[PP_\x91\x01RV[` \x81R_\x82Q\x80` \x84\x01Ra$\xCE\x81`@\x85\x01` \x87\x01a$\x8EV[`\x1F\x01`\x1F\x19\x16\x91\x90\x91\x01`@\x01\x92\x91PPV[_` \x82\x84\x03\x12\x15a$\xF2W_\x80\xFD[P5\x91\x90PV[_\x80_\x80`\x80\x85\x87\x03\x12\x15a%\x0CW_\x80\xFD[a%\x15\x85a!\xE0V[\x93Pa%#` \x86\x01a!\xE0V[\x92P`@\x85\x015\x91Pa%8``\x86\x01a!\xE0V[\x90P\x92\x95\x91\x94P\x92PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82\x81R`@\x81\x01`\x03\x83\x10a%zWcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x82` \x83\x01R\x93\x92PPPV[_` \x82\x84\x03\x12\x15a%\x97W_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x15bWa\x15ba%\x9EV[\x81\x81\x03\x81\x81\x11\x15a\x15bWa\x15ba%\x9EV[\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R`@\x80\x85\x01Q\x90\x83\x01R``\x80\x85\x01Q\x90\x83\x01R\x82Q`\x80\x83\x01R\x82\x01Q`\xA0\x82\x01R`\xC0\x81\x01a!\xB0V[_\x83Qa&#\x81\x84` \x88\x01a$\x8EV[\x83Q\x90\x83\x01\x90a&7\x81\x83` \x88\x01a$\x8EV[\x01\x94\x93PPPPV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a&nWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_\x82Qa&\xAC\x81\x84` \x87\x01a$\x8EV[\x91\x90\x91\x01\x92\x91PPV[_\x83Qa&\xC7\x81\x84` \x88\x01a$\x8EV[_\x92\x01\x91\x82RP`\x01`\x01`\xF8\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x02\x01\x91\x90PV[_\x82Qa&\xF9\x81\x84` \x87\x01a$\x8EV[_\x92\x01\x91\x82RP`\x01\x01\x91\x90PV[_\x83Qa'\x19\x81\x84` \x88\x01a$\x8EV[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x02\x01\x92\x91PPV[_\x83Qa'G\x81\x84` \x88\x01a$\x8EV[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x91\x90\x92\x01\x90\x81R`\x01\x01\x92\x91PPV[_\x84Qa'u\x81\x84` \x89\x01a$\x8EV[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x91\x90\x93\x01\x90\x81R`\x01`\x01`\xF0\x1B\x03\x19\x91\x90\x91\x16`\x01\x82\x01R`\x03\x01\x92\x91PPV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x15bWa\x15ba%\x9EV[`\x01\x81\x81[\x80\x85\x11\x15a'\xF5W\x81_\x19\x04\x82\x11\x15a'\xDBWa'\xDBa%\x9EV[\x80\x85\x16\x15a'\xE8W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a'\xC0V[P\x92P\x92\x90PV[_\x82a(\x0BWP`\x01a\x15bV[\x81a(\x17WP_a\x15bV[\x81`\x01\x81\x14a(-W`\x02\x81\x14a(7Wa(SV[`\x01\x91PPa\x15bV[`\xFF\x84\x11\x15a(HWa(Ha%\x9EV[PP`\x01\x82\x1Ba\x15bV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a(vWP\x81\x81\na\x15bV[a(\x80\x83\x83a'\xBBV[\x80_\x19\x04\x82\x11\x15a(\x93Wa(\x93a%\x9EV[\x02\x93\x92PPPV[_a\x15_\x83\x83a'\xFDV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
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
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
            );
            const SIGNATURE: &'static str = "Delegated(address,address,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    229u8, 84u8, 26u8, 107u8, 97u8, 3u8, 212u8, 250u8, 126u8, 2u8, 30u8, 213u8,
                    79u8, 173u8, 57u8, 198u8, 111u8, 39u8, 167u8, 107u8, 209u8, 61u8, 55u8, 76u8,
                    246u8, 36u8, 10u8, 230u8, 189u8, 11u8, 183u8, 43u8,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = delegateReturn;
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
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
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = undelegateReturn;
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
        deregisterValidator(deregisterValidatorCall),
        #[allow(missing_docs)]
        exitEscrowPeriod(exitEscrowPeriodCall),
        #[allow(missing_docs)]
        getVersion(getVersionCall),
        #[allow(missing_docs)]
        initialize(initializeCall),
        #[allow(missing_docs)]
        initializedAtBlock(initializedAtBlockCall),
        #[allow(missing_docs)]
        lightClient(lightClientCall),
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
        undelegate(undelegateCall),
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
            [62u8, 157u8, 249u8, 181u8],
            [77u8, 153u8, 221u8, 22u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [85u8, 68u8, 194u8, 241u8],
            [106u8, 145u8, 28u8, 207u8],
            [113u8, 80u8, 24u8, 166u8],
            [141u8, 165u8, 203u8, 91u8],
            [155u8, 48u8, 165u8, 230u8],
            [158u8, 154u8, 143u8, 49u8],
            [163u8, 6u8, 106u8, 171u8],
            [173u8, 60u8, 177u8, 204u8],
            [179u8, 230u8, 235u8, 213u8],
            [181u8, 112u8, 14u8, 104u8],
            [181u8, 236u8, 179u8, 68u8],
            [190u8, 32u8, 48u8, 148u8],
            [242u8, 253u8, 227u8, 139u8],
            [250u8, 82u8, 199u8, 216u8],
            [252u8, 12u8, 84u8, 106u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableCalls {
        const NAME: &'static str = "StakeTableCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 23usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::_hashBlsKey(_) => <_hashBlsKeyCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::blsKeys(_) => <blsKeysCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::claimValidatorExit(_) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::claimWithdrawal(_) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::delegate(_) => <delegateCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::deregisterValidator(_) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::exitEscrowPeriod(_) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getVersion(_) => <getVersionCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initialize(_) => <initializeCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initializedAtBlock(_) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::lightClient(_) => <lightClientCall as alloy_sol_types::SolCall>::SELECTOR,
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
                Self::undelegate(_) => <undelegateCall as alloy_sol_types::SolCall>::SELECTOR,
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
                    fn delegate(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <delegateCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::delegate)
                    }
                    delegate
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
                    fn undelegate(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <undelegateCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::undelegate)
                    }
                    undelegate
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
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
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
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
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
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
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
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
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
        ERC1967InvalidImplementation(ERC1967InvalidImplementation),
        #[allow(missing_docs)]
        ERC1967NonPayable(ERC1967NonPayable),
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
        UUPSUnauthorizedCallContext(UUPSUnauthorizedCallContext),
        #[allow(missing_docs)]
        UUPSUnsupportedProxiableUUID(UUPSUnsupportedProxiableUUID),
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
            [12u8, 237u8, 62u8, 80u8],
            [17u8, 140u8, 218u8, 167u8],
            [20u8, 37u8, 234u8, 66u8],
            [30u8, 79u8, 189u8, 247u8],
            [42u8, 27u8, 45u8, 216u8],
            [76u8, 156u8, 140u8, 227u8],
            [80u8, 138u8, 121u8, 63u8],
            [90u8, 119u8, 67u8, 87u8],
            [146u8, 102u8, 83u8, 81u8],
            [153u8, 115u8, 247u8, 216u8],
            [153u8, 150u8, 179u8, 21u8],
            [170u8, 29u8, 73u8, 164u8],
            [179u8, 152u8, 151u8, 159u8],
            [208u8, 208u8, 79u8, 96u8],
            [215u8, 230u8, 188u8, 248u8],
            [217u8, 46u8, 35u8, 61u8],
            [220u8, 129u8, 219u8, 133u8],
            [224u8, 124u8, 141u8, 186u8],
            [234u8, 180u8, 169u8, 99u8],
            [242u8, 83u8, 20u8, 166u8],
            [249u8, 46u8, 232u8, 169u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableErrors {
        const NAME: &'static str = "StakeTableErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 23usize;
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
                Self::ERC1967InvalidImplementation(_) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ERC1967NonPayable(_) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::SELECTOR
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
                Self::ValidatorInactive(_) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ValidatorNotExited(_) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::SELECTOR
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
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                },
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encoded_size(inner)
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
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encoded_size(inner)
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
                Self::ERC1967InvalidImplementation(inner) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::ERC1967NonPayable(inner) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
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
                Self::ValidatorInactive(inner) => {
                    <ValidatorInactive as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
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
        Delegated(Delegated),
        #[allow(missing_docs)]
        Initialized(Initialized),
        #[allow(missing_docs)]
        OwnershipTransferred(OwnershipTransferred),
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
                77u8, 16u8, 189u8, 4u8, 151u8, 117u8, 199u8, 123u8, 215u8, 242u8, 85u8, 25u8, 90u8,
                251u8, 165u8, 8u8, 128u8, 40u8, 236u8, 179u8, 199u8, 194u8, 119u8, 211u8, 147u8,
                204u8, 255u8, 121u8, 52u8, 242u8, 249u8, 44u8,
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
                229u8, 84u8, 26u8, 107u8, 97u8, 3u8, 212u8, 250u8, 126u8, 2u8, 30u8, 213u8, 79u8,
                173u8, 57u8, 198u8, 111u8, 39u8, 167u8, 107u8, 209u8, 61u8, 55u8, 76u8, 246u8,
                36u8, 10u8, 230u8, 189u8, 11u8, 183u8, 43u8,
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
        const COUNT: usize = 10usize;
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
                Some(<Delegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Delegated as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Delegated)
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
                Self::Delegated(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
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
                Self::Delegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
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
            self.call_builder(&claimValidatorExitCall { validator })
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
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(&self) -> alloy_contract::SolCallBuilder<T, &P, getVersionCall, N> {
            self.call_builder(&getVersionCall {})
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
        ///Creates a new call builder for the [`undelegate`] function.
        pub fn undelegate(
            &self,
            validator: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, undelegateCall, N> {
            self.call_builder(&undelegateCall { validator, amount })
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
        ///Creates a new event filter for the [`Delegated`] event.
        pub fn Delegated_filter(&self) -> alloy_contract::Event<T, &P, Delegated, N> {
            self.event_filter::<Delegated>()
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
