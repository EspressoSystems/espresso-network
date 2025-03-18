///Module containing a contract's types and functions.
/**

```solidity
library BN254 {
    type BaseField is uint256;
    type ScalarField is uint256;
    struct G1Point { BaseField x; BaseField y; }
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
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ScalarField(alloy::sol_types::private::primitives::aliases::U256);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<ScalarField>
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
        impl ScalarField {
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
        impl alloy_sol_types::SolType for ScalarField {
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
        impl alloy_sol_types::EventTopic for ScalarField {
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
library IPlonkVerifier {
    struct PlonkProof { BN254.G1Point wire0; BN254.G1Point wire1; BN254.G1Point wire2; BN254.G1Point wire3; BN254.G1Point wire4; BN254.G1Point prodPerm; BN254.G1Point split0; BN254.G1Point split1; BN254.G1Point split2; BN254.G1Point split3; BN254.G1Point split4; BN254.G1Point zeta; BN254.G1Point zetaOmega; BN254.ScalarField wireEval0; BN254.ScalarField wireEval1; BN254.ScalarField wireEval2; BN254.ScalarField wireEval3; BN254.ScalarField wireEval4; BN254.ScalarField sigmaEval0; BN254.ScalarField sigmaEval1; BN254.ScalarField sigmaEval2; BN254.ScalarField sigmaEval3; BN254.ScalarField prodPermZetaOmegaEval; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod IPlonkVerifier {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /**```solidity
    struct PlonkProof { BN254.G1Point wire0; BN254.G1Point wire1; BN254.G1Point wire2; BN254.G1Point wire3; BN254.G1Point wire4; BN254.G1Point prodPerm; BN254.G1Point split0; BN254.G1Point split1; BN254.G1Point split2; BN254.G1Point split3; BN254.G1Point split4; BN254.G1Point zeta; BN254.G1Point zetaOmega; BN254.ScalarField wireEval0; BN254.ScalarField wireEval1; BN254.ScalarField wireEval2; BN254.ScalarField wireEval3; BN254.ScalarField wireEval4; BN254.ScalarField sigmaEval0; BN254.ScalarField sigmaEval1; BN254.ScalarField sigmaEval2; BN254.ScalarField sigmaEval3; BN254.ScalarField prodPermZetaOmegaEval; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct PlonkProof {
        #[allow(missing_docs)]
        pub wire0: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wire1: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wire2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wire3: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wire4: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub prodPerm: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub split0: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub split1: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub split2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub split3: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub split4: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub zeta: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub zetaOmega: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wireEval0: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wireEval1: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wireEval2: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wireEval3: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub wireEval4: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigmaEval0: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigmaEval1: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigmaEval2: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigmaEval3: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub prodPermZetaOmegaEval: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
        impl ::core::convert::From<PlonkProof> for UnderlyingRustTuple<'_> {
            fn from(value: PlonkProof) -> Self {
                (
                    value.wire0,
                    value.wire1,
                    value.wire2,
                    value.wire3,
                    value.wire4,
                    value.prodPerm,
                    value.split0,
                    value.split1,
                    value.split2,
                    value.split3,
                    value.split4,
                    value.zeta,
                    value.zetaOmega,
                    value.wireEval0,
                    value.wireEval1,
                    value.wireEval2,
                    value.wireEval3,
                    value.wireEval4,
                    value.sigmaEval0,
                    value.sigmaEval1,
                    value.sigmaEval2,
                    value.sigmaEval3,
                    value.prodPermZetaOmegaEval,
                )
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for PlonkProof {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    wire0: tuple.0,
                    wire1: tuple.1,
                    wire2: tuple.2,
                    wire3: tuple.3,
                    wire4: tuple.4,
                    prodPerm: tuple.5,
                    split0: tuple.6,
                    split1: tuple.7,
                    split2: tuple.8,
                    split3: tuple.9,
                    split4: tuple.10,
                    zeta: tuple.11,
                    zetaOmega: tuple.12,
                    wireEval0: tuple.13,
                    wireEval1: tuple.14,
                    wireEval2: tuple.15,
                    wireEval3: tuple.16,
                    wireEval4: tuple.17,
                    sigmaEval0: tuple.18,
                    sigmaEval1: tuple.19,
                    sigmaEval2: tuple.20,
                    sigmaEval3: tuple.21,
                    prodPermZetaOmegaEval: tuple.22,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for PlonkProof {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for PlonkProof {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.wire0),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.wire1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.wire2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.wire3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.wire4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.prodPerm),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split0),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.zeta),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.zetaOmega),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.wireEval0),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.wireEval1),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.wireEval2),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.wireEval3),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.wireEval4),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.sigmaEval0),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.sigmaEval1),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.sigmaEval2),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.sigmaEval3),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.prodPermZetaOmegaEval,
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
        impl alloy_sol_types::SolType for PlonkProof {
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
        impl alloy_sol_types::SolStruct for PlonkProof {
            const NAME: &'static str = "PlonkProof";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "PlonkProof(BN254.G1Point wire0,BN254.G1Point wire1,BN254.G1Point wire2,BN254.G1Point wire3,BN254.G1Point wire4,BN254.G1Point prodPerm,BN254.G1Point split0,BN254.G1Point split1,BN254.G1Point split2,BN254.G1Point split3,BN254.G1Point split4,BN254.G1Point zeta,BN254.G1Point zetaOmega,uint256 wireEval0,uint256 wireEval1,uint256 wireEval2,uint256 wireEval3,uint256 wireEval4,uint256 sigmaEval0,uint256 sigmaEval1,uint256 sigmaEval2,uint256 sigmaEval3,uint256 prodPermZetaOmegaEval)",
                )
            }
            #[inline]
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
                let mut components = alloy_sol_types::private::Vec::with_capacity(13);
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components.push(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type());
                components
                    .extend(<BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components());
                components
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.wire0).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.wire1).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.wire2).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.wire3).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.wire4).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.prodPerm)
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.split0).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.split1).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.split2).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.split3).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.split4).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.zeta).0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(&self.zetaOmega)
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.wireEval0,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.wireEval1,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.wireEval2,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.wireEval3,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.wireEval4,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.sigmaEval0,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.sigmaEval1,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.sigmaEval2,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.sigmaEval3,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.prodPermZetaOmegaEval,
                    )
                    .0,
                ]
                .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for PlonkProof {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wire0,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wire1,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wire2,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wire3,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wire4,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.prodPerm,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.split0,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.split1,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.split2,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.split3,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.split4,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.zeta,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.zetaOmega,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wireEval0,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wireEval1,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wireEval2,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wireEval3,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.wireEval4,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigmaEval0,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigmaEval1,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigmaEval2,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigmaEval3,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.prodPermZetaOmegaEval,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wire0,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wire1,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wire2,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wire3,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wire4,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.prodPerm,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.split0,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.split1,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.split2,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.split3,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.split4,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.zeta, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.zetaOmega,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wireEval0,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wireEval1,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wireEval2,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wireEval3,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.wireEval4,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigmaEval0,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigmaEval1,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigmaEval2,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigmaEval3,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.prodPermZetaOmegaEval,
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
    /**Creates a new wrapper around an on-chain [`IPlonkVerifier`](self) contract instance.

    See the [wrapper's documentation](`IPlonkVerifierInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> IPlonkVerifierInstance<T, P, N> {
        IPlonkVerifierInstance::<T, P, N>::new(address, provider)
    }
    /**A [`IPlonkVerifier`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`IPlonkVerifier`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct IPlonkVerifierInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for IPlonkVerifierInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("IPlonkVerifierInstance")
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
        > IPlonkVerifierInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`IPlonkVerifier`](self) contract instance.

        See the [wrapper's documentation](`IPlonkVerifierInstance`) for more details.*/
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
    impl<T, P: ::core::clone::Clone, N> IPlonkVerifierInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> IPlonkVerifierInstance<T, P, N> {
            IPlonkVerifierInstance {
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
        > IPlonkVerifierInstance<T, P, N>
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
        > IPlonkVerifierInstance<T, P, N>
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
    type ScalarField is uint256;
    struct G1Point {
        BaseField x;
        BaseField y;
    }
}

library IPlonkVerifier {
    struct PlonkProof {
        BN254.G1Point wire0;
        BN254.G1Point wire1;
        BN254.G1Point wire2;
        BN254.G1Point wire3;
        BN254.G1Point wire4;
        BN254.G1Point prodPerm;
        BN254.G1Point split0;
        BN254.G1Point split1;
        BN254.G1Point split2;
        BN254.G1Point split3;
        BN254.G1Point split4;
        BN254.G1Point zeta;
        BN254.G1Point zetaOmega;
        BN254.ScalarField wireEval0;
        BN254.ScalarField wireEval1;
        BN254.ScalarField wireEval2;
        BN254.ScalarField wireEval3;
        BN254.ScalarField wireEval4;
        BN254.ScalarField sigmaEval0;
        BN254.ScalarField sigmaEval1;
        BN254.ScalarField sigmaEval2;
        BN254.ScalarField sigmaEval3;
        BN254.ScalarField prodPermZetaOmegaEval;
    }
}

interface LightClient {
    struct LightClientState {
        uint64 viewNum;
        uint64 blockHeight;
        BN254.ScalarField blockCommRoot;
    }
    struct StakeTableState {
        uint256 threshold;
        BN254.ScalarField blsKeyComm;
        BN254.ScalarField schnorrKeyComm;
        BN254.ScalarField amountComm;
    }

    error AddressEmptyCode(address target);
    error ERC1967InvalidImplementation(address implementation);
    error ERC1967NonPayable();
    error FailedInnerCall();
    error InsufficientSnapshotHistory();
    error InvalidAddress();
    error InvalidArgs();
    error InvalidHotShotBlockForCommitmentCheck();
    error InvalidInitialization();
    error InvalidMaxStateHistory();
    error InvalidProof();
    error NoChangeRequired();
    error NotInitializing();
    error OutdatedState();
    error OwnableInvalidOwner(address owner);
    error OwnableUnauthorizedAccount(address account);
    error ProverNotPermissioned();
    error UUPSUnauthorizedCallContext();
    error UUPSUnsupportedProxiableUUID(bytes32 slot);
    error WrongStakeTableUsed();

    event Initialized(uint64 version);
    event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, BN254.ScalarField blockCommRoot);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event PermissionedProverNotRequired();
    event PermissionedProverRequired(address permissionedProver);
    event Upgrade(address implementation);
    event Upgraded(address indexed implementation);

    constructor();

    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    function currentBlockNumber() external view returns (uint256);
    function disablePermissionedProverMode() external;
    function finalizedState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    function genesisStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
    function genesisState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    function getHotShotCommitment(uint256 hotShotBlockHeight) external view returns (BN254.ScalarField hotShotBlockCommRoot, uint64 hotshotBlockHeight);
    function getStateHistoryCount() external view returns (uint256);
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    function initialize(LightClientState memory _genesis, StakeTableState memory _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) external;
    function isPermissionedProverEnabled() external view returns (bool);
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) external view returns (bool);
    function newFinalizedState(LightClientState memory newState, IPlonkVerifier.PlonkProof memory proof) external;
    function owner() external view returns (address);
    function permissionedProver() external view returns (address);
    function proxiableUUID() external view returns (bytes32);
    function renounceOwnership() external;
    function setPermissionedProver(address prover) external;
    function setstateHistoryRetentionPeriod(uint32 historySeconds) external;
    function stateHistoryCommitments(uint256) external view returns (uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, BN254.ScalarField hotShotBlockCommRoot);
    function stateHistoryFirstIndex() external view returns (uint64);
    function stateHistoryRetentionPeriod() external view returns (uint32);
    function transferOwnership(address newOwner) external;
    function upgradeToAndCall(address newImplementation, bytes memory data) external payable;
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
    "name": "currentBlockNumber",
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
    "name": "disablePermissionedProverMode",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "finalizedState",
    "inputs": [],
    "outputs": [
      {
        "name": "viewNum",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "blockHeight",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "blockCommRoot",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "genesisStakeTableState",
    "inputs": [],
    "outputs": [
      {
        "name": "threshold",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "blsKeyComm",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      },
      {
        "name": "schnorrKeyComm",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      },
      {
        "name": "amountComm",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "genesisState",
    "inputs": [],
    "outputs": [
      {
        "name": "viewNum",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "blockHeight",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "blockCommRoot",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getHotShotCommitment",
    "inputs": [
      {
        "name": "hotShotBlockHeight",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [
      {
        "name": "hotShotBlockCommRoot",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      },
      {
        "name": "hotshotBlockHeight",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "getStateHistoryCount",
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
        "name": "_genesis",
        "type": "tuple",
        "internalType": "struct LightClient.LightClientState",
        "components": [
          {
            "name": "viewNum",
            "type": "uint64",
            "internalType": "uint64"
          },
          {
            "name": "blockHeight",
            "type": "uint64",
            "internalType": "uint64"
          },
          {
            "name": "blockCommRoot",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          }
        ]
      },
      {
        "name": "_genesisStakeTableState",
        "type": "tuple",
        "internalType": "struct LightClient.StakeTableState",
        "components": [
          {
            "name": "threshold",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "blsKeyComm",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "schnorrKeyComm",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "amountComm",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          }
        ]
      },
      {
        "name": "_stateHistoryRetentionPeriod",
        "type": "uint32",
        "internalType": "uint32"
      },
      {
        "name": "owner",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "isPermissionedProverEnabled",
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
    "name": "lagOverEscapeHatchThreshold",
    "inputs": [
      {
        "name": "blockNumber",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "blockThreshold",
        "type": "uint256",
        "internalType": "uint256"
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
    "name": "newFinalizedState",
    "inputs": [
      {
        "name": "newState",
        "type": "tuple",
        "internalType": "struct LightClient.LightClientState",
        "components": [
          {
            "name": "viewNum",
            "type": "uint64",
            "internalType": "uint64"
          },
          {
            "name": "blockHeight",
            "type": "uint64",
            "internalType": "uint64"
          },
          {
            "name": "blockCommRoot",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          }
        ]
      },
      {
        "name": "proof",
        "type": "tuple",
        "internalType": "struct IPlonkVerifier.PlonkProof",
        "components": [
          {
            "name": "wire0",
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
            "name": "wire1",
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
            "name": "wire2",
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
            "name": "wire3",
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
            "name": "wire4",
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
            "name": "prodPerm",
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
            "name": "split0",
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
            "name": "split1",
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
            "name": "split2",
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
            "name": "split3",
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
            "name": "split4",
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
            "name": "zeta",
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
            "name": "zetaOmega",
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
            "name": "wireEval0",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "wireEval1",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "wireEval2",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "wireEval3",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "wireEval4",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "sigmaEval0",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "sigmaEval1",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "sigmaEval2",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "sigmaEval3",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "prodPermZetaOmegaEval",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          }
        ]
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
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
    "name": "permissionedProver",
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
    "name": "renounceOwnership",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setPermissionedProver",
    "inputs": [
      {
        "name": "prover",
        "type": "address",
        "internalType": "address"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setstateHistoryRetentionPeriod",
    "inputs": [
      {
        "name": "historySeconds",
        "type": "uint32",
        "internalType": "uint32"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "stateHistoryCommitments",
    "inputs": [
      {
        "name": "",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [
      {
        "name": "l1BlockHeight",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "l1BlockTimestamp",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "hotShotBlockHeight",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "hotShotBlockCommRoot",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "stateHistoryFirstIndex",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "stateHistoryRetentionPeriod",
    "inputs": [],
    "outputs": [
      {
        "name": "",
        "type": "uint32",
        "internalType": "uint32"
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
    "name": "NewState",
    "inputs": [
      {
        "name": "viewNum",
        "type": "uint64",
        "indexed": true,
        "internalType": "uint64"
      },
      {
        "name": "blockHeight",
        "type": "uint64",
        "indexed": true,
        "internalType": "uint64"
      },
      {
        "name": "blockCommRoot",
        "type": "uint256",
        "indexed": false,
        "internalType": "BN254.ScalarField"
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
    "name": "PermissionedProverNotRequired",
    "inputs": [],
    "anonymous": false
  },
  {
    "type": "event",
    "name": "PermissionedProverRequired",
    "inputs": [
      {
        "name": "permissionedProver",
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
    "name": "InsufficientSnapshotHistory",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidAddress",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidArgs",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidHotShotBlockForCommitmentCheck",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidInitialization",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidMaxStateHistory",
    "inputs": []
  },
  {
    "type": "error",
    "name": "InvalidProof",
    "inputs": []
  },
  {
    "type": "error",
    "name": "NoChangeRequired",
    "inputs": []
  },
  {
    "type": "error",
    "name": "NotInitializing",
    "inputs": []
  },
  {
    "type": "error",
    "name": "OutdatedState",
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
    "name": "ProverNotPermissioned",
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
    "name": "WrongStakeTableUsed",
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
pub mod LightClient {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x60a06040523060805234801562000014575f80fd5b506200001f62000025565b620000d9565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff1615620000765760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b0390811614620000d65780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b608051612968620001005f395f8181611094015281816110bd015261123a01526129685ff3fe608060405260043610610147575f3560e01c8063826e41fc116100b3578063ad3cb1cc1161006d578063ad3cb1cc1461046a578063c23b9e9e146104a7578063d24d933d146104df578063e03033011461050e578063f2fde38b1461052d578063f9e50d191461054c575f80fd5b8063826e41fc146103345780638584d23f1461035f5780638da5cb5b1461039b57806396c1ca61146103d75780639baa3cc9146103f65780639fdb54a714610415575f80fd5b8063378ec23b11610104578063378ec23b14610288578063426d3194146102a45780634f1ef286146102e557806352d1902d146102f857806369cc6a041461030c578063715018a614610320575f80fd5b8063013fa5fc1461014b57806302b592f31461016c5780630d8e6e2c146101c95780632063d4f7146101f45780632f79889d14610213578063313df7b114610251575b5f80fd5b348015610156575f80fd5b5061016a610165366004611f1d565b610560565b005b348015610177575f80fd5b5061018b610186366004611f36565b610613565b6040516101c094939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b3480156101d4575f80fd5b5060408051600181525f60208201819052918101919091526060016101c0565b3480156101ff575f80fd5b5061016a61020e366004612081565b61065c565b34801561021e575f80fd5b5060085461023990600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016101c0565b34801561025c575f80fd5b50600854610270906001600160a01b031681565b6040516001600160a01b0390911681526020016101c0565b348015610293575f80fd5b50435b6040519081526020016101c0565b3480156102af575f80fd5b505f546001546002546003546102c59392919084565b6040805194855260208501939093529183015260608201526080016101c0565b61016a6102f3366004612232565b6107b6565b348015610303575f80fd5b506102966107d5565b348015610317575f80fd5b5061016a6107f0565b34801561032b575f80fd5b5061016a61085e565b34801561033f575f80fd5b506008546001600160a01b031615155b60405190151581526020016101c0565b34801561036a575f80fd5b5061037e610379366004611f36565b61086f565b604080519283526001600160401b039091166020830152016101c0565b3480156103a6575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b0316610270565b3480156103e2575f80fd5b5061016a6103f13660046122e4565b61099a565b348015610401575f80fd5b5061016a6104103660046122fd565b610a23565b348015610420575f80fd5b50600654600754610444916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016101c0565b348015610475575f80fd5b5061049a604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516101c091906123c8565b3480156104b2575f80fd5b506008546104ca90600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016101c0565b3480156104ea575f80fd5b50600454600554610444916001600160401b0380821692600160401b909204169083565b348015610519575f80fd5b5061034f6105283660046123fa565b610b45565b348015610538575f80fd5b5061016a610547366004611f1d565b610c9f565b348015610557575f80fd5b50600954610296565b610568610ce1565b6001600160a01b03811661058f5760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b03908116908216036105be5760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b60098181548110610622575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b6008546001600160a01b03161515801561068157506008546001600160a01b03163314155b1561069f576040516301474c8f60e71b815260040160405180910390fd5b60065482516001600160401b0391821691161115806106d8575060065460208301516001600160401b03600160401b9092048216911611155b156106f65760405163051c46ef60e01b815260040160405180910390fd5b6107038260400151610d3c565b61070d8282610dac565b81516006805460208501516001600160401b03908116600160401b026001600160801b0319909216931692909217919091179055604082015160075561075a6107534390565b4284610ea0565b81602001516001600160401b0316825f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae684604001516040516107aa91815260200190565b60405180910390a35050565b6107be611089565b6107c78261112d565b6107d1828261116e565b5050565b5f6107de61122f565b505f8051602061293c83398151915290565b6107f8610ce1565b6008546001600160a01b03161561084357600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b610866610ce1565b61085c5f611278565b600980545f9182919061088360018361242e565b8154811061089357610893612441565b5f918252602090912060029091020154600160801b90046001600160401b031684106108d257604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b8181101561099357846009828154811061090257610902612441565b5f918252602090912060029091020154600160801b90046001600160401b0316111561098b576009818154811061093b5761093b612441565b905f5260205f209060020201600101546009828154811061095e5761095e612441565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b6001016108e6565b5050915091565b6109a2610ce1565b610e108163ffffffff1610806109c157506301e133808163ffffffff16115b806109df575060085463ffffffff600160a01b909104811690821611155b156109fd576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff1615906001600160401b03165f81158015610a675750825b90505f826001600160401b03166001148015610a825750303b155b905081158015610a90575080155b15610aae5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610ad857845460ff60401b1916600160401b1785555b610ae1866112e8565b610ae96112f9565b610af4898989611301565b8315610b3a57845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6009545f9043841180610b56575080155b80610ba05750600854600980549091600160c01b90046001600160401b0316908110610b8457610b84612441565b5f9182526020909120600290910201546001600160401b031684105b15610bbe5760405163b0b4387760e01b815260040160405180910390fd5b5f8080610bcc60018561242e565b90505b81610c6857600854600160c01b90046001600160401b03168110610c68578660098281548110610c0157610c01612441565b5f9182526020909120600290910201546001600160401b031611610c56576001915060098181548110610c3657610c36612441565b5f9182526020909120600290910201546001600160401b03169250610c68565b80610c6081612455565b915050610bcf565b81610c865760405163b0b4387760e01b815260040160405180910390fd5b85610c91848961242e565b119450505050505b92915050565b610ca7610ce1565b6001600160a01b038116610cd557604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610cde81611278565b50565b33610d137f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b03161461085c5760405163118cdaa760e01b8152336004820152602401610ccc565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f00000018110806107d15760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401610ccc565b5f610db561142d565b9050610dbf611c7e565b83516001600160401b0390811682526020850151168160016020020152604084810151828201526001546060830152600254608083015260035460a08301525f5460c08301525163ce537a7760e01b8152735fbdb2315678afecb367f032d93f642f64180aa39063ce537a7790610e3e90859085908890600401612670565b602060405180830381865af4158015610e59573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610e7d919061289e565b610e9a576040516309bde33960e01b815260040160405180910390fd5b50505050565b60095415801590610f15575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b0316908110610ee057610ee0612441565b5f918252602090912060029091020154610f0a90600160401b90046001600160401b0316846128bd565b6001600160401b0316115b15610fa857600854600980549091600160c01b90046001600160401b0316908110610f4257610f42612441565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b0316906018610f82836128e4565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061110f57507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b03166111035f8051602061293c833981519152546001600160a01b031690565b6001600160a01b031614155b1561085c5760405163703e46dd60e11b815260040160405180910390fd5b611135610ce1565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d90602001610608565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa9250505080156111c8575060408051601f3d908101601f191682019092526111c591810190612909565b60015b6111f057604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610ccc565b5f8051602061293c833981519152811461122057604051632a87526960e21b815260048101829052602401610ccc565b61122a8383611a5c565b505050565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161461085c5760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b6112f0611ab1565b610cde81611afa565b61085c611ab1565b82516001600160401b0316151580611325575060208301516001600160401b031615155b8061133257506020820151155b8061133f57506040820151155b8061134c57506060820151155b8061135657508151155b806113685750610e108163ffffffff16105b8061137c57506301e133808163ffffffff16115b1561139a576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b611435611c9c565b621000008152600760208201527f23783d0e9777b7af65fbf849da7edb75e74b1eaf503e025d7f2f7f80991befa26040820151527f2a4e2fe8adfa53f468525582d5184c4c70bbdb946c21f216418a9879705e54a76020604083015101527f0624b2c1e77f24afceaf39451743b9fa80d5853fca7ba00389c675650774009b6060820151527f250d7719e94ca2df00dfe327938f5a8d4d837779b99837ca777a53d39127b1796020606083015101527f0dc09515152eaea66d0db2f571cc995e369d26fe647394f10db5398c917519dc6080820151527f1273144d6cec2c4a68b24a149379c0f5592bb7fbddbe32fa171919950ca404cb6020608083015101527f119521bb68caec216e2f05eeb466fb3abfe1f39baf7fe7cb392ea057b6a2d9bf60a0820151527f2d52adeaba8045e53ab526fe9982d0ea452def6b3ea0253d27a19ef3b46e8428602060a083015101527f16c3b5b217d302975a920d13374524d7a52e4a50fd7fb930842271ebf4a84efd60c0820151527f200788916b907b196972bde304318e885a2521514b2db5e4a11899c51204f089602060c083015101527f1127581afe753defca9aef12e7332db9978a200b1699ce3888c0f3aea6111dc360e0820151527f0881e13f00723be1a04872ed02b2d078c31e80feaf27724e262ce97c2cb0bb1d602060e083015101527f1482a3a6bb91d6483d153683e2404f2f5546e0e895530fdf132091498406e3de610100820151527efa52db3d52d905ead1248102f3a80a43a90d8400c68f60a62c543c417b2f4b602061010083015101527f0a57dadd4a55199525ac6ac6fabc87a4cccfdc98142bcef9dbf47de00ecc5164610120820151527f18d95abd9b8e12c36936aa218cfff582548a6bbff25c338c2006eaeb1fe5b696602061012083015101527f2bc40e91dd169b8bc143a02952a1b6c6e627bfeb7a2bbe5078e14123f3c54c1c610140820151527f108d65a20c579b6d9883275eb6889fc3f5fc79735ca9f611a13b67daa2fbc8d0602061014083015101527f21bc1f86d0608e5f0626b467ee6f8282b619223f60a7acb0fc63ba7bdaf783be610160820151527f05ef3282f8eef01515fb9a8a7d6ca06b8b007d1d512403efb268fb03ce5f09e9602061016083015101527f2cab66c1cb5a83869e73ac34fbe467486999babd541d9010ee9a804806eee4ef610180820151527f2db1982419c5a4a17593acff9535fa967683d75c8aec01319b64b84aada2ad84602061018083015101527f2c38667c6c7eb868bdd30c34dd3f4b84d9b9b1a27d7867b364c8b7831423e9086101a0820151527f2b2cb4044dd51165c48138219d51cf8d1689f91ed3eeefead6e055eb488a2ce260206101a083015101527f2d48e54703011df2c74a14dafde3af4fd83ec71875d8ddc3554658640cc955016101c0820151527f243a99d80d32eb5408b59d5b08302bede070d3fb0a8efe2f2262f865bffb4d0d60206101c083015101527f0455d2325bf6269a66f07d838f55f36947a3cd9b87edd8480bced95cbb45cc116101e0820151527f0f66d9085a6ed60b838179987e240992bff4c0516ccf6ccde4a1ca94ce8b986460206101e083015101527f2bac0d23c8585d1487ec611b5effc97e5852fea43a7cba36ccdd2c207931f394610200820151527f1860b54e01a06aea5adb4b13bf5baebab92b736807a3a89ff2040992b06ee6ec602061020083015101527f0c0bfa1c2fc6f8ed01233d5168db1e1dfe725504f032f669f50a92ae77c72906610220820151527f0d741e124c7d1069b8a400cbcdcfd90128a533901ad4de1e037fe72984dc34cf602061022083015101527f01cfed30085c9efce04668205794aa39b1a8ee591234b4c77a22f8c26d899e05610240820151527f2ab68ac82d36cedb647d14a5b0035e8c9a0be84780b7bae1133a27a880966ed1602061024083015101527f072e1d50f8b5cf8d574b3847276477d95bbd5116351000841f728da44f4043b5610260820151527f23f8ea6eacd0876d57220f57eabacbe76a2323411663731a251d5dca36f1b59f602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b611a6582611b02565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a2805115611aa95761122a8282611b65565b6107d1611bd7565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff1661085c57604051631afcd79f60e31b815260040160405180910390fd5b610ca7611ab1565b806001600160a01b03163b5f03611b3757604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610ccc565b5f8051602061293c83398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b031684604051611b819190612920565b5f60405180830381855af49150503d805f8114611bb9576040519150601f19603f3d011682016040523d82523d5f602084013e611bbe565b606091505b5091509150611bce858383611bf6565b95945050505050565b341561085c5760405163b398979f60e01b815260040160405180910390fd5b606082611c0b57611c0682611c55565b611c4e565b8151158015611c2257506001600160a01b0384163b155b15611c4b57604051639996b31560e01b81526001600160a01b0385166004820152602401610ccc565b50805b9392505050565b805115611c655780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b6040518060e001604052806007906020820280368337509192915050565b604051806102c001604052805f81526020015f8152602001611ccf60405180604001604052805f81526020015f81525090565b8152602001611cef60405180604001604052805f81526020015f81525090565b8152602001611d0f60405180604001604052805f81526020015f81525090565b8152602001611d2f60405180604001604052805f81526020015f81525090565b8152602001611d4f60405180604001604052805f81526020015f81525090565b8152602001611d6f60405180604001604052805f81526020015f81525090565b8152602001611d8f60405180604001604052805f81526020015f81525090565b8152602001611daf60405180604001604052805f81526020015f81525090565b8152602001611dcf60405180604001604052805f81526020015f81525090565b8152602001611def60405180604001604052805f81526020015f81525090565b8152602001611e0f60405180604001604052805f81526020015f81525090565b8152602001611e2f60405180604001604052805f81526020015f81525090565b8152602001611e4f60405180604001604052805f81526020015f81525090565b8152602001611e6f60405180604001604052805f81526020015f81525090565b8152602001611e8f60405180604001604052805f81526020015f81525090565b8152602001611eaf60405180604001604052805f81526020015f81525090565b8152602001611ecf60405180604001604052805f81526020015f81525090565b8152602001611eef60405180604001604052805f81526020015f81525090565b81525f6020820181905260409091015290565b80356001600160a01b0381168114611f18575f80fd5b919050565b5f60208284031215611f2d575f80fd5b611c4e82611f02565b5f60208284031215611f46575f80fd5b5035919050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715611f8457611f84611f4d565b60405290565b604051601f8201601f191681016001600160401b0381118282101715611fb257611fb2611f4d565b604052919050565b80356001600160401b0381168114611f18575f80fd5b5f60608284031215611fe0575f80fd5b604051606081018181106001600160401b038211171561200257612002611f4d565b60405290508061201183611fba565b815261201f60208401611fba565b6020820152604083013560408201525092915050565b5f60408284031215612045575f80fd5b604051604081018181106001600160401b038211171561206757612067611f4d565b604052823581526020928301359281019290925250919050565b5f808284036104e0811215612094575f80fd5b61209e8585611fd0565b925061048080605f19830112156120b3575f80fd5b6120bb611f61565b91506120ca8660608701612035565b82526120d98660a08701612035565b60208301526120eb8660e08701612035565b60408301526101206120ff87828801612035565b606084015261016061211388828901612035565b60808501526101a061212789828a01612035565b60a08601526101e061213b8a828b01612035565b60c087015261022061214f8b828c01612035565b60e08801526102606121638c828d01612035565b6101008901526102a06121788d828e01612035565b878a015261218a8d6102e08e01612035565b6101408a015261219e8d6103208e01612035565b868a01526121b08d6103608e01612035565b6101808a01526103a08c0135948901949094526103c08b01356101c08901526103e08b0135928801929092526104008a01356102008801526104208a013590870152610440890135610240870152610460890135908601529287013561028085015250506104a0850135908201526104c0909301356102c08401525092909150565b5f8060408385031215612243575f80fd5b61224c83611f02565b91506020808401356001600160401b0380821115612268575f80fd5b818601915086601f83011261227b575f80fd5b81358181111561228d5761228d611f4d565b61229f601f8201601f19168501611f8a565b915080825287848285010111156122b4575f80fd5b80848401858401375f848284010152508093505050509250929050565b803563ffffffff81168114611f18575f80fd5b5f602082840312156122f4575f80fd5b611c4e826122d1565b5f805f80848603610120811215612312575f80fd5b61231c8787611fd0565b94506080605f198201121561232f575f80fd5b50604051608081018181106001600160401b038211171561235257612352611f4d565b8060405250606086013581526080860135602082015260a0860135604082015260c086013560608201528093505061238c60e086016122d1565b915061239b6101008601611f02565b905092959194509250565b5f5b838110156123c05781810151838201526020016123a8565b50505f910152565b602081525f82518060208401526123e68160408501602087016123a6565b601f01601f19169190910160400192915050565b5f806040838503121561240b575f80fd5b50508035926020909101359150565b634e487b7160e01b5f52601160045260245ffd5b81810381811115610c9957610c9961241a565b634e487b7160e01b5f52603260045260245ffd5b5f816124635761246361241a565b505f190190565b805f5b6007811015610e9a57815184526020938401939091019060010161246d565b6124a182825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e08401525060808101516101006125048185018380518252602090810151910152565b60a083015191506101406125248186018480518252602090810151910152565b60c084015192506101806125448187018580518252602090810151910152565b60e085015193506101c06125648188018680518252602090810151910152565b928501519350610200926125848785018680518252602090810151910152565b61012086015194506102406125a58189018780518252602090810151910152565b928601519450610280926125c58885018780518252602090810151910152565b61016087015195506102c06125e6818a018880518252602090810151910152565b9287015180516103008a0152602001516103208901526101a0870151610340890152908601516103608801526101e0860151610380880152928501516103a08701526102208501516103c0870152918401516103e08601526102608401516104008601528301516104208501526102a0830151610440850152909101516104609092019190915250565b5f610a6082019050845182526020850151602083015260408501516126a2604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a08501516101006126ef8185018380518252602090810151910152565b60c0870151915061014061270f8186018480518252602090810151910152565b60e0880151925061018061272f8187018580518252602090810151910152565b9188015192506101c09161274f8684018580518252602090810151910152565b61012089015193506102006127708188018680518252602090810151910152565b918901519350610240916127908784018680518252602090810151910152565b6101608a015194506102806127b18189018780518252602090810151910152565b918a015180516102c08901526020908101516102e08901526101a08b015180516103008a0152810151610320890152938a015180516103408901528401516103608801526101e08a015180516103808901528401516103a088015289015180516103c08801528301516103e087015261022089015180516104008801528301516104208701529088015180516104408701528201516104608601526102608801518051610480870152909101516104a08501528601516104c0840152506102a08501516104e083015261288861050083018561246a565b6128966105e083018461248c565b949350505050565b5f602082840312156128ae575f80fd5b81518015158114611c4e575f80fd5b6001600160401b038281168282160390808211156128dd576128dd61241a565b5092915050565b5f6001600160401b038083168181036128ff576128ff61241a565b6001019392505050565b5f60208284031215612919575f80fd5b5051919050565b5f82516129318184602087016123a6565b919091019291505056fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15b\0\0\x14W_\x80\xFD[Pb\0\0\x1Fb\0\0%V[b\0\0\xD9V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15b\0\0vW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14b\0\0\xD6W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80Qa)hb\0\x01\0_9_\x81\x81a\x10\x94\x01R\x81\x81a\x10\xBD\x01Ra\x12:\x01Ra)h_\xF3\xFE`\x80`@R`\x046\x10a\x01GW_5`\xE0\x1C\x80c\x82nA\xFC\x11a\0\xB3W\x80c\xAD<\xB1\xCC\x11a\0mW\x80c\xAD<\xB1\xCC\x14a\x04jW\x80c\xC2;\x9E\x9E\x14a\x04\xA7W\x80c\xD2M\x93=\x14a\x04\xDFW\x80c\xE003\x01\x14a\x05\x0EW\x80c\xF2\xFD\xE3\x8B\x14a\x05-W\x80c\xF9\xE5\r\x19\x14a\x05LW_\x80\xFD[\x80c\x82nA\xFC\x14a\x034W\x80c\x85\x84\xD2?\x14a\x03_W\x80c\x8D\xA5\xCB[\x14a\x03\x9BW\x80c\x96\xC1\xCAa\x14a\x03\xD7W\x80c\x9B\xAA<\xC9\x14a\x03\xF6W\x80c\x9F\xDBT\xA7\x14a\x04\x15W_\x80\xFD[\x80c7\x8E\xC2;\x11a\x01\x04W\x80c7\x8E\xC2;\x14a\x02\x88W\x80cBm1\x94\x14a\x02\xA4W\x80cO\x1E\xF2\x86\x14a\x02\xE5W\x80cR\xD1\x90-\x14a\x02\xF8W\x80ci\xCCj\x04\x14a\x03\x0CW\x80cqP\x18\xA6\x14a\x03 W_\x80\xFD[\x80c\x01?\xA5\xFC\x14a\x01KW\x80c\x02\xB5\x92\xF3\x14a\x01lW\x80c\r\x8En,\x14a\x01\xC9W\x80c c\xD4\xF7\x14a\x01\xF4W\x80c/y\x88\x9D\x14a\x02\x13W\x80c1=\xF7\xB1\x14a\x02QW[_\x80\xFD[4\x80\x15a\x01VW_\x80\xFD[Pa\x01ja\x01e6`\x04a\x1F\x1DV[a\x05`V[\0[4\x80\x15a\x01wW_\x80\xFD[Pa\x01\x8Ba\x01\x866`\x04a\x1F6V[a\x06\x13V[`@Qa\x01\xC0\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\xD4W_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x01\xC0V[4\x80\x15a\x01\xFFW_\x80\xFD[Pa\x01ja\x02\x0E6`\x04a \x81V[a\x06\\V[4\x80\x15a\x02\x1EW_\x80\xFD[P`\x08Ta\x029\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\\W_\x80\xFD[P`\x08Ta\x02p\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\x93W_\x80\xFD[PC[`@Q\x90\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\xAFW_\x80\xFD[P_T`\x01T`\x02T`\x03Ta\x02\xC5\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x01\xC0V[a\x01ja\x02\xF36`\x04a\"2V[a\x07\xB6V[4\x80\x15a\x03\x03W_\x80\xFD[Pa\x02\x96a\x07\xD5V[4\x80\x15a\x03\x17W_\x80\xFD[Pa\x01ja\x07\xF0V[4\x80\x15a\x03+W_\x80\xFD[Pa\x01ja\x08^V[4\x80\x15a\x03?W_\x80\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15[`@Q\x90\x15\x15\x81R` \x01a\x01\xC0V[4\x80\x15a\x03jW_\x80\xFD[Pa\x03~a\x03y6`\x04a\x1F6V[a\x08oV[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x01\xC0V[4\x80\x15a\x03\xA6W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x02pV[4\x80\x15a\x03\xE2W_\x80\xFD[Pa\x01ja\x03\xF16`\x04a\"\xE4V[a\t\x9AV[4\x80\x15a\x04\x01W_\x80\xFD[Pa\x01ja\x04\x106`\x04a\"\xFDV[a\n#V[4\x80\x15a\x04 W_\x80\xFD[P`\x06T`\x07Ta\x04D\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x01\xC0V[4\x80\x15a\x04uW_\x80\xFD[Pa\x04\x9A`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x01\xC0\x91\x90a#\xC8V[4\x80\x15a\x04\xB2W_\x80\xFD[P`\x08Ta\x04\xCA\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x04\xEAW_\x80\xFD[P`\x04T`\x05Ta\x04D\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x05\x19W_\x80\xFD[Pa\x03Oa\x05(6`\x04a#\xFAV[a\x0BEV[4\x80\x15a\x058W_\x80\xFD[Pa\x01ja\x05G6`\x04a\x1F\x1DV[a\x0C\x9FV[4\x80\x15a\x05WW_\x80\xFD[P`\tTa\x02\x96V[a\x05ha\x0C\xE1V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x05\x8FW`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\x05\xBEW`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\x06\"W_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x06\x81WP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x06\x9FW`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x82Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x06\xD8WP`\x06T` \x83\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x06\xF6W`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x07\x03\x82`@\x01Qa\r<V[a\x07\r\x82\x82a\r\xACV[\x81Q`\x06\x80T` \x85\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x16\x92\x90\x92\x17\x91\x90\x91\x17\x90U`@\x82\x01Q`\x07Ua\x07Za\x07SC\x90V[B\x84a\x0E\xA0V[\x81` \x01Q`\x01`\x01`@\x1B\x03\x16\x82_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x84`@\x01Q`@Qa\x07\xAA\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPV[a\x07\xBEa\x10\x89V[a\x07\xC7\x82a\x11-V[a\x07\xD1\x82\x82a\x11nV[PPV[_a\x07\xDEa\x12/V[P_\x80Q` a)<\x839\x81Q\x91R\x90V[a\x07\xF8a\x0C\xE1V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x08CW`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x08fa\x0C\xE1V[a\x08\\_a\x12xV[`\t\x80T_\x91\x82\x91\x90a\x08\x83`\x01\x83a$.V[\x81T\x81\x10a\x08\x93Wa\x08\x93a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x08\xD2W`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\t\x93W\x84`\t\x82\x81T\x81\x10a\t\x02Wa\t\x02a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\t\x8BW`\t\x81\x81T\x81\x10a\t;Wa\t;a$AV[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\t^Wa\t^a$AV[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x08\xE6V[PP\x91P\x91V[a\t\xA2a\x0C\xE1V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\t\xC1WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\t\xDFWP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\t\xFDW`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\ngWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\n\x82WP0;\x15[\x90P\x81\x15\x80\x15a\n\x90WP\x80\x15[\x15a\n\xAEW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\n\xD8W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\n\xE1\x86a\x12\xE8V[a\n\xE9a\x12\xF9V[a\n\xF4\x89\x89\x89a\x13\x01V[\x83\x15a\x0B:W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\tT_\x90C\x84\x11\x80a\x0BVWP\x80\x15[\x80a\x0B\xA0WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0B\x84Wa\x0B\x84a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a\x0B\xBEW`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a\x0B\xCC`\x01\x85a$.V[\x90P[\x81a\x0ChW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a\x0ChW\x86`\t\x82\x81T\x81\x10a\x0C\x01Wa\x0C\x01a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a\x0CVW`\x01\x91P`\t\x81\x81T\x81\x10a\x0C6Wa\x0C6a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa\x0ChV[\x80a\x0C`\x81a$UV[\x91PPa\x0B\xCFV[\x81a\x0C\x86W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a\x0C\x91\x84\x89a$.V[\x11\x94PPPPP[\x92\x91PPV[a\x0C\xA7a\x0C\xE1V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0C\xD5W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x0C\xDE\x81a\x12xV[PV[3a\r\x13\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x08\\W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x0C\xCCV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x07\xD1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x0C\xCCV[_a\r\xB5a\x14-V[\x90Pa\r\xBFa\x1C~V[\x83Q`\x01`\x01`@\x1B\x03\x90\x81\x16\x82R` \x85\x01Q\x16\x81`\x01` \x02\x01R`@\x84\x81\x01Q\x82\x82\x01R`\x01T``\x83\x01R`\x02T`\x80\x83\x01R`\x03T`\xA0\x83\x01R_T`\xC0\x83\x01RQc\xCESzw`\xE0\x1B\x81Rs_\xBD\xB21Vx\xAF\xEC\xB3g\xF02\xD9?d/d\x18\n\xA3\x90c\xCESzw\x90a\x0E>\x90\x85\x90\x85\x90\x88\x90`\x04\x01a&pV[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a\x0EYW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x0E}\x91\x90a(\x9EV[a\x0E\x9AW`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPV[`\tT\x15\x80\x15\x90a\x0F\x15WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0E\xE0Wa\x0E\xE0a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta\x0F\n\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a(\xBDV[`\x01`\x01`@\x1B\x03\x16\x11[\x15a\x0F\xA8W`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0FBWa\x0FBa$AV[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a\x0F\x82\x83a(\xE4V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x11\x0FWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x11\x03_\x80Q` a)<\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x08\\W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x115a\x0C\xE1V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x06\x08V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x11\xC8WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x11\xC5\x91\x81\x01\x90a)\tV[`\x01[a\x11\xF0W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[_\x80Q` a)<\x839\x81Q\x91R\x81\x14a\x12 W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0C\xCCV[a\x12*\x83\x83a\x1A\\V[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x08\\W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x12\xF0a\x1A\xB1V[a\x0C\xDE\x81a\x1A\xFAV[a\x08\\a\x1A\xB1V[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a\x13%WP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a\x132WP` \x82\x01Q\x15[\x80a\x13?WP`@\x82\x01Q\x15[\x80a\x13LWP``\x82\x01Q\x15[\x80a\x13VWP\x81Q\x15[\x80a\x13hWPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a\x13|WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a\x13\x9AW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x145a\x1C\x9CV[b\x10\0\0\x81R`\x07` \x82\x01R\x7F#x=\x0E\x97w\xB7\xAFe\xFB\xF8I\xDA~\xDBu\xE7K\x1E\xAFP>\x02]\x7F/\x7F\x80\x99\x1B\xEF\xA2`@\x82\x01QR\x7F*N/\xE8\xAD\xFAS\xF4hRU\x82\xD5\x18LLp\xBB\xDB\x94l!\xF2\x16A\x8A\x98yp^T\xA7` `@\x83\x01Q\x01R\x7F\x06$\xB2\xC1\xE7\x7F$\xAF\xCE\xAF9E\x17C\xB9\xFA\x80\xD5\x85?\xCA{\xA0\x03\x89\xC6ue\x07t\0\x9B``\x82\x01QR\x7F%\rw\x19\xE9L\xA2\xDF\0\xDF\xE3'\x93\x8FZ\x8DM\x83wy\xB9\x987\xCAwzS\xD3\x91'\xB1y` ``\x83\x01Q\x01R\x7F\r\xC0\x95\x15\x15.\xAE\xA6m\r\xB2\xF5q\xCC\x99^6\x9D&\xFEds\x94\xF1\r\xB59\x8C\x91u\x19\xDC`\x80\x82\x01QR\x7F\x12s\x14Ml\xEC,Jh\xB2J\x14\x93y\xC0\xF5Y+\xB7\xFB\xDD\xBE2\xFA\x17\x19\x19\x95\x0C\xA4\x04\xCB` `\x80\x83\x01Q\x01R\x7F\x11\x95!\xBBh\xCA\xEC!n/\x05\xEE\xB4f\xFB:\xBF\xE1\xF3\x9B\xAF\x7F\xE7\xCB9.\xA0W\xB6\xA2\xD9\xBF`\xA0\x82\x01QR\x7F-R\xAD\xEA\xBA\x80E\xE5:\xB5&\xFE\x99\x82\xD0\xEAE-\xEFk>\xA0%='\xA1\x9E\xF3\xB4n\x84(` `\xA0\x83\x01Q\x01R\x7F\x16\xC3\xB5\xB2\x17\xD3\x02\x97Z\x92\r\x137E$\xD7\xA5.JP\xFD\x7F\xB90\x84\"q\xEB\xF4\xA8N\xFD`\xC0\x82\x01QR\x7F \x07\x88\x91k\x90{\x19ir\xBD\xE3\x041\x8E\x88Z%!QK-\xB5\xE4\xA1\x18\x99\xC5\x12\x04\xF0\x89` `\xC0\x83\x01Q\x01R\x7F\x11'X\x1A\xFEu=\xEF\xCA\x9A\xEF\x12\xE73-\xB9\x97\x8A \x0B\x16\x99\xCE8\x88\xC0\xF3\xAE\xA6\x11\x1D\xC3`\xE0\x82\x01QR\x7F\x08\x81\xE1?\0r;\xE1\xA0Hr\xED\x02\xB2\xD0x\xC3\x1E\x80\xFE\xAF'rN&,\xE9|,\xB0\xBB\x1D` `\xE0\x83\x01Q\x01R\x7F\x14\x82\xA3\xA6\xBB\x91\xD6H=\x156\x83\xE2@O/UF\xE0\xE8\x95S\x0F\xDF\x13 \x91I\x84\x06\xE3\xDEa\x01\0\x82\x01QR~\xFAR\xDB=R\xD9\x05\xEA\xD1$\x81\x02\xF3\xA8\nC\xA9\r\x84\0\xC6\x8F`\xA6,T<A{/K` a\x01\0\x83\x01Q\x01R\x7F\nW\xDA\xDDJU\x19\x95%\xACj\xC6\xFA\xBC\x87\xA4\xCC\xCF\xDC\x98\x14+\xCE\xF9\xDB\xF4}\xE0\x0E\xCCQda\x01 \x82\x01QR\x7F\x18\xD9Z\xBD\x9B\x8E\x12\xC3i6\xAA!\x8C\xFF\xF5\x82T\x8Ak\xBF\xF2\\3\x8C \x06\xEA\xEB\x1F\xE5\xB6\x96` a\x01 \x83\x01Q\x01R\x7F+\xC4\x0E\x91\xDD\x16\x9B\x8B\xC1C\xA0)R\xA1\xB6\xC6\xE6'\xBF\xEBz+\xBEPx\xE1A#\xF3\xC5L\x1Ca\x01@\x82\x01QR\x7F\x10\x8De\xA2\x0CW\x9Bm\x98\x83'^\xB6\x88\x9F\xC3\xF5\xFCys\\\xA9\xF6\x11\xA1;g\xDA\xA2\xFB\xC8\xD0` a\x01@\x83\x01Q\x01R\x7F!\xBC\x1F\x86\xD0`\x8E_\x06&\xB4g\xEEo\x82\x82\xB6\x19\"?`\xA7\xAC\xB0\xFCc\xBA{\xDA\xF7\x83\xBEa\x01`\x82\x01QR\x7F\x05\xEF2\x82\xF8\xEE\xF0\x15\x15\xFB\x9A\x8A}l\xA0k\x8B\0}\x1DQ$\x03\xEF\xB2h\xFB\x03\xCE_\t\xE9` a\x01`\x83\x01Q\x01R\x7F,\xABf\xC1\xCBZ\x83\x86\x9Es\xAC4\xFB\xE4gHi\x99\xBA\xBDT\x1D\x90\x10\xEE\x9A\x80H\x06\xEE\xE4\xEFa\x01\x80\x82\x01QR\x7F-\xB1\x98$\x19\xC5\xA4\xA1u\x93\xAC\xFF\x955\xFA\x96v\x83\xD7\\\x8A\xEC\x011\x9Bd\xB8J\xAD\xA2\xAD\x84` a\x01\x80\x83\x01Q\x01R\x7F,8f|l~\xB8h\xBD\xD3\x0C4\xDD?K\x84\xD9\xB9\xB1\xA2}xg\xB3d\xC8\xB7\x83\x14#\xE9\x08a\x01\xA0\x82\x01QR\x7F+,\xB4\x04M\xD5\x11e\xC4\x818!\x9DQ\xCF\x8D\x16\x89\xF9\x1E\xD3\xEE\xEF\xEA\xD6\xE0U\xEBH\x8A,\xE2` a\x01\xA0\x83\x01Q\x01R\x7F-H\xE5G\x03\x01\x1D\xF2\xC7J\x14\xDA\xFD\xE3\xAFO\xD8>\xC7\x18u\xD8\xDD\xC3UFXd\x0C\xC9U\x01a\x01\xC0\x82\x01QR\x7F$:\x99\xD8\r2\xEBT\x08\xB5\x9D[\x080+\xED\xE0p\xD3\xFB\n\x8E\xFE/\"b\xF8e\xBF\xFBM\r` a\x01\xC0\x83\x01Q\x01R\x7F\x04U\xD22[\xF6&\x9Af\xF0}\x83\x8FU\xF3iG\xA3\xCD\x9B\x87\xED\xD8H\x0B\xCE\xD9\\\xBBE\xCC\x11a\x01\xE0\x82\x01QR\x7F\x0Ff\xD9\x08Zn\xD6\x0B\x83\x81y\x98~$\t\x92\xBF\xF4\xC0Ql\xCFl\xCD\xE4\xA1\xCA\x94\xCE\x8B\x98d` a\x01\xE0\x83\x01Q\x01R\x7F+\xAC\r#\xC8X]\x14\x87\xECa\x1B^\xFF\xC9~XR\xFE\xA4:|\xBA6\xCC\xDD, y1\xF3\x94a\x02\0\x82\x01QR\x7F\x18`\xB5N\x01\xA0j\xEAZ\xDBK\x13\xBF[\xAE\xBA\xB9+sh\x07\xA3\xA8\x9F\xF2\x04\t\x92\xB0n\xE6\xEC` a\x02\0\x83\x01Q\x01R\x7F\x0C\x0B\xFA\x1C/\xC6\xF8\xED\x01#=Qh\xDB\x1E\x1D\xFErU\x04\xF02\xF6i\xF5\n\x92\xAEw\xC7)\x06a\x02 \x82\x01QR\x7F\rt\x1E\x12L}\x10i\xB8\xA4\0\xCB\xCD\xCF\xD9\x01(\xA53\x90\x1A\xD4\xDE\x1E\x03\x7F\xE7)\x84\xDC4\xCF` a\x02 \x83\x01Q\x01R\x7F\x01\xCF\xED0\x08\\\x9E\xFC\xE0Fh W\x94\xAA9\xB1\xA8\xEEY\x124\xB4\xC7z\"\xF8\xC2m\x89\x9E\x05a\x02@\x82\x01QR\x7F*\xB6\x8A\xC8-6\xCE\xDBd}\x14\xA5\xB0\x03^\x8C\x9A\x0B\xE8G\x80\xB7\xBA\xE1\x13:'\xA8\x80\x96n\xD1` a\x02@\x83\x01Q\x01R\x7F\x07.\x1DP\xF8\xB5\xCF\x8DWK8G'dw\xD9[\xBDQ\x165\x10\0\x84\x1Fr\x8D\xA4O@C\xB5a\x02`\x82\x01QR\x7F#\xF8\xEAn\xAC\xD0\x87mW\"\x0FW\xEA\xBA\xCB\xE7j##A\x16cs\x1A%\x1D]\xCA6\xF1\xB5\x9F` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[a\x1Ae\x82a\x1B\x02V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a\x1A\xA9Wa\x12*\x82\x82a\x1BeV[a\x07\xD1a\x1B\xD7V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x08\\W`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0C\xA7a\x1A\xB1V[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a\x1B7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[_\x80Q` a)<\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa\x1B\x81\x91\x90a) V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a\x1B\xB9W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x1B\xBEV[``\x91P[P\x91P\x91Pa\x1B\xCE\x85\x83\x83a\x1B\xF6V[\x95\x94PPPPPV[4\x15a\x08\\W`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a\x1C\x0BWa\x1C\x06\x82a\x1CUV[a\x1CNV[\x81Q\x15\x80\x15a\x1C\"WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a\x1CKW`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[P\x80[\x93\x92PPPV[\x80Q\x15a\x1CeW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80`\xE0\x01`@R\x80`\x07\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a\x1C\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1C\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\x0F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D/`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1DO`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1Do`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\x8F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xAF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\x0F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E/`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1EO`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1Eo`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\x8F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xAF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R_` \x82\x01\x81\x90R`@\x90\x91\x01R\x90V[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1F-W_\x80\xFD[a\x1CN\x82a\x1F\x02V[_` \x82\x84\x03\x12\x15a\x1FFW_\x80\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x1F\x84Wa\x1F\x84a\x1FMV[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x1F\xB2Wa\x1F\xB2a\x1FMV[`@R\x91\x90PV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[_``\x82\x84\x03\x12\x15a\x1F\xE0W_\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a \x02Wa \x02a\x1FMV[`@R\x90P\x80a \x11\x83a\x1F\xBAV[\x81Ra \x1F` \x84\x01a\x1F\xBAV[` \x82\x01R`@\x83\x015`@\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a EW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a gWa ga\x1FMV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80\x82\x84\x03a\x04\xE0\x81\x12\x15a \x94W_\x80\xFD[a \x9E\x85\x85a\x1F\xD0V[\x92Pa\x04\x80\x80`_\x19\x83\x01\x12\x15a \xB3W_\x80\xFD[a \xBBa\x1FaV[\x91Pa \xCA\x86``\x87\x01a 5V[\x82Ra \xD9\x86`\xA0\x87\x01a 5V[` \x83\x01Ra \xEB\x86`\xE0\x87\x01a 5V[`@\x83\x01Ra\x01 a \xFF\x87\x82\x88\x01a 5V[``\x84\x01Ra\x01`a!\x13\x88\x82\x89\x01a 5V[`\x80\x85\x01Ra\x01\xA0a!'\x89\x82\x8A\x01a 5V[`\xA0\x86\x01Ra\x01\xE0a!;\x8A\x82\x8B\x01a 5V[`\xC0\x87\x01Ra\x02 a!O\x8B\x82\x8C\x01a 5V[`\xE0\x88\x01Ra\x02`a!c\x8C\x82\x8D\x01a 5V[a\x01\0\x89\x01Ra\x02\xA0a!x\x8D\x82\x8E\x01a 5V[\x87\x8A\x01Ra!\x8A\x8Da\x02\xE0\x8E\x01a 5V[a\x01@\x8A\x01Ra!\x9E\x8Da\x03 \x8E\x01a 5V[\x86\x8A\x01Ra!\xB0\x8Da\x03`\x8E\x01a 5V[a\x01\x80\x8A\x01Ra\x03\xA0\x8C\x015\x94\x89\x01\x94\x90\x94Ra\x03\xC0\x8B\x015a\x01\xC0\x89\x01Ra\x03\xE0\x8B\x015\x92\x88\x01\x92\x90\x92Ra\x04\0\x8A\x015a\x02\0\x88\x01Ra\x04 \x8A\x015\x90\x87\x01Ra\x04@\x89\x015a\x02@\x87\x01Ra\x04`\x89\x015\x90\x86\x01R\x92\x87\x015a\x02\x80\x85\x01RPPa\x04\xA0\x85\x015\x90\x82\x01Ra\x04\xC0\x90\x93\x015a\x02\xC0\x84\x01RP\x92\x90\x91PV[_\x80`@\x83\x85\x03\x12\x15a\"CW_\x80\xFD[a\"L\x83a\x1F\x02V[\x91P` \x80\x84\x015`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\"hW_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a\"{W_\x80\xFD[\x815\x81\x81\x11\x15a\"\x8DWa\"\x8Da\x1FMV[a\"\x9F`\x1F\x82\x01`\x1F\x19\x16\x85\x01a\x1F\x8AV[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a\"\xB4W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[_` \x82\x84\x03\x12\x15a\"\xF4W_\x80\xFD[a\x1CN\x82a\"\xD1V[_\x80_\x80\x84\x86\x03a\x01 \x81\x12\x15a#\x12W_\x80\xFD[a#\x1C\x87\x87a\x1F\xD0V[\x94P`\x80`_\x19\x82\x01\x12\x15a#/W_\x80\xFD[P`@Q`\x80\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a#RWa#Ra\x1FMV[\x80`@RP``\x86\x015\x81R`\x80\x86\x015` \x82\x01R`\xA0\x86\x015`@\x82\x01R`\xC0\x86\x015``\x82\x01R\x80\x93PPa#\x8C`\xE0\x86\x01a\"\xD1V[\x91Pa#\x9Ba\x01\0\x86\x01a\x1F\x02V[\x90P\x92\x95\x91\x94P\x92PV[_[\x83\x81\x10\x15a#\xC0W\x81\x81\x01Q\x83\x82\x01R` \x01a#\xA8V[PP_\x91\x01RV[` \x81R_\x82Q\x80` \x84\x01Ra#\xE6\x81`@\x85\x01` \x87\x01a#\xA6V[`\x1F\x01`\x1F\x19\x16\x91\x90\x91\x01`@\x01\x92\x91PPV[_\x80`@\x83\x85\x03\x12\x15a$\x0BW_\x80\xFD[PP\x805\x92` \x90\x91\x015\x91PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a\x0C\x99Wa\x0C\x99a$\x1AV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_\x81a$cWa$ca$\x1AV[P_\x19\x01\x90V[\x80_[`\x07\x81\x10\x15a\x0E\x9AW\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a$mV[a$\xA1\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Qa\x01\0a%\x04\x81\x85\x01\x83\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xA0\x83\x01Q\x91Pa\x01@a%$\x81\x86\x01\x84\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xC0\x84\x01Q\x92Pa\x01\x80a%D\x81\x87\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xE0\x85\x01Q\x93Pa\x01\xC0a%d\x81\x88\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x85\x01Q\x93Pa\x02\0\x92a%\x84\x87\x85\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01 \x86\x01Q\x94Pa\x02@a%\xA5\x81\x89\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x86\x01Q\x94Pa\x02\x80\x92a%\xC5\x88\x85\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01`\x87\x01Q\x95Pa\x02\xC0a%\xE6\x81\x8A\x01\x88\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x87\x01Q\x80Qa\x03\0\x8A\x01R` \x01Qa\x03 \x89\x01Ra\x01\xA0\x87\x01Qa\x03@\x89\x01R\x90\x86\x01Qa\x03`\x88\x01Ra\x01\xE0\x86\x01Qa\x03\x80\x88\x01R\x92\x85\x01Qa\x03\xA0\x87\x01Ra\x02 \x85\x01Qa\x03\xC0\x87\x01R\x91\x84\x01Qa\x03\xE0\x86\x01Ra\x02`\x84\x01Qa\x04\0\x86\x01R\x83\x01Qa\x04 \x85\x01Ra\x02\xA0\x83\x01Qa\x04@\x85\x01R\x90\x91\x01Qa\x04`\x90\x92\x01\x91\x90\x91RPV[_a\n`\x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa&\xA2`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Qa\x01\0a&\xEF\x81\x85\x01\x83\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xC0\x87\x01Q\x91Pa\x01@a'\x0F\x81\x86\x01\x84\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xE0\x88\x01Q\x92Pa\x01\x80a'/\x81\x87\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x88\x01Q\x92Pa\x01\xC0\x91a'O\x86\x84\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01 \x89\x01Q\x93Pa\x02\0a'p\x81\x88\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x89\x01Q\x93Pa\x02@\x91a'\x90\x87\x84\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01`\x8A\x01Q\x94Pa\x02\x80a'\xB1\x81\x89\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x8A\x01Q\x80Qa\x02\xC0\x89\x01R` \x90\x81\x01Qa\x02\xE0\x89\x01Ra\x01\xA0\x8B\x01Q\x80Qa\x03\0\x8A\x01R\x81\x01Qa\x03 \x89\x01R\x93\x8A\x01Q\x80Qa\x03@\x89\x01R\x84\x01Qa\x03`\x88\x01Ra\x01\xE0\x8A\x01Q\x80Qa\x03\x80\x89\x01R\x84\x01Qa\x03\xA0\x88\x01R\x89\x01Q\x80Qa\x03\xC0\x88\x01R\x83\x01Qa\x03\xE0\x87\x01Ra\x02 \x89\x01Q\x80Qa\x04\0\x88\x01R\x83\x01Qa\x04 \x87\x01R\x90\x88\x01Q\x80Qa\x04@\x87\x01R\x82\x01Qa\x04`\x86\x01Ra\x02`\x88\x01Q\x80Qa\x04\x80\x87\x01R\x90\x91\x01Qa\x04\xA0\x85\x01R\x86\x01Qa\x04\xC0\x84\x01RPa\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra(\x88a\x05\0\x83\x01\x85a$jV[a(\x96a\x05\xE0\x83\x01\x84a$\x8CV[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a(\xAEW_\x80\xFD[\x81Q\x80\x15\x15\x81\x14a\x1CNW_\x80\xFD[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15a(\xDDWa(\xDDa$\x1AV[P\x92\x91PPV[_`\x01`\x01`@\x1B\x03\x80\x83\x16\x81\x81\x03a(\xFFWa(\xFFa$\x1AV[`\x01\x01\x93\x92PPPV[_` \x82\x84\x03\x12\x15a)\x19W_\x80\xFD[PQ\x91\x90PV[_\x82Qa)1\x81\x84` \x87\x01a#\xA6V[\x91\x90\x91\x01\x92\x91PPV\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405260043610610147575f3560e01c8063826e41fc116100b3578063ad3cb1cc1161006d578063ad3cb1cc1461046a578063c23b9e9e146104a7578063d24d933d146104df578063e03033011461050e578063f2fde38b1461052d578063f9e50d191461054c575f80fd5b8063826e41fc146103345780638584d23f1461035f5780638da5cb5b1461039b57806396c1ca61146103d75780639baa3cc9146103f65780639fdb54a714610415575f80fd5b8063378ec23b11610104578063378ec23b14610288578063426d3194146102a45780634f1ef286146102e557806352d1902d146102f857806369cc6a041461030c578063715018a614610320575f80fd5b8063013fa5fc1461014b57806302b592f31461016c5780630d8e6e2c146101c95780632063d4f7146101f45780632f79889d14610213578063313df7b114610251575b5f80fd5b348015610156575f80fd5b5061016a610165366004611f1d565b610560565b005b348015610177575f80fd5b5061018b610186366004611f36565b610613565b6040516101c094939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b3480156101d4575f80fd5b5060408051600181525f60208201819052918101919091526060016101c0565b3480156101ff575f80fd5b5061016a61020e366004612081565b61065c565b34801561021e575f80fd5b5060085461023990600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016101c0565b34801561025c575f80fd5b50600854610270906001600160a01b031681565b6040516001600160a01b0390911681526020016101c0565b348015610293575f80fd5b50435b6040519081526020016101c0565b3480156102af575f80fd5b505f546001546002546003546102c59392919084565b6040805194855260208501939093529183015260608201526080016101c0565b61016a6102f3366004612232565b6107b6565b348015610303575f80fd5b506102966107d5565b348015610317575f80fd5b5061016a6107f0565b34801561032b575f80fd5b5061016a61085e565b34801561033f575f80fd5b506008546001600160a01b031615155b60405190151581526020016101c0565b34801561036a575f80fd5b5061037e610379366004611f36565b61086f565b604080519283526001600160401b039091166020830152016101c0565b3480156103a6575f80fd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b0316610270565b3480156103e2575f80fd5b5061016a6103f13660046122e4565b61099a565b348015610401575f80fd5b5061016a6104103660046122fd565b610a23565b348015610420575f80fd5b50600654600754610444916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016101c0565b348015610475575f80fd5b5061049a604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516101c091906123c8565b3480156104b2575f80fd5b506008546104ca90600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016101c0565b3480156104ea575f80fd5b50600454600554610444916001600160401b0380821692600160401b909204169083565b348015610519575f80fd5b5061034f6105283660046123fa565b610b45565b348015610538575f80fd5b5061016a610547366004611f1d565b610c9f565b348015610557575f80fd5b50600954610296565b610568610ce1565b6001600160a01b03811661058f5760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b03908116908216036105be5760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b60098181548110610622575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b6008546001600160a01b03161515801561068157506008546001600160a01b03163314155b1561069f576040516301474c8f60e71b815260040160405180910390fd5b60065482516001600160401b0391821691161115806106d8575060065460208301516001600160401b03600160401b9092048216911611155b156106f65760405163051c46ef60e01b815260040160405180910390fd5b6107038260400151610d3c565b61070d8282610dac565b81516006805460208501516001600160401b03908116600160401b026001600160801b0319909216931692909217919091179055604082015160075561075a6107534390565b4284610ea0565b81602001516001600160401b0316825f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae684604001516040516107aa91815260200190565b60405180910390a35050565b6107be611089565b6107c78261112d565b6107d1828261116e565b5050565b5f6107de61122f565b505f8051602061293c83398151915290565b6107f8610ce1565b6008546001600160a01b03161561084357600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b610866610ce1565b61085c5f611278565b600980545f9182919061088360018361242e565b8154811061089357610893612441565b5f918252602090912060029091020154600160801b90046001600160401b031684106108d257604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b8181101561099357846009828154811061090257610902612441565b5f918252602090912060029091020154600160801b90046001600160401b0316111561098b576009818154811061093b5761093b612441565b905f5260205f209060020201600101546009828154811061095e5761095e612441565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b6001016108e6565b5050915091565b6109a2610ce1565b610e108163ffffffff1610806109c157506301e133808163ffffffff16115b806109df575060085463ffffffff600160a01b909104811690821611155b156109fd576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a008054600160401b810460ff1615906001600160401b03165f81158015610a675750825b90505f826001600160401b03166001148015610a825750303b155b905081158015610a90575080155b15610aae5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610ad857845460ff60401b1916600160401b1785555b610ae1866112e8565b610ae96112f9565b610af4898989611301565b8315610b3a57845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6009545f9043841180610b56575080155b80610ba05750600854600980549091600160c01b90046001600160401b0316908110610b8457610b84612441565b5f9182526020909120600290910201546001600160401b031684105b15610bbe5760405163b0b4387760e01b815260040160405180910390fd5b5f8080610bcc60018561242e565b90505b81610c6857600854600160c01b90046001600160401b03168110610c68578660098281548110610c0157610c01612441565b5f9182526020909120600290910201546001600160401b031611610c56576001915060098181548110610c3657610c36612441565b5f9182526020909120600290910201546001600160401b03169250610c68565b80610c6081612455565b915050610bcf565b81610c865760405163b0b4387760e01b815260040160405180910390fd5b85610c91848961242e565b119450505050505b92915050565b610ca7610ce1565b6001600160a01b038116610cd557604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610cde81611278565b50565b33610d137f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b03161461085c5760405163118cdaa760e01b8152336004820152602401610ccc565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f00000018110806107d15760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401610ccc565b5f610db561142d565b9050610dbf611c7e565b83516001600160401b0390811682526020850151168160016020020152604084810151828201526001546060830152600254608083015260035460a08301525f5460c08301525163ce537a7760e01b8152735fbdb2315678afecb367f032d93f642f64180aa39063ce537a7790610e3e90859085908890600401612670565b602060405180830381865af4158015610e59573d5f803e3d5ffd5b505050506040513d601f19601f82011682018060405250810190610e7d919061289e565b610e9a576040516309bde33960e01b815260040160405180910390fd5b50505050565b60095415801590610f15575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b0316908110610ee057610ee0612441565b5f918252602090912060029091020154610f0a90600160401b90046001600160401b0316846128bd565b6001600160401b0316115b15610fa857600854600980549091600160c01b90046001600160401b0316908110610f4257610f42612441565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b0316906018610f82836128e4565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016148061110f57507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b03166111035f8051602061293c833981519152546001600160a01b031690565b6001600160a01b031614155b1561085c5760405163703e46dd60e11b815260040160405180910390fd5b611135610ce1565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d90602001610608565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa9250505080156111c8575060408051601f3d908101601f191682019092526111c591810190612909565b60015b6111f057604051634c9c8ce360e01b81526001600160a01b0383166004820152602401610ccc565b5f8051602061293c833981519152811461122057604051632a87526960e21b815260048101829052602401610ccc565b61122a8383611a5c565b505050565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161461085c5760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b6112f0611ab1565b610cde81611afa565b61085c611ab1565b82516001600160401b0316151580611325575060208301516001600160401b031615155b8061133257506020820151155b8061133f57506040820151155b8061134c57506060820151155b8061135657508151155b806113685750610e108163ffffffff16105b8061137c57506301e133808163ffffffff16115b1561139a576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b611435611c9c565b621000008152600760208201527f23783d0e9777b7af65fbf849da7edb75e74b1eaf503e025d7f2f7f80991befa26040820151527f2a4e2fe8adfa53f468525582d5184c4c70bbdb946c21f216418a9879705e54a76020604083015101527f0624b2c1e77f24afceaf39451743b9fa80d5853fca7ba00389c675650774009b6060820151527f250d7719e94ca2df00dfe327938f5a8d4d837779b99837ca777a53d39127b1796020606083015101527f0dc09515152eaea66d0db2f571cc995e369d26fe647394f10db5398c917519dc6080820151527f1273144d6cec2c4a68b24a149379c0f5592bb7fbddbe32fa171919950ca404cb6020608083015101527f119521bb68caec216e2f05eeb466fb3abfe1f39baf7fe7cb392ea057b6a2d9bf60a0820151527f2d52adeaba8045e53ab526fe9982d0ea452def6b3ea0253d27a19ef3b46e8428602060a083015101527f16c3b5b217d302975a920d13374524d7a52e4a50fd7fb930842271ebf4a84efd60c0820151527f200788916b907b196972bde304318e885a2521514b2db5e4a11899c51204f089602060c083015101527f1127581afe753defca9aef12e7332db9978a200b1699ce3888c0f3aea6111dc360e0820151527f0881e13f00723be1a04872ed02b2d078c31e80feaf27724e262ce97c2cb0bb1d602060e083015101527f1482a3a6bb91d6483d153683e2404f2f5546e0e895530fdf132091498406e3de610100820151527efa52db3d52d905ead1248102f3a80a43a90d8400c68f60a62c543c417b2f4b602061010083015101527f0a57dadd4a55199525ac6ac6fabc87a4cccfdc98142bcef9dbf47de00ecc5164610120820151527f18d95abd9b8e12c36936aa218cfff582548a6bbff25c338c2006eaeb1fe5b696602061012083015101527f2bc40e91dd169b8bc143a02952a1b6c6e627bfeb7a2bbe5078e14123f3c54c1c610140820151527f108d65a20c579b6d9883275eb6889fc3f5fc79735ca9f611a13b67daa2fbc8d0602061014083015101527f21bc1f86d0608e5f0626b467ee6f8282b619223f60a7acb0fc63ba7bdaf783be610160820151527f05ef3282f8eef01515fb9a8a7d6ca06b8b007d1d512403efb268fb03ce5f09e9602061016083015101527f2cab66c1cb5a83869e73ac34fbe467486999babd541d9010ee9a804806eee4ef610180820151527f2db1982419c5a4a17593acff9535fa967683d75c8aec01319b64b84aada2ad84602061018083015101527f2c38667c6c7eb868bdd30c34dd3f4b84d9b9b1a27d7867b364c8b7831423e9086101a0820151527f2b2cb4044dd51165c48138219d51cf8d1689f91ed3eeefead6e055eb488a2ce260206101a083015101527f2d48e54703011df2c74a14dafde3af4fd83ec71875d8ddc3554658640cc955016101c0820151527f243a99d80d32eb5408b59d5b08302bede070d3fb0a8efe2f2262f865bffb4d0d60206101c083015101527f0455d2325bf6269a66f07d838f55f36947a3cd9b87edd8480bced95cbb45cc116101e0820151527f0f66d9085a6ed60b838179987e240992bff4c0516ccf6ccde4a1ca94ce8b986460206101e083015101527f2bac0d23c8585d1487ec611b5effc97e5852fea43a7cba36ccdd2c207931f394610200820151527f1860b54e01a06aea5adb4b13bf5baebab92b736807a3a89ff2040992b06ee6ec602061020083015101527f0c0bfa1c2fc6f8ed01233d5168db1e1dfe725504f032f669f50a92ae77c72906610220820151527f0d741e124c7d1069b8a400cbcdcfd90128a533901ad4de1e037fe72984dc34cf602061022083015101527f01cfed30085c9efce04668205794aa39b1a8ee591234b4c77a22f8c26d899e05610240820151527f2ab68ac82d36cedb647d14a5b0035e8c9a0be84780b7bae1133a27a880966ed1602061024083015101527f072e1d50f8b5cf8d574b3847276477d95bbd5116351000841f728da44f4043b5610260820151527f23f8ea6eacd0876d57220f57eabacbe76a2323411663731a251d5dca36f1b59f602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b611a6582611b02565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a2805115611aa95761122a8282611b65565b6107d1611bd7565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a0054600160401b900460ff1661085c57604051631afcd79f60e31b815260040160405180910390fd5b610ca7611ab1565b806001600160a01b03163b5f03611b3757604051634c9c8ce360e01b81526001600160a01b0382166004820152602401610ccc565b5f8051602061293c83398151915280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f80846001600160a01b031684604051611b819190612920565b5f60405180830381855af49150503d805f8114611bb9576040519150601f19603f3d011682016040523d82523d5f602084013e611bbe565b606091505b5091509150611bce858383611bf6565b95945050505050565b341561085c5760405163b398979f60e01b815260040160405180910390fd5b606082611c0b57611c0682611c55565b611c4e565b8151158015611c2257506001600160a01b0384163b155b15611c4b57604051639996b31560e01b81526001600160a01b0385166004820152602401610ccc565b50805b9392505050565b805115611c655780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b6040518060e001604052806007906020820280368337509192915050565b604051806102c001604052805f81526020015f8152602001611ccf60405180604001604052805f81526020015f81525090565b8152602001611cef60405180604001604052805f81526020015f81525090565b8152602001611d0f60405180604001604052805f81526020015f81525090565b8152602001611d2f60405180604001604052805f81526020015f81525090565b8152602001611d4f60405180604001604052805f81526020015f81525090565b8152602001611d6f60405180604001604052805f81526020015f81525090565b8152602001611d8f60405180604001604052805f81526020015f81525090565b8152602001611daf60405180604001604052805f81526020015f81525090565b8152602001611dcf60405180604001604052805f81526020015f81525090565b8152602001611def60405180604001604052805f81526020015f81525090565b8152602001611e0f60405180604001604052805f81526020015f81525090565b8152602001611e2f60405180604001604052805f81526020015f81525090565b8152602001611e4f60405180604001604052805f81526020015f81525090565b8152602001611e6f60405180604001604052805f81526020015f81525090565b8152602001611e8f60405180604001604052805f81526020015f81525090565b8152602001611eaf60405180604001604052805f81526020015f81525090565b8152602001611ecf60405180604001604052805f81526020015f81525090565b8152602001611eef60405180604001604052805f81526020015f81525090565b81525f6020820181905260409091015290565b80356001600160a01b0381168114611f18575f80fd5b919050565b5f60208284031215611f2d575f80fd5b611c4e82611f02565b5f60208284031215611f46575f80fd5b5035919050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715611f8457611f84611f4d565b60405290565b604051601f8201601f191681016001600160401b0381118282101715611fb257611fb2611f4d565b604052919050565b80356001600160401b0381168114611f18575f80fd5b5f60608284031215611fe0575f80fd5b604051606081018181106001600160401b038211171561200257612002611f4d565b60405290508061201183611fba565b815261201f60208401611fba565b6020820152604083013560408201525092915050565b5f60408284031215612045575f80fd5b604051604081018181106001600160401b038211171561206757612067611f4d565b604052823581526020928301359281019290925250919050565b5f808284036104e0811215612094575f80fd5b61209e8585611fd0565b925061048080605f19830112156120b3575f80fd5b6120bb611f61565b91506120ca8660608701612035565b82526120d98660a08701612035565b60208301526120eb8660e08701612035565b60408301526101206120ff87828801612035565b606084015261016061211388828901612035565b60808501526101a061212789828a01612035565b60a08601526101e061213b8a828b01612035565b60c087015261022061214f8b828c01612035565b60e08801526102606121638c828d01612035565b6101008901526102a06121788d828e01612035565b878a015261218a8d6102e08e01612035565b6101408a015261219e8d6103208e01612035565b868a01526121b08d6103608e01612035565b6101808a01526103a08c0135948901949094526103c08b01356101c08901526103e08b0135928801929092526104008a01356102008801526104208a013590870152610440890135610240870152610460890135908601529287013561028085015250506104a0850135908201526104c0909301356102c08401525092909150565b5f8060408385031215612243575f80fd5b61224c83611f02565b91506020808401356001600160401b0380821115612268575f80fd5b818601915086601f83011261227b575f80fd5b81358181111561228d5761228d611f4d565b61229f601f8201601f19168501611f8a565b915080825287848285010111156122b4575f80fd5b80848401858401375f848284010152508093505050509250929050565b803563ffffffff81168114611f18575f80fd5b5f602082840312156122f4575f80fd5b611c4e826122d1565b5f805f80848603610120811215612312575f80fd5b61231c8787611fd0565b94506080605f198201121561232f575f80fd5b50604051608081018181106001600160401b038211171561235257612352611f4d565b8060405250606086013581526080860135602082015260a0860135604082015260c086013560608201528093505061238c60e086016122d1565b915061239b6101008601611f02565b905092959194509250565b5f5b838110156123c05781810151838201526020016123a8565b50505f910152565b602081525f82518060208401526123e68160408501602087016123a6565b601f01601f19169190910160400192915050565b5f806040838503121561240b575f80fd5b50508035926020909101359150565b634e487b7160e01b5f52601160045260245ffd5b81810381811115610c9957610c9961241a565b634e487b7160e01b5f52603260045260245ffd5b5f816124635761246361241a565b505f190190565b805f5b6007811015610e9a57815184526020938401939091019060010161246d565b6124a182825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e08401525060808101516101006125048185018380518252602090810151910152565b60a083015191506101406125248186018480518252602090810151910152565b60c084015192506101806125448187018580518252602090810151910152565b60e085015193506101c06125648188018680518252602090810151910152565b928501519350610200926125848785018680518252602090810151910152565b61012086015194506102406125a58189018780518252602090810151910152565b928601519450610280926125c58885018780518252602090810151910152565b61016087015195506102c06125e6818a018880518252602090810151910152565b9287015180516103008a0152602001516103208901526101a0870151610340890152908601516103608801526101e0860151610380880152928501516103a08701526102208501516103c0870152918401516103e08601526102608401516104008601528301516104208501526102a0830151610440850152909101516104609092019190915250565b5f610a6082019050845182526020850151602083015260408501516126a2604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a08501516101006126ef8185018380518252602090810151910152565b60c0870151915061014061270f8186018480518252602090810151910152565b60e0880151925061018061272f8187018580518252602090810151910152565b9188015192506101c09161274f8684018580518252602090810151910152565b61012089015193506102006127708188018680518252602090810151910152565b918901519350610240916127908784018680518252602090810151910152565b6101608a015194506102806127b18189018780518252602090810151910152565b918a015180516102c08901526020908101516102e08901526101a08b015180516103008a0152810151610320890152938a015180516103408901528401516103608801526101e08a015180516103808901528401516103a088015289015180516103c08801528301516103e087015261022089015180516104008801528301516104208701529088015180516104408701528201516104608601526102608801518051610480870152909101516104a08501528601516104c0840152506102a08501516104e083015261288861050083018561246a565b6128966105e083018461248c565b949350505050565b5f602082840312156128ae575f80fd5b81518015158114611c4e575f80fd5b6001600160401b038281168282160390808211156128dd576128dd61241a565b5092915050565b5f6001600160401b038083168181036128ff576128ff61241a565b6001019392505050565b5f60208284031215612919575f80fd5b5051919050565b5f82516129318184602087016123a6565b919091019291505056fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbca164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x01GW_5`\xE0\x1C\x80c\x82nA\xFC\x11a\0\xB3W\x80c\xAD<\xB1\xCC\x11a\0mW\x80c\xAD<\xB1\xCC\x14a\x04jW\x80c\xC2;\x9E\x9E\x14a\x04\xA7W\x80c\xD2M\x93=\x14a\x04\xDFW\x80c\xE003\x01\x14a\x05\x0EW\x80c\xF2\xFD\xE3\x8B\x14a\x05-W\x80c\xF9\xE5\r\x19\x14a\x05LW_\x80\xFD[\x80c\x82nA\xFC\x14a\x034W\x80c\x85\x84\xD2?\x14a\x03_W\x80c\x8D\xA5\xCB[\x14a\x03\x9BW\x80c\x96\xC1\xCAa\x14a\x03\xD7W\x80c\x9B\xAA<\xC9\x14a\x03\xF6W\x80c\x9F\xDBT\xA7\x14a\x04\x15W_\x80\xFD[\x80c7\x8E\xC2;\x11a\x01\x04W\x80c7\x8E\xC2;\x14a\x02\x88W\x80cBm1\x94\x14a\x02\xA4W\x80cO\x1E\xF2\x86\x14a\x02\xE5W\x80cR\xD1\x90-\x14a\x02\xF8W\x80ci\xCCj\x04\x14a\x03\x0CW\x80cqP\x18\xA6\x14a\x03 W_\x80\xFD[\x80c\x01?\xA5\xFC\x14a\x01KW\x80c\x02\xB5\x92\xF3\x14a\x01lW\x80c\r\x8En,\x14a\x01\xC9W\x80c c\xD4\xF7\x14a\x01\xF4W\x80c/y\x88\x9D\x14a\x02\x13W\x80c1=\xF7\xB1\x14a\x02QW[_\x80\xFD[4\x80\x15a\x01VW_\x80\xFD[Pa\x01ja\x01e6`\x04a\x1F\x1DV[a\x05`V[\0[4\x80\x15a\x01wW_\x80\xFD[Pa\x01\x8Ba\x01\x866`\x04a\x1F6V[a\x06\x13V[`@Qa\x01\xC0\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x01\xD4W_\x80\xFD[P`@\x80Q`\x01\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x01\xC0V[4\x80\x15a\x01\xFFW_\x80\xFD[Pa\x01ja\x02\x0E6`\x04a \x81V[a\x06\\V[4\x80\x15a\x02\x1EW_\x80\xFD[P`\x08Ta\x029\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\\W_\x80\xFD[P`\x08Ta\x02p\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\x93W_\x80\xFD[PC[`@Q\x90\x81R` \x01a\x01\xC0V[4\x80\x15a\x02\xAFW_\x80\xFD[P_T`\x01T`\x02T`\x03Ta\x02\xC5\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x01\xC0V[a\x01ja\x02\xF36`\x04a\"2V[a\x07\xB6V[4\x80\x15a\x03\x03W_\x80\xFD[Pa\x02\x96a\x07\xD5V[4\x80\x15a\x03\x17W_\x80\xFD[Pa\x01ja\x07\xF0V[4\x80\x15a\x03+W_\x80\xFD[Pa\x01ja\x08^V[4\x80\x15a\x03?W_\x80\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15[`@Q\x90\x15\x15\x81R` \x01a\x01\xC0V[4\x80\x15a\x03jW_\x80\xFD[Pa\x03~a\x03y6`\x04a\x1F6V[a\x08oV[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x01\xC0V[4\x80\x15a\x03\xA6W_\x80\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x02pV[4\x80\x15a\x03\xE2W_\x80\xFD[Pa\x01ja\x03\xF16`\x04a\"\xE4V[a\t\x9AV[4\x80\x15a\x04\x01W_\x80\xFD[Pa\x01ja\x04\x106`\x04a\"\xFDV[a\n#V[4\x80\x15a\x04 W_\x80\xFD[P`\x06T`\x07Ta\x04D\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x01\xC0V[4\x80\x15a\x04uW_\x80\xFD[Pa\x04\x9A`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x01\xC0\x91\x90a#\xC8V[4\x80\x15a\x04\xB2W_\x80\xFD[P`\x08Ta\x04\xCA\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x01\xC0V[4\x80\x15a\x04\xEAW_\x80\xFD[P`\x04T`\x05Ta\x04D\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x05\x19W_\x80\xFD[Pa\x03Oa\x05(6`\x04a#\xFAV[a\x0BEV[4\x80\x15a\x058W_\x80\xFD[Pa\x01ja\x05G6`\x04a\x1F\x1DV[a\x0C\x9FV[4\x80\x15a\x05WW_\x80\xFD[P`\tTa\x02\x96V[a\x05ha\x0C\xE1V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x05\x8FW`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\x05\xBEW`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\x06\"W_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x06\x81WP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x06\x9FW`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x82Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x06\xD8WP`\x06T` \x83\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x06\xF6W`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x07\x03\x82`@\x01Qa\r<V[a\x07\r\x82\x82a\r\xACV[\x81Q`\x06\x80T` \x85\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x16\x92\x90\x92\x17\x91\x90\x91\x17\x90U`@\x82\x01Q`\x07Ua\x07Za\x07SC\x90V[B\x84a\x0E\xA0V[\x81` \x01Q`\x01`\x01`@\x1B\x03\x16\x82_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x84`@\x01Q`@Qa\x07\xAA\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPV[a\x07\xBEa\x10\x89V[a\x07\xC7\x82a\x11-V[a\x07\xD1\x82\x82a\x11nV[PPV[_a\x07\xDEa\x12/V[P_\x80Q` a)<\x839\x81Q\x91R\x90V[a\x07\xF8a\x0C\xE1V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x08CW`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x08fa\x0C\xE1V[a\x08\\_a\x12xV[`\t\x80T_\x91\x82\x91\x90a\x08\x83`\x01\x83a$.V[\x81T\x81\x10a\x08\x93Wa\x08\x93a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x08\xD2W`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\t\x93W\x84`\t\x82\x81T\x81\x10a\t\x02Wa\t\x02a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\t\x8BW`\t\x81\x81T\x81\x10a\t;Wa\t;a$AV[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\t^Wa\t^a$AV[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x08\xE6V[PP\x91P\x91V[a\t\xA2a\x0C\xE1V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\t\xC1WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\t\xDFWP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\t\xFDW`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\ngWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\n\x82WP0;\x15[\x90P\x81\x15\x80\x15a\n\x90WP\x80\x15[\x15a\n\xAEW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\n\xD8W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\n\xE1\x86a\x12\xE8V[a\n\xE9a\x12\xF9V[a\n\xF4\x89\x89\x89a\x13\x01V[\x83\x15a\x0B:W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\tT_\x90C\x84\x11\x80a\x0BVWP\x80\x15[\x80a\x0B\xA0WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0B\x84Wa\x0B\x84a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a\x0B\xBEW`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a\x0B\xCC`\x01\x85a$.V[\x90P[\x81a\x0ChW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a\x0ChW\x86`\t\x82\x81T\x81\x10a\x0C\x01Wa\x0C\x01a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a\x0CVW`\x01\x91P`\t\x81\x81T\x81\x10a\x0C6Wa\x0C6a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa\x0ChV[\x80a\x0C`\x81a$UV[\x91PPa\x0B\xCFV[\x81a\x0C\x86W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a\x0C\x91\x84\x89a$.V[\x11\x94PPPPP[\x92\x91PPV[a\x0C\xA7a\x0C\xE1V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0C\xD5W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x0C\xDE\x81a\x12xV[PV[3a\r\x13\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x08\\W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x0C\xCCV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x07\xD1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x0C\xCCV[_a\r\xB5a\x14-V[\x90Pa\r\xBFa\x1C~V[\x83Q`\x01`\x01`@\x1B\x03\x90\x81\x16\x82R` \x85\x01Q\x16\x81`\x01` \x02\x01R`@\x84\x81\x01Q\x82\x82\x01R`\x01T``\x83\x01R`\x02T`\x80\x83\x01R`\x03T`\xA0\x83\x01R_T`\xC0\x83\x01RQc\xCESzw`\xE0\x1B\x81Rs_\xBD\xB21Vx\xAF\xEC\xB3g\xF02\xD9?d/d\x18\n\xA3\x90c\xCESzw\x90a\x0E>\x90\x85\x90\x85\x90\x88\x90`\x04\x01a&pV[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a\x0EYW=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x0E}\x91\x90a(\x9EV[a\x0E\x9AW`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPV[`\tT\x15\x80\x15\x90a\x0F\x15WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0E\xE0Wa\x0E\xE0a$AV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta\x0F\n\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a(\xBDV[`\x01`\x01`@\x1B\x03\x16\x11[\x15a\x0F\xA8W`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x0FBWa\x0FBa$AV[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a\x0F\x82\x83a(\xE4V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x11\x0FWP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x11\x03_\x80Q` a)<\x839\x81Q\x91RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x08\\W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x115a\x0C\xE1V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x06\x08V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x11\xC8WP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x11\xC5\x91\x81\x01\x90a)\tV[`\x01[a\x11\xF0W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[_\x80Q` a)<\x839\x81Q\x91R\x81\x14a\x12 W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x0C\xCCV[a\x12*\x83\x83a\x1A\\V[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x08\\W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x12\xF0a\x1A\xB1V[a\x0C\xDE\x81a\x1A\xFAV[a\x08\\a\x1A\xB1V[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a\x13%WP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a\x132WP` \x82\x01Q\x15[\x80a\x13?WP`@\x82\x01Q\x15[\x80a\x13LWP``\x82\x01Q\x15[\x80a\x13VWP\x81Q\x15[\x80a\x13hWPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a\x13|WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a\x13\x9AW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[a\x145a\x1C\x9CV[b\x10\0\0\x81R`\x07` \x82\x01R\x7F#x=\x0E\x97w\xB7\xAFe\xFB\xF8I\xDA~\xDBu\xE7K\x1E\xAFP>\x02]\x7F/\x7F\x80\x99\x1B\xEF\xA2`@\x82\x01QR\x7F*N/\xE8\xAD\xFAS\xF4hRU\x82\xD5\x18LLp\xBB\xDB\x94l!\xF2\x16A\x8A\x98yp^T\xA7` `@\x83\x01Q\x01R\x7F\x06$\xB2\xC1\xE7\x7F$\xAF\xCE\xAF9E\x17C\xB9\xFA\x80\xD5\x85?\xCA{\xA0\x03\x89\xC6ue\x07t\0\x9B``\x82\x01QR\x7F%\rw\x19\xE9L\xA2\xDF\0\xDF\xE3'\x93\x8FZ\x8DM\x83wy\xB9\x987\xCAwzS\xD3\x91'\xB1y` ``\x83\x01Q\x01R\x7F\r\xC0\x95\x15\x15.\xAE\xA6m\r\xB2\xF5q\xCC\x99^6\x9D&\xFEds\x94\xF1\r\xB59\x8C\x91u\x19\xDC`\x80\x82\x01QR\x7F\x12s\x14Ml\xEC,Jh\xB2J\x14\x93y\xC0\xF5Y+\xB7\xFB\xDD\xBE2\xFA\x17\x19\x19\x95\x0C\xA4\x04\xCB` `\x80\x83\x01Q\x01R\x7F\x11\x95!\xBBh\xCA\xEC!n/\x05\xEE\xB4f\xFB:\xBF\xE1\xF3\x9B\xAF\x7F\xE7\xCB9.\xA0W\xB6\xA2\xD9\xBF`\xA0\x82\x01QR\x7F-R\xAD\xEA\xBA\x80E\xE5:\xB5&\xFE\x99\x82\xD0\xEAE-\xEFk>\xA0%='\xA1\x9E\xF3\xB4n\x84(` `\xA0\x83\x01Q\x01R\x7F\x16\xC3\xB5\xB2\x17\xD3\x02\x97Z\x92\r\x137E$\xD7\xA5.JP\xFD\x7F\xB90\x84\"q\xEB\xF4\xA8N\xFD`\xC0\x82\x01QR\x7F \x07\x88\x91k\x90{\x19ir\xBD\xE3\x041\x8E\x88Z%!QK-\xB5\xE4\xA1\x18\x99\xC5\x12\x04\xF0\x89` `\xC0\x83\x01Q\x01R\x7F\x11'X\x1A\xFEu=\xEF\xCA\x9A\xEF\x12\xE73-\xB9\x97\x8A \x0B\x16\x99\xCE8\x88\xC0\xF3\xAE\xA6\x11\x1D\xC3`\xE0\x82\x01QR\x7F\x08\x81\xE1?\0r;\xE1\xA0Hr\xED\x02\xB2\xD0x\xC3\x1E\x80\xFE\xAF'rN&,\xE9|,\xB0\xBB\x1D` `\xE0\x83\x01Q\x01R\x7F\x14\x82\xA3\xA6\xBB\x91\xD6H=\x156\x83\xE2@O/UF\xE0\xE8\x95S\x0F\xDF\x13 \x91I\x84\x06\xE3\xDEa\x01\0\x82\x01QR~\xFAR\xDB=R\xD9\x05\xEA\xD1$\x81\x02\xF3\xA8\nC\xA9\r\x84\0\xC6\x8F`\xA6,T<A{/K` a\x01\0\x83\x01Q\x01R\x7F\nW\xDA\xDDJU\x19\x95%\xACj\xC6\xFA\xBC\x87\xA4\xCC\xCF\xDC\x98\x14+\xCE\xF9\xDB\xF4}\xE0\x0E\xCCQda\x01 \x82\x01QR\x7F\x18\xD9Z\xBD\x9B\x8E\x12\xC3i6\xAA!\x8C\xFF\xF5\x82T\x8Ak\xBF\xF2\\3\x8C \x06\xEA\xEB\x1F\xE5\xB6\x96` a\x01 \x83\x01Q\x01R\x7F+\xC4\x0E\x91\xDD\x16\x9B\x8B\xC1C\xA0)R\xA1\xB6\xC6\xE6'\xBF\xEBz+\xBEPx\xE1A#\xF3\xC5L\x1Ca\x01@\x82\x01QR\x7F\x10\x8De\xA2\x0CW\x9Bm\x98\x83'^\xB6\x88\x9F\xC3\xF5\xFCys\\\xA9\xF6\x11\xA1;g\xDA\xA2\xFB\xC8\xD0` a\x01@\x83\x01Q\x01R\x7F!\xBC\x1F\x86\xD0`\x8E_\x06&\xB4g\xEEo\x82\x82\xB6\x19\"?`\xA7\xAC\xB0\xFCc\xBA{\xDA\xF7\x83\xBEa\x01`\x82\x01QR\x7F\x05\xEF2\x82\xF8\xEE\xF0\x15\x15\xFB\x9A\x8A}l\xA0k\x8B\0}\x1DQ$\x03\xEF\xB2h\xFB\x03\xCE_\t\xE9` a\x01`\x83\x01Q\x01R\x7F,\xABf\xC1\xCBZ\x83\x86\x9Es\xAC4\xFB\xE4gHi\x99\xBA\xBDT\x1D\x90\x10\xEE\x9A\x80H\x06\xEE\xE4\xEFa\x01\x80\x82\x01QR\x7F-\xB1\x98$\x19\xC5\xA4\xA1u\x93\xAC\xFF\x955\xFA\x96v\x83\xD7\\\x8A\xEC\x011\x9Bd\xB8J\xAD\xA2\xAD\x84` a\x01\x80\x83\x01Q\x01R\x7F,8f|l~\xB8h\xBD\xD3\x0C4\xDD?K\x84\xD9\xB9\xB1\xA2}xg\xB3d\xC8\xB7\x83\x14#\xE9\x08a\x01\xA0\x82\x01QR\x7F+,\xB4\x04M\xD5\x11e\xC4\x818!\x9DQ\xCF\x8D\x16\x89\xF9\x1E\xD3\xEE\xEF\xEA\xD6\xE0U\xEBH\x8A,\xE2` a\x01\xA0\x83\x01Q\x01R\x7F-H\xE5G\x03\x01\x1D\xF2\xC7J\x14\xDA\xFD\xE3\xAFO\xD8>\xC7\x18u\xD8\xDD\xC3UFXd\x0C\xC9U\x01a\x01\xC0\x82\x01QR\x7F$:\x99\xD8\r2\xEBT\x08\xB5\x9D[\x080+\xED\xE0p\xD3\xFB\n\x8E\xFE/\"b\xF8e\xBF\xFBM\r` a\x01\xC0\x83\x01Q\x01R\x7F\x04U\xD22[\xF6&\x9Af\xF0}\x83\x8FU\xF3iG\xA3\xCD\x9B\x87\xED\xD8H\x0B\xCE\xD9\\\xBBE\xCC\x11a\x01\xE0\x82\x01QR\x7F\x0Ff\xD9\x08Zn\xD6\x0B\x83\x81y\x98~$\t\x92\xBF\xF4\xC0Ql\xCFl\xCD\xE4\xA1\xCA\x94\xCE\x8B\x98d` a\x01\xE0\x83\x01Q\x01R\x7F+\xAC\r#\xC8X]\x14\x87\xECa\x1B^\xFF\xC9~XR\xFE\xA4:|\xBA6\xCC\xDD, y1\xF3\x94a\x02\0\x82\x01QR\x7F\x18`\xB5N\x01\xA0j\xEAZ\xDBK\x13\xBF[\xAE\xBA\xB9+sh\x07\xA3\xA8\x9F\xF2\x04\t\x92\xB0n\xE6\xEC` a\x02\0\x83\x01Q\x01R\x7F\x0C\x0B\xFA\x1C/\xC6\xF8\xED\x01#=Qh\xDB\x1E\x1D\xFErU\x04\xF02\xF6i\xF5\n\x92\xAEw\xC7)\x06a\x02 \x82\x01QR\x7F\rt\x1E\x12L}\x10i\xB8\xA4\0\xCB\xCD\xCF\xD9\x01(\xA53\x90\x1A\xD4\xDE\x1E\x03\x7F\xE7)\x84\xDC4\xCF` a\x02 \x83\x01Q\x01R\x7F\x01\xCF\xED0\x08\\\x9E\xFC\xE0Fh W\x94\xAA9\xB1\xA8\xEEY\x124\xB4\xC7z\"\xF8\xC2m\x89\x9E\x05a\x02@\x82\x01QR\x7F*\xB6\x8A\xC8-6\xCE\xDBd}\x14\xA5\xB0\x03^\x8C\x9A\x0B\xE8G\x80\xB7\xBA\xE1\x13:'\xA8\x80\x96n\xD1` a\x02@\x83\x01Q\x01R\x7F\x07.\x1DP\xF8\xB5\xCF\x8DWK8G'dw\xD9[\xBDQ\x165\x10\0\x84\x1Fr\x8D\xA4O@C\xB5a\x02`\x82\x01QR\x7F#\xF8\xEAn\xAC\xD0\x87mW\"\x0FW\xEA\xBA\xCB\xE7j##A\x16cs\x1A%\x1D]\xCA6\xF1\xB5\x9F` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[a\x1Ae\x82a\x1B\x02V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a\x1A\xA9Wa\x12*\x82\x82a\x1BeV[a\x07\xD1a\x1B\xD7V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0T`\x01`@\x1B\x90\x04`\xFF\x16a\x08\\W`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x0C\xA7a\x1A\xB1V[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a\x1B7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[_\x80Q` a)<\x839\x81Q\x91R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``_\x80\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa\x1B\x81\x91\x90a) V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a\x1B\xB9W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a\x1B\xBEV[``\x91P[P\x91P\x91Pa\x1B\xCE\x85\x83\x83a\x1B\xF6V[\x95\x94PPPPPV[4\x15a\x08\\W`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a\x1C\x0BWa\x1C\x06\x82a\x1CUV[a\x1CNV[\x81Q\x15\x80\x15a\x1C\"WP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a\x1CKW`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x0C\xCCV[P\x80[\x93\x92PPPV[\x80Q\x15a\x1CeW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80`\xE0\x01`@R\x80`\x07\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a\x1C\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1C\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\x0F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D/`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1DO`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1Do`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\x8F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xAF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1D\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\x0F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E/`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1EO`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1Eo`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\x8F`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xAF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xCF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a\x1E\xEF`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R_` \x82\x01\x81\x90R`@\x90\x91\x01R\x90V[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1F-W_\x80\xFD[a\x1CN\x82a\x1F\x02V[_` \x82\x84\x03\x12\x15a\x1FFW_\x80\xFD[P5\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x1F\x84Wa\x1F\x84a\x1FMV[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a\x1F\xB2Wa\x1F\xB2a\x1FMV[`@R\x91\x90PV[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[_``\x82\x84\x03\x12\x15a\x1F\xE0W_\x80\xFD[`@Q``\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a \x02Wa \x02a\x1FMV[`@R\x90P\x80a \x11\x83a\x1F\xBAV[\x81Ra \x1F` \x84\x01a\x1F\xBAV[` \x82\x01R`@\x83\x015`@\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a EW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a gWa ga\x1FMV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80\x82\x84\x03a\x04\xE0\x81\x12\x15a \x94W_\x80\xFD[a \x9E\x85\x85a\x1F\xD0V[\x92Pa\x04\x80\x80`_\x19\x83\x01\x12\x15a \xB3W_\x80\xFD[a \xBBa\x1FaV[\x91Pa \xCA\x86``\x87\x01a 5V[\x82Ra \xD9\x86`\xA0\x87\x01a 5V[` \x83\x01Ra \xEB\x86`\xE0\x87\x01a 5V[`@\x83\x01Ra\x01 a \xFF\x87\x82\x88\x01a 5V[``\x84\x01Ra\x01`a!\x13\x88\x82\x89\x01a 5V[`\x80\x85\x01Ra\x01\xA0a!'\x89\x82\x8A\x01a 5V[`\xA0\x86\x01Ra\x01\xE0a!;\x8A\x82\x8B\x01a 5V[`\xC0\x87\x01Ra\x02 a!O\x8B\x82\x8C\x01a 5V[`\xE0\x88\x01Ra\x02`a!c\x8C\x82\x8D\x01a 5V[a\x01\0\x89\x01Ra\x02\xA0a!x\x8D\x82\x8E\x01a 5V[\x87\x8A\x01Ra!\x8A\x8Da\x02\xE0\x8E\x01a 5V[a\x01@\x8A\x01Ra!\x9E\x8Da\x03 \x8E\x01a 5V[\x86\x8A\x01Ra!\xB0\x8Da\x03`\x8E\x01a 5V[a\x01\x80\x8A\x01Ra\x03\xA0\x8C\x015\x94\x89\x01\x94\x90\x94Ra\x03\xC0\x8B\x015a\x01\xC0\x89\x01Ra\x03\xE0\x8B\x015\x92\x88\x01\x92\x90\x92Ra\x04\0\x8A\x015a\x02\0\x88\x01Ra\x04 \x8A\x015\x90\x87\x01Ra\x04@\x89\x015a\x02@\x87\x01Ra\x04`\x89\x015\x90\x86\x01R\x92\x87\x015a\x02\x80\x85\x01RPPa\x04\xA0\x85\x015\x90\x82\x01Ra\x04\xC0\x90\x93\x015a\x02\xC0\x84\x01RP\x92\x90\x91PV[_\x80`@\x83\x85\x03\x12\x15a\"CW_\x80\xFD[a\"L\x83a\x1F\x02V[\x91P` \x80\x84\x015`\x01`\x01`@\x1B\x03\x80\x82\x11\x15a\"hW_\x80\xFD[\x81\x86\x01\x91P\x86`\x1F\x83\x01\x12a\"{W_\x80\xFD[\x815\x81\x81\x11\x15a\"\x8DWa\"\x8Da\x1FMV[a\"\x9F`\x1F\x82\x01`\x1F\x19\x16\x85\x01a\x1F\x8AV[\x91P\x80\x82R\x87\x84\x82\x85\x01\x01\x11\x15a\"\xB4W_\x80\xFD[\x80\x84\x84\x01\x85\x84\x017_\x84\x82\x84\x01\x01RP\x80\x93PPPP\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x1F\x18W_\x80\xFD[_` \x82\x84\x03\x12\x15a\"\xF4W_\x80\xFD[a\x1CN\x82a\"\xD1V[_\x80_\x80\x84\x86\x03a\x01 \x81\x12\x15a#\x12W_\x80\xFD[a#\x1C\x87\x87a\x1F\xD0V[\x94P`\x80`_\x19\x82\x01\x12\x15a#/W_\x80\xFD[P`@Q`\x80\x81\x01\x81\x81\x10`\x01`\x01`@\x1B\x03\x82\x11\x17\x15a#RWa#Ra\x1FMV[\x80`@RP``\x86\x015\x81R`\x80\x86\x015` \x82\x01R`\xA0\x86\x015`@\x82\x01R`\xC0\x86\x015``\x82\x01R\x80\x93PPa#\x8C`\xE0\x86\x01a\"\xD1V[\x91Pa#\x9Ba\x01\0\x86\x01a\x1F\x02V[\x90P\x92\x95\x91\x94P\x92PV[_[\x83\x81\x10\x15a#\xC0W\x81\x81\x01Q\x83\x82\x01R` \x01a#\xA8V[PP_\x91\x01RV[` \x81R_\x82Q\x80` \x84\x01Ra#\xE6\x81`@\x85\x01` \x87\x01a#\xA6V[`\x1F\x01`\x1F\x19\x16\x91\x90\x91\x01`@\x01\x92\x91PPV[_\x80`@\x83\x85\x03\x12\x15a$\x0BW_\x80\xFD[PP\x805\x92` \x90\x91\x015\x91PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a\x0C\x99Wa\x0C\x99a$\x1AV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_\x81a$cWa$ca$\x1AV[P_\x19\x01\x90V[\x80_[`\x07\x81\x10\x15a\x0E\x9AW\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a$mV[a$\xA1\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Qa\x01\0a%\x04\x81\x85\x01\x83\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xA0\x83\x01Q\x91Pa\x01@a%$\x81\x86\x01\x84\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xC0\x84\x01Q\x92Pa\x01\x80a%D\x81\x87\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xE0\x85\x01Q\x93Pa\x01\xC0a%d\x81\x88\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x85\x01Q\x93Pa\x02\0\x92a%\x84\x87\x85\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01 \x86\x01Q\x94Pa\x02@a%\xA5\x81\x89\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x86\x01Q\x94Pa\x02\x80\x92a%\xC5\x88\x85\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01`\x87\x01Q\x95Pa\x02\xC0a%\xE6\x81\x8A\x01\x88\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x92\x87\x01Q\x80Qa\x03\0\x8A\x01R` \x01Qa\x03 \x89\x01Ra\x01\xA0\x87\x01Qa\x03@\x89\x01R\x90\x86\x01Qa\x03`\x88\x01Ra\x01\xE0\x86\x01Qa\x03\x80\x88\x01R\x92\x85\x01Qa\x03\xA0\x87\x01Ra\x02 \x85\x01Qa\x03\xC0\x87\x01R\x91\x84\x01Qa\x03\xE0\x86\x01Ra\x02`\x84\x01Qa\x04\0\x86\x01R\x83\x01Qa\x04 \x85\x01Ra\x02\xA0\x83\x01Qa\x04@\x85\x01R\x90\x91\x01Qa\x04`\x90\x92\x01\x91\x90\x91RPV[_a\n`\x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa&\xA2`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Qa\x01\0a&\xEF\x81\x85\x01\x83\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xC0\x87\x01Q\x91Pa\x01@a'\x0F\x81\x86\x01\x84\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[`\xE0\x88\x01Q\x92Pa\x01\x80a'/\x81\x87\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x88\x01Q\x92Pa\x01\xC0\x91a'O\x86\x84\x01\x85\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01 \x89\x01Q\x93Pa\x02\0a'p\x81\x88\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x89\x01Q\x93Pa\x02@\x91a'\x90\x87\x84\x01\x86\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[a\x01`\x8A\x01Q\x94Pa\x02\x80a'\xB1\x81\x89\x01\x87\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[\x91\x8A\x01Q\x80Qa\x02\xC0\x89\x01R` \x90\x81\x01Qa\x02\xE0\x89\x01Ra\x01\xA0\x8B\x01Q\x80Qa\x03\0\x8A\x01R\x81\x01Qa\x03 \x89\x01R\x93\x8A\x01Q\x80Qa\x03@\x89\x01R\x84\x01Qa\x03`\x88\x01Ra\x01\xE0\x8A\x01Q\x80Qa\x03\x80\x89\x01R\x84\x01Qa\x03\xA0\x88\x01R\x89\x01Q\x80Qa\x03\xC0\x88\x01R\x83\x01Qa\x03\xE0\x87\x01Ra\x02 \x89\x01Q\x80Qa\x04\0\x88\x01R\x83\x01Qa\x04 \x87\x01R\x90\x88\x01Q\x80Qa\x04@\x87\x01R\x82\x01Qa\x04`\x86\x01Ra\x02`\x88\x01Q\x80Qa\x04\x80\x87\x01R\x90\x91\x01Qa\x04\xA0\x85\x01R\x86\x01Qa\x04\xC0\x84\x01RPa\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra(\x88a\x05\0\x83\x01\x85a$jV[a(\x96a\x05\xE0\x83\x01\x84a$\x8CV[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a(\xAEW_\x80\xFD[\x81Q\x80\x15\x15\x81\x14a\x1CNW_\x80\xFD[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x80\x82\x11\x15a(\xDDWa(\xDDa$\x1AV[P\x92\x91PPV[_`\x01`\x01`@\x1B\x03\x80\x83\x16\x81\x81\x03a(\xFFWa(\xFFa$\x1AV[`\x01\x01\x93\x92PPPV[_` \x82\x84\x03\x12\x15a)\x19W_\x80\xFD[PQ\x91\x90PV[_\x82Qa)1\x81\x84` \x87\x01a#\xA6V[\x91\x90\x91\x01\x92\x91PPV\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xA1dsolcC\0\x08\x17\0\n",
    );
    /**```solidity
    struct LightClientState { uint64 viewNum; uint64 blockHeight; BN254.ScalarField blockCommRoot; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct LightClientState {
        #[allow(missing_docs)]
        pub viewNum: u64,
        #[allow(missing_docs)]
        pub blockHeight: u64,
        #[allow(missing_docs)]
        pub blockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            alloy::sol_types::sol_data::Uint<64>,
            alloy::sol_types::sol_data::Uint<64>,
            BN254::ScalarField,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            u64,
            u64,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
        impl ::core::convert::From<LightClientState> for UnderlyingRustTuple<'_> {
            fn from(value: LightClientState) -> Self {
                (value.viewNum, value.blockHeight, value.blockCommRoot)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for LightClientState {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    viewNum: tuple.0,
                    blockHeight: tuple.1,
                    blockCommRoot: tuple.2,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for LightClientState {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for LightClientState {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.viewNum,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.blockHeight,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.blockCommRoot),
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
        impl alloy_sol_types::SolType for LightClientState {
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
        impl alloy_sol_types::SolStruct for LightClientState {
            const NAME: &'static str = "LightClientState";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "LightClientState(uint64 viewNum,uint64 blockHeight,uint256 blockCommRoot)",
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.viewNum)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.blockHeight)
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                            &self.blockCommRoot,
                        )
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for LightClientState {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.viewNum,
                    )
                    + <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.blockHeight,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.blockCommRoot,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.viewNum,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.blockHeight,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.blockCommRoot,
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
    /**```solidity
    struct StakeTableState { uint256 threshold; BN254.ScalarField blsKeyComm; BN254.ScalarField schnorrKeyComm; BN254.ScalarField amountComm; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct StakeTableState {
        #[allow(missing_docs)]
        pub threshold: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub blsKeyComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrKeyComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub amountComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            BN254::ScalarField,
            BN254::ScalarField,
            BN254::ScalarField,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
        impl ::core::convert::From<StakeTableState> for UnderlyingRustTuple<'_> {
            fn from(value: StakeTableState) -> Self {
                (
                    value.threshold,
                    value.blsKeyComm,
                    value.schnorrKeyComm,
                    value.amountComm,
                )
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for StakeTableState {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    threshold: tuple.0,
                    blsKeyComm: tuple.1,
                    schnorrKeyComm: tuple.2,
                    amountComm: tuple.3,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for StakeTableState {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for StakeTableState {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.threshold,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.blsKeyComm),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrKeyComm,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.amountComm),
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
        impl alloy_sol_types::SolType for StakeTableState {
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
        impl alloy_sol_types::SolStruct for StakeTableState {
            const NAME: &'static str = "StakeTableState";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "StakeTableState(uint256 threshold,uint256 blsKeyComm,uint256 schnorrKeyComm,uint256 amountComm)",
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.threshold)
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                            &self.blsKeyComm,
                        )
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                            &self.schnorrKeyComm,
                        )
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                            &self.amountComm,
                        )
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for StakeTableState {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.threshold,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.blsKeyComm,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.schnorrKeyComm,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.amountComm,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.threshold,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.blsKeyComm,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.schnorrKeyComm,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.amountComm,
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
    /**Custom error with signature `InsufficientSnapshotHistory()` and selector `0xb0b43877`.
    ```solidity
    error InsufficientSnapshotHistory();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InsufficientSnapshotHistory {}
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
        impl ::core::convert::From<InsufficientSnapshotHistory> for UnderlyingRustTuple<'_> {
            fn from(value: InsufficientSnapshotHistory) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InsufficientSnapshotHistory {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InsufficientSnapshotHistory {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InsufficientSnapshotHistory()";
            const SELECTOR: [u8; 4] = [176u8, 180u8, 56u8, 119u8];
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
    /**Custom error with signature `InvalidAddress()` and selector `0xe6c4247b`.
    ```solidity
    error InvalidAddress();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidAddress {}
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
        impl ::core::convert::From<InvalidAddress> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidAddress) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidAddress {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidAddress {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidAddress()";
            const SELECTOR: [u8; 4] = [230u8, 196u8, 36u8, 123u8];
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
    /**Custom error with signature `InvalidArgs()` and selector `0xa1ba07ee`.
    ```solidity
    error InvalidArgs();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidArgs {}
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
        impl ::core::convert::From<InvalidArgs> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidArgs) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidArgs {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidArgs {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidArgs()";
            const SELECTOR: [u8; 4] = [161u8, 186u8, 7u8, 238u8];
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
    /**Custom error with signature `InvalidHotShotBlockForCommitmentCheck()` and selector `0x615a9264`.
    ```solidity
    error InvalidHotShotBlockForCommitmentCheck();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidHotShotBlockForCommitmentCheck {}
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
        impl ::core::convert::From<InvalidHotShotBlockForCommitmentCheck> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidHotShotBlockForCommitmentCheck) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidHotShotBlockForCommitmentCheck {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidHotShotBlockForCommitmentCheck {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidHotShotBlockForCommitmentCheck()";
            const SELECTOR: [u8; 4] = [97u8, 90u8, 146u8, 100u8];
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
    /**Custom error with signature `InvalidMaxStateHistory()` and selector `0xf4a0eee0`.
    ```solidity
    error InvalidMaxStateHistory();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidMaxStateHistory {}
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
        impl ::core::convert::From<InvalidMaxStateHistory> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidMaxStateHistory) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidMaxStateHistory {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidMaxStateHistory {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidMaxStateHistory()";
            const SELECTOR: [u8; 4] = [244u8, 160u8, 238u8, 224u8];
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
    /**Custom error with signature `InvalidProof()` and selector `0x09bde339`.
    ```solidity
    error InvalidProof();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidProof {}
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
        impl ::core::convert::From<InvalidProof> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidProof) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidProof {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidProof {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidProof()";
            const SELECTOR: [u8; 4] = [9u8, 189u8, 227u8, 57u8];
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
    /**Custom error with signature `NoChangeRequired()` and selector `0xa863aec9`.
    ```solidity
    error NoChangeRequired();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct NoChangeRequired {}
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
        impl ::core::convert::From<NoChangeRequired> for UnderlyingRustTuple<'_> {
            fn from(value: NoChangeRequired) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for NoChangeRequired {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for NoChangeRequired {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "NoChangeRequired()";
            const SELECTOR: [u8; 4] = [168u8, 99u8, 174u8, 201u8];
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
    /**Custom error with signature `OutdatedState()` and selector `0x051c46ef`.
    ```solidity
    error OutdatedState();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct OutdatedState {}
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
        impl ::core::convert::From<OutdatedState> for UnderlyingRustTuple<'_> {
            fn from(value: OutdatedState) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for OutdatedState {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for OutdatedState {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "OutdatedState()";
            const SELECTOR: [u8; 4] = [5u8, 28u8, 70u8, 239u8];
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
    /**Custom error with signature `ProverNotPermissioned()` and selector `0xa3a64780`.
    ```solidity
    error ProverNotPermissioned();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ProverNotPermissioned {}
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
        impl ::core::convert::From<ProverNotPermissioned> for UnderlyingRustTuple<'_> {
            fn from(value: ProverNotPermissioned) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for ProverNotPermissioned {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for ProverNotPermissioned {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "ProverNotPermissioned()";
            const SELECTOR: [u8; 4] = [163u8, 166u8, 71u8, 128u8];
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
    /**Custom error with signature `WrongStakeTableUsed()` and selector `0x51618089`.
    ```solidity
    error WrongStakeTableUsed();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct WrongStakeTableUsed {}
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
        impl ::core::convert::From<WrongStakeTableUsed> for UnderlyingRustTuple<'_> {
            fn from(value: WrongStakeTableUsed) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for WrongStakeTableUsed {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for WrongStakeTableUsed {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "WrongStakeTableUsed()";
            const SELECTOR: [u8; 4] = [81u8, 97u8, 128u8, 137u8];
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
    /**Event with signature `NewState(uint64,uint64,uint256)` and selector `0xa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae6`.
    ```solidity
    event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, BN254.ScalarField blockCommRoot);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct NewState {
        #[allow(missing_docs)]
        pub viewNum: u64,
        #[allow(missing_docs)]
        pub blockHeight: u64,
        #[allow(missing_docs)]
        pub blockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
        impl alloy_sol_types::SolEvent for NewState {
            type DataTuple<'a> = (BN254::ScalarField,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            const SIGNATURE: &'static str = "NewState(uint64,uint64,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    160u8, 74u8, 119u8, 57u8, 36u8, 80u8, 90u8, 65u8, 133u8, 100u8, 54u8, 55u8,
                    37u8, 245u8, 104u8, 50u8, 245u8, 119u8, 46u8, 107u8, 141u8, 13u8, 189u8, 110u8,
                    252u8, 231u8, 36u8, 223u8, 232u8, 3u8, 218u8, 230u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    viewNum: topics.1,
                    blockHeight: topics.2,
                    blockCommRoot: data.0,
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
                (<BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                    &self.blockCommRoot,
                ),)
            }
            #[inline]
            fn topics(&self) -> <Self::TopicList as alloy_sol_types::SolType>::RustType {
                (
                    Self::SIGNATURE_HASH.into(),
                    self.viewNum.clone(),
                    self.blockHeight.clone(),
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
                out[1usize] = <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.viewNum);
                out[2usize] = <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic(&self.blockHeight);
                Ok(())
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::private::IntoLogData for NewState {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&NewState> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &NewState) -> alloy_sol_types::private::LogData {
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
    /**Event with signature `PermissionedProverNotRequired()` and selector `0x9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450`.
    ```solidity
    event PermissionedProverNotRequired();
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct PermissionedProverNotRequired {}
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::SolEvent for PermissionedProverNotRequired {
            type DataTuple<'a> = ();
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "PermissionedProverNotRequired()";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    154u8, 95u8, 87u8, 222u8, 133u8, 109u8, 214u8, 104u8, 197u8, 77u8, 217u8, 94u8,
                    92u8, 85u8, 223u8, 147u8, 67u8, 33u8, 113u8, 203u8, 202u8, 73u8, 168u8, 119u8,
                    109u8, 86u8, 32u8, 234u8, 89u8, 192u8, 36u8, 80u8,
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
        impl alloy_sol_types::private::IntoLogData for PermissionedProverNotRequired {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&PermissionedProverNotRequired> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &PermissionedProverNotRequired) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    /**Event with signature `PermissionedProverRequired(address)` and selector `0x8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072`.
    ```solidity
    event PermissionedProverRequired(address permissionedProver);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct PermissionedProverRequired {
        #[allow(missing_docs)]
        pub permissionedProver: alloy::sol_types::private::Address,
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
        impl alloy_sol_types::SolEvent for PermissionedProverRequired {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "PermissionedProverRequired(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    128u8, 23u8, 187u8, 136u8, 127u8, 223u8, 143u8, 202u8, 67u8, 20u8, 169u8,
                    212u8, 15u8, 110u8, 115u8, 179u8, 184u8, 16u8, 2u8, 214u8, 126u8, 92u8, 250u8,
                    133u8, 216u8, 129u8, 115u8, 175u8, 106u8, 164u8, 96u8, 114u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self {
                    permissionedProver: data.0,
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
                        &self.permissionedProver,
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
        impl alloy_sol_types::private::IntoLogData for PermissionedProverRequired {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&PermissionedProverRequired> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &PermissionedProverRequired) -> alloy_sol_types::private::LogData {
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
    /**Function with signature `currentBlockNumber()` and selector `0x378ec23b`.
    ```solidity
    function currentBlockNumber() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentBlockNumberCall {}
    ///Container type for the return parameters of the [`currentBlockNumber()`](currentBlockNumberCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentBlockNumberReturn {
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
            impl ::core::convert::From<currentBlockNumberCall> for UnderlyingRustTuple<'_> {
                fn from(value: currentBlockNumberCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for currentBlockNumberCall {
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
            impl ::core::convert::From<currentBlockNumberReturn> for UnderlyingRustTuple<'_> {
                fn from(value: currentBlockNumberReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for currentBlockNumberReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for currentBlockNumberCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = currentBlockNumberReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "currentBlockNumber()";
            const SELECTOR: [u8; 4] = [55u8, 142u8, 194u8, 59u8];
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
    /**Function with signature `disablePermissionedProverMode()` and selector `0x69cc6a04`.
    ```solidity
    function disablePermissionedProverMode() external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct disablePermissionedProverModeCall {}
    ///Container type for the return parameters of the [`disablePermissionedProverMode()`](disablePermissionedProverModeCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct disablePermissionedProverModeReturn {}
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
            impl ::core::convert::From<disablePermissionedProverModeCall> for UnderlyingRustTuple<'_> {
                fn from(value: disablePermissionedProverModeCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for disablePermissionedProverModeCall {
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
            impl ::core::convert::From<disablePermissionedProverModeReturn> for UnderlyingRustTuple<'_> {
                fn from(value: disablePermissionedProverModeReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for disablePermissionedProverModeReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for disablePermissionedProverModeCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = disablePermissionedProverModeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "disablePermissionedProverMode()";
            const SELECTOR: [u8; 4] = [105u8, 204u8, 106u8, 4u8];
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
    /**Function with signature `finalizedState()` and selector `0x9fdb54a7`.
    ```solidity
    function finalizedState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct finalizedStateCall {}
    ///Container type for the return parameters of the [`finalizedState()`](finalizedStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct finalizedStateReturn {
        #[allow(missing_docs)]
        pub viewNum: u64,
        #[allow(missing_docs)]
        pub blockHeight: u64,
        #[allow(missing_docs)]
        pub blockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<finalizedStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: finalizedStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for finalizedStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                u64,
                u64,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<finalizedStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: finalizedStateReturn) -> Self {
                    (value.viewNum, value.blockHeight, value.blockCommRoot)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for finalizedStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        viewNum: tuple.0,
                        blockHeight: tuple.1,
                        blockCommRoot: tuple.2,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for finalizedStateCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = finalizedStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "finalizedState()";
            const SELECTOR: [u8; 4] = [159u8, 219u8, 84u8, 167u8];
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
    /**Function with signature `genesisStakeTableState()` and selector `0x426d3194`.
    ```solidity
    function genesisStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStakeTableStateCall {}
    ///Container type for the return parameters of the [`genesisStakeTableState()`](genesisStakeTableStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStakeTableStateReturn {
        #[allow(missing_docs)]
        pub threshold: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub blsKeyComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub schnorrKeyComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub amountComm: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<genesisStakeTableStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: genesisStakeTableStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for genesisStakeTableStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                BN254::ScalarField,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::primitives::aliases::U256,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<genesisStakeTableStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: genesisStakeTableStateReturn) -> Self {
                    (
                        value.threshold,
                        value.blsKeyComm,
                        value.schnorrKeyComm,
                        value.amountComm,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for genesisStakeTableStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        threshold: tuple.0,
                        blsKeyComm: tuple.1,
                        schnorrKeyComm: tuple.2,
                        amountComm: tuple.3,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for genesisStakeTableStateCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = genesisStakeTableStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                BN254::ScalarField,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "genesisStakeTableState()";
            const SELECTOR: [u8; 4] = [66u8, 109u8, 49u8, 148u8];
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
    /**Function with signature `genesisState()` and selector `0xd24d933d`.
    ```solidity
    function genesisState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStateCall {}
    ///Container type for the return parameters of the [`genesisState()`](genesisStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStateReturn {
        #[allow(missing_docs)]
        pub viewNum: u64,
        #[allow(missing_docs)]
        pub blockHeight: u64,
        #[allow(missing_docs)]
        pub blockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<genesisStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: genesisStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for genesisStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                u64,
                u64,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<genesisStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: genesisStateReturn) -> Self {
                    (value.viewNum, value.blockHeight, value.blockCommRoot)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for genesisStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        viewNum: tuple.0,
                        blockHeight: tuple.1,
                        blockCommRoot: tuple.2,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for genesisStateCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = genesisStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "genesisState()";
            const SELECTOR: [u8; 4] = [210u8, 77u8, 147u8, 61u8];
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
    /**Function with signature `getHotShotCommitment(uint256)` and selector `0x8584d23f`.
    ```solidity
    function getHotShotCommitment(uint256 hotShotBlockHeight) external view returns (BN254.ScalarField hotShotBlockCommRoot, uint64 hotshotBlockHeight);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getHotShotCommitmentCall {
        #[allow(missing_docs)]
        pub hotShotBlockHeight: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`getHotShotCommitment(uint256)`](getHotShotCommitmentCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getHotShotCommitmentReturn {
        #[allow(missing_docs)]
        pub hotShotBlockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub hotshotBlockHeight: u64,
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
            impl ::core::convert::From<getHotShotCommitmentCall> for UnderlyingRustTuple<'_> {
                fn from(value: getHotShotCommitmentCall) -> Self {
                    (value.hotShotBlockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getHotShotCommitmentCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        hotShotBlockHeight: tuple.0,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> =
                (BN254::ScalarField, alloy::sol_types::sol_data::Uint<64>);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
                u64,
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
            impl ::core::convert::From<getHotShotCommitmentReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getHotShotCommitmentReturn) -> Self {
                    (value.hotShotBlockCommRoot, value.hotshotBlockHeight)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getHotShotCommitmentReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        hotShotBlockCommRoot: tuple.0,
                        hotshotBlockHeight: tuple.1,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getHotShotCommitmentCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getHotShotCommitmentReturn;
            type ReturnTuple<'a> = (BN254::ScalarField, alloy::sol_types::sol_data::Uint<64>);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getHotShotCommitment(uint256)";
            const SELECTOR: [u8; 4] = [133u8, 132u8, 210u8, 63u8];
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
                        &self.hotShotBlockHeight,
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
    /**Function with signature `getStateHistoryCount()` and selector `0xf9e50d19`.
    ```solidity
    function getStateHistoryCount() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getStateHistoryCountCall {}
    ///Container type for the return parameters of the [`getStateHistoryCount()`](getStateHistoryCountCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getStateHistoryCountReturn {
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
            impl ::core::convert::From<getStateHistoryCountCall> for UnderlyingRustTuple<'_> {
                fn from(value: getStateHistoryCountCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getStateHistoryCountCall {
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
            impl ::core::convert::From<getStateHistoryCountReturn> for UnderlyingRustTuple<'_> {
                fn from(value: getStateHistoryCountReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for getStateHistoryCountReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getStateHistoryCountCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = getStateHistoryCountReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "getStateHistoryCount()";
            const SELECTOR: [u8; 4] = [249u8, 229u8, 13u8, 25u8];
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
    /**Function with signature `initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)` and selector `0x9baa3cc9`.
    ```solidity
    function initialize(LightClientState memory _genesis, StakeTableState memory _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeCall {
        #[allow(missing_docs)]
        pub _genesis: <LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _genesisStakeTableState: <StakeTableState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _stateHistoryRetentionPeriod: u32,
        #[allow(missing_docs)]
        pub owner: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)`](initializeCall) function.
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
                LightClientState,
                StakeTableState,
                alloy::sol_types::sol_data::Uint<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClientState as alloy::sol_types::SolType>::RustType,
                <StakeTableState as alloy::sol_types::SolType>::RustType,
                u32,
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
                        value._genesis,
                        value._genesisStakeTableState,
                        value._stateHistoryRetentionPeriod,
                        value.owner,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _genesis: tuple.0,
                        _genesisStakeTableState: tuple.1,
                        _stateHistoryRetentionPeriod: tuple.2,
                        owner: tuple.3,
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
                LightClientState,
                StakeTableState,
                alloy::sol_types::sol_data::Uint<32>,
                alloy::sol_types::sol_data::Address,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)";
            const SELECTOR: [u8; 4] = [155u8, 170u8, 60u8, 201u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <LightClientState as alloy_sol_types::SolType>::tokenize(&self._genesis),
                    <StakeTableState as alloy_sol_types::SolType>::tokenize(
                        &self._genesisStakeTableState,
                    ),
                    <alloy::sol_types::sol_data::Uint<32> as alloy_sol_types::SolType>::tokenize(
                        &self._stateHistoryRetentionPeriod,
                    ),
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
    /**Function with signature `isPermissionedProverEnabled()` and selector `0x826e41fc`.
    ```solidity
    function isPermissionedProverEnabled() external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPermissionedProverEnabledCall {}
    ///Container type for the return parameters of the [`isPermissionedProverEnabled()`](isPermissionedProverEnabledCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPermissionedProverEnabledReturn {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<isPermissionedProverEnabledCall> for UnderlyingRustTuple<'_> {
                fn from(value: isPermissionedProverEnabledCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isPermissionedProverEnabledCall {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<isPermissionedProverEnabledReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isPermissionedProverEnabledReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isPermissionedProverEnabledReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isPermissionedProverEnabledCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = isPermissionedProverEnabledReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isPermissionedProverEnabled()";
            const SELECTOR: [u8; 4] = [130u8, 110u8, 65u8, 252u8];
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
    /**Function with signature `lagOverEscapeHatchThreshold(uint256,uint256)` and selector `0xe0303301`.
    ```solidity
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lagOverEscapeHatchThresholdCall {
        #[allow(missing_docs)]
        pub blockNumber: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub blockThreshold: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`lagOverEscapeHatchThreshold(uint256,uint256)`](lagOverEscapeHatchThresholdCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lagOverEscapeHatchThresholdReturn {
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
            impl ::core::convert::From<lagOverEscapeHatchThresholdCall> for UnderlyingRustTuple<'_> {
                fn from(value: lagOverEscapeHatchThresholdCall) -> Self {
                    (value.blockNumber, value.blockThreshold)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for lagOverEscapeHatchThresholdCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blockNumber: tuple.0,
                        blockThreshold: tuple.1,
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
            impl ::core::convert::From<lagOverEscapeHatchThresholdReturn> for UnderlyingRustTuple<'_> {
                fn from(value: lagOverEscapeHatchThresholdReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for lagOverEscapeHatchThresholdReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for lagOverEscapeHatchThresholdCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = lagOverEscapeHatchThresholdReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "lagOverEscapeHatchThreshold(uint256,uint256)";
            const SELECTOR: [u8; 4] = [224u8, 48u8, 51u8, 1u8];
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
                        &self.blockNumber,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.blockThreshold,
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
    /**Function with signature `newFinalizedState((uint64,uint64,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0x2063d4f7`.
    ```solidity
    function newFinalizedState(LightClientState memory newState, IPlonkVerifier.PlonkProof memory proof) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedStateCall {
        #[allow(missing_docs)]
        pub newState: <LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`newFinalizedState((uint64,uint64,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](newFinalizedStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedStateReturn {}
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
            type UnderlyingSolTuple<'a> = (LightClientState, IPlonkVerifier::PlonkProof);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClientState as alloy::sol_types::SolType>::RustType,
                <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<newFinalizedStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedStateCall) -> Self {
                    (value.newState, value.proof)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newState: tuple.0,
                        proof: tuple.1,
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
            impl ::core::convert::From<newFinalizedStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedStateReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedStateCall {
            type Parameters<'a> = (LightClientState, IPlonkVerifier::PlonkProof);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedStateReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "newFinalizedState((uint64,uint64,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [32u8, 99u8, 212u8, 247u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <LightClientState as alloy_sol_types::SolType>::tokenize(&self.newState),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(&self.proof),
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
    /**Function with signature `permissionedProver()` and selector `0x313df7b1`.
    ```solidity
    function permissionedProver() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct permissionedProverCall {}
    ///Container type for the return parameters of the [`permissionedProver()`](permissionedProverCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct permissionedProverReturn {
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
            impl ::core::convert::From<permissionedProverCall> for UnderlyingRustTuple<'_> {
                fn from(value: permissionedProverCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for permissionedProverCall {
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
            impl ::core::convert::From<permissionedProverReturn> for UnderlyingRustTuple<'_> {
                fn from(value: permissionedProverReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for permissionedProverReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for permissionedProverCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = permissionedProverReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "permissionedProver()";
            const SELECTOR: [u8; 4] = [49u8, 61u8, 247u8, 177u8];
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
    /**Function with signature `setPermissionedProver(address)` and selector `0x013fa5fc`.
    ```solidity
    function setPermissionedProver(address prover) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setPermissionedProverCall {
        #[allow(missing_docs)]
        pub prover: alloy::sol_types::private::Address,
    }
    ///Container type for the return parameters of the [`setPermissionedProver(address)`](setPermissionedProverCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setPermissionedProverReturn {}
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
            impl ::core::convert::From<setPermissionedProverCall> for UnderlyingRustTuple<'_> {
                fn from(value: setPermissionedProverCall) -> Self {
                    (value.prover,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setPermissionedProverCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { prover: tuple.0 }
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
            impl ::core::convert::From<setPermissionedProverReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setPermissionedProverReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setPermissionedProverReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setPermissionedProverCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = setPermissionedProverReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setPermissionedProver(address)";
            const SELECTOR: [u8; 4] = [1u8, 63u8, 165u8, 252u8];
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
                        &self.prover,
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
    /**Function with signature `setstateHistoryRetentionPeriod(uint32)` and selector `0x96c1ca61`.
    ```solidity
    function setstateHistoryRetentionPeriod(uint32 historySeconds) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setstateHistoryRetentionPeriodCall {
        #[allow(missing_docs)]
        pub historySeconds: u32,
    }
    ///Container type for the return parameters of the [`setstateHistoryRetentionPeriod(uint32)`](setstateHistoryRetentionPeriodCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setstateHistoryRetentionPeriodReturn {}
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
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u32,);
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
            impl ::core::convert::From<setstateHistoryRetentionPeriodCall> for UnderlyingRustTuple<'_> {
                fn from(value: setstateHistoryRetentionPeriodCall) -> Self {
                    (value.historySeconds,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setstateHistoryRetentionPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        historySeconds: tuple.0,
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
            impl ::core::convert::From<setstateHistoryRetentionPeriodReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setstateHistoryRetentionPeriodReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setstateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setstateHistoryRetentionPeriodCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = setstateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setstateHistoryRetentionPeriod(uint32)";
            const SELECTOR: [u8; 4] = [150u8, 193u8, 202u8, 97u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<32> as alloy_sol_types::SolType>::tokenize(
                        &self.historySeconds,
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
    /**Function with signature `stateHistoryCommitments(uint256)` and selector `0x02b592f3`.
    ```solidity
    function stateHistoryCommitments(uint256) external view returns (uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, BN254.ScalarField hotShotBlockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryCommitmentsCall {
        #[allow(missing_docs)]
        pub _0: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`stateHistoryCommitments(uint256)`](stateHistoryCommitmentsCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryCommitmentsReturn {
        #[allow(missing_docs)]
        pub l1BlockHeight: u64,
        #[allow(missing_docs)]
        pub l1BlockTimestamp: u64,
        #[allow(missing_docs)]
        pub hotShotBlockHeight: u64,
        #[allow(missing_docs)]
        pub hotShotBlockCommRoot: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<stateHistoryCommitmentsCall> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryCommitmentsCall) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryCommitmentsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                u64,
                u64,
                u64,
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<stateHistoryCommitmentsReturn> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryCommitmentsReturn) -> Self {
                    (
                        value.l1BlockHeight,
                        value.l1BlockTimestamp,
                        value.hotShotBlockHeight,
                        value.hotShotBlockCommRoot,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryCommitmentsReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        l1BlockHeight: tuple.0,
                        l1BlockTimestamp: tuple.1,
                        hotShotBlockHeight: tuple.2,
                        hotShotBlockCommRoot: tuple.3,
                    }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for stateHistoryCommitmentsCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryCommitmentsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "stateHistoryCommitments(uint256)";
            const SELECTOR: [u8; 4] = [2u8, 181u8, 146u8, 243u8];
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
    /**Function with signature `stateHistoryFirstIndex()` and selector `0x2f79889d`.
    ```solidity
    function stateHistoryFirstIndex() external view returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryFirstIndexCall {}
    ///Container type for the return parameters of the [`stateHistoryFirstIndex()`](stateHistoryFirstIndexCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryFirstIndexReturn {
        #[allow(missing_docs)]
        pub _0: u64,
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
            impl ::core::convert::From<stateHistoryFirstIndexCall> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryFirstIndexCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryFirstIndexCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u64,);
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
            impl ::core::convert::From<stateHistoryFirstIndexReturn> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryFirstIndexReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryFirstIndexReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for stateHistoryFirstIndexCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryFirstIndexReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "stateHistoryFirstIndex()";
            const SELECTOR: [u8; 4] = [47u8, 121u8, 136u8, 157u8];
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
    /**Function with signature `stateHistoryRetentionPeriod()` and selector `0xc23b9e9e`.
    ```solidity
    function stateHistoryRetentionPeriod() external view returns (uint32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryRetentionPeriodCall {}
    ///Container type for the return parameters of the [`stateHistoryRetentionPeriod()`](stateHistoryRetentionPeriodCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryRetentionPeriodReturn {
        #[allow(missing_docs)]
        pub _0: u32,
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
            impl ::core::convert::From<stateHistoryRetentionPeriodCall> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryRetentionPeriodCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryRetentionPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u32,);
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
            impl ::core::convert::From<stateHistoryRetentionPeriodReturn> for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryRetentionPeriodReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for stateHistoryRetentionPeriodCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "stateHistoryRetentionPeriod()";
            const SELECTOR: [u8; 4] = [194u8, 59u8, 158u8, 158u8];
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
    ///Container for all the [`LightClient`](self) function calls.
    pub enum LightClientCalls {
        #[allow(missing_docs)]
        UPGRADE_INTERFACE_VERSION(UPGRADE_INTERFACE_VERSIONCall),
        #[allow(missing_docs)]
        currentBlockNumber(currentBlockNumberCall),
        #[allow(missing_docs)]
        disablePermissionedProverMode(disablePermissionedProverModeCall),
        #[allow(missing_docs)]
        finalizedState(finalizedStateCall),
        #[allow(missing_docs)]
        genesisStakeTableState(genesisStakeTableStateCall),
        #[allow(missing_docs)]
        genesisState(genesisStateCall),
        #[allow(missing_docs)]
        getHotShotCommitment(getHotShotCommitmentCall),
        #[allow(missing_docs)]
        getStateHistoryCount(getStateHistoryCountCall),
        #[allow(missing_docs)]
        getVersion(getVersionCall),
        #[allow(missing_docs)]
        initialize(initializeCall),
        #[allow(missing_docs)]
        isPermissionedProverEnabled(isPermissionedProverEnabledCall),
        #[allow(missing_docs)]
        lagOverEscapeHatchThreshold(lagOverEscapeHatchThresholdCall),
        #[allow(missing_docs)]
        newFinalizedState(newFinalizedStateCall),
        #[allow(missing_docs)]
        owner(ownerCall),
        #[allow(missing_docs)]
        permissionedProver(permissionedProverCall),
        #[allow(missing_docs)]
        proxiableUUID(proxiableUUIDCall),
        #[allow(missing_docs)]
        renounceOwnership(renounceOwnershipCall),
        #[allow(missing_docs)]
        setPermissionedProver(setPermissionedProverCall),
        #[allow(missing_docs)]
        setstateHistoryRetentionPeriod(setstateHistoryRetentionPeriodCall),
        #[allow(missing_docs)]
        stateHistoryCommitments(stateHistoryCommitmentsCall),
        #[allow(missing_docs)]
        stateHistoryFirstIndex(stateHistoryFirstIndexCall),
        #[allow(missing_docs)]
        stateHistoryRetentionPeriod(stateHistoryRetentionPeriodCall),
        #[allow(missing_docs)]
        transferOwnership(transferOwnershipCall),
        #[allow(missing_docs)]
        upgradeToAndCall(upgradeToAndCallCall),
    }
    #[automatically_derived]
    impl LightClientCalls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 63u8, 165u8, 252u8],
            [2u8, 181u8, 146u8, 243u8],
            [13u8, 142u8, 110u8, 44u8],
            [32u8, 99u8, 212u8, 247u8],
            [47u8, 121u8, 136u8, 157u8],
            [49u8, 61u8, 247u8, 177u8],
            [55u8, 142u8, 194u8, 59u8],
            [66u8, 109u8, 49u8, 148u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [105u8, 204u8, 106u8, 4u8],
            [113u8, 80u8, 24u8, 166u8],
            [130u8, 110u8, 65u8, 252u8],
            [133u8, 132u8, 210u8, 63u8],
            [141u8, 165u8, 203u8, 91u8],
            [150u8, 193u8, 202u8, 97u8],
            [155u8, 170u8, 60u8, 201u8],
            [159u8, 219u8, 84u8, 167u8],
            [173u8, 60u8, 177u8, 204u8],
            [194u8, 59u8, 158u8, 158u8],
            [210u8, 77u8, 147u8, 61u8],
            [224u8, 48u8, 51u8, 1u8],
            [242u8, 253u8, 227u8, 139u8],
            [249u8, 229u8, 13u8, 25u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for LightClientCalls {
        const NAME: &'static str = "LightClientCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 24usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::currentBlockNumber(_) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::disablePermissionedProverMode(_) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::finalizedState(_) => {
                    <finalizedStateCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::genesisStakeTableState(_) => {
                    <genesisStakeTableStateCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::genesisState(_) => <genesisStateCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::getHotShotCommitment(_) => {
                    <getHotShotCommitmentCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getStateHistoryCount(_) => {
                    <getStateHistoryCountCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::getVersion(_) => <getVersionCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::initialize(_) => <initializeCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::isPermissionedProverEnabled(_) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::lagOverEscapeHatchThreshold(_) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::newFinalizedState(_) => {
                    <newFinalizedStateCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::owner(_) => <ownerCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::permissionedProver(_) => {
                    <permissionedProverCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::proxiableUUID(_) => <proxiableUUIDCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::renounceOwnership(_) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::setPermissionedProver(_) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::setstateHistoryRetentionPeriod(_) => {
                    <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::stateHistoryCommitments(_) => {
                    <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::stateHistoryFirstIndex(_) => {
                    <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::stateHistoryRetentionPeriod(_) => {
                    <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::transferOwnership(_) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::upgradeToAndCall(_) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::SELECTOR
                },
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
            static DECODE_SHIMS: &[fn(&[u8], bool) -> alloy_sol_types::Result<LightClientCalls>] =
                &[
                    {
                        fn setPermissionedProver(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::setPermissionedProver)
                        }
                        setPermissionedProver
                    },
                    {
                        fn stateHistoryCommitments(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::stateHistoryCommitments)
                        }
                        stateHistoryCommitments
                    },
                    {
                        fn getVersion(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::getVersion)
                        }
                        getVersion
                    },
                    {
                        fn newFinalizedState(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <newFinalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::newFinalizedState)
                        }
                        newFinalizedState
                    },
                    {
                        fn stateHistoryFirstIndex(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::stateHistoryFirstIndex)
                        }
                        stateHistoryFirstIndex
                    },
                    {
                        fn permissionedProver(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <permissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::permissionedProver)
                        }
                        permissionedProver
                    },
                    {
                        fn currentBlockNumber(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::currentBlockNumber)
                        }
                        currentBlockNumber
                    },
                    {
                        fn genesisStakeTableState(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::genesisStakeTableState)
                        }
                        genesisStakeTableState
                    },
                    {
                        fn upgradeToAndCall(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::upgradeToAndCall)
                        }
                        upgradeToAndCall
                    },
                    {
                        fn proxiableUUID(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::proxiableUUID)
                        }
                        proxiableUUID
                    },
                    {
                        fn disablePermissionedProverMode(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::disablePermissionedProverMode)
                        }
                        disablePermissionedProverMode
                    },
                    {
                        fn renounceOwnership(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::renounceOwnership)
                        }
                        renounceOwnership
                    },
                    {
                        fn isPermissionedProverEnabled(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::isPermissionedProverEnabled)
                        }
                        isPermissionedProverEnabled
                    },
                    {
                        fn getHotShotCommitment(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::getHotShotCommitment)
                        }
                        getHotShotCommitment
                    },
                    {
                        fn owner(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                                .map(LightClientCalls::owner)
                        }
                        owner
                    },
                    {
                        fn setstateHistoryRetentionPeriod(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::setstateHistoryRetentionPeriod)
                        }
                        setstateHistoryRetentionPeriod
                    },
                    {
                        fn initialize(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::initialize)
                        }
                        initialize
                    },
                    {
                        fn finalizedState(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <finalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::finalizedState)
                        }
                        finalizedState
                    },
                    {
                        fn UPGRADE_INTERFACE_VERSION(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::UPGRADE_INTERFACE_VERSION)
                        }
                        UPGRADE_INTERFACE_VERSION
                    },
                    {
                        fn stateHistoryRetentionPeriod(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::stateHistoryRetentionPeriod)
                        }
                        stateHistoryRetentionPeriod
                    },
                    {
                        fn genesisState(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <genesisStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::genesisState)
                        }
                        genesisState
                    },
                    {
                        fn lagOverEscapeHatchThreshold(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientCalls::lagOverEscapeHatchThreshold)
                        }
                        lagOverEscapeHatchThreshold
                    },
                    {
                        fn transferOwnership(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::transferOwnership)
                        }
                        transferOwnership
                    },
                    {
                        fn getStateHistoryCount(
                            data: &[u8],
                            validate: bool,
                        ) -> alloy_sol_types::Result<LightClientCalls> {
                            <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data, validate,
                            )
                            .map(LightClientCalls::getStateHistoryCount)
                        }
                        getStateHistoryCount
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
                }
                Self::currentBlockNumber(inner) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::disablePermissionedProverMode(inner) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::finalizedState(inner) => {
                    <finalizedStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::genesisStakeTableState(inner) => {
                    <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::genesisState(inner) => {
                    <genesisStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::getHotShotCommitment(inner) => {
                    <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::getStateHistoryCount(inner) => {
                    <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::isPermissionedProverEnabled(inner) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::lagOverEscapeHatchThreshold(inner) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::newFinalizedState(inner) => {
                    <newFinalizedStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::permissionedProver(inner) => {
                    <permissionedProverCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setPermissionedProver(inner) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setstateHistoryRetentionPeriod(inner) => {
                    <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::stateHistoryCommitments(inner) => {
                    <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::stateHistoryFirstIndex(inner) => {
                    <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::stateHistoryRetentionPeriod(inner) => {
                    <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
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
                Self::currentBlockNumber(inner) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::disablePermissionedProverMode(inner) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::finalizedState(inner) => {
                    <finalizedStateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::genesisStakeTableState(inner) => {
                    <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::genesisState(inner) => {
                    <genesisStateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::getHotShotCommitment(inner) => {
                    <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::getStateHistoryCount(inner) => {
                    <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::getVersion(inner) => {
                    <getVersionCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::initialize(inner) => {
                    <initializeCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isPermissionedProverEnabled(inner) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::lagOverEscapeHatchThreshold(inner) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::newFinalizedState(inner) => {
                    <newFinalizedStateCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::permissionedProver(inner) => {
                    <permissionedProverCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::renounceOwnership(inner) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::setPermissionedProver(inner) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::setstateHistoryRetentionPeriod(inner) => {
                    <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::stateHistoryCommitments(inner) => {
                    <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::stateHistoryFirstIndex(inner) => {
                    <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::stateHistoryRetentionPeriod(inner) => {
                    <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`LightClient`](self) custom errors.
    pub enum LightClientErrors {
        #[allow(missing_docs)]
        AddressEmptyCode(AddressEmptyCode),
        #[allow(missing_docs)]
        ERC1967InvalidImplementation(ERC1967InvalidImplementation),
        #[allow(missing_docs)]
        ERC1967NonPayable(ERC1967NonPayable),
        #[allow(missing_docs)]
        FailedInnerCall(FailedInnerCall),
        #[allow(missing_docs)]
        InsufficientSnapshotHistory(InsufficientSnapshotHistory),
        #[allow(missing_docs)]
        InvalidAddress(InvalidAddress),
        #[allow(missing_docs)]
        InvalidArgs(InvalidArgs),
        #[allow(missing_docs)]
        InvalidHotShotBlockForCommitmentCheck(InvalidHotShotBlockForCommitmentCheck),
        #[allow(missing_docs)]
        InvalidInitialization(InvalidInitialization),
        #[allow(missing_docs)]
        InvalidMaxStateHistory(InvalidMaxStateHistory),
        #[allow(missing_docs)]
        InvalidProof(InvalidProof),
        #[allow(missing_docs)]
        NoChangeRequired(NoChangeRequired),
        #[allow(missing_docs)]
        NotInitializing(NotInitializing),
        #[allow(missing_docs)]
        OutdatedState(OutdatedState),
        #[allow(missing_docs)]
        OwnableInvalidOwner(OwnableInvalidOwner),
        #[allow(missing_docs)]
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
        #[allow(missing_docs)]
        ProverNotPermissioned(ProverNotPermissioned),
        #[allow(missing_docs)]
        UUPSUnauthorizedCallContext(UUPSUnauthorizedCallContext),
        #[allow(missing_docs)]
        UUPSUnsupportedProxiableUUID(UUPSUnsupportedProxiableUUID),
        #[allow(missing_docs)]
        WrongStakeTableUsed(WrongStakeTableUsed),
    }
    #[automatically_derived]
    impl LightClientErrors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [5u8, 28u8, 70u8, 239u8],
            [9u8, 189u8, 227u8, 57u8],
            [17u8, 140u8, 218u8, 167u8],
            [20u8, 37u8, 234u8, 66u8],
            [30u8, 79u8, 189u8, 247u8],
            [76u8, 156u8, 140u8, 227u8],
            [81u8, 97u8, 128u8, 137u8],
            [97u8, 90u8, 146u8, 100u8],
            [153u8, 150u8, 179u8, 21u8],
            [161u8, 186u8, 7u8, 238u8],
            [163u8, 166u8, 71u8, 128u8],
            [168u8, 99u8, 174u8, 201u8],
            [170u8, 29u8, 73u8, 164u8],
            [176u8, 180u8, 56u8, 119u8],
            [179u8, 152u8, 151u8, 159u8],
            [215u8, 230u8, 188u8, 248u8],
            [224u8, 124u8, 141u8, 186u8],
            [230u8, 196u8, 36u8, 123u8],
            [244u8, 160u8, 238u8, 224u8],
            [249u8, 46u8, 232u8, 169u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for LightClientErrors {
        const NAME: &'static str = "LightClientErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 20usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AddressEmptyCode(_) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::SELECTOR
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
                Self::InsufficientSnapshotHistory(_) => {
                    <InsufficientSnapshotHistory as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidAddress(_) => <InvalidAddress as alloy_sol_types::SolError>::SELECTOR,
                Self::InvalidArgs(_) => <InvalidArgs as alloy_sol_types::SolError>::SELECTOR,
                Self::InvalidHotShotBlockForCommitmentCheck(_) => {
                    <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidInitialization(_) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidMaxStateHistory(_) => {
                    <InvalidMaxStateHistory as alloy_sol_types::SolError>::SELECTOR
                },
                Self::InvalidProof(_) => <InvalidProof as alloy_sol_types::SolError>::SELECTOR,
                Self::NoChangeRequired(_) => {
                    <NoChangeRequired as alloy_sol_types::SolError>::SELECTOR
                },
                Self::NotInitializing(_) => {
                    <NotInitializing as alloy_sol_types::SolError>::SELECTOR
                },
                Self::OutdatedState(_) => <OutdatedState as alloy_sol_types::SolError>::SELECTOR,
                Self::OwnableInvalidOwner(_) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::SELECTOR
                },
                Self::OwnableUnauthorizedAccount(_) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                },
                Self::ProverNotPermissioned(_) => {
                    <ProverNotPermissioned as alloy_sol_types::SolError>::SELECTOR
                },
                Self::UUPSUnauthorizedCallContext(_) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::SELECTOR
                },
                Self::UUPSUnsupportedProxiableUUID(_) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::SELECTOR
                },
                Self::WrongStakeTableUsed(_) => {
                    <WrongStakeTableUsed as alloy_sol_types::SolError>::SELECTOR
                },
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
            )
                -> alloy_sol_types::Result<LightClientErrors>] = &[
                {
                    fn OutdatedState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <OutdatedState as alloy_sol_types::SolError>::abi_decode_raw(data, validate)
                            .map(LightClientErrors::OutdatedState)
                    }
                    OutdatedState
                },
                {
                    fn InvalidProof(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidProof as alloy_sol_types::SolError>::abi_decode_raw(data, validate)
                            .map(LightClientErrors::InvalidProof)
                    }
                    InvalidProof
                },
                {
                    fn OwnableUnauthorizedAccount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::OwnableUnauthorizedAccount)
                    }
                    OwnableUnauthorizedAccount
                },
                {
                    fn FailedInnerCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn OwnableInvalidOwner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::OwnableInvalidOwner)
                    }
                    OwnableInvalidOwner
                },
                {
                    fn ERC1967InvalidImplementation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::ERC1967InvalidImplementation)
                    }
                    ERC1967InvalidImplementation
                },
                {
                    fn WrongStakeTableUsed(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::WrongStakeTableUsed)
                    }
                    WrongStakeTableUsed
                },
                {
                    fn InvalidHotShotBlockForCommitmentCheck(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(
                                LightClientErrors::InvalidHotShotBlockForCommitmentCheck,
                            )
                    }
                    InvalidHotShotBlockForCommitmentCheck
                },
                {
                    fn AddressEmptyCode(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::AddressEmptyCode)
                    }
                    AddressEmptyCode
                },
                {
                    fn InvalidArgs(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidArgs as alloy_sol_types::SolError>::abi_decode_raw(data, validate)
                            .map(LightClientErrors::InvalidArgs)
                    }
                    InvalidArgs
                },
                {
                    fn ProverNotPermissioned(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <ProverNotPermissioned as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::ProverNotPermissioned)
                    }
                    ProverNotPermissioned
                },
                {
                    fn NoChangeRequired(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <NoChangeRequired as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::NoChangeRequired)
                    }
                    NoChangeRequired
                },
                {
                    fn UUPSUnsupportedProxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::UUPSUnsupportedProxiableUUID)
                    }
                    UUPSUnsupportedProxiableUUID
                },
                {
                    fn InsufficientSnapshotHistory(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::InsufficientSnapshotHistory)
                    }
                    InsufficientSnapshotHistory
                },
                {
                    fn ERC1967NonPayable(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::ERC1967NonPayable)
                    }
                    ERC1967NonPayable
                },
                {
                    fn NotInitializing(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::NotInitializing)
                    }
                    NotInitializing
                },
                {
                    fn UUPSUnauthorizedCallContext(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::UUPSUnauthorizedCallContext)
                    }
                    UUPSUnauthorizedCallContext
                },
                {
                    fn InvalidAddress(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidAddress as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::InvalidAddress)
                    }
                    InvalidAddress
                },
                {
                    fn InvalidMaxStateHistory(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::InvalidMaxStateHistory)
                    }
                    InvalidMaxStateHistory
                },
                {
                    fn InvalidInitialization(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientErrors> {
                        <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(LightClientErrors::InvalidInitialization)
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
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encoded_size(
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
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InsufficientSnapshotHistory(inner) => {
                    <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidAddress(inner) => {
                    <InvalidAddress as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidArgs(inner) => {
                    <InvalidArgs as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::InvalidHotShotBlockForCommitmentCheck(inner) => {
                    <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidInitialization(inner) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidMaxStateHistory(inner) => {
                    <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::InvalidProof(inner) => {
                    <InvalidProof as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::NoChangeRequired(inner) => {
                    <NoChangeRequired as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::NotInitializing(inner) => {
                    <NotInitializing as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::OutdatedState(inner) => {
                    <OutdatedState as alloy_sol_types::SolError>::abi_encoded_size(inner)
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
                Self::ProverNotPermissioned(inner) => {
                    <ProverNotPermissioned as alloy_sol_types::SolError>::abi_encoded_size(
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
                Self::WrongStakeTableUsed(inner) => {
                    <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::FailedInnerCall(inner) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InsufficientSnapshotHistory(inner) => {
                    <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidAddress(inner) => {
                    <InvalidAddress as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidArgs(inner) => {
                    <InvalidArgs as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidHotShotBlockForCommitmentCheck(inner) => {
                    <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::InvalidMaxStateHistory(inner) => {
                    <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::InvalidProof(inner) => {
                    <InvalidProof as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::NoChangeRequired(inner) => {
                    <NoChangeRequired as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::OutdatedState(inner) => {
                    <OutdatedState as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::ProverNotPermissioned(inner) => {
                    <ProverNotPermissioned as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::WrongStakeTableUsed(inner) => {
                    <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
            }
        }
    }
    ///Container for all the [`LightClient`](self) events.
    pub enum LightClientEvents {
        #[allow(missing_docs)]
        Initialized(Initialized),
        #[allow(missing_docs)]
        NewState(NewState),
        #[allow(missing_docs)]
        OwnershipTransferred(OwnershipTransferred),
        #[allow(missing_docs)]
        PermissionedProverNotRequired(PermissionedProverNotRequired),
        #[allow(missing_docs)]
        PermissionedProverRequired(PermissionedProverRequired),
        #[allow(missing_docs)]
        Upgrade(Upgrade),
        #[allow(missing_docs)]
        Upgraded(Upgraded),
    }
    #[automatically_derived]
    impl LightClientEvents {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                128u8, 23u8, 187u8, 136u8, 127u8, 223u8, 143u8, 202u8, 67u8, 20u8, 169u8, 212u8,
                15u8, 110u8, 115u8, 179u8, 184u8, 16u8, 2u8, 214u8, 126u8, 92u8, 250u8, 133u8,
                216u8, 129u8, 115u8, 175u8, 106u8, 164u8, 96u8, 114u8,
            ],
            [
                139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8, 31u8, 208u8,
                164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8, 218u8, 175u8, 227u8,
                180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
            ],
            [
                154u8, 95u8, 87u8, 222u8, 133u8, 109u8, 214u8, 104u8, 197u8, 77u8, 217u8, 94u8,
                92u8, 85u8, 223u8, 147u8, 67u8, 33u8, 113u8, 203u8, 202u8, 73u8, 168u8, 119u8,
                109u8, 86u8, 32u8, 234u8, 89u8, 192u8, 36u8, 80u8,
            ],
            [
                160u8, 74u8, 119u8, 57u8, 36u8, 80u8, 90u8, 65u8, 133u8, 100u8, 54u8, 55u8, 37u8,
                245u8, 104u8, 50u8, 245u8, 119u8, 46u8, 107u8, 141u8, 13u8, 189u8, 110u8, 252u8,
                231u8, 36u8, 223u8, 232u8, 3u8, 218u8, 230u8,
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
                247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8, 154u8,
                22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8, 185u8, 62u8, 237u8,
                168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for LightClientEvents {
        const NAME: &'static str = "LightClientEvents";
        const COUNT: usize = 7usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::Initialized)
                },
                Some(<NewState as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <NewState as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::NewState)
                },
                Some(<OwnershipTransferred as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::OwnershipTransferred)
                },
                Some(
                    <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                    topics, data, validate,
                )
                .map(Self::PermissionedProverNotRequired),
                Some(<PermissionedProverRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <PermissionedProverRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data, validate,
                    )
                    .map(Self::PermissionedProverRequired)
                },
                Some(<Upgrade as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgrade as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Upgrade)
                },
                Some(<Upgraded as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgraded as alloy_sol_types::SolEvent>::decode_raw_log(topics, data, validate)
                        .map(Self::Upgraded)
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
    impl alloy_sol_types::private::IntoLogData for LightClientEvents {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::NewState(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::PermissionedProverNotRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::PermissionedProverRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::Upgrade(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
                Self::Upgraded(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::NewState(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::PermissionedProverNotRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::PermissionedProverRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
                Self::Upgrade(inner) => alloy_sol_types::private::IntoLogData::into_log_data(inner),
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                },
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`LightClient`](self) contract instance.

    See the [wrapper's documentation](`LightClientInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> LightClientInstance<T, P, N> {
        LightClientInstance::<T, P, N>::new(address, provider)
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
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<LightClientInstance<T, P, N>>>
    {
        LightClientInstance::<T, P, N>::deploy(provider)
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
        LightClientInstance::<T, P, N>::deploy_builder(provider)
    }
    /**A [`LightClient`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`LightClient`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct LightClientInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for LightClientInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("LightClientInstance")
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
        > LightClientInstance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`LightClient`](self) contract instance.

        See the [wrapper's documentation](`LightClientInstance`) for more details.*/
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
        pub async fn deploy(provider: P) -> alloy_contract::Result<LightClientInstance<T, P, N>> {
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
    impl<T, P: ::core::clone::Clone, N> LightClientInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> LightClientInstance<T, P, N> {
            LightClientInstance {
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
        > LightClientInstance<T, P, N>
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
        ///Creates a new call builder for the [`currentBlockNumber`] function.
        pub fn currentBlockNumber(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, currentBlockNumberCall, N> {
            self.call_builder(&currentBlockNumberCall {})
        }
        ///Creates a new call builder for the [`disablePermissionedProverMode`] function.
        pub fn disablePermissionedProverMode(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, disablePermissionedProverModeCall, N> {
            self.call_builder(&disablePermissionedProverModeCall {})
        }
        ///Creates a new call builder for the [`finalizedState`] function.
        pub fn finalizedState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, finalizedStateCall, N> {
            self.call_builder(&finalizedStateCall {})
        }
        ///Creates a new call builder for the [`genesisStakeTableState`] function.
        pub fn genesisStakeTableState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, genesisStakeTableStateCall, N> {
            self.call_builder(&genesisStakeTableStateCall {})
        }
        ///Creates a new call builder for the [`genesisState`] function.
        pub fn genesisState(&self) -> alloy_contract::SolCallBuilder<T, &P, genesisStateCall, N> {
            self.call_builder(&genesisStateCall {})
        }
        ///Creates a new call builder for the [`getHotShotCommitment`] function.
        pub fn getHotShotCommitment(
            &self,
            hotShotBlockHeight: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, getHotShotCommitmentCall, N> {
            self.call_builder(&getHotShotCommitmentCall { hotShotBlockHeight })
        }
        ///Creates a new call builder for the [`getStateHistoryCount`] function.
        pub fn getStateHistoryCount(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, getStateHistoryCountCall, N> {
            self.call_builder(&getStateHistoryCountCall {})
        }
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(&self) -> alloy_contract::SolCallBuilder<T, &P, getVersionCall, N> {
            self.call_builder(&getVersionCall {})
        }
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(
            &self,
            _genesis: <LightClientState as alloy::sol_types::SolType>::RustType,
            _genesisStakeTableState: <StakeTableState as alloy::sol_types::SolType>::RustType,
            _stateHistoryRetentionPeriod: u32,
            owner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeCall, N> {
            self.call_builder(&initializeCall {
                _genesis,
                _genesisStakeTableState,
                _stateHistoryRetentionPeriod,
                owner,
            })
        }
        ///Creates a new call builder for the [`isPermissionedProverEnabled`] function.
        pub fn isPermissionedProverEnabled(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, isPermissionedProverEnabledCall, N> {
            self.call_builder(&isPermissionedProverEnabledCall {})
        }
        ///Creates a new call builder for the [`lagOverEscapeHatchThreshold`] function.
        pub fn lagOverEscapeHatchThreshold(
            &self,
            blockNumber: alloy::sol_types::private::primitives::aliases::U256,
            blockThreshold: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, lagOverEscapeHatchThresholdCall, N> {
            self.call_builder(&lagOverEscapeHatchThresholdCall {
                blockNumber,
                blockThreshold,
            })
        }
        ///Creates a new call builder for the [`newFinalizedState`] function.
        pub fn newFinalizedState(
            &self,
            newState: <LightClientState as alloy::sol_types::SolType>::RustType,
            proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, newFinalizedStateCall, N> {
            self.call_builder(&newFinalizedStateCall { newState, proof })
        }
        ///Creates a new call builder for the [`owner`] function.
        pub fn owner(&self) -> alloy_contract::SolCallBuilder<T, &P, ownerCall, N> {
            self.call_builder(&ownerCall {})
        }
        ///Creates a new call builder for the [`permissionedProver`] function.
        pub fn permissionedProver(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, permissionedProverCall, N> {
            self.call_builder(&permissionedProverCall {})
        }
        ///Creates a new call builder for the [`proxiableUUID`] function.
        pub fn proxiableUUID(&self) -> alloy_contract::SolCallBuilder<T, &P, proxiableUUIDCall, N> {
            self.call_builder(&proxiableUUIDCall {})
        }
        ///Creates a new call builder for the [`renounceOwnership`] function.
        pub fn renounceOwnership(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceOwnershipCall, N> {
            self.call_builder(&renounceOwnershipCall {})
        }
        ///Creates a new call builder for the [`setPermissionedProver`] function.
        pub fn setPermissionedProver(
            &self,
            prover: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, setPermissionedProverCall, N> {
            self.call_builder(&setPermissionedProverCall { prover })
        }
        ///Creates a new call builder for the [`setstateHistoryRetentionPeriod`] function.
        pub fn setstateHistoryRetentionPeriod(
            &self,
            historySeconds: u32,
        ) -> alloy_contract::SolCallBuilder<T, &P, setstateHistoryRetentionPeriodCall, N> {
            self.call_builder(&setstateHistoryRetentionPeriodCall { historySeconds })
        }
        ///Creates a new call builder for the [`stateHistoryCommitments`] function.
        pub fn stateHistoryCommitments(
            &self,
            _0: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, stateHistoryCommitmentsCall, N> {
            self.call_builder(&stateHistoryCommitmentsCall { _0 })
        }
        ///Creates a new call builder for the [`stateHistoryFirstIndex`] function.
        pub fn stateHistoryFirstIndex(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, stateHistoryFirstIndexCall, N> {
            self.call_builder(&stateHistoryFirstIndexCall {})
        }
        ///Creates a new call builder for the [`stateHistoryRetentionPeriod`] function.
        pub fn stateHistoryRetentionPeriod(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, stateHistoryRetentionPeriodCall, N> {
            self.call_builder(&stateHistoryRetentionPeriodCall {})
        }
        ///Creates a new call builder for the [`transferOwnership`] function.
        pub fn transferOwnership(
            &self,
            newOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, transferOwnershipCall, N> {
            self.call_builder(&transferOwnershipCall { newOwner })
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
    }
    /// Event filters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > LightClientInstance<T, P, N>
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
        ///Creates a new event filter for the [`Initialized`] event.
        pub fn Initialized_filter(&self) -> alloy_contract::Event<T, &P, Initialized, N> {
            self.event_filter::<Initialized>()
        }
        ///Creates a new event filter for the [`NewState`] event.
        pub fn NewState_filter(&self) -> alloy_contract::Event<T, &P, NewState, N> {
            self.event_filter::<NewState>()
        }
        ///Creates a new event filter for the [`OwnershipTransferred`] event.
        pub fn OwnershipTransferred_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, OwnershipTransferred, N> {
            self.event_filter::<OwnershipTransferred>()
        }
        ///Creates a new event filter for the [`PermissionedProverNotRequired`] event.
        pub fn PermissionedProverNotRequired_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, PermissionedProverNotRequired, N> {
            self.event_filter::<PermissionedProverNotRequired>()
        }
        ///Creates a new event filter for the [`PermissionedProverRequired`] event.
        pub fn PermissionedProverRequired_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, PermissionedProverRequired, N> {
            self.event_filter::<PermissionedProverRequired>()
        }
        ///Creates a new event filter for the [`Upgrade`] event.
        pub fn Upgrade_filter(&self) -> alloy_contract::Event<T, &P, Upgrade, N> {
            self.event_filter::<Upgrade>()
        }
        ///Creates a new event filter for the [`Upgraded`] event.
        pub fn Upgraded_filter(&self) -> alloy_contract::Event<T, &P, Upgraded, N> {
            self.event_filter::<Upgraded>()
        }
    }
}
