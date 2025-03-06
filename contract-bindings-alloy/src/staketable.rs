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
        pub x: <BaseField as alloy::sol_types::SolType>::RustType,
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
        pub x0: <BaseField as alloy::sol_types::SolType>::RustType,
        pub x1: <BaseField as alloy::sol_types::SolType>::RustType,
        pub y0: <BaseField as alloy::sol_types::SolType>::RustType,
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
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /**```solidity
    struct EdOnBN254Point { uint256 x; uint256 y; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EdOnBN254Point {
        pub x: alloy::sol_types::private::primitives::aliases::U256,
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

    error BLSSigVerificationFailed();
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
    error UnknownValidator();
    error ValidatorAlreadyExited();
    error ValidatorAlreadyRegistered();
    error ValidatorNotExited();

    event ConsensusKeysUpdated(address account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
    event Delegated(address delegator, address validator, uint256 amount);
    event Initialized(uint64 version);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event Undelegated(address delegator, address validator, uint256 amount);
    event ValidatorExit(address validator);
    event ValidatorRegistered(address account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
    event Withdrawal(address account, uint256 amount);

    constructor(address _tokenAddress, uint256 _exitEscrowPeriod, address _initialOwner);

    function _hashBlsKey(BN254.G2Point memory blsVK) external pure returns (bytes32);
    function _isEqualBlsKey(BN254.G2Point memory a, BN254.G2Point memory b) external pure returns (bool);
    function admin() external view returns (address);
    function claimValidatorExit(address validator) external;
    function claimWithdrawal(address validator) external;
    function delegate(address validator, uint256 amount) external;
    function deregisterValidator() external;
    function exitEscrowPeriod() external view returns (uint256);
    function initialize() external;
    function initializedAtBlock() external view returns (uint256);
    function owner() external view returns (address);
    function registerValidator(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, uint16 commission) external;
    function renounceOwnership() external;
    function tokenAddress() external view returns (address);
    function transferOwnership(address newOwner) external;
    function undelegate(address validator, uint256 amount) external;
    function updateConsensusKeys(BN254.G2Point memory newBlsVK, EdOnBN254.EdOnBN254Point memory newSchnorrVK, BN254.G1Point memory newBlsSig) external;
    function validatorExits(address validator) external view returns (uint256 unlocksAt);
    function validators(address validator) external view returns (bool isRegistered, ValidatorStatus status, uint256 delegatedAmount);
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "constructor",
    "inputs": [
      {
        "name": "_tokenAddress",
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
    "stateMutability": "nonpayable"
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
    "name": "_isEqualBlsKey",
    "inputs": [
      {
        "name": "a",
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
        "name": "b",
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
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "admin",
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
    "name": "initialize",
    "inputs": [],
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
    "name": "tokenAddress",
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
        "name": "validator",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [
      {
        "name": "isRegistered",
        "type": "bool",
        "internalType": "bool"
      },
      {
        "name": "status",
        "type": "uint8",
        "internalType": "enum StakeTable.ValidatorStatus"
      },
      {
        "name": "delegatedAmount",
        "type": "uint256",
        "internalType": "uint256"
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
        "indexed": false,
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
        "indexed": false,
        "internalType": "address"
      },
      {
        "name": "validator",
        "type": "address",
        "indexed": false,
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
        "indexed": false,
        "internalType": "address"
      },
      {
        "name": "validator",
        "type": "address",
        "indexed": false,
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
    "name": "ValidatorExit",
    "inputs": [
      {
        "name": "validator",
        "type": "address",
        "indexed": false,
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
        "indexed": false,
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
        "indexed": false,
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
    "name": "BLSSigVerificationFailed",
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
    "name": "UnknownValidator",
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
    "name": "ValidatorNotExited",
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
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x608060405234801562000010575f80fd5b50604051620021b9380380620021b983398101604081905262000033916200022b565b806001600160a01b0381166200006257604051631e4fbdf760e01b81525f600482015260240160405180910390fd5b6200006d81620000ab565b5062000078620000fa565b50600680546001600160a01b039093166001600160a01b0319938416179055600755600880549091163317905562000269565b5f80546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000810460ff1615906001600160401b03165f81158015620001445750825b90505f826001600160401b03166001148015620001605750303b155b9050811580156200016f575080155b156200018e5760405163f92ee8a960e01b815260040160405180910390fd5b84546001600160401b03191660011785558315620001bd57845460ff60401b1916680100000000000000001785555b4360015583156200020857845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b5050505050565b80516001600160a01b038116811462000226575f80fd5b919050565b5f805f606084860312156200023e575f80fd5b62000249846200020f565b92506020840151915062000260604085016200020f565b90509250925092565b611f4280620002775f395ff3fe608060405234801561000f575f80fd5b506004361061011c575f3560e01c80638da5cb5b116100a9578063b5ecb3441161006e578063b5ecb3441461021b578063f2f80a181461023a578063f2fde38b1461025d578063f851a44014610270578063fa52c7d814610283575f80fd5b80638da5cb5b146101b55780639b30a5e6146101d95780639d76ea58146101ec5780639e9a8f31146101ff578063a3066aab14610208575f80fd5b80634d99dd16116100ef5780634d99dd16146101775780635544c2f11461018a5780636a911ccf1461019d578063715018a6146101a55780638129fc1c146101ad575f80fd5b8063026e402b1461012057806313b9057a146101355780632140fecd146101485780633e9df9b51461015b575b5f80fd5b61013361012e366004611936565b6102c5565b005b610133610143366004611a3b565b61040f565b610133610156366004611a99565b61052b565b61016460015481565b6040519081526020015b60405180910390f35b610133610185366004611936565b61062c565b610133610198366004611ab2565b610798565b61013361081d565b61013361089e565b6101336108b1565b5f546001600160a01b03165b6040516001600160a01b03909116815260200161016e565b6101646101e7366004611af6565b6109b5565b6006546101c1906001600160a01b031681565b61016460075481565b610133610216366004611a99565b610a0f565b610164610229366004611a99565b60036020525f908152604090205481565b61024d610248366004611b10565b610b12565b604051901515815260200161016e565b61013361026b366004611a99565b610b5b565b6008546101c1906001600160a01b031681565b6102b6610291366004611a99565b60026020525f90815260409020805460019091015460ff808316926101009004169083565b60405161016e93929190611b58565b6102ce82610b98565b6102d782610bd0565b600654604051636eb1769f60e11b81523360048201523060248201525f916001600160a01b03169063dd62ed3e90604401602060405180830381865afa158015610323573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906103479190611b8e565b9050818110156103795760405163054365bb60e31b815260048101829052602481018390526044015b60405180910390fd5b6001600160a01b0383165f90815260026020526040812060010180548492906103a3908490611bb9565b90915550506006546103c0906001600160a01b0316333085610c06565b604080513381526001600160a01b03851660208201529081018390527fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b906060015b60405180910390a1505050565b61041833610c97565b61042183610cd0565b604080513360208201525f9101604051602081830303815290604052905061044a818487610d0f565b6127108261ffff1611156104715760405163dc81db8560e01b815260040160405180910390fd5b604080516060810190915260018152602081015f81525f602091820181905233815260028252604090208251815490151560ff19821681178355928401519192839161ff001990911661ffff19909116176101008360018111156104d7576104d7611b44565b0217905550604091820151600190910155517ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9061051c903390889088908790611bcc565b60405180910390a15050505050565b335f908152600360205260408120549081900361055b576040516379298a5360e11b815260040160405180910390fd5b8042101561057c57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0382165f908152600460209081526040808320338452909152812054908190036105c057604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f9081526004602090815260408083203380855292528220919091556006546105f692169083610da4565b60408051338152602081018390527f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659101610402565b61063582610b98565b61063e82610bd0565b6001335f90815260026020526040902054610100900460ff16600181111561066857610668611b44565b036106865760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b0382165f908152600460209081526040808320338452909152902054818110156106cd57604051639266535160e01b815260048101839052602401610370565b6001600160a01b0383165f908152600460209081526040808320338452909152812080548492906106ff908490611c2f565b925050819055506040518060400160405280838152602001600754426107259190611bb9565b90526001600160a01b0384165f8181526005602090815260408083203380855290835292819020855181559482015160019095019490945583519182528101919091529081018390527f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c90606001610402565b6107a133610b98565b6107aa33610bd0565b6107b382610cd0565b604080513360208201525f910160405160208183030381529060405290506107dc818386610d0f565b7f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d33858560405161080f93929190611c42565b60405180910390a150505050565b61082633610b98565b61082f33610bd0565b335f908152600260205260409020805461ff0019166101001790556007546108579042611bb9565b335f8181526003602090815260409182902093909355519081527ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd16910160405180910390a1565b6108a6610e27565b6108af5f610e53565b565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156108f65750825b90505f8267ffffffffffffffff1660011480156109125750303b155b905081158015610920575080155b1561093e5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561096857845460ff60401b1916600160401b1785555b4360015583156109ae57845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200161051c565b5050505050565b5f815f01518260200151836040015184606001516040516020016109f2949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f908152600560209081526040808320338452909152902060010154421015610a5557604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0381165f90815260056020908152604080832033845290915281205490819003610a9957604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038083165f908152600560209081526040808320338085529252822082815560010191909155600654610ad592169083610da4565b60408051338152602081018390527f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65910160405180910390a15050565b805182515f91148015610b2c575081602001518360200151145b8015610b3f575081604001518360400151145b8015610b52575081606001518360600151145b90505b92915050565b610b63610e27565b6001600160a01b038116610b8c57604051631e4fbdf760e01b81525f6004820152602401610370565b610b9581610e53565b50565b6001600160a01b0381165f9081526002602052604090205460ff16610b95576040516357fdf40b60e01b815260040160405180910390fd5b6001600160a01b0381165f9081526003602052604090205415610b955760405163eab4a96360e01b815260040160405180910390fd5b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af13d15601f3d1160015f5114161716915050806109ae5760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610370565b6001600160a01b0381165f9081526002602052604090205460ff1615610b955760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f8082526020820152610ced8282610ea2565b15610d0b576040516306cf438f60e01b815260040160405180910390fd5b5050565b610d1882610ebe565b5f604051806060016040528060248152602001611ef26024913990505f8482604051602001610d48929190611cc3565b60405160208183030381529060405290505f610d6382610f59565b9050610d808185610d7388611046565b610d7b6110bd565b61118a565b610d9c5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af13d15601f3d1160015f511416171691505080610e215760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610370565b50505050565b5f546001600160a01b031633146108af5760405163118cdaa760e01b8152336004820152602401610370565b5f80546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b805182515f91148015610b525750506020908101519101511490565b805160208201515f915f80516020611f16833981519152911590151615610ee457505050565b825160208401518260038485858609850908838283091483821084841016169350505081610f545760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610370565b505050565b604080518082019091525f80825260208201525f610f768361126a565b90505f80516020611f1683398151915260035f8284850990508280610f9d57610f9d611cd7565b84820990508280610fb057610fb0611cd7565b82820890505f80610fc083611473565b925090505b80611029578480610fd857610fd8611cd7565b6001870895508480610fec57610fec611cd7565b86870992508480610fff57610fff611cd7565b8684099250848061101257611012611cd7565b848408925061102083611473565b92509050610fc5565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f808252602082015281516020830151159015161561106d575090565b6040518060400160405280835f015181526020015f80516020611f16833981519152846020015161109e9190611ceb565b6110b5905f80516020611f16833981519152611c2f565b905292915050565b6110e460405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f5191508061125c5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610370565b50151590505b949350505050565b5f806112758361156a565b80519091506030811461128a5761128a611d0a565b5f8167ffffffffffffffff8111156112a4576112a461195e565b6040519080825280601f01601f1916602001820160405280156112ce576020820181803683370190505b5090505f5b8281101561133d578360016112e88386611c2f565b6112f29190611c2f565b8151811061130257611302611d1e565b602001015160f81c60f81b82828151811061131f5761131f611d1e565b60200101906001600160f81b03191690815f1a9053506001016112d3565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b828110156113cd5783816113798588611c2f565b6113839190611bb9565b8151811061139357611393611d1e565b602001015160f81c60f81b60f81c8282815181106113b3576113b3611d1e565b60ff90921660209283029190910190910152600101611365565b505f6113d8826118b4565b90506101005f80516020611f168339815191525f6113f68689611c2f565b90505f5b81811015611463575f8860016114108486611c2f565b61141a9190611c2f565b8151811061142a5761142a611d1e565b016020015160f81c9050838061144257611442611cd7565b8587099550838061145557611455611cd7565b8187089550506001016113fa565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f80516020611f16833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f519250836115305760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610370565b80600184901b1115611549576115468382611c2f565b92505b808061155757611557611cd7565b8384099690961496919550909350505050565b604080516030808252606082810190935290602090600160f91b905f908460208201818036833701905050905080866040516020016115aa929190611cc3565b6040516020818303038152906040529050808460f81b6040516020016115d1929190611d32565b6040516020818303038152906040529050806040516020016115f39190611d5c565b60408051601f1981840301815290829052915061010160f01b9061161d9083908390602001611d74565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff81111561168d5761168d61195e565b6040519080825280601f01601f1916602001820160405280156116b7576020820181803683370190505b5090505f826040516020016116ce91815260200190565b60405160208183030381529060405290505f5b8151811015611737578181815181106116fc576116fc611d1e565b602001015160f81c60f81b83828151811061171957611719611d1e565b60200101906001600160f81b03191690815f1a9053506001016116e1565b505f8460405160200161174c91815260200190565b60408051601f19818403018152602083019091525f80835291985091505b898110156117de575f83828151811061178557611785611d1e565b602001015160f81c60f81b8383815181106117a2576117a2611d1e565b602001015160f81c60f81b18905088816040516020016117c3929190611d98565b60408051601f1981840301815291905298505060010161176a565b508688876040516020016117f493929190611dbc565b6040516020818303038152906040529650868051906020012093508360405160200161182291815260200190565b60405160208183030381529060405291505f5b6118428a60ff8d16611c2f565b8110156118a35782818151811061185b5761185b611d1e565b01602001516001600160f81b03191684611875838d611bb9565b8151811061188557611885611d1e565b60200101906001600160f81b03191690815f1a905350600101611835565b50919b9a5050505050505050505050565b5f80805b8351811015611914578381815181106118d3576118d3611d1e565b602002602001015160ff168160086118eb9190611def565b6118f6906002611ee6565b6119009190611def565b61190a9083611bb9565b91506001016118b8565b5092915050565b80356001600160a01b0381168114611931575f80fd5b919050565b5f8060408385031215611947575f80fd5b6119508361191b565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b5f60808284031215611982575f80fd5b6040516080810181811067ffffffffffffffff821117156119b157634e487b7160e01b5f52604160045260245ffd5b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f604082840312156119f2575f80fd5b6040516040810181811067ffffffffffffffff82111715611a2157634e487b7160e01b5f52604160045260245ffd5b604052823581526020928301359281019290925250919050565b5f805f806101208587031215611a4f575f80fd5b611a598686611972565b9350611a6886608087016119e2565b9250611a778660c087016119e2565b915061010085013561ffff81168114611a8e575f80fd5b939692955090935050565b5f60208284031215611aa9575f80fd5b610b528261191b565b5f805f6101008486031215611ac5575f80fd5b611acf8585611972565b9250611ade85608086016119e2565b9150611aed8560c086016119e2565b90509250925092565b5f60808284031215611b06575f80fd5b610b528383611972565b5f806101008385031215611b22575f80fd5b611b2c8484611972565b9150611b3b8460808501611972565b90509250929050565b634e487b7160e01b5f52602160045260245ffd5b83151581526060810160028410611b7d57634e487b7160e01b5f52602160045260245ffd5b602082019390935260400152919050565b5f60208284031215611b9e575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b80820180821115610b5557610b55611ba5565b6001600160a01b03851681526101008101611c0b6020830186805182526020810151602083015260408101516040830152606081015160608301525050565b835160a0830152602084015160c083015261ffff831660e083015295945050505050565b81810381811115610b5557610b55611ba5565b6001600160a01b038416815260e08101611c806020830185805182526020810151602083015260408101516040830152606081015160608301525050565b825160a0830152602083015160c0830152611262565b5f81515f5b81811015611cb55760208185018101518683015201611c9b565b505f93019283525090919050565b5f611262611cd18386611c96565b84611c96565b634e487b7160e01b5f52601260045260245ffd5b5f82611d0557634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f611d3d8285611c96565b5f81526001600160f81b03199390931660018401525050600201919050565b5f611d678284611c96565b5f81526001019392505050565b5f611d7f8285611c96565b6001600160f01b03199390931683525050600201919050565b5f611da38285611c96565b6001600160f81b03199390931683525050600101919050565b5f611dc78286611c96565b6001600160f81b031994909416845250506001600160f01b0319166001820152600301919050565b8082028115828204841417610b5557610b55611ba5565b600181815b80851115611e4057815f1904821115611e2657611e26611ba5565b80851615611e3357918102915b93841c9390800290611e0b565b509250929050565b5f82611e5657506001610b55565b81611e6257505f610b55565b8160018114611e785760028114611e8257611e9e565b6001915050610b55565b60ff841115611e9357611e93611ba5565b50506001821b610b55565b5060208310610133831016604e8410600b8410161715611ec1575081810a610b55565b611ecb8383611e06565b805f1904821115611ede57611ede611ba5565b029392505050565b5f610b528383611e4856fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15b\0\0\x10W_\x80\xFD[P`@Qb\0!\xB98\x03\x80b\0!\xB9\x839\x81\x01`@\x81\x90Rb\0\x003\x91b\0\x02+V[\x80`\x01`\x01`\xA0\x1B\x03\x81\x16b\0\0bW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01`@Q\x80\x91\x03\x90\xFD[b\0\0m\x81b\0\0\xABV[Pb\0\0xb\0\0\xFAV[P`\x06\x80T`\x01`\x01`\xA0\x1B\x03\x90\x93\x16`\x01`\x01`\xA0\x1B\x03\x19\x93\x84\x16\x17\x90U`\x07U`\x08\x80T\x90\x91\x163\x17\x90Ub\0\x02iV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15b\0\x01DWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15b\0\x01`WP0;\x15[\x90P\x81\x15\x80\x15b\0\x01oWP\x80\x15[\x15b\0\x01\x8EW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84T`\x01`\x01`@\x1B\x03\x19\x16`\x01\x17\x85U\x83\x15b\0\x01\xBDW\x84T`\xFF`@\x1B\x19\x16h\x01\0\0\0\0\0\0\0\0\x17\x85U[C`\x01U\x83\x15b\0\x02\x08W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x02&W_\x80\xFD[\x91\x90PV[_\x80_``\x84\x86\x03\x12\x15b\0\x02>W_\x80\xFD[b\0\x02I\x84b\0\x02\x0FV[\x92P` \x84\x01Q\x91Pb\0\x02``@\x85\x01b\0\x02\x0FV[\x90P\x92P\x92P\x92V[a\x1FB\x80b\0\x02w_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\x01\x1CW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xA9W\x80c\xB5\xEC\xB3D\x11a\0nW\x80c\xB5\xEC\xB3D\x14a\x02\x1BW\x80c\xF2\xF8\n\x18\x14a\x02:W\x80c\xF2\xFD\xE3\x8B\x14a\x02]W\x80c\xF8Q\xA4@\x14a\x02pW\x80c\xFAR\xC7\xD8\x14a\x02\x83W_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x01\xB5W\x80c\x9B0\xA5\xE6\x14a\x01\xD9W\x80c\x9Dv\xEAX\x14a\x01\xECW\x80c\x9E\x9A\x8F1\x14a\x01\xFFW\x80c\xA3\x06j\xAB\x14a\x02\x08W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\0\xEFW\x80cM\x99\xDD\x16\x14a\x01wW\x80cUD\xC2\xF1\x14a\x01\x8AW\x80cj\x91\x1C\xCF\x14a\x01\x9DW\x80cqP\x18\xA6\x14a\x01\xA5W\x80c\x81)\xFC\x1C\x14a\x01\xADW_\x80\xFD[\x80c\x02n@+\x14a\x01 W\x80c\x13\xB9\x05z\x14a\x015W\x80c!@\xFE\xCD\x14a\x01HW\x80c>\x9D\xF9\xB5\x14a\x01[W[_\x80\xFD[a\x013a\x01.6`\x04a\x196V[a\x02\xC5V[\0[a\x013a\x01C6`\x04a\x1A;V[a\x04\x0FV[a\x013a\x01V6`\x04a\x1A\x99V[a\x05+V[a\x01d`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x013a\x01\x856`\x04a\x196V[a\x06,V[a\x013a\x01\x986`\x04a\x1A\xB2V[a\x07\x98V[a\x013a\x08\x1DV[a\x013a\x08\x9EV[a\x013a\x08\xB1V[_T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01nV[a\x01da\x01\xE76`\x04a\x1A\xF6V[a\t\xB5V[`\x06Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x01d`\x07T\x81V[a\x013a\x02\x166`\x04a\x1A\x99V[a\n\x0FV[a\x01da\x02)6`\x04a\x1A\x99V[`\x03` R_\x90\x81R`@\x90 T\x81V[a\x02Ma\x02H6`\x04a\x1B\x10V[a\x0B\x12V[`@Q\x90\x15\x15\x81R` \x01a\x01nV[a\x013a\x02k6`\x04a\x1A\x99V[a\x0B[V[`\x08Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x02\xB6a\x02\x916`\x04a\x1A\x99V[`\x02` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x80\x83\x16\x92a\x01\0\x90\x04\x16\x90\x83V[`@Qa\x01n\x93\x92\x91\x90a\x1BXV[a\x02\xCE\x82a\x0B\x98V[a\x02\xD7\x82a\x0B\xD0V[`\x06T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03#W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x03G\x91\x90a\x1B\x8EV[\x90P\x81\x81\x10\x15a\x03yW`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x02` R`@\x81 `\x01\x01\x80T\x84\x92\x90a\x03\xA3\x90\x84\x90a\x1B\xB9V[\x90\x91UPP`\x06Ta\x03\xC0\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0C\x06V[`@\x80Q3\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16` \x82\x01R\x90\x81\x01\x83\x90R\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90``\x01[`@Q\x80\x91\x03\x90\xA1PPPV[a\x04\x183a\x0C\x97V[a\x04!\x83a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x04J\x81\x84\x87a\r\x0FV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x04qW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q``\x81\x01\x90\x91R`\x01\x81R` \x81\x01_\x81R_` \x91\x82\x01\x81\x90R3\x81R`\x02\x82R`@\x90 \x82Q\x81T\x90\x15\x15`\xFF\x19\x82\x16\x81\x17\x83U\x92\x84\x01Q\x91\x92\x83\x91a\xFF\0\x19\x90\x91\x16a\xFF\xFF\x19\x90\x91\x16\x17a\x01\0\x83`\x01\x81\x11\x15a\x04\xD7Wa\x04\xD7a\x1BDV[\x02\x17\x90UP`@\x91\x82\x01Q`\x01\x90\x91\x01UQ\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90a\x05\x1C\x903\x90\x88\x90\x88\x90\x87\x90a\x1B\xCCV[`@Q\x80\x91\x03\x90\xA1PPPPPV[3_\x90\x81R`\x03` R`@\x81 T\x90\x81\x90\x03a\x05[W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x05|W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x05\xC0W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x06Ta\x05\xF6\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01a\x04\x02V[a\x065\x82a\x0B\x98V[a\x06>\x82a\x0B\xD0V[`\x013_\x90\x81R`\x02` R`@\x90 Ta\x01\0\x90\x04`\xFF\x16`\x01\x81\x11\x15a\x06hWa\x06ha\x1BDV[\x03a\x06\x86W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\x06\xCDW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x06\xFF\x90\x84\x90a\x1C/V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x07TBa\x07%\x91\x90a\x1B\xB9V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x83Q\x91\x82R\x81\x01\x91\x90\x91R\x90\x81\x01\x83\x90R\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x90``\x01a\x04\x02V[a\x07\xA13a\x0B\x98V[a\x07\xAA3a\x0B\xD0V[a\x07\xB3\x82a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x07\xDC\x81\x83\x86a\r\x0FV[\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D3\x85\x85`@Qa\x08\x0F\x93\x92\x91\x90a\x1CBV[`@Q\x80\x91\x03\x90\xA1PPPPV[a\x08&3a\x0B\x98V[a\x08/3a\x0B\xD0V[3_\x90\x81R`\x02` R`@\x90 \x80Ta\xFF\0\x19\x16a\x01\0\x17\x90U`\x07Ta\x08W\x90Ba\x1B\xB9V[3_\x81\x81R`\x03` \x90\x81R`@\x91\x82\x90 \x93\x90\x93UQ\x90\x81R\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\x01`@Q\x80\x91\x03\x90\xA1V[a\x08\xA6a\x0E'V[a\x08\xAF_a\x0ESV[V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x08\xF6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\t\x12WP0;\x15[\x90P\x81\x15\x80\x15a\t WP\x80\x15[\x15a\t>W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\thW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\t\xAEW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01a\x05\x1CV[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xF2\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\nUW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\n\x99W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x06Ta\n\xD5\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0B,WP\x81` \x01Q\x83` \x01Q\x14[\x80\x15a\x0B?WP\x81`@\x01Q\x83`@\x01Q\x14[\x80\x15a\x0BRWP\x81``\x01Q\x83``\x01Q\x14[\x90P[\x92\x91PPV[a\x0Bca\x0E'V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0B\x8CW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x03pV[a\x0B\x95\x81a\x0ESV[PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16a\x0B\x95W`@QcW\xFD\xF4\x0B`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 T\x15a\x0B\x95W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\t\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16\x15a\x0B\x95W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0C\xED\x82\x82a\x0E\xA2V[\x15a\r\x0BW`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPV[a\r\x18\x82a\x0E\xBEV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a\x1E\xF2`$\x919\x90P_\x84\x82`@Q` \x01a\rH\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\rc\x82a\x0FYV[\x90Pa\r\x80\x81\x85a\rs\x88a\x10FV[a\r{a\x10\xBDV[a\x11\x8AV[a\r\x9CW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\x0E!W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x03pV[PPPPV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xAFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03pV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0BRWPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` a\x1F\x16\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x0E\xE4WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0FTW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x0Fv\x83a\x12jV[\x90P_\x80Q` a\x1F\x16\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x0F\x9DWa\x0F\x9Da\x1C\xD7V[\x84\x82\t\x90P\x82\x80a\x0F\xB0Wa\x0F\xB0a\x1C\xD7V[\x82\x82\x08\x90P_\x80a\x0F\xC0\x83a\x14sV[\x92P\x90P[\x80a\x10)W\x84\x80a\x0F\xD8Wa\x0F\xD8a\x1C\xD7V[`\x01\x87\x08\x95P\x84\x80a\x0F\xECWa\x0F\xECa\x1C\xD7V[\x86\x87\t\x92P\x84\x80a\x0F\xFFWa\x0F\xFFa\x1C\xD7V[\x86\x84\t\x92P\x84\x80a\x10\x12Wa\x10\x12a\x1C\xD7V[\x84\x84\x08\x92Pa\x10 \x83a\x14sV[\x92P\x90Pa\x0F\xC5V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x10mWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a\x1F\x16\x839\x81Q\x91R\x84` \x01Qa\x10\x9E\x91\x90a\x1C\xEBV[a\x10\xB5\x90_\x80Q` a\x1F\x16\x839\x81Q\x91Ra\x1C/V[\x90R\x92\x91PPV[a\x10\xE4`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x12\\W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x03pV[P\x15\x15\x90P[\x94\x93PPPPV[_\x80a\x12u\x83a\x15jV[\x80Q\x90\x91P`0\x81\x14a\x12\x8AWa\x12\x8Aa\x1D\nV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x12\xA4Wa\x12\xA4a\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x12\xCEW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x13=W\x83`\x01a\x12\xE8\x83\x86a\x1C/V[a\x12\xF2\x91\x90a\x1C/V[\x81Q\x81\x10a\x13\x02Wa\x13\x02a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x13\x1FWa\x13\x1Fa\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x12\xD3V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x13\xCDW\x83\x81a\x13y\x85\x88a\x1C/V[a\x13\x83\x91\x90a\x1B\xB9V[\x81Q\x81\x10a\x13\x93Wa\x13\x93a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x13\xB3Wa\x13\xB3a\x1D\x1EV[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x13eV[P_a\x13\xD8\x82a\x18\xB4V[\x90Pa\x01\0_\x80Q` a\x1F\x16\x839\x81Q\x91R_a\x13\xF6\x86\x89a\x1C/V[\x90P_[\x81\x81\x10\x15a\x14cW_\x88`\x01a\x14\x10\x84\x86a\x1C/V[a\x14\x1A\x91\x90a\x1C/V[\x81Q\x81\x10a\x14*Wa\x14*a\x1D\x1EV[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x14BWa\x14Ba\x1C\xD7V[\x85\x87\t\x95P\x83\x80a\x14UWa\x14Ua\x1C\xD7V[\x81\x87\x08\x95PP`\x01\x01a\x13\xFAV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a\x1F\x16\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x150W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[\x80`\x01\x84\x90\x1B\x11\x15a\x15IWa\x15F\x83\x82a\x1C/V[\x92P[\x80\x80a\x15WWa\x15Wa\x1C\xD7V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x15\xAA\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x15\xD1\x92\x91\x90a\x1D2V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x15\xF3\x91\x90a\x1D\\V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x16\x1D\x90\x83\x90\x83\x90` \x01a\x1DtV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x16\x8DWa\x16\x8Da\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x16\xB7W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x16\xCE\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x177W\x81\x81\x81Q\x81\x10a\x16\xFCWa\x16\xFCa\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x17\x19Wa\x17\x19a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x16\xE1V[P_\x84`@Q` \x01a\x17L\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a\x17\xDEW_\x83\x82\x81Q\x81\x10a\x17\x85Wa\x17\x85a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x17\xA2Wa\x17\xA2a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a\x17\xC3\x92\x91\x90a\x1D\x98V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x17jV[P\x86\x88\x87`@Q` \x01a\x17\xF4\x93\x92\x91\x90a\x1D\xBCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a\x18\"\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a\x18B\x8A`\xFF\x8D\x16a\x1C/V[\x81\x10\x15a\x18\xA3W\x82\x81\x81Q\x81\x10a\x18[Wa\x18[a\x1D\x1EV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a\x18u\x83\x8Da\x1B\xB9V[\x81Q\x81\x10a\x18\x85Wa\x18\x85a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x185V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x19\x14W\x83\x81\x81Q\x81\x10a\x18\xD3Wa\x18\xD3a\x1D\x1EV[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a\x18\xEB\x91\x90a\x1D\xEFV[a\x18\xF6\x90`\x02a\x1E\xE6V[a\x19\0\x91\x90a\x1D\xEFV[a\x19\n\x90\x83a\x1B\xB9V[\x91P`\x01\x01a\x18\xB8V[P\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x191W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\x19GW_\x80\xFD[a\x19P\x83a\x19\x1BV[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[_`\x80\x82\x84\x03\x12\x15a\x19\x82W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x19\xB1WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\x19\xF2W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x1A!WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a\x1AOW_\x80\xFD[a\x1AY\x86\x86a\x19rV[\x93Pa\x1Ah\x86`\x80\x87\x01a\x19\xE2V[\x92Pa\x1Aw\x86`\xC0\x87\x01a\x19\xE2V[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a\x1A\x8EW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a\x1A\xA9W_\x80\xFD[a\x0BR\x82a\x19\x1BV[_\x80_a\x01\0\x84\x86\x03\x12\x15a\x1A\xC5W_\x80\xFD[a\x1A\xCF\x85\x85a\x19rV[\x92Pa\x1A\xDE\x85`\x80\x86\x01a\x19\xE2V[\x91Pa\x1A\xED\x85`\xC0\x86\x01a\x19\xE2V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a\x1B\x06W_\x80\xFD[a\x0BR\x83\x83a\x19rV[_\x80a\x01\0\x83\x85\x03\x12\x15a\x1B\"W_\x80\xFD[a\x1B,\x84\x84a\x19rV[\x91Pa\x1B;\x84`\x80\x85\x01a\x19rV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x83\x15\x15\x81R``\x81\x01`\x02\x84\x10a\x1B}WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x82\x01\x93\x90\x93R`@\x01R\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1B\x9EW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x85\x16\x81Ra\x01\0\x81\x01a\x1C\x0B` \x83\x01\x86\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x83Q`\xA0\x83\x01R` \x84\x01Q`\xC0\x83\x01Ra\xFF\xFF\x83\x16`\xE0\x83\x01R\x95\x94PPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x84\x16\x81R`\xE0\x81\x01a\x1C\x80` \x83\x01\x85\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x82Q`\xA0\x83\x01R` \x83\x01Q`\xC0\x83\x01Ra\x12bV[_\x81Q_[\x81\x81\x10\x15a\x1C\xB5W` \x81\x85\x01\x81\x01Q\x86\x83\x01R\x01a\x1C\x9BV[P_\x93\x01\x92\x83RP\x90\x91\x90PV[_a\x12ba\x1C\xD1\x83\x86a\x1C\x96V[\x84a\x1C\x96V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a\x1D\x05WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a\x1D=\x82\x85a\x1C\x96V[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a\x1Dg\x82\x84a\x1C\x96V[_\x81R`\x01\x01\x93\x92PPPV[_a\x1D\x7F\x82\x85a\x1C\x96V[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a\x1D\xA3\x82\x85a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a\x1D\xC7\x82\x86a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0BUWa\x0BUa\x1B\xA5V[`\x01\x81\x81[\x80\x85\x11\x15a\x1E@W\x81_\x19\x04\x82\x11\x15a\x1E&Wa\x1E&a\x1B\xA5V[\x80\x85\x16\x15a\x1E3W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x1E\x0BV[P\x92P\x92\x90PV[_\x82a\x1EVWP`\x01a\x0BUV[\x81a\x1EbWP_a\x0BUV[\x81`\x01\x81\x14a\x1ExW`\x02\x81\x14a\x1E\x82Wa\x1E\x9EV[`\x01\x91PPa\x0BUV[`\xFF\x84\x11\x15a\x1E\x93Wa\x1E\x93a\x1B\xA5V[PP`\x01\x82\x1Ba\x0BUV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x1E\xC1WP\x81\x81\na\x0BUV[a\x1E\xCB\x83\x83a\x1E\x06V[\x80_\x19\x04\x82\x11\x15a\x1E\xDEWa\x1E\xDEa\x1B\xA5V[\x02\x93\x92PPPV[_a\x0BR\x83\x83a\x1EHV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405234801561000f575f80fd5b506004361061011c575f3560e01c80638da5cb5b116100a9578063b5ecb3441161006e578063b5ecb3441461021b578063f2f80a181461023a578063f2fde38b1461025d578063f851a44014610270578063fa52c7d814610283575f80fd5b80638da5cb5b146101b55780639b30a5e6146101d95780639d76ea58146101ec5780639e9a8f31146101ff578063a3066aab14610208575f80fd5b80634d99dd16116100ef5780634d99dd16146101775780635544c2f11461018a5780636a911ccf1461019d578063715018a6146101a55780638129fc1c146101ad575f80fd5b8063026e402b1461012057806313b9057a146101355780632140fecd146101485780633e9df9b51461015b575b5f80fd5b61013361012e366004611936565b6102c5565b005b610133610143366004611a3b565b61040f565b610133610156366004611a99565b61052b565b61016460015481565b6040519081526020015b60405180910390f35b610133610185366004611936565b61062c565b610133610198366004611ab2565b610798565b61013361081d565b61013361089e565b6101336108b1565b5f546001600160a01b03165b6040516001600160a01b03909116815260200161016e565b6101646101e7366004611af6565b6109b5565b6006546101c1906001600160a01b031681565b61016460075481565b610133610216366004611a99565b610a0f565b610164610229366004611a99565b60036020525f908152604090205481565b61024d610248366004611b10565b610b12565b604051901515815260200161016e565b61013361026b366004611a99565b610b5b565b6008546101c1906001600160a01b031681565b6102b6610291366004611a99565b60026020525f90815260409020805460019091015460ff808316926101009004169083565b60405161016e93929190611b58565b6102ce82610b98565b6102d782610bd0565b600654604051636eb1769f60e11b81523360048201523060248201525f916001600160a01b03169063dd62ed3e90604401602060405180830381865afa158015610323573d5f803e3d5ffd5b505050506040513d601f19601f820116820180604052508101906103479190611b8e565b9050818110156103795760405163054365bb60e31b815260048101829052602481018390526044015b60405180910390fd5b6001600160a01b0383165f90815260026020526040812060010180548492906103a3908490611bb9565b90915550506006546103c0906001600160a01b0316333085610c06565b604080513381526001600160a01b03851660208201529081018390527fe5541a6b6103d4fa7e021ed54fad39c66f27a76bd13d374cf6240ae6bd0bb72b906060015b60405180910390a1505050565b61041833610c97565b61042183610cd0565b604080513360208201525f9101604051602081830303815290604052905061044a818487610d0f565b6127108261ffff1611156104715760405163dc81db8560e01b815260040160405180910390fd5b604080516060810190915260018152602081015f81525f602091820181905233815260028252604090208251815490151560ff19821681178355928401519192839161ff001990911661ffff19909116176101008360018111156104d7576104d7611b44565b0217905550604091820151600190910155517ff6e8359c57520b469634736bfc3bb7ec5cbd1a0bd28b10a8275793bb730b797f9061051c903390889088908790611bcc565b60405180910390a15050505050565b335f908152600360205260408120549081900361055b576040516379298a5360e11b815260040160405180910390fd5b8042101561057c57604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0382165f908152600460209081526040808320338452909152812054908190036105c057604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038084165f9081526004602090815260408083203380855292528220919091556006546105f692169083610da4565b60408051338152602081018390527f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b659101610402565b61063582610b98565b61063e82610bd0565b6001335f90815260026020526040902054610100900460ff16600181111561066857610668611b44565b036106865760405163eab4a96360e01b815260040160405180910390fd5b6001600160a01b0382165f908152600460209081526040808320338452909152902054818110156106cd57604051639266535160e01b815260048101839052602401610370565b6001600160a01b0383165f908152600460209081526040808320338452909152812080548492906106ff908490611c2f565b925050819055506040518060400160405280838152602001600754426107259190611bb9565b90526001600160a01b0384165f8181526005602090815260408083203380855290835292819020855181559482015160019095019490945583519182528101919091529081018390527f4d10bd049775c77bd7f255195afba5088028ecb3c7c277d393ccff7934f2f92c90606001610402565b6107a133610b98565b6107aa33610bd0565b6107b382610cd0565b604080513360208201525f910160405160208183030381529060405290506107dc818386610d0f565b7f80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d33858560405161080f93929190611c42565b60405180910390a150505050565b61082633610b98565b61082f33610bd0565b335f908152600260205260409020805461ff0019166101001790556007546108579042611bb9565b335f8181526003602090815260409182902093909355519081527ffb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd16910160405180910390a1565b6108a6610e27565b6108af5f610e53565b565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff16159067ffffffffffffffff165f811580156108f65750825b90505f8267ffffffffffffffff1660011480156109125750303b155b905081158015610920575080155b1561093e5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff19166001178555831561096857845460ff60401b1916600160401b1785555b4360015583156109ae57845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200161051c565b5050505050565b5f815f01518260200151836040015184606001516040516020016109f2949392919093845260208401929092526040830152606082015260800190565b604051602081830303815290604052805190602001209050919050565b6001600160a01b0381165f908152600560209081526040808320338452909152902060010154421015610a5557604051635a77435760e01b815260040160405180910390fd5b6001600160a01b0381165f90815260056020908152604080832033845290915281205490819003610a9957604051630686827b60e51b815260040160405180910390fd5b6001600160a01b038083165f908152600560209081526040808320338085529252822082815560010191909155600654610ad592169083610da4565b60408051338152602081018390527f7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65910160405180910390a15050565b805182515f91148015610b2c575081602001518360200151145b8015610b3f575081604001518360400151145b8015610b52575081606001518360600151145b90505b92915050565b610b63610e27565b6001600160a01b038116610b8c57604051631e4fbdf760e01b81525f6004820152602401610370565b610b9581610e53565b50565b6001600160a01b0381165f9081526002602052604090205460ff16610b95576040516357fdf40b60e01b815260040160405180910390fd5b6001600160a01b0381165f9081526003602052604090205415610b955760405163eab4a96360e01b815260040160405180910390fd5b5f6040516323b872dd60e01b81526001600160a01b03851660048201526001600160a01b038416602482015282604482015260205f6064835f8a5af13d15601f3d1160015f5114161716915050806109ae5760405162461bcd60e51b81526020600482015260146024820152731514905394d1915497d19493d357d1905253115160621b6044820152606401610370565b6001600160a01b0381165f9081526002602052604090205460ff1615610b955760405163132e7efb60e31b815260040160405180910390fd5b604080518082019091525f8082526020820152610ced8282610ea2565b15610d0b576040516306cf438f60e01b815260040160405180910390fd5b5050565b610d1882610ebe565b5f604051806060016040528060248152602001611ef26024913990505f8482604051602001610d48929190611cc3565b60405160208183030381529060405290505f610d6382610f59565b9050610d808185610d7388611046565b610d7b6110bd565b61118a565b610d9c5760405162ced3e560e41b815260040160405180910390fd5b505050505050565b5f60405163a9059cbb60e01b81526001600160a01b038416600482015282602482015260205f6044835f895af13d15601f3d1160015f511416171691505080610e215760405162461bcd60e51b815260206004820152600f60248201526e1514905394d1915497d19052531151608a1b6044820152606401610370565b50505050565b5f546001600160a01b031633146108af5760405163118cdaa760e01b8152336004820152602401610370565b5f80546001600160a01b038381166001600160a01b0319831681178455604051919092169283917f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e09190a35050565b805182515f91148015610b525750506020908101519101511490565b805160208201515f915f80516020611f16833981519152911590151615610ee457505050565b825160208401518260038485858609850908838283091483821084841016169350505081610f545760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610370565b505050565b604080518082019091525f80825260208201525f610f768361126a565b90505f80516020611f1683398151915260035f8284850990508280610f9d57610f9d611cd7565b84820990508280610fb057610fb0611cd7565b82820890505f80610fc083611473565b925090505b80611029578480610fd857610fd8611cd7565b6001870895508480610fec57610fec611cd7565b86870992508480610fff57610fff611cd7565b8684099250848061101257611012611cd7565b848408925061102083611473565b92509050610fc5565b506040805180820190915294855260208501525091949350505050565b604080518082019091525f808252602082015281516020830151159015161561106d575090565b6040518060400160405280835f015181526020015f80516020611f16833981519152846020015161109e9190611ceb565b6110b5905f80516020611f16833981519152611c2f565b905292915050565b6110e460405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f5191508061125c5760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610370565b50151590505b949350505050565b5f806112758361156a565b80519091506030811461128a5761128a611d0a565b5f8167ffffffffffffffff8111156112a4576112a461195e565b6040519080825280601f01601f1916602001820160405280156112ce576020820181803683370190505b5090505f5b8281101561133d578360016112e88386611c2f565b6112f29190611c2f565b8151811061130257611302611d1e565b602001015160f81c60f81b82828151811061131f5761131f611d1e565b60200101906001600160f81b03191690815f1a9053506001016112d3565b5060408051601f80825261040082019092525f9082602082016103e0803683370190505090505f5b828110156113cd5783816113798588611c2f565b6113839190611bb9565b8151811061139357611393611d1e565b602001015160f81c60f81b60f81c8282815181106113b3576113b3611d1e565b60ff90921660209283029190910190910152600101611365565b505f6113d8826118b4565b90506101005f80516020611f168339815191525f6113f68689611c2f565b90505f5b81811015611463575f8860016114108486611c2f565b61141a9190611c2f565b8151811061142a5761142a611d1e565b016020015160f81c9050838061144257611442611cd7565b8587099550838061145557611455611cd7565b8187089550506001016113fa565b50929a9950505050505050505050565b5f805f805f7f0c19139cb84c680a6e14116da060561765e05aa45a1c72a34f082305b61f3f5290505f5f80516020611f16833981519152905060405160208152602080820152602060408201528760608201528260808201528160a082015260205f60c08360055afa9450505f519250836115305760405162461bcd60e51b815260206004820152601b60248201527f706f7720707265636f6d70696c652063616c6c206661696c65642100000000006044820152606401610370565b80600184901b1115611549576115468382611c2f565b92505b808061155757611557611cd7565b8384099690961496919550909350505050565b604080516030808252606082810190935290602090600160f91b905f908460208201818036833701905050905080866040516020016115aa929190611cc3565b6040516020818303038152906040529050808460f81b6040516020016115d1929190611d32565b6040516020818303038152906040529050806040516020016115f39190611d5c565b60408051601f1981840301815290829052915061010160f01b9061161d9083908390602001611d74565b60408051808303601f190181528282528051602091820120818401819052600160f81b848401526001600160f01b031985166041850152825160238186030181526043909401909252825190830120919350905f60ff881667ffffffffffffffff81111561168d5761168d61195e565b6040519080825280601f01601f1916602001820160405280156116b7576020820181803683370190505b5090505f826040516020016116ce91815260200190565b60405160208183030381529060405290505f5b8151811015611737578181815181106116fc576116fc611d1e565b602001015160f81c60f81b83828151811061171957611719611d1e565b60200101906001600160f81b03191690815f1a9053506001016116e1565b505f8460405160200161174c91815260200190565b60408051601f19818403018152602083019091525f80835291985091505b898110156117de575f83828151811061178557611785611d1e565b602001015160f81c60f81b8383815181106117a2576117a2611d1e565b602001015160f81c60f81b18905088816040516020016117c3929190611d98565b60408051601f1981840301815291905298505060010161176a565b508688876040516020016117f493929190611dbc565b6040516020818303038152906040529650868051906020012093508360405160200161182291815260200190565b60405160208183030381529060405291505f5b6118428a60ff8d16611c2f565b8110156118a35782818151811061185b5761185b611d1e565b01602001516001600160f81b03191684611875838d611bb9565b8151811061188557611885611d1e565b60200101906001600160f81b03191690815f1a905350600101611835565b50919b9a5050505050505050505050565b5f80805b8351811015611914578381815181106118d3576118d3611d1e565b602002602001015160ff168160086118eb9190611def565b6118f6906002611ee6565b6119009190611def565b61190a9083611bb9565b91506001016118b8565b5092915050565b80356001600160a01b0381168114611931575f80fd5b919050565b5f8060408385031215611947575f80fd5b6119508361191b565b946020939093013593505050565b634e487b7160e01b5f52604160045260245ffd5b5f60808284031215611982575f80fd5b6040516080810181811067ffffffffffffffff821117156119b157634e487b7160e01b5f52604160045260245ffd5b8060405250809150823581526020830135602082015260408301356040820152606083013560608201525092915050565b5f604082840312156119f2575f80fd5b6040516040810181811067ffffffffffffffff82111715611a2157634e487b7160e01b5f52604160045260245ffd5b604052823581526020928301359281019290925250919050565b5f805f806101208587031215611a4f575f80fd5b611a598686611972565b9350611a6886608087016119e2565b9250611a778660c087016119e2565b915061010085013561ffff81168114611a8e575f80fd5b939692955090935050565b5f60208284031215611aa9575f80fd5b610b528261191b565b5f805f6101008486031215611ac5575f80fd5b611acf8585611972565b9250611ade85608086016119e2565b9150611aed8560c086016119e2565b90509250925092565b5f60808284031215611b06575f80fd5b610b528383611972565b5f806101008385031215611b22575f80fd5b611b2c8484611972565b9150611b3b8460808501611972565b90509250929050565b634e487b7160e01b5f52602160045260245ffd5b83151581526060810160028410611b7d57634e487b7160e01b5f52602160045260245ffd5b602082019390935260400152919050565b5f60208284031215611b9e575f80fd5b5051919050565b634e487b7160e01b5f52601160045260245ffd5b80820180821115610b5557610b55611ba5565b6001600160a01b03851681526101008101611c0b6020830186805182526020810151602083015260408101516040830152606081015160608301525050565b835160a0830152602084015160c083015261ffff831660e083015295945050505050565b81810381811115610b5557610b55611ba5565b6001600160a01b038416815260e08101611c806020830185805182526020810151602083015260408101516040830152606081015160608301525050565b825160a0830152602083015160c0830152611262565b5f81515f5b81811015611cb55760208185018101518683015201611c9b565b505f93019283525090919050565b5f611262611cd18386611c96565b84611c96565b634e487b7160e01b5f52601260045260245ffd5b5f82611d0557634e487b7160e01b5f52601260045260245ffd5b500690565b634e487b7160e01b5f52600160045260245ffd5b634e487b7160e01b5f52603260045260245ffd5b5f611d3d8285611c96565b5f81526001600160f81b03199390931660018401525050600201919050565b5f611d678284611c96565b5f81526001019392505050565b5f611d7f8285611c96565b6001600160f01b03199390931683525050600201919050565b5f611da38285611c96565b6001600160f81b03199390931683525050600101919050565b5f611dc78286611c96565b6001600160f81b031994909416845250506001600160f01b0319166001820152600301919050565b8082028115828204841417610b5557610b55611ba5565b600181815b80851115611e4057815f1904821115611e2657611e26611ba5565b80851615611e3357918102915b93841c9390800290611e0b565b509250929050565b5f82611e5657506001610b55565b81611e6257505f610b55565b8160018114611e785760028114611e8257611e9e565b6001915050610b55565b60ff841115611e9357611e93611ba5565b50506001821b610b55565b5060208310610133831016604e8410600b8410161715611ec1575081810a610b55565b611ecb8383611e06565b805f1904821115611ede57611ede611ba5565b029392505050565b5f610b528383611e4856fe424c535f5349475f424e32353447315f584d443a4b454343414b5f4e4354485f4e554c5f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\x01\x1CW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xA9W\x80c\xB5\xEC\xB3D\x11a\0nW\x80c\xB5\xEC\xB3D\x14a\x02\x1BW\x80c\xF2\xF8\n\x18\x14a\x02:W\x80c\xF2\xFD\xE3\x8B\x14a\x02]W\x80c\xF8Q\xA4@\x14a\x02pW\x80c\xFAR\xC7\xD8\x14a\x02\x83W_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x01\xB5W\x80c\x9B0\xA5\xE6\x14a\x01\xD9W\x80c\x9Dv\xEAX\x14a\x01\xECW\x80c\x9E\x9A\x8F1\x14a\x01\xFFW\x80c\xA3\x06j\xAB\x14a\x02\x08W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\0\xEFW\x80cM\x99\xDD\x16\x14a\x01wW\x80cUD\xC2\xF1\x14a\x01\x8AW\x80cj\x91\x1C\xCF\x14a\x01\x9DW\x80cqP\x18\xA6\x14a\x01\xA5W\x80c\x81)\xFC\x1C\x14a\x01\xADW_\x80\xFD[\x80c\x02n@+\x14a\x01 W\x80c\x13\xB9\x05z\x14a\x015W\x80c!@\xFE\xCD\x14a\x01HW\x80c>\x9D\xF9\xB5\x14a\x01[W[_\x80\xFD[a\x013a\x01.6`\x04a\x196V[a\x02\xC5V[\0[a\x013a\x01C6`\x04a\x1A;V[a\x04\x0FV[a\x013a\x01V6`\x04a\x1A\x99V[a\x05+V[a\x01d`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x013a\x01\x856`\x04a\x196V[a\x06,V[a\x013a\x01\x986`\x04a\x1A\xB2V[a\x07\x98V[a\x013a\x08\x1DV[a\x013a\x08\x9EV[a\x013a\x08\xB1V[_T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01nV[a\x01da\x01\xE76`\x04a\x1A\xF6V[a\t\xB5V[`\x06Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x01d`\x07T\x81V[a\x013a\x02\x166`\x04a\x1A\x99V[a\n\x0FV[a\x01da\x02)6`\x04a\x1A\x99V[`\x03` R_\x90\x81R`@\x90 T\x81V[a\x02Ma\x02H6`\x04a\x1B\x10V[a\x0B\x12V[`@Q\x90\x15\x15\x81R` \x01a\x01nV[a\x013a\x02k6`\x04a\x1A\x99V[a\x0B[V[`\x08Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x02\xB6a\x02\x916`\x04a\x1A\x99V[`\x02` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x80\x83\x16\x92a\x01\0\x90\x04\x16\x90\x83V[`@Qa\x01n\x93\x92\x91\x90a\x1BXV[a\x02\xCE\x82a\x0B\x98V[a\x02\xD7\x82a\x0B\xD0V[`\x06T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03#W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x03G\x91\x90a\x1B\x8EV[\x90P\x81\x81\x10\x15a\x03yW`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x02` R`@\x81 `\x01\x01\x80T\x84\x92\x90a\x03\xA3\x90\x84\x90a\x1B\xB9V[\x90\x91UPP`\x06Ta\x03\xC0\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0C\x06V[`@\x80Q3\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16` \x82\x01R\x90\x81\x01\x83\x90R\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90``\x01[`@Q\x80\x91\x03\x90\xA1PPPV[a\x04\x183a\x0C\x97V[a\x04!\x83a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x04J\x81\x84\x87a\r\x0FV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x04qW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q``\x81\x01\x90\x91R`\x01\x81R` \x81\x01_\x81R_` \x91\x82\x01\x81\x90R3\x81R`\x02\x82R`@\x90 \x82Q\x81T\x90\x15\x15`\xFF\x19\x82\x16\x81\x17\x83U\x92\x84\x01Q\x91\x92\x83\x91a\xFF\0\x19\x90\x91\x16a\xFF\xFF\x19\x90\x91\x16\x17a\x01\0\x83`\x01\x81\x11\x15a\x04\xD7Wa\x04\xD7a\x1BDV[\x02\x17\x90UP`@\x91\x82\x01Q`\x01\x90\x91\x01UQ\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90a\x05\x1C\x903\x90\x88\x90\x88\x90\x87\x90a\x1B\xCCV[`@Q\x80\x91\x03\x90\xA1PPPPPV[3_\x90\x81R`\x03` R`@\x81 T\x90\x81\x90\x03a\x05[W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x05|W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x05\xC0W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x06Ta\x05\xF6\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01a\x04\x02V[a\x065\x82a\x0B\x98V[a\x06>\x82a\x0B\xD0V[`\x013_\x90\x81R`\x02` R`@\x90 Ta\x01\0\x90\x04`\xFF\x16`\x01\x81\x11\x15a\x06hWa\x06ha\x1BDV[\x03a\x06\x86W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\x06\xCDW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x06\xFF\x90\x84\x90a\x1C/V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x07TBa\x07%\x91\x90a\x1B\xB9V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x83Q\x91\x82R\x81\x01\x91\x90\x91R\x90\x81\x01\x83\x90R\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x90``\x01a\x04\x02V[a\x07\xA13a\x0B\x98V[a\x07\xAA3a\x0B\xD0V[a\x07\xB3\x82a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x07\xDC\x81\x83\x86a\r\x0FV[\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D3\x85\x85`@Qa\x08\x0F\x93\x92\x91\x90a\x1CBV[`@Q\x80\x91\x03\x90\xA1PPPPV[a\x08&3a\x0B\x98V[a\x08/3a\x0B\xD0V[3_\x90\x81R`\x02` R`@\x90 \x80Ta\xFF\0\x19\x16a\x01\0\x17\x90U`\x07Ta\x08W\x90Ba\x1B\xB9V[3_\x81\x81R`\x03` \x90\x81R`@\x91\x82\x90 \x93\x90\x93UQ\x90\x81R\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\x01`@Q\x80\x91\x03\x90\xA1V[a\x08\xA6a\x0E'V[a\x08\xAF_a\x0ESV[V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x08\xF6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\t\x12WP0;\x15[\x90P\x81\x15\x80\x15a\t WP\x80\x15[\x15a\t>W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\thW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\t\xAEW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01a\x05\x1CV[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xF2\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\nUW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\n\x99W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x06Ta\n\xD5\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0B,WP\x81` \x01Q\x83` \x01Q\x14[\x80\x15a\x0B?WP\x81`@\x01Q\x83`@\x01Q\x14[\x80\x15a\x0BRWP\x81``\x01Q\x83``\x01Q\x14[\x90P[\x92\x91PPV[a\x0Bca\x0E'V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0B\x8CW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x03pV[a\x0B\x95\x81a\x0ESV[PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16a\x0B\x95W`@QcW\xFD\xF4\x0B`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 T\x15a\x0B\x95W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\t\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16\x15a\x0B\x95W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0C\xED\x82\x82a\x0E\xA2V[\x15a\r\x0BW`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPV[a\r\x18\x82a\x0E\xBEV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a\x1E\xF2`$\x919\x90P_\x84\x82`@Q` \x01a\rH\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\rc\x82a\x0FYV[\x90Pa\r\x80\x81\x85a\rs\x88a\x10FV[a\r{a\x10\xBDV[a\x11\x8AV[a\r\x9CW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\x0E!W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x03pV[PPPPV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xAFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03pV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0BRWPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` a\x1F\x16\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x0E\xE4WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0FTW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x0Fv\x83a\x12jV[\x90P_\x80Q` a\x1F\x16\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x0F\x9DWa\x0F\x9Da\x1C\xD7V[\x84\x82\t\x90P\x82\x80a\x0F\xB0Wa\x0F\xB0a\x1C\xD7V[\x82\x82\x08\x90P_\x80a\x0F\xC0\x83a\x14sV[\x92P\x90P[\x80a\x10)W\x84\x80a\x0F\xD8Wa\x0F\xD8a\x1C\xD7V[`\x01\x87\x08\x95P\x84\x80a\x0F\xECWa\x0F\xECa\x1C\xD7V[\x86\x87\t\x92P\x84\x80a\x0F\xFFWa\x0F\xFFa\x1C\xD7V[\x86\x84\t\x92P\x84\x80a\x10\x12Wa\x10\x12a\x1C\xD7V[\x84\x84\x08\x92Pa\x10 \x83a\x14sV[\x92P\x90Pa\x0F\xC5V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x10mWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a\x1F\x16\x839\x81Q\x91R\x84` \x01Qa\x10\x9E\x91\x90a\x1C\xEBV[a\x10\xB5\x90_\x80Q` a\x1F\x16\x839\x81Q\x91Ra\x1C/V[\x90R\x92\x91PPV[a\x10\xE4`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x12\\W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x03pV[P\x15\x15\x90P[\x94\x93PPPPV[_\x80a\x12u\x83a\x15jV[\x80Q\x90\x91P`0\x81\x14a\x12\x8AWa\x12\x8Aa\x1D\nV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x12\xA4Wa\x12\xA4a\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x12\xCEW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x13=W\x83`\x01a\x12\xE8\x83\x86a\x1C/V[a\x12\xF2\x91\x90a\x1C/V[\x81Q\x81\x10a\x13\x02Wa\x13\x02a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x13\x1FWa\x13\x1Fa\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x12\xD3V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x13\xCDW\x83\x81a\x13y\x85\x88a\x1C/V[a\x13\x83\x91\x90a\x1B\xB9V[\x81Q\x81\x10a\x13\x93Wa\x13\x93a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x13\xB3Wa\x13\xB3a\x1D\x1EV[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x13eV[P_a\x13\xD8\x82a\x18\xB4V[\x90Pa\x01\0_\x80Q` a\x1F\x16\x839\x81Q\x91R_a\x13\xF6\x86\x89a\x1C/V[\x90P_[\x81\x81\x10\x15a\x14cW_\x88`\x01a\x14\x10\x84\x86a\x1C/V[a\x14\x1A\x91\x90a\x1C/V[\x81Q\x81\x10a\x14*Wa\x14*a\x1D\x1EV[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x14BWa\x14Ba\x1C\xD7V[\x85\x87\t\x95P\x83\x80a\x14UWa\x14Ua\x1C\xD7V[\x81\x87\x08\x95PP`\x01\x01a\x13\xFAV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a\x1F\x16\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x150W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[\x80`\x01\x84\x90\x1B\x11\x15a\x15IWa\x15F\x83\x82a\x1C/V[\x92P[\x80\x80a\x15WWa\x15Wa\x1C\xD7V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x15\xAA\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x15\xD1\x92\x91\x90a\x1D2V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x15\xF3\x91\x90a\x1D\\V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x16\x1D\x90\x83\x90\x83\x90` \x01a\x1DtV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x16\x8DWa\x16\x8Da\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x16\xB7W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x16\xCE\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x177W\x81\x81\x81Q\x81\x10a\x16\xFCWa\x16\xFCa\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x17\x19Wa\x17\x19a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x16\xE1V[P_\x84`@Q` \x01a\x17L\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a\x17\xDEW_\x83\x82\x81Q\x81\x10a\x17\x85Wa\x17\x85a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x17\xA2Wa\x17\xA2a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a\x17\xC3\x92\x91\x90a\x1D\x98V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x17jV[P\x86\x88\x87`@Q` \x01a\x17\xF4\x93\x92\x91\x90a\x1D\xBCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a\x18\"\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a\x18B\x8A`\xFF\x8D\x16a\x1C/V[\x81\x10\x15a\x18\xA3W\x82\x81\x81Q\x81\x10a\x18[Wa\x18[a\x1D\x1EV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a\x18u\x83\x8Da\x1B\xB9V[\x81Q\x81\x10a\x18\x85Wa\x18\x85a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x185V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x19\x14W\x83\x81\x81Q\x81\x10a\x18\xD3Wa\x18\xD3a\x1D\x1EV[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a\x18\xEB\x91\x90a\x1D\xEFV[a\x18\xF6\x90`\x02a\x1E\xE6V[a\x19\0\x91\x90a\x1D\xEFV[a\x19\n\x90\x83a\x1B\xB9V[\x91P`\x01\x01a\x18\xB8V[P\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x191W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\x19GW_\x80\xFD[a\x19P\x83a\x19\x1BV[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[_`\x80\x82\x84\x03\x12\x15a\x19\x82W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x19\xB1WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\x19\xF2W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x1A!WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a\x1AOW_\x80\xFD[a\x1AY\x86\x86a\x19rV[\x93Pa\x1Ah\x86`\x80\x87\x01a\x19\xE2V[\x92Pa\x1Aw\x86`\xC0\x87\x01a\x19\xE2V[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a\x1A\x8EW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a\x1A\xA9W_\x80\xFD[a\x0BR\x82a\x19\x1BV[_\x80_a\x01\0\x84\x86\x03\x12\x15a\x1A\xC5W_\x80\xFD[a\x1A\xCF\x85\x85a\x19rV[\x92Pa\x1A\xDE\x85`\x80\x86\x01a\x19\xE2V[\x91Pa\x1A\xED\x85`\xC0\x86\x01a\x19\xE2V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a\x1B\x06W_\x80\xFD[a\x0BR\x83\x83a\x19rV[_\x80a\x01\0\x83\x85\x03\x12\x15a\x1B\"W_\x80\xFD[a\x1B,\x84\x84a\x19rV[\x91Pa\x1B;\x84`\x80\x85\x01a\x19rV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x83\x15\x15\x81R``\x81\x01`\x02\x84\x10a\x1B}WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x82\x01\x93\x90\x93R`@\x01R\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1B\x9EW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x85\x16\x81Ra\x01\0\x81\x01a\x1C\x0B` \x83\x01\x86\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x83Q`\xA0\x83\x01R` \x84\x01Q`\xC0\x83\x01Ra\xFF\xFF\x83\x16`\xE0\x83\x01R\x95\x94PPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x84\x16\x81R`\xE0\x81\x01a\x1C\x80` \x83\x01\x85\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x82Q`\xA0\x83\x01R` \x83\x01Q`\xC0\x83\x01Ra\x12bV[_\x81Q_[\x81\x81\x10\x15a\x1C\xB5W` \x81\x85\x01\x81\x01Q\x86\x83\x01R\x01a\x1C\x9BV[P_\x93\x01\x92\x83RP\x90\x91\x90PV[_a\x12ba\x1C\xD1\x83\x86a\x1C\x96V[\x84a\x1C\x96V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a\x1D\x05WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a\x1D=\x82\x85a\x1C\x96V[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a\x1Dg\x82\x84a\x1C\x96V[_\x81R`\x01\x01\x93\x92PPPV[_a\x1D\x7F\x82\x85a\x1C\x96V[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a\x1D\xA3\x82\x85a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a\x1D\xC7\x82\x86a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0BUWa\x0BUa\x1B\xA5V[`\x01\x81\x81[\x80\x85\x11\x15a\x1E@W\x81_\x19\x04\x82\x11\x15a\x1E&Wa\x1E&a\x1B\xA5V[\x80\x85\x16\x15a\x1E3W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x1E\x0BV[P\x92P\x92\x90PV[_\x82a\x1EVWP`\x01a\x0BUV[\x81a\x1EbWP_a\x0BUV[\x81`\x01\x81\x14a\x1ExW`\x02\x81\x14a\x1E\x82Wa\x1E\x9EV[`\x01\x91PPa\x0BUV[`\xFF\x84\x11\x15a\x1E\x93Wa\x1E\x93a\x1B\xA5V[PP`\x01\x82\x1Ba\x0BUV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x1E\xC1WP\x81\x81\na\x0BUV[a\x1E\xCB\x83\x83a\x1E\x06V[\x80_\x19\x04\x82\x11\x15a\x1E\xDEWa\x1E\xDEa\x1B\xA5V[\x02\x93\x92PPPV[_a\x0BR\x83\x83a\x1EHV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1dsolcC\0\x08\x17\0\n",
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
                >(_) => {}
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
    /**Custom error with signature `InsufficientAllowance(uint256,uint256)` and selector `0x2a1b2dd8`.
    ```solidity
    error InsufficientAllowance(uint256, uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InsufficientAllowance {
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
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
                >(_) => {}
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
    /**Custom error with signature `UnknownValidator()` and selector `0x57fdf40b`.
    ```solidity
    error UnknownValidator();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UnknownValidator {}
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
                >(_) => {}
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnknownValidator> for UnderlyingRustTuple<'_> {
            fn from(value: UnknownValidator) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for UnknownValidator {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UnknownValidator {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UnknownValidator()";
            const SELECTOR: [u8; 4] = [87u8, 253u8, 244u8, 11u8];
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
                >(_) => {}
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
    /**Event with signature `ConsensusKeysUpdated(address,(uint256,uint256,uint256,uint256),(uint256,uint256))` and selector `0x80d8a4a1663328a998d4555ba21d8bba6ef1576a8c5e9d27f9c545f1a3d52b1d`.
    ```solidity
    event ConsensusKeysUpdated(address account, BN254.G2Point blsVK, EdOnBN254.EdOnBN254Point schnorrVK);
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
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                    account: data.0,
                    blsVK: data.1,
                    schnorrVK: data.2,
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
                        &self.account,
                    ),
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.blsVK),
                    <EdOnBN254::EdOnBN254Point as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrVK,
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
    event Delegated(address delegator, address validator, uint256 amount);
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
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                    delegator: data.0,
                    validator: data.1,
                    amount: data.2,
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
                        &self.delegator,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
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
    event Undelegated(address delegator, address validator, uint256 amount);
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
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                    delegator: data.0,
                    validator: data.1,
                    amount: data.2,
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
                        &self.delegator,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.validator,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
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
    /**Event with signature `ValidatorExit(address)` and selector `0xfb24305354c87762d557487ae4a564e8d03ecbb9a97dd8afff8e1f6fcaf0dd16`.
    ```solidity
    event ValidatorExit(address validator);
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
            type DataTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                Self { validator: data.0 }
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
                        &self.validator,
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
    event ValidatorRegistered(address account, BN254.G2Point blsVk, EdOnBN254.EdOnBN254Point schnorrVk, uint16 commission);
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
                alloy::sol_types::sol_data::Address,
                BN254::G2Point,
                EdOnBN254::EdOnBN254Point,
                alloy::sol_types::sol_data::Uint<16>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                    account: data.0,
                    blsVk: data.1,
                    schnorrVk: data.2,
                    commission: data.3,
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
                        &self.account,
                    ),
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
    event Withdrawal(address account, uint256 amount);
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
            type DataTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
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
                    account: data.0,
                    amount: data.1,
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
                        &self.account,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.amount,
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
    constructor(address _tokenAddress, uint256 _exitEscrowPeriod, address _initialOwner);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct constructorCall {
        pub _tokenAddress: alloy::sol_types::private::Address,
        pub _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
        pub _initialOwner: alloy::sol_types::private::Address,
    }
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<constructorCall> for UnderlyingRustTuple<'_> {
                fn from(value: constructorCall) -> Self {
                    (
                        value._tokenAddress,
                        value._exitEscrowPeriod,
                        value._initialOwner,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for constructorCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _tokenAddress: tuple.0,
                        _exitEscrowPeriod: tuple.1,
                        _initialOwner: tuple.2,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolConstructor for constructorCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self._exitEscrowPeriod,
                    ),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self._initialOwner,
                    ),
                )
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
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`_hashBlsKey((uint256,uint256,uint256,uint256))`](_hashBlsKeyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _hashBlsKeyReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
    /**Function with signature `_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))` and selector `0xf2f80a18`.
    ```solidity
    function _isEqualBlsKey(BN254.G2Point memory a, BN254.G2Point memory b) external pure returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _isEqualBlsKeyCall {
        pub a: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        pub b: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))`](_isEqualBlsKeyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _isEqualBlsKeyReturn {
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
            type UnderlyingSolTuple<'a> = (BN254::G2Point, BN254::G2Point);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
                <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<_isEqualBlsKeyCall> for UnderlyingRustTuple<'_> {
                fn from(value: _isEqualBlsKeyCall) -> Self {
                    (value.a, value.b)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _isEqualBlsKeyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        a: tuple.0,
                        b: tuple.1,
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<_isEqualBlsKeyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: _isEqualBlsKeyReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _isEqualBlsKeyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for _isEqualBlsKeyCall {
            type Parameters<'a> = (BN254::G2Point, BN254::G2Point);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = _isEqualBlsKeyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [242u8, 248u8, 10u8, 24u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.a),
                    <BN254::G2Point as alloy_sol_types::SolType>::tokenize(&self.b),
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
    /**Function with signature `admin()` and selector `0xf851a440`.
    ```solidity
    function admin() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct adminCall {}
    ///Container type for the return parameters of the [`admin()`](adminCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct adminReturn {
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<adminCall> for UnderlyingRustTuple<'_> {
                fn from(value: adminCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for adminCall {
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<adminReturn> for UnderlyingRustTuple<'_> {
                fn from(value: adminReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for adminReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for adminCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = adminReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "admin()";
            const SELECTOR: [u8; 4] = [248u8, 81u8, 164u8, 64u8];
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
    /**Function with signature `claimValidatorExit(address)` and selector `0x2140fecd`.
    ```solidity
    function claimValidatorExit(address validator) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct claimValidatorExitCall {
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
                    >(_) => {}
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
                    >(_) => {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
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
        pub validator: alloy::sol_types::private::Address,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
                    >(_) => {}
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
                    >(_) => {}
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
                    >(_) => {}
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
                    >(_) => {}
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
    /**Function with signature `initialize()` and selector `0x8129fc1c`.
    ```solidity
    function initialize() external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeCall {}
    ///Container type for the return parameters of the [`initialize()`](initializeCall) function.
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
            type UnderlyingSolTuple<'a> = ();
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = ();
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeCall {
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
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initialize()";
            const SELECTOR: [u8; 4] = [129u8, 41u8, 252u8, 28u8];
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
                    >(_) => {}
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
                    >(_) => {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
    /**Function with signature `registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)` and selector `0x13b9057a`.
    ```solidity
    function registerValidator(BN254.G2Point memory blsVK, EdOnBN254.EdOnBN254Point memory schnorrVK, BN254.G1Point memory blsSig, uint16 commission) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct registerValidatorCall {
        pub blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        pub schnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
        pub blsSig: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
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
                    >(_) => {}
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
                    >(_) => {}
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
                    >(_) => {}
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
                    >(_) => {}
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
    /**Function with signature `tokenAddress()` and selector `0x9d76ea58`.
    ```solidity
    function tokenAddress() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct tokenAddressCall {}
    ///Container type for the return parameters of the [`tokenAddress()`](tokenAddressCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct tokenAddressReturn {
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<tokenAddressCall> for UnderlyingRustTuple<'_> {
                fn from(value: tokenAddressCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for tokenAddressCall {
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<tokenAddressReturn> for UnderlyingRustTuple<'_> {
                fn from(value: tokenAddressReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for tokenAddressReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for tokenAddressCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = tokenAddressReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "tokenAddress()";
            const SELECTOR: [u8; 4] = [157u8, 118u8, 234u8, 88u8];
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
                    >(_) => {}
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
                    >(_) => {}
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
        pub validator: alloy::sol_types::private::Address,
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
        pub newBlsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        pub newSchnorrVK: <EdOnBN254::EdOnBN254Point as alloy::sol_types::SolType>::RustType,
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
                    >(_) => {}
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
                    >(_) => {}
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
    /**Function with signature `validatorExits(address)` and selector `0xb5ecb344`.
    ```solidity
    function validatorExits(address validator) external view returns (uint256 unlocksAt);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorExitsCall {
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`validatorExits(address)`](validatorExitsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorExitsReturn {
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
            type UnderlyingRustTuple<'a> = (alloy::sol_types::private::primitives::aliases::U256,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {}
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
    function validators(address validator) external view returns (bool isRegistered, ValidatorStatus status, uint256 delegatedAmount);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsCall {
        pub validator: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`validators(address)`](validatorsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct validatorsReturn {
        pub isRegistered: bool,
        pub status: <ValidatorStatus as alloy::sol_types::SolType>::RustType,
        pub delegatedAmount: alloy::sol_types::private::primitives::aliases::U256,
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
                    >(_) => {}
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<validatorsCall> for UnderlyingRustTuple<'_> {
                fn from(value: validatorsCall) -> Self {
                    (value.validator,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { validator: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Bool,
                ValidatorStatus,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                bool,
                <ValidatorStatus as alloy::sol_types::SolType>::RustType,
                alloy::sol_types::private::primitives::aliases::U256,
            );
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
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
                    (value.isRegistered, value.status, value.delegatedAmount)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for validatorsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        isRegistered: tuple.0,
                        status: tuple.1,
                        delegatedAmount: tuple.2,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for validatorsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = validatorsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Bool,
                ValidatorStatus,
                alloy::sol_types::sol_data::Uint<256>,
            );
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
    ///Container for all the [`StakeTable`](self) function calls.
    pub enum StakeTableCalls {
        _hashBlsKey(_hashBlsKeyCall),
        _isEqualBlsKey(_isEqualBlsKeyCall),
        admin(adminCall),
        claimValidatorExit(claimValidatorExitCall),
        claimWithdrawal(claimWithdrawalCall),
        delegate(delegateCall),
        deregisterValidator(deregisterValidatorCall),
        exitEscrowPeriod(exitEscrowPeriodCall),
        initialize(initializeCall),
        initializedAtBlock(initializedAtBlockCall),
        owner(ownerCall),
        registerValidator(registerValidatorCall),
        renounceOwnership(renounceOwnershipCall),
        tokenAddress(tokenAddressCall),
        transferOwnership(transferOwnershipCall),
        undelegate(undelegateCall),
        updateConsensusKeys(updateConsensusKeysCall),
        validatorExits(validatorExitsCall),
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
            [19u8, 185u8, 5u8, 122u8],
            [33u8, 64u8, 254u8, 205u8],
            [62u8, 157u8, 249u8, 181u8],
            [77u8, 153u8, 221u8, 22u8],
            [85u8, 68u8, 194u8, 241u8],
            [106u8, 145u8, 28u8, 207u8],
            [113u8, 80u8, 24u8, 166u8],
            [129u8, 41u8, 252u8, 28u8],
            [141u8, 165u8, 203u8, 91u8],
            [155u8, 48u8, 165u8, 230u8],
            [157u8, 118u8, 234u8, 88u8],
            [158u8, 154u8, 143u8, 49u8],
            [163u8, 6u8, 106u8, 171u8],
            [181u8, 236u8, 179u8, 68u8],
            [242u8, 248u8, 10u8, 24u8],
            [242u8, 253u8, 227u8, 139u8],
            [248u8, 81u8, 164u8, 64u8],
            [250u8, 82u8, 199u8, 216u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableCalls {
        const NAME: &'static str = "StakeTableCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 19usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::_hashBlsKey(_) => <_hashBlsKeyCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::_isEqualBlsKey(_) => {
                    <_isEqualBlsKeyCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::admin(_) => <adminCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::claimValidatorExit(_) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::claimWithdrawal(_) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::delegate(_) => <delegateCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::deregisterValidator(_) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::exitEscrowPeriod(_) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initialize(_) => <initializeCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initializedAtBlock(_) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::owner(_) => <ownerCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::registerValidator(_) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::renounceOwnership(_) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::tokenAddress(_) => <tokenAddressCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::transferOwnership(_) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::undelegate(_) => <undelegateCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::updateConsensusKeys(_) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::validatorExits(_) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::SELECTOR
                }
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
                    fn tokenAddress(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <tokenAddressCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::tokenAddress)
                    }
                    tokenAddress
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
                    fn _isEqualBlsKey(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <_isEqualBlsKeyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(StakeTableCalls::_isEqualBlsKey)
                    }
                    _isEqualBlsKey
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
                    fn admin(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<StakeTableCalls> {
                        <adminCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(StakeTableCalls::admin)
                    }
                    admin
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
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::_isEqualBlsKey(inner) => {
                    <_isEqualBlsKeyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::admin(inner) => {
                    <adminCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::tokenAddress(inner) => {
                    <tokenAddressCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::_hashBlsKey(inner) => {
                    <_hashBlsKeyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::_isEqualBlsKey(inner) => {
                    <_isEqualBlsKeyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::admin(inner) => {
                    <adminCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::claimValidatorExit(inner) => {
                    <claimValidatorExitCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::claimWithdrawal(inner) => {
                    <claimWithdrawalCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::delegate(inner) => {
                    <delegateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::deregisterValidator(inner) => {
                    <deregisterValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::exitEscrowPeriod(inner) => {
                    <exitEscrowPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::initializedAtBlock(inner) => {
                    <initializedAtBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::registerValidator(inner) => {
                    <registerValidatorCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::tokenAddress(inner) => {
                    <tokenAddressCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::undelegate(inner) => {
                    <undelegateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::updateConsensusKeys(inner) => {
                    <updateConsensusKeysCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::validatorExits(inner) => {
                    <validatorExitsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::validators(inner) => {
                    <validatorsCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
            }
        }
    }
    ///Container for all the [`StakeTable`](self) custom errors.
    pub enum StakeTableErrors {
        BLSSigVerificationFailed(BLSSigVerificationFailed),
        InsufficientAllowance(InsufficientAllowance),
        InsufficientBalance(InsufficientBalance),
        InvalidCommission(InvalidCommission),
        InvalidInitialization(InvalidInitialization),
        InvalidSchnorrVK(InvalidSchnorrVK),
        NotInitializing(NotInitializing),
        NothingToWithdraw(NothingToWithdraw),
        OwnableInvalidOwner(OwnableInvalidOwner),
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
        PrematureWithdrawal(PrematureWithdrawal),
        UnknownValidator(UnknownValidator),
        ValidatorAlreadyExited(ValidatorAlreadyExited),
        ValidatorAlreadyRegistered(ValidatorAlreadyRegistered),
        ValidatorNotExited(ValidatorNotExited),
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
            [6u8, 207u8, 67u8, 143u8],
            [12u8, 237u8, 62u8, 80u8],
            [17u8, 140u8, 218u8, 167u8],
            [30u8, 79u8, 189u8, 247u8],
            [42u8, 27u8, 45u8, 216u8],
            [87u8, 253u8, 244u8, 11u8],
            [90u8, 119u8, 67u8, 87u8],
            [146u8, 102u8, 83u8, 81u8],
            [153u8, 115u8, 247u8, 216u8],
            [208u8, 208u8, 79u8, 96u8],
            [215u8, 230u8, 188u8, 248u8],
            [220u8, 129u8, 219u8, 133u8],
            [234u8, 180u8, 169u8, 99u8],
            [242u8, 83u8, 20u8, 166u8],
            [249u8, 46u8, 232u8, 169u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for StakeTableErrors {
        const NAME: &'static str = "StakeTableErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 15usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::BLSSigVerificationFailed(_) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::SELECTOR
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
                Self::UnknownValidator(_) => {
                    <UnknownValidator as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorAlreadyExited(_) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorAlreadyRegistered(_) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ValidatorNotExited(_) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::SELECTOR
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<StakeTableErrors>] =
                &[
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
                        fn UnknownValidator(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<StakeTableErrors> {
                            <UnknownValidator as alloy_sol_types::SolError>::abi_decode_raw(
                                data, validate,
                            )
                            .map(StakeTableErrors::UnknownValidator)
                        }
                        UnknownValidator
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
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::UnknownValidator(inner) => {
                    <UnknownValidator as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::BLSSigVerificationFailed(inner) => {
                    <BLSSigVerificationFailed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::InsufficientAllowance(inner) => {
                    <InsufficientAllowance as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::InsufficientBalance(inner) => {
                    <InsufficientBalance as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::InvalidCommission(inner) => {
                    <InvalidCommission as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::InvalidSchnorrVK(inner) => {
                    <InvalidSchnorrVK as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::NothingToWithdraw(inner) => {
                    <NothingToWithdraw as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::OwnableInvalidOwner(inner) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::OwnableUnauthorizedAccount(inner) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::PrematureWithdrawal(inner) => {
                    <PrematureWithdrawal as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::UnknownValidator(inner) => {
                    <UnknownValidator as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
                Self::ValidatorAlreadyExited(inner) => {
                    <ValidatorAlreadyExited as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::ValidatorAlreadyRegistered(inner) => {
                    <ValidatorAlreadyRegistered as alloy_sol_types::SolError>::abi_encode_raw(
                        inner, out,
                    )
                }
                Self::ValidatorNotExited(inner) => {
                    <ValidatorNotExited as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                }
            }
        }
    }
    ///Container for all the [`StakeTable`](self) events.
    pub enum StakeTableEvents {
        ConsensusKeysUpdated(ConsensusKeysUpdated),
        Delegated(Delegated),
        Initialized(Initialized),
        OwnershipTransferred(OwnershipTransferred),
        Undelegated(Undelegated),
        ValidatorExit(ValidatorExit),
        ValidatorRegistered(ValidatorRegistered),
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
                251u8, 36u8, 48u8, 83u8, 84u8, 200u8, 119u8, 98u8, 213u8, 87u8, 72u8, 122u8, 228u8,
                165u8, 100u8, 232u8, 208u8, 62u8, 203u8, 185u8, 169u8, 125u8, 216u8, 175u8, 255u8,
                142u8, 31u8, 111u8, 202u8, 240u8, 221u8, 22u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for StakeTableEvents {
        const NAME: &'static str = "StakeTableEvents";
        const COUNT: usize = 8usize;
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
                }
                Some(<Delegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Delegated as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Delegated)
                }
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Initialized)
                }
                Some(<OwnershipTransferred as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::OwnershipTransferred)
                }
                Some(<Undelegated as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Undelegated as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Undelegated)
                }
                Some(<ValidatorExit as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ValidatorExit as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::ValidatorExit)
                }
                Some(<ValidatorRegistered as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <ValidatorRegistered as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::ValidatorRegistered)
                }
                Some(<Withdrawal as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Withdrawal as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Withdrawal)
                }
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
                }
                Self::Delegated(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::ValidatorRegistered(inner) => {
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
                Self::Delegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Undelegated(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ValidatorExit(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::ValidatorRegistered(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Withdrawal(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
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
        _tokenAddress: alloy::sol_types::private::Address,
        _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
        _initialOwner: alloy::sol_types::private::Address,
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<StakeTableInstance<T, P, N>>>
    {
        StakeTableInstance::<T, P, N>::deploy(
            provider,
            _tokenAddress,
            _exitEscrowPeriod,
            _initialOwner,
        )
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
        _tokenAddress: alloy::sol_types::private::Address,
        _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
        _initialOwner: alloy::sol_types::private::Address,
    ) -> alloy_contract::RawCallBuilder<T, P, N> {
        StakeTableInstance::<T, P, N>::deploy_builder(
            provider,
            _tokenAddress,
            _exitEscrowPeriod,
            _initialOwner,
        )
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
        pub async fn deploy(
            provider: P,
            _tokenAddress: alloy::sol_types::private::Address,
            _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
            _initialOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::Result<StakeTableInstance<T, P, N>> {
            let call_builder =
                Self::deploy_builder(provider, _tokenAddress, _exitEscrowPeriod, _initialOwner);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
        and constructor arguments, if any.

        This is a simple wrapper around creating a `RawCallBuilder` with the data set to
        the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(
            provider: P,
            _tokenAddress: alloy::sol_types::private::Address,
            _exitEscrowPeriod: alloy::sol_types::private::primitives::aliases::U256,
            _initialOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::RawCallBuilder<T, P, N> {
            alloy_contract::RawCallBuilder::new_raw_deploy(
                provider,
                [
                    &BYTECODE[..],
                    &alloy_sol_types::SolConstructor::abi_encode(&constructorCall {
                        _tokenAddress,
                        _exitEscrowPeriod,
                        _initialOwner,
                    })[..],
                ]
                .concat()
                .into(),
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
        ///Creates a new call builder for the [`_hashBlsKey`] function.
        pub fn _hashBlsKey(
            &self,
            blsVK: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, _hashBlsKeyCall, N> {
            self.call_builder(&_hashBlsKeyCall { blsVK })
        }
        ///Creates a new call builder for the [`_isEqualBlsKey`] function.
        pub fn _isEqualBlsKey(
            &self,
            a: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
            b: <BN254::G2Point as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, _isEqualBlsKeyCall, N> {
            self.call_builder(&_isEqualBlsKeyCall { a, b })
        }
        ///Creates a new call builder for the [`admin`] function.
        pub fn admin(&self) -> alloy_contract::SolCallBuilder<T, &P, adminCall, N> {
            self.call_builder(&adminCall {})
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
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(&self) -> alloy_contract::SolCallBuilder<T, &P, initializeCall, N> {
            self.call_builder(&initializeCall {})
        }
        ///Creates a new call builder for the [`initializedAtBlock`] function.
        pub fn initializedAtBlock(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializedAtBlockCall, N> {
            self.call_builder(&initializedAtBlockCall {})
        }
        ///Creates a new call builder for the [`owner`] function.
        pub fn owner(&self) -> alloy_contract::SolCallBuilder<T, &P, ownerCall, N> {
            self.call_builder(&ownerCall {})
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
        ///Creates a new call builder for the [`tokenAddress`] function.
        pub fn tokenAddress(&self) -> alloy_contract::SolCallBuilder<T, &P, tokenAddressCall, N> {
            self.call_builder(&tokenAddressCall {})
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
            validator: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, validatorsCall, N> {
            self.call_builder(&validatorsCall { validator })
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
