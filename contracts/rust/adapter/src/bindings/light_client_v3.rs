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
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            pub const fn from_underlying(
                value: alloy::sol_types::private::primitives::aliases::U256,
            ) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into_underlying(
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
        impl From<alloy::sol_types::private::primitives::aliases::U256> for BaseField {
            fn from(value: alloy::sol_types::private::primitives::aliases::U256) -> Self {
                Self::from_underlying(value)
            }
        }
        #[automatically_derived]
        impl From<BaseField> for alloy::sol_types::private::primitives::aliases::U256 {
            fn from(value: BaseField) -> Self {
                value.into_underlying()
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            pub const fn from_underlying(
                value: alloy::sol_types::private::primitives::aliases::U256,
            ) -> Self {
                Self(value)
            }
            /// Return the underlying value.
            #[inline]
            pub const fn into_underlying(
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
        impl From<alloy::sol_types::private::primitives::aliases::U256> for ScalarField {
            fn from(value: alloy::sol_types::private::primitives::aliases::U256) -> Self {
                Self::from_underlying(value)
            }
        }
        #[automatically_derived]
        impl From<ScalarField> for alloy::sol_types::private::primitives::aliases::U256 {
            fn from(value: ScalarField) -> Self {
                value.into_underlying()
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> BN254Instance<P, N> {
        BN254Instance::<P, N>::new(address, provider)
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
    pub struct BN254Instance<P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for BN254Instance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("BN254Instance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        BN254Instance<P, N>
    {
        /**Creates a new wrapper around an on-chain [`BN254`](self) contract instance.

        See the [wrapper's documentation](`BN254Instance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
            Self {
                address,
                provider,
                _network: ::core::marker::PhantomData,
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
    impl<P: ::core::clone::Clone, N> BN254Instance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> BN254Instance<P, N> {
            BN254Instance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        BN254Instance<P, N>
    {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<&P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        BN254Instance<P, N>
    {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<&P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
    }
}
///Module containing a contract's types and functions.
/**

```solidity
library IPlonkVerifier {
    struct PlonkProof { BN254.G1Point wire0; BN254.G1Point wire1; BN254.G1Point wire2; BN254.G1Point wire3; BN254.G1Point wire4; BN254.G1Point prodPerm; BN254.G1Point split0; BN254.G1Point split1; BN254.G1Point split2; BN254.G1Point split3; BN254.G1Point split4; BN254.G1Point zeta; BN254.G1Point zetaOmega; BN254.ScalarField wireEval0; BN254.ScalarField wireEval1; BN254.ScalarField wireEval2; BN254.ScalarField wireEval3; BN254.ScalarField wireEval4; BN254.ScalarField sigmaEval0; BN254.ScalarField sigmaEval1; BN254.ScalarField sigmaEval2; BN254.ScalarField sigmaEval3; BN254.ScalarField prodPermZetaOmegaEval; }
    struct VerifyingKey { uint256 domainSize; uint256 numInputs; BN254.G1Point sigma0; BN254.G1Point sigma1; BN254.G1Point sigma2; BN254.G1Point sigma3; BN254.G1Point sigma4; BN254.G1Point q1; BN254.G1Point q2; BN254.G1Point q3; BN254.G1Point q4; BN254.G1Point qM12; BN254.G1Point qM34; BN254.G1Point qO; BN254.G1Point qC; BN254.G1Point qH1; BN254.G1Point qH2; BN254.G1Point qH3; BN254.G1Point qH4; BN254.G1Point qEcc; bytes32 g2LSB; bytes32 g2MSB; }
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
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(serde::Serialize, serde::Deserialize)]
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
                    "PlonkProof(G1Point wire0,G1Point wire1,G1Point wire2,G1Point wire3,G1Point wire4,G1Point prodPerm,G1Point split0,G1Point split1,G1Point split2,G1Point split3,G1Point split4,G1Point zeta,G1Point zetaOmega,uint256 wireEval0,uint256 wireEval1,uint256 wireEval2,uint256 wireEval3,uint256 wireEval4,uint256 sigmaEval0,uint256 sigmaEval1,uint256 sigmaEval2,uint256 sigmaEval3,uint256 prodPermZetaOmegaEval)",
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
    #[derive(serde::Serialize, serde::Deserialize)]
    /**```solidity
    struct VerifyingKey { uint256 domainSize; uint256 numInputs; BN254.G1Point sigma0; BN254.G1Point sigma1; BN254.G1Point sigma2; BN254.G1Point sigma3; BN254.G1Point sigma4; BN254.G1Point q1; BN254.G1Point q2; BN254.G1Point q3; BN254.G1Point q4; BN254.G1Point qM12; BN254.G1Point qM34; BN254.G1Point qO; BN254.G1Point qC; BN254.G1Point qH1; BN254.G1Point qH2; BN254.G1Point qH3; BN254.G1Point qH4; BN254.G1Point qEcc; bytes32 g2LSB; bytes32 g2MSB; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct VerifyingKey {
        #[allow(missing_docs)]
        pub domainSize: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub numInputs: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub sigma0: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigma1: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigma2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigma3: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub sigma4: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub q1: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub q2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub q3: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub q4: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qM12: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qM34: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qO: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qC: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qH1: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qH2: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qH3: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qH4: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub qEcc: <BN254::G1Point as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub g2LSB: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub g2MSB: alloy::sol_types::private::FixedBytes<32>,
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
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            BN254::G1Point,
            alloy::sol_types::sol_data::FixedBytes<32>,
            alloy::sol_types::sol_data::FixedBytes<32>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            alloy::sol_types::private::primitives::aliases::U256,
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
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
            <BN254::G1Point as alloy::sol_types::SolType>::RustType,
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
        impl ::core::convert::From<VerifyingKey> for UnderlyingRustTuple<'_> {
            fn from(value: VerifyingKey) -> Self {
                (
                    value.domainSize,
                    value.numInputs,
                    value.sigma0,
                    value.sigma1,
                    value.sigma2,
                    value.sigma3,
                    value.sigma4,
                    value.q1,
                    value.q2,
                    value.q3,
                    value.q4,
                    value.qM12,
                    value.qM34,
                    value.qO,
                    value.qC,
                    value.qH1,
                    value.qH2,
                    value.qH3,
                    value.qH4,
                    value.qEcc,
                    value.g2LSB,
                    value.g2MSB,
                )
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for VerifyingKey {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    domainSize: tuple.0,
                    numInputs: tuple.1,
                    sigma0: tuple.2,
                    sigma1: tuple.3,
                    sigma2: tuple.4,
                    sigma3: tuple.5,
                    sigma4: tuple.6,
                    q1: tuple.7,
                    q2: tuple.8,
                    q3: tuple.9,
                    q4: tuple.10,
                    qM12: tuple.11,
                    qM34: tuple.12,
                    qO: tuple.13,
                    qC: tuple.14,
                    qH1: tuple.15,
                    qH2: tuple.16,
                    qH3: tuple.17,
                    qH4: tuple.18,
                    qEcc: tuple.19,
                    g2LSB: tuple.20,
                    g2MSB: tuple.21,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for VerifyingKey {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for VerifyingKey {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.domainSize),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.numInputs),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.sigma0),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.sigma1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.sigma2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.sigma3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.sigma4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.q1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.q2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.q3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.q4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qM12),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qM34),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qO),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qC),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qH1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qH2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qH3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qH4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.qEcc),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.g2LSB),
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.g2MSB),
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
        impl alloy_sol_types::SolType for VerifyingKey {
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
        impl alloy_sol_types::SolStruct for VerifyingKey {
            const NAME: &'static str = "VerifyingKey";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "VerifyingKey(uint256 domainSize,uint256 numInputs,G1Point sigma0,G1Point sigma1,G1Point sigma2,G1Point sigma3,G1Point sigma4,G1Point q1,G1Point q2,G1Point q3,G1Point q4,G1Point qM12,G1Point qM34,G1Point qO,G1Point qC,G1Point qH1,G1Point qH2,G1Point qH3,G1Point qH4,G1Point qEcc,bytes32 g2LSB,bytes32 g2MSB)",
                )
            }
            #[inline]
            fn eip712_components(
            ) -> alloy_sol_types::private::Vec<alloy_sol_types::private::Cow<'static, str>>
            {
                let mut components = alloy_sol_types::private::Vec::with_capacity(18);
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.domainSize)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.numInputs)
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.sigma0,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.sigma1,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.sigma2,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.sigma3,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.sigma4,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.q1,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.q2,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.q3,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.q4,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qM12,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qM34,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qO,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qC,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qH1,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qH2,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qH3,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qH4,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.qEcc,
                        )
                        .0,
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.g2LSB)
                        .0,
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.g2MSB)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for VerifyingKey {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.domainSize,
                    )
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.numInputs,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigma0,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigma1,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigma2,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigma3,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sigma4,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.q1,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.q2,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.q3,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.q4,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qM12,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qM34,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qO,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qC,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qH1,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qH2,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qH3,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qH4,
                    )
                    + <BN254::G1Point as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.qEcc,
                    )
                    + <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.g2LSB)
                    + <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(&rust.g2MSB)
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
                    &rust.domainSize,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.numInputs,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigma0,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigma1,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigma2,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigma3,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sigma4,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q1, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q2, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q3, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q4, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qM12, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qM34, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qO, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qC, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH1, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH2, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH3, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH4, out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qEcc, out,
                );
                <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.g2LSB,
                    out,
                );
                <alloy::sol_types::sol_data::FixedBytes<
                    32,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.g2MSB,
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
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> IPlonkVerifierInstance<P, N> {
        IPlonkVerifierInstance::<P, N>::new(address, provider)
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
    pub struct IPlonkVerifierInstance<P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for IPlonkVerifierInstance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("IPlonkVerifierInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        IPlonkVerifierInstance<P, N>
    {
        /**Creates a new wrapper around an on-chain [`IPlonkVerifier`](self) contract instance.

        See the [wrapper's documentation](`IPlonkVerifierInstance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
            Self {
                address,
                provider,
                _network: ::core::marker::PhantomData,
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
    impl<P: ::core::clone::Clone, N> IPlonkVerifierInstance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> IPlonkVerifierInstance<P, N> {
            IPlonkVerifierInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        IPlonkVerifierInstance<P, N>
    {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<&P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        IPlonkVerifierInstance<P, N>
    {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<&P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
    }
}
///Module containing a contract's types and functions.
/**

```solidity
library LightClient {
    struct LightClientState { uint64 viewNum; uint64 blockHeight; BN254.ScalarField blockCommRoot; }
    struct StakeTableState { uint256 threshold; BN254.ScalarField blsKeyComm; BN254.ScalarField schnorrKeyComm; BN254.ScalarField amountComm; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod LightClient {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`LightClient`](self) contract instance.

    See the [wrapper's documentation](`LightClientInstance`) for more details.*/
    #[inline]
    pub const fn new<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> LightClientInstance<P, N> {
        LightClientInstance::<P, N>::new(address, provider)
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
    pub struct LightClientInstance<P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for LightClientInstance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("LightClientInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientInstance<P, N>
    {
        /**Creates a new wrapper around an on-chain [`LightClient`](self) contract instance.

        See the [wrapper's documentation](`LightClientInstance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
            Self {
                address,
                provider,
                _network: ::core::marker::PhantomData,
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
    impl<P: ::core::clone::Clone, N> LightClientInstance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> LightClientInstance<P, N> {
            LightClientInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientInstance<P, N>
    {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<&P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientInstance<P, N>
    {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<&P, E, N> {
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
    struct VerifyingKey {
        uint256 domainSize;
        uint256 numInputs;
        BN254.G1Point sigma0;
        BN254.G1Point sigma1;
        BN254.G1Point sigma2;
        BN254.G1Point sigma3;
        BN254.G1Point sigma4;
        BN254.G1Point q1;
        BN254.G1Point q2;
        BN254.G1Point q3;
        BN254.G1Point q4;
        BN254.G1Point qM12;
        BN254.G1Point qM34;
        BN254.G1Point qO;
        BN254.G1Point qC;
        BN254.G1Point qH1;
        BN254.G1Point qH2;
        BN254.G1Point qH3;
        BN254.G1Point qH4;
        BN254.G1Point qEcc;
        bytes32 g2LSB;
        bytes32 g2MSB;
    }
}

library LightClient {
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
}

interface LightClientV3 {
    error AddressEmptyCode(address target);
    error DeprecatedApi();
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
    error InvalidScalar();
    error MissingEpochRootUpdate();
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
    event NewEpoch(uint64 epoch);
    event NewState(uint64 indexed viewNum, uint64 indexed blockHeight, BN254.ScalarField blockCommRoot);
    event OwnershipTransferred(address indexed previousOwner, address indexed newOwner);
    event PermissionedProverNotRequired();
    event PermissionedProverRequired(address permissionedProver);
    event Upgrade(address implementation);
    event Upgraded(address indexed implementation);

    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    function _getVk() external pure returns (IPlonkVerifier.VerifyingKey memory vk);
    function authRoot() external view returns (uint256);
    function blocksPerEpoch() external view returns (uint64);
    function currentBlockNumber() external view returns (uint256);
    function currentEpoch() external view returns (uint64);
    function disablePermissionedProverMode() external;
    function epochFromBlockNumber(uint64 _blockNum, uint64 _blocksPerEpoch) external pure returns (uint64);
    function epochStartBlock() external view returns (uint64);
    function finalizedState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    function genesisStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
    function genesisState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    function getHotShotCommitment(uint256 hotShotBlockHeight) external view returns (BN254.ScalarField hotShotBlockCommRoot, uint64 hotshotBlockHeight);
    function getStateHistoryCount() external view returns (uint256);
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    function initialize(LightClient.LightClientState memory _genesis, LightClient.StakeTableState memory _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) external;
    function initializeV2(uint64 _blocksPerEpoch, uint64 _epochStartBlock) external;
    function initializeV3() external;
    function isEpochRoot(uint64 blockHeight) external view returns (bool);
    function isGtEpochRoot(uint64 blockHeight) external view returns (bool);
    function isPermissionedProverEnabled() external view returns (bool);
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 blockThreshold) external view returns (bool);
    function newFinalizedState(LightClient.LightClientState memory, IPlonkVerifier.PlonkProof memory) external pure;
    function newFinalizedState(LightClient.LightClientState memory, LightClient.StakeTableState memory, IPlonkVerifier.PlonkProof memory) external pure;
    function newFinalizedState(LightClient.LightClientState memory newState, LightClient.StakeTableState memory nextStakeTable, uint256 newAuthRoot, IPlonkVerifier.PlonkProof memory proof) external;
    function owner() external view returns (address);
    function permissionedProver() external view returns (address);
    function proxiableUUID() external view returns (bytes32);
    function renounceOwnership() external;
    function setPermissionedProver(address prover) external;
    function setStateHistoryRetentionPeriod(uint32 historySeconds) external;
    function setstateHistoryRetentionPeriod(uint32 historySeconds) external;
    function stateHistoryCommitments(uint256) external view returns (uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, BN254.ScalarField hotShotBlockCommRoot);
    function stateHistoryFirstIndex() external view returns (uint64);
    function stateHistoryRetentionPeriod() external view returns (uint32);
    function transferOwnership(address newOwner) external;
    function updateEpochStartBlock(uint64 newEpochStartBlock) external;
    function upgradeToAndCall(address newImplementation, bytes memory data) external payable;
    function votingStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
}
```

...which was generated by the following JSON ABI:
```json
[
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
    "name": "_getVk",
    "inputs": [],
    "outputs": [
      {
        "name": "vk",
        "type": "tuple",
        "internalType": "struct IPlonkVerifier.VerifyingKey",
        "components": [
          {
            "name": "domainSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "numInputs",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sigma0",
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
            "name": "sigma1",
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
            "name": "sigma2",
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
            "name": "sigma3",
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
            "name": "sigma4",
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
            "name": "q1",
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
            "name": "q2",
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
            "name": "q3",
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
            "name": "q4",
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
            "name": "qM12",
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
            "name": "qM34",
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
            "name": "qO",
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
            "name": "qC",
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
            "name": "qH1",
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
            "name": "qH2",
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
            "name": "qH3",
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
            "name": "qH4",
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
            "name": "qEcc",
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
            "name": "g2LSB",
            "type": "bytes32",
            "internalType": "bytes32"
          },
          {
            "name": "g2MSB",
            "type": "bytes32",
            "internalType": "bytes32"
          }
        ]
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "authRoot",
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
    "name": "blocksPerEpoch",
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
    "name": "currentEpoch",
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
    "name": "disablePermissionedProverMode",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "epochFromBlockNumber",
    "inputs": [
      {
        "name": "_blockNum",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "_blocksPerEpoch",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "epochStartBlock",
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
    "name": "initializeV2",
    "inputs": [
      {
        "name": "_blocksPerEpoch",
        "type": "uint64",
        "internalType": "uint64"
      },
      {
        "name": "_epochStartBlock",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "initializeV3",
    "inputs": [],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "isEpochRoot",
    "inputs": [
      {
        "name": "blockHeight",
        "type": "uint64",
        "internalType": "uint64"
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
    "name": "isGtEpochRoot",
    "inputs": [
      {
        "name": "blockHeight",
        "type": "uint64",
        "internalType": "uint64"
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
        "name": "",
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
        "name": "",
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
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "newFinalizedState",
    "inputs": [
      {
        "name": "",
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
        "name": "",
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
        "name": "",
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
    "stateMutability": "pure"
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
        "name": "nextStakeTable",
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
        "name": "newAuthRoot",
        "type": "uint256",
        "internalType": "uint256"
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
    "name": "setStateHistoryRetentionPeriod",
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
    "name": "updateEpochStartBlock",
    "inputs": [
      {
        "name": "newEpochStartBlock",
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
    "name": "votingStakeTableState",
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
    "name": "NewEpoch",
    "inputs": [
      {
        "name": "epoch",
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
    "name": "DeprecatedApi",
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
    "name": "InvalidScalar",
    "inputs": []
  },
  {
    "type": "error",
    "name": "MissingEpochRootUpdate",
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
pub mod LightClientV3 {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x60a060405230608052348015610013575f5ffd5b5061001c610021565b6100d3565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff16156100715760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b03908116146100d05780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b6080516136b86100f95f395f8181611c5b01528181611c840152611e0101526136b85ff3fe608060405260043610610228575f3560e01c8063715018a6116101295780639fdb54a7116100a8578063d24d933d1161006d578063d24d933d1461075d578063e03033011461078c578063f0682054146107ab578063f2fde38b146107ca578063f9e50d19146107e9575f5ffd5b80639fdb54a714610655578063aabd5db3146106aa578063ad3cb1cc146106c9578063b33bc49114610706578063c23b9e9e14610725575f5ffd5b80638da5cb5b116100ee5780638da5cb5b146105a757806390c14390146105e357806396c1ca6114610602578063998328e8146106215780639baa3cc914610636575f5ffd5b8063715018a61461050a578063757c37ad1461051e5780637667180814610538578063826e41fc1461054c5780638584d23f1461056b575f5ffd5b8063300c89dd116101b5578063426d31941161017a578063426d31941461048f578063433dba9f146104b05780634f1ef286146104cf57806352d1902d146104e257806369cc6a04146104f6575f5ffd5b8063300c89dd146103e3578063313df7b114610402578063378ec23b1461043957806338e454b1146104555780633ed55b7b14610469575f5ffd5b806312173c2c116101fb57806312173c2c14610317578063167ac618146103385780632063d4f71461035757806325297427146103765780632f79889d146103a5575f5ffd5b8063013fa5fc1461022c57806302b592f31461024d5780630625e19b146102aa5780630d8e6e2c146102ec575b5f5ffd5b348015610237575f5ffd5b5061024b6102463660046128f0565b6107fd565b005b348015610258575f5ffd5b5061026c610267366004612909565b6108b0565b6040516102a194939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b3480156102b5575f5ffd5b50600b54600c54600d54600e546102cc9392919084565b6040805194855260208501939093529183015260608201526080016102a1565b3480156102f7575f5ffd5b5060408051600381525f60208201819052918101919091526060016102a1565b348015610322575f5ffd5b5061032b6108f9565b6040516102a19190612920565b348015610343575f5ffd5b5061024b610352366004612b37565b61090e565b348015610362575f5ffd5b5061024b610371366004612df9565b610985565b348015610381575f5ffd5b50610395610390366004612b37565b61099e565b60405190151581526020016102a1565b3480156103b0575f5ffd5b506008546103cb90600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016102a1565b3480156103ee575f5ffd5b506103956103fd366004612b37565b610a00565b34801561040d575f5ffd5b50600854610421906001600160a01b031681565b6040516001600160a01b0390911681526020016102a1565b348015610444575f5ffd5b50435b6040519081526020016102a1565b348015610460575f5ffd5b5061024b610a95565b348015610474575f5ffd5b50600a546103cb90600160401b90046001600160401b031681565b34801561049a575f5ffd5b505f546001546002546003546102cc9392919084565b3480156104bb575f5ffd5b5061024b6104ca366004612e40565b610b8c565b61024b6104dd366004612e59565b610ba0565b3480156104ed575f5ffd5b50610447610bbf565b348015610501575f5ffd5b5061024b610bda565b348015610515575f5ffd5b5061024b610c48565b348015610529575f5ffd5b5061024b610371366004612f5c565b348015610543575f5ffd5b506103cb610c59565b348015610557575f5ffd5b506008546001600160a01b03161515610395565b348015610576575f5ffd5b5061058a610585366004612909565b610c7e565b604080519283526001600160401b039091166020830152016102a1565b3480156105b2575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b0316610421565b3480156105ee575f5ffd5b506103cb6105fd366004612fa0565b610da9565b34801561060d575f5ffd5b5061024b61061c366004612e40565b610e18565b34801561062c575f5ffd5b50610447600f5481565b348015610641575f5ffd5b5061024b610650366004612fc8565b610ea1565b348015610660575f5ffd5b50600654600754610684916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016102a1565b3480156106b5575f5ffd5b5061024b6106c436600461301d565b610fb0565b3480156106d4575f5ffd5b506106f9604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516102a19190613061565b348015610711575f5ffd5b5061024b610720366004612fa0565b6112d5565b348015610730575f5ffd5b5060085461074890600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016102a1565b348015610768575f5ffd5b50600454600554610684916001600160401b0380821692600160401b909204169083565b348015610797575f5ffd5b506103956107a6366004613096565b61142e565b3480156107b6575f5ffd5b50600a546103cb906001600160401b031681565b3480156107d5575f5ffd5b5061024b6107e43660046128f0565b611586565b3480156107f4575f5ffd5b50600954610447565b6108056115c5565b6001600160a01b03811661082c5760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b039081169082160361085b5760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b600981815481106108bf575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b610901612657565b610909611620565b905090565b6109166115c5565b600a80546fffffffffffffffff0000000000000000198116600160401b6001600160401b0385811682029283179485905561095c94919091048116928116911617610da9565b600a60106101000a8154816001600160401b0302191690836001600160401b0316021790555050565b604051634e405c8d60e01b815260040160405180910390fd5b5f6001600160401b03821615806109be5750600a546001600160401b0316155b156109ca57505f919050565b600a546001600160401b03166109e18360056130ca565b6109eb91906130fd565b6001600160401b03161592915050565b919050565b5f6001600160401b0382161580610a205750600a546001600160401b0316155b15610a2c57505f919050565b600a54610a42906001600160401b0316836130fd565b6001600160401b03161580610a8f5750600a54610a6a906005906001600160401b031661312a565b600a546001600160401b0391821691610a849116846130fd565b6001600160401b0316115b92915050565b610a9d6115c5565b5f51602061368c5f395f51905f52805460039190600160401b900460ff1680610ad3575080546001600160401b03808416911610155b15610af15760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b9081178355600a54610b2b9291810482169116610da9565b6010805467ffffffffffffffff19166001600160401b03928316179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050565b610b946115c5565b610b9d81610e18565b50565b610ba8611c50565b610bb182611cf4565b610bbb8282611d35565b5050565b5f610bc8611df6565b505f51602061366c5f395f51905f5290565b610be26115c5565b6008546001600160a01b031615610c2d57600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b610c506115c5565b610c465f611e3f565b600654600a545f91610909916001600160401b03600160401b90920482169116610da9565b600980545f91829190610c92600183613149565b81548110610ca257610ca261315c565b5f918252602090912060029091020154600160801b90046001600160401b03168410610ce157604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b81811015610da2578460098281548110610d1157610d1161315c565b5f918252602090912060029091020154600160801b90046001600160401b03161115610d9a5760098181548110610d4a57610d4a61315c565b905f5260205f2090600202016001015460098281548110610d6d57610d6d61315c565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b600101610cf5565b5050915091565b5f816001600160401b03165f03610dc157505f610a8f565b826001600160401b03165f03610dd957506001610a8f565b610de382846130fd565b6001600160401b03165f03610e0357610dfc8284613170565b9050610a8f565b610e0d8284613170565b610dfc9060016130ca565b610e206115c5565b610e108163ffffffff161080610e3f57506301e133808163ffffffff16115b80610e5d575060085463ffffffff600160a01b909104811690821611155b15610e7b576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b5f51602061368c5f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015610ed25750825b90505f826001600160401b03166001148015610eed5750303b155b905081158015610efb575080155b15610f195760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610f4357845460ff60401b1916600160401b1785555b610f4c86611eaf565b610f54611ec0565b610f5f898989611ec8565b8315610fa557845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6008546001600160a01b031615158015610fd557506008546001600160a01b03163314155b15610ff3576040516301474c8f60e71b815260040160405180910390fd5b60065484516001600160401b03918216911611158061102c575060065460208501516001600160401b03600160401b9092048216911611155b1561104a5760405163051c46ef60e01b815260040160405180910390fd5b6110578460400151611ff4565b6110648360200151611ff4565b6110718360400151611ff4565b61107e8360600151611ff4565b5f611087610c59565b6020860151600a549192505f916110a791906001600160401b0316610da9565b6010549091506001600160401b03908116908216106110eb576110cd8660200151610a00565b156110eb5760405163080ae8d960e01b815260040160405180910390fd5b6010546001600160401b03908116908216111561119757600261110e838361312a565b6001600160401b0316106111355760405163080ae8d960e01b815260040160405180910390fd5b6111408260016130ca565b6001600160401b0316816001600160401b0316148015611179575060065461117790600160401b90046001600160401b031661099e565b155b156111975760405163080ae8d960e01b815260040160405180910390fd5b6111a386868686612035565b85516006805460208901516001600160401b03908116600160401b026001600160801b0319909216938116939093171790556040870151600755600f8590556010548116908216108015906112005750611200866020015161099e565b1561126a578451600b556020850151600c556040850151600d556060850151600e557f31eabd9099fdb25dacddd206abff87311e553441fc9d0fcdef201062d7e7071b61124e8260016130ca565b6040516001600160401b03909116815260200160405180910390a15b61127543428861225f565b85602001516001600160401b0316865f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae688604001516040516112c591815260200190565b60405180910390a3505050505050565b6112dd6115c5565b5f51602061368c5f395f51905f52805460029190600160401b900460ff1680611313575080546001600160401b03808416911610155b156113315760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b178255600590851611611379576040516350dd03f760e11b815260040160405180910390fd5b5f54600b55600154600c55600254600d55600354600e55600a80546001600160401b03858116600160401b026001600160801b0319909216908716171790556113c28385610da9565b600a805467ffffffffffffffff60801b1916600160801b6001600160401b0393841602179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b6009545f904384118061143f575080155b806114895750600854600980549091600160c01b90046001600160401b031690811061146d5761146d61315c565b5f9182526020909120600290910201546001600160401b031684105b156114a75760405163b0b4387760e01b815260040160405180910390fd5b5f80806114b5600185613149565b90505b8161155157600854600160c01b90046001600160401b031681106115515786600982815481106114ea576114ea61315c565b5f9182526020909120600290910201546001600160401b03161161153f57600191506009818154811061151f5761151f61315c565b5f9182526020909120600290910201546001600160401b03169250611551565b806115498161319d565b9150506114b8565b8161156f5760405163b0b4387760e01b815260040160405180910390fd5b8561157a8489613149565b11979650505050505050565b61158e6115c5565b6001600160a01b0381166115bc57604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610b9d81611e3f565b336115f77f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610c465760405163118cdaa760e01b81523360048201526024016115b3565b611628612657565b621000008152600560208201527f2949260dc9e9621bb41dcb96ba7054b4bd5e7e230fdba5f3411260401c55f59d6040820151527f05d036973845e2e9d2ad9a795b351535a2576d51d27f21ff8372be92bd6f39466020604083015101527f0ba2c5ae9360efec9e3968e33f57fd33059e57385c1ea7db6430426b82e0871a6060820151527f1e333b5398c953194076772a861b7bf6a4c80c4a4c2e54eb9ca67aec5ff19fc96020606083015101527f0d9e9b9f38dd9fbbd5cd8b5a1d1c8aa4e777e526e06efe39345bf3ce4c5bb4aa6080820151527f10417eaf9ba330bbf56caf331a362114153a9c95ae914fbd1f99cb84d59fbf566020608083015101527f155dfc3a039f16ab99fa9663569ff06e5bfda91748a79821d80dafc7f1d92e5e60a0820151527f15daee81e8ffcac886bf9cc7453d659a987da1feb893c1fe9a94583f337f6dfa602060a083015101527f1c6f995727083f56734a4863c3bf4433b5353ad8d20f15d554a8cd2be28ef92d60c0820151527f0736ebbf0d73d42c428d5dd66ba4d9d9513a642d94147db629964d6d032776a8602060c083015101527f2c4aa1a42d17f226532742b7da21ed908ee6a1c13d824b269d21abcd59c8672360e0820151527f05c4163ca9cab2e65abbb41b6175591cf92460000c96fb9daa1f01d50af4936c602060e083015101527f215ecf683c65ee3dca3c2fc04b4864b1f2a538ef923af6380d420fa6b5a9f496610100820151527f1d03c378f3d7063d12c459ac659ce7a27c439cd6ad184c172352815f3a380d37602061010083015101527f20bc29548f10bd07fde418d49a5692f8919694571ab64c90f583dc434a5fec0c610120820151527f244e5fcb51c747a56fe6fdb32f0b01ef3bc55627f6f9afcd98dddbede50308a3602061012083015101527f0e3646b352d00a3482e89811f4966fb646889dadb561ebb7bb7c223e8196d5b3610140820151527f1b10219a6293abaf30388f39e4c7b925f89b6f57cb81654e1ad755294e790f09602061014083015101527f2b29b36cd6d33062a9a86e24bd178d69b1cebdc1a39c7977d547e7617b5747f9610160820151527f17062161c0a63cd17cee5b14821d7820e7fa432323b122ba59c44dd01f6a9238602061016083015101527f1198db3cec1a66ccdb90886bb96fcf175316c6ea78f73f23f4a11bcf4320e11a610180820151527f063b1f963e732bd20d86e1fef855788c1aacf26babb526d84e30633a2b5a9469602061018083015101527f23809a6a5bb0bf088f97efe15168a39471a3a4e41b8d6db0100e15fa68b09f636101a0820151527f0aba7b69ab7fdda68dac9065a5ee9fb50abfe57bdb5ab359cc5b56dff65cbea160206101a083015101527f1f038064d3ca1f37c56ecfe41701f15a412c63d3c9ad52fcfd3fd4c64da8b5f26101c0820151527f2689fe5cc59e4be112c2479969c25a7f603a5d71a2e7924480c9f4eafc2c298f60206101c083015101527f113021e93328a91531e40871481c4714e0b99a6afb10c779eeb2b07a7ae6f4e76101e0820151527f1a36bb2620cdb40c4dad25257716a9d8eb1e45f715ada98e424697aaf4d95c8660206101e083015101527f08f3f88ffb9e43261294b7faf582c513f9c7d0749db6dcc434d7493b8c975b2f610200820151527f2e3e0458741119ad1422072b6815fda80a3896640f018d282c88f1506b54e0e6602061020083015101527f100a5c0a4e1ac2791d1f68bc9c25b39ccfbb5d628c53d5547f89aa0cab8324d2610220820151527f05bf9e97428c387fbbc5f9cbf6effb33b57655494c2ab9f7cc5d445a0ea56bea602061022083015101527f067f3e0ce69cbbe32337f0538bf6119c72f7fd4d92857b785caf04a225b94d46610240820151527f211a076271069fb1fae1522ab8a4779480b50ed8c4648d201341e444e8ee2d15602061024083015101527f0b931b96997d9db8bc198c750098cad2960df407880f7b2cb51c85376d5fc849610260820151527f0e9121af76d7d9616432ded6a4de93cf146f5b7353a74f8a7265d6377fd4edc7602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611cd657507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611cca5f51602061366c5f395f51905f52546001600160a01b031690565b6001600160a01b031614155b15610c465760405163703e46dd60e11b815260040160405180910390fd5b611cfc6115c5565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d906020016108a5565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611d8f575060408051601f3d908101601f19168201909252611d8c918101906131b2565b60015b611db757604051634c9c8ce360e01b81526001600160a01b03831660048201526024016115b3565b5f51602061366c5f395f51905f528114611de757604051632a87526960e21b8152600481018290526024016115b3565b611df18383612448565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610c465760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611eb761249d565b610b9d816124d3565b610c4661249d565b82516001600160401b0316151580611eec575060208301516001600160401b031615155b80611ef957506020820151155b80611f0657506040820151155b80611f1357506060820151155b80611f1d57508151155b80611f2f5750610e108163ffffffff16105b80611f4357506301e133808163ffffffff16115b15611f61576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001811080610bbb5760405163016c173360e21b815260040160405180910390fd5b5f61203e6108f9565b90506120486128bc565b600c548152600d54602080830191909152600e546040830152600b54606080840191909152600a549188015190916001600160401b03600160401b90910481169116108015906120a057506120a0876020015161099e565b156120e9576040805187516020808301919091528801518183015290870151606080830191909152870151608082015260a0016040516020818303038152906040529050612124565b60408051600b546020820152600c5491810191909152600d546060820152600e54608082015260a00160405160208183030381529060405290505b6040805188516001600160401b039081166020808401919091528a015116818301529088015160608201525f9060800160408051601f198184030181529082905261217591849089906020016131e0565b60408051601f19818403018152919052805160209091012090506121b97f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000182613202565b60808401526040516354e8bd6760e01b815273ffffffffffffffffffffffffffffffffffffffff906354e8bd67906121f990879087908a906004016133f7565b602060405180830381865af4158015612214573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906122389190613617565b612255576040516309bde33960e01b815260040160405180910390fd5b5050505050505050565b600954158015906122d4575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b031690811061229f5761229f61315c565b5f9182526020909120600290910201546122c990600160401b90046001600160401b03168461312a565b6001600160401b0316115b1561236757600854600980549091600160c01b90046001600160401b03169081106123015761230161315c565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b031690601861234183613636565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b612451826124db565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561249557611df1828261253e565b610bbb6125b0565b5f51602061368c5f395f51905f5254600160401b900460ff16610c4657604051631afcd79f60e31b815260040160405180910390fd5b61158e61249d565b806001600160a01b03163b5f0361251057604051634c9c8ce360e01b81526001600160a01b03821660048201526024016115b3565b5f51602061366c5f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b03168460405161255a9190613660565b5f60405180830381855af49150503d805f8114612592576040519150601f19603f3d011682016040523d82523d5f602084013e612597565b606091505b50915091506125a78583836125cf565b95945050505050565b3415610c465760405163b398979f60e01b815260040160405180910390fd5b6060826125e4576125df8261262e565b612627565b81511580156125fb57506001600160a01b0384163b155b1561262457604051639996b31560e01b81526001600160a01b03851660048201526024016115b3565b50805b9392505050565b80511561263e5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b604051806102c001604052805f81526020015f815260200161268a60405180604001604052805f81526020015f81525090565b81526020016126aa60405180604001604052805f81526020015f81525090565b81526020016126ca60405180604001604052805f81526020015f81525090565b81526020016126ea60405180604001604052805f81526020015f81525090565b815260200161270a60405180604001604052805f81526020015f81525090565b815260200161272a60405180604001604052805f81526020015f81525090565b815260200161274a60405180604001604052805f81526020015f81525090565b815260200161276a60405180604001604052805f81526020015f81525090565b815260200161278a60405180604001604052805f81526020015f81525090565b81526020016127aa60405180604001604052805f81526020015f81525090565b81526020016127ca60405180604001604052805f81526020015f81525090565b81526020016127ea60405180604001604052805f81526020015f81525090565b815260200161280a60405180604001604052805f81526020015f81525090565b815260200161282a60405180604001604052805f81526020015f81525090565b815260200161284a60405180604001604052805f81526020015f81525090565b815260200161286a60405180604001604052805f81526020015f81525090565b815260200161288a60405180604001604052805f81526020015f81525090565b81526020016128aa60405180604001604052805f81526020015f81525090565b81526020015f81526020015f81525090565b6040518060a001604052806005906020820280368337509192915050565b80356001600160a01b03811681146109fb575f5ffd5b5f60208284031215612900575f5ffd5b612627826128da565b5f60208284031215612919575f5ffd5b5035919050565b5f6105008201905082518252602083015160208301526040830151612952604084018280518252602090810151910152565b50606083015180516080840152602081015160a0840152506080830151805160c0840152602081015160e08401525060a0830151805161010084015260208101516101208401525060c0830151805161014084015260208101516101608401525060e0830151805161018084015260208101516101a08401525061010083015180516101c084015260208101516101e08401525061012083015180516102008401526020810151610220840152506101408301518051610240840152602081015161026084015250610160830151805161028084015260208101516102a08401525061018083015180516102c084015260208101516102e0840152506101a083015180516103008401526020810151610320840152506101c083015180516103408401526020810151610360840152506101e0830151805161038084015260208101516103a08401525061020083015180516103c084015260208101516103e08401525061022083015180516104008401526020810151610420840152506102408301518051610440840152602081015161046084015250610260830151805161048084015260208101516104a0840152506102808301516104c08301526102a0909201516104e09091015290565b80356001600160401b03811681146109fb575f5ffd5b5f60208284031215612b47575f5ffd5b61262782612b21565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715612b8757612b87612b50565b60405290565b604051601f8201601f191681016001600160401b0381118282101715612bb557612bb5612b50565b604052919050565b5f60608284031215612bcd575f5ffd5b604051606081016001600160401b0381118282101715612bef57612bef612b50565b604052905080612bfe83612b21565b8152612c0c60208401612b21565b6020820152604092830135920191909152919050565b5f60408284031215612c32575f5ffd5b604080519081016001600160401b0381118282101715612c5457612c54612b50565b604052823581526020928301359281019290925250919050565b5f6104808284031215612c7f575f5ffd5b612c87612b64565b9050612c938383612c22565b8152612ca28360408401612c22565b6020820152612cb48360808401612c22565b6040820152612cc68360c08401612c22565b6060820152612cd9836101008401612c22565b6080820152612cec836101408401612c22565b60a0820152612cff836101808401612c22565b60c0820152612d12836101c08401612c22565b60e0820152612d25836102008401612c22565b610100820152612d39836102408401612c22565b610120820152612d4d836102808401612c22565b610140820152612d61836102c08401612c22565b610160820152612d75836103008401612c22565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f6104e08385031215612e0b575f5ffd5b612e158484612bbd565b9150612e248460608501612c6e565b90509250929050565b803563ffffffff811681146109fb575f5ffd5b5f60208284031215612e50575f5ffd5b61262782612e2d565b5f5f60408385031215612e6a575f5ffd5b612e73836128da565b915060208301356001600160401b03811115612e8d575f5ffd5b8301601f81018513612e9d575f5ffd5b80356001600160401b03811115612eb657612eb6612b50565b612ec9601f8201601f1916602001612b8d565b818152866020838501011115612edd575f5ffd5b816020840160208301375f602083830101528093505050509250929050565b5f60808284031215612f0c575f5ffd5b604051608081016001600160401b0381118282101715612f2e57612f2e612b50565b6040908152833582526020808501359083015283810135908201526060928301359281019290925250919050565b5f5f5f6105608486031215612f6f575f5ffd5b612f798585612bbd565b9250612f888560608601612efc565b9150612f978560e08601612c6e565b90509250925092565b5f5f60408385031215612fb1575f5ffd5b612fba83612b21565b9150612e2460208401612b21565b5f5f5f5f6101208587031215612fdc575f5ffd5b612fe68686612bbd565b9350612ff58660608701612efc565b925061300360e08601612e2d565b915061301261010086016128da565b905092959194509250565b5f5f5f5f6105808587031215613031575f5ffd5b61303b8686612bbd565b935061304a8660608701612efc565b925060e08501359150613012866101008701612c6e565b602081525f82518060208401528060208501604085015e5f604082850101526040601f19601f83011684010191505092915050565b5f5f604083850312156130a7575f5ffd5b50508035926020909101359150565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b038181168382160190811115610a8f57610a8f6130b6565b634e487b7160e01b5f52601260045260245ffd5b5f6001600160401b03831680613115576131156130e9565b806001600160401b0384160691505092915050565b6001600160401b038281168282160390811115610a8f57610a8f6130b6565b81810381811115610a8f57610a8f6130b6565b634e487b7160e01b5f52603260045260245ffd5b5f6001600160401b03831680613188576131886130e9565b806001600160401b0384160491505092915050565b5f816131ab576131ab6130b6565b505f190190565b5f602082840312156131c2575f5ffd5b5051919050565b5f81518060208401855e5f93019283525090919050565b5f6131f46131ee83876131c9565b856131c9565b928352505060200192915050565b5f82613210576132106130e9565b500690565b805f5b6005811015613237578151845260209384019390910190600101613218565b50505050565b61325282825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e0840152506080810151805161010084015260208101516101208401525060a0810151805161014084015260208101516101608401525060c0810151805161018084015260208101516101a08401525060e081015180516101c084015260208101516101e08401525061010081015180516102008401526020810151610220840152506101208101518051610240840152602081015161026084015250610140810151805161028084015260208101516102a08401525061016081015180516102c084015260208101516102e08401525061018081015180516103008401526020810151610320840152506101a08101516103408301526101c08101516103608301526101e08101516103808301526102008101516103a08301526102208101516103c08301526102408101516103e08301526102608101516104008301526102808101516104208301526102a08101516104408301526102c0015161046090910152565b5f610a208201905084518252602085015160208301526040850151613429604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a0850151805161010084015260208101516101208401525060c0850151805161014084015260208101516101608401525060e0850151805161018084015260208101516101a08401525061010085015180516101c084015260208101516101e08401525061012085015180516102008401526020810151610220840152506101408501518051610240840152602081015161026084015250610160850151805161028084015260208101516102a08401525061018085015180516102c084015260208101516102e0840152506101a085015180516103008401526020810151610320840152506101c085015180516103408401526020810151610360840152506101e0850151805161038084015260208101516103a08401525061020085015180516103c084015260208101516103e08401525061022085015180516104008401526020810151610420840152506102408501518051610440840152602081015161046084015250610260850151805161048084015260208101516104a0840152506102808501516104c08301526102a08501516104e0830152613601610500830185613215565b61360f6105a083018461323d565b949350505050565b5f60208284031215613627575f5ffd5b81518015158114612627575f5ffd5b5f6001600160401b0382166001600160401b038103613657576136576130b6565b60010192915050565b5f61262782846131c956fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbcf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15a\0\x13W__\xFD[Pa\0\x1Ca\0!V[a\0\xD3V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15a\0qW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14a\0\xD0W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80Qa6\xB8a\0\xF9_9_\x81\x81a\x1C[\x01R\x81\x81a\x1C\x84\x01Ra\x1E\x01\x01Ra6\xB8_\xF3\xFE`\x80`@R`\x046\x10a\x02(W_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x01)W\x80c\x9F\xDBT\xA7\x11a\0\xA8W\x80c\xD2M\x93=\x11a\0mW\x80c\xD2M\x93=\x14a\x07]W\x80c\xE003\x01\x14a\x07\x8CW\x80c\xF0h T\x14a\x07\xABW\x80c\xF2\xFD\xE3\x8B\x14a\x07\xCAW\x80c\xF9\xE5\r\x19\x14a\x07\xE9W__\xFD[\x80c\x9F\xDBT\xA7\x14a\x06UW\x80c\xAA\xBD]\xB3\x14a\x06\xAAW\x80c\xAD<\xB1\xCC\x14a\x06\xC9W\x80c\xB3;\xC4\x91\x14a\x07\x06W\x80c\xC2;\x9E\x9E\x14a\x07%W__\xFD[\x80c\x8D\xA5\xCB[\x11a\0\xEEW\x80c\x8D\xA5\xCB[\x14a\x05\xA7W\x80c\x90\xC1C\x90\x14a\x05\xE3W\x80c\x96\xC1\xCAa\x14a\x06\x02W\x80c\x99\x83(\xE8\x14a\x06!W\x80c\x9B\xAA<\xC9\x14a\x066W__\xFD[\x80cqP\x18\xA6\x14a\x05\nW\x80cu|7\xAD\x14a\x05\x1EW\x80cvg\x18\x08\x14a\x058W\x80c\x82nA\xFC\x14a\x05LW\x80c\x85\x84\xD2?\x14a\x05kW__\xFD[\x80c0\x0C\x89\xDD\x11a\x01\xB5W\x80cBm1\x94\x11a\x01zW\x80cBm1\x94\x14a\x04\x8FW\x80cC=\xBA\x9F\x14a\x04\xB0W\x80cO\x1E\xF2\x86\x14a\x04\xCFW\x80cR\xD1\x90-\x14a\x04\xE2W\x80ci\xCCj\x04\x14a\x04\xF6W__\xFD[\x80c0\x0C\x89\xDD\x14a\x03\xE3W\x80c1=\xF7\xB1\x14a\x04\x02W\x80c7\x8E\xC2;\x14a\x049W\x80c8\xE4T\xB1\x14a\x04UW\x80c>\xD5[{\x14a\x04iW__\xFD[\x80c\x12\x17<,\x11a\x01\xFBW\x80c\x12\x17<,\x14a\x03\x17W\x80c\x16z\xC6\x18\x14a\x038W\x80c c\xD4\xF7\x14a\x03WW\x80c%)t'\x14a\x03vW\x80c/y\x88\x9D\x14a\x03\xA5W__\xFD[\x80c\x01?\xA5\xFC\x14a\x02,W\x80c\x02\xB5\x92\xF3\x14a\x02MW\x80c\x06%\xE1\x9B\x14a\x02\xAAW\x80c\r\x8En,\x14a\x02\xECW[__\xFD[4\x80\x15a\x027W__\xFD[Pa\x02Ka\x02F6`\x04a(\xF0V[a\x07\xFDV[\0[4\x80\x15a\x02XW__\xFD[Pa\x02la\x02g6`\x04a)\tV[a\x08\xB0V[`@Qa\x02\xA1\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\xB5W__\xFD[P`\x0BT`\x0CT`\rT`\x0ETa\x02\xCC\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x02\xA1V[4\x80\x15a\x02\xF7W__\xFD[P`@\x80Q`\x03\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02\xA1V[4\x80\x15a\x03\"W__\xFD[Pa\x03+a\x08\xF9V[`@Qa\x02\xA1\x91\x90a) V[4\x80\x15a\x03CW__\xFD[Pa\x02Ka\x03R6`\x04a+7V[a\t\x0EV[4\x80\x15a\x03bW__\xFD[Pa\x02Ka\x03q6`\x04a-\xF9V[a\t\x85V[4\x80\x15a\x03\x81W__\xFD[Pa\x03\x95a\x03\x906`\x04a+7V[a\t\x9EV[`@Q\x90\x15\x15\x81R` \x01a\x02\xA1V[4\x80\x15a\x03\xB0W__\xFD[P`\x08Ta\x03\xCB\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x03\xEEW__\xFD[Pa\x03\x95a\x03\xFD6`\x04a+7V[a\n\0V[4\x80\x15a\x04\rW__\xFD[P`\x08Ta\x04!\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x04DW__\xFD[PC[`@Q\x90\x81R` \x01a\x02\xA1V[4\x80\x15a\x04`W__\xFD[Pa\x02Ka\n\x95V[4\x80\x15a\x04tW__\xFD[P`\nTa\x03\xCB\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x04\x9AW__\xFD[P_T`\x01T`\x02T`\x03Ta\x02\xCC\x93\x92\x91\x90\x84V[4\x80\x15a\x04\xBBW__\xFD[Pa\x02Ka\x04\xCA6`\x04a.@V[a\x0B\x8CV[a\x02Ka\x04\xDD6`\x04a.YV[a\x0B\xA0V[4\x80\x15a\x04\xEDW__\xFD[Pa\x04Ga\x0B\xBFV[4\x80\x15a\x05\x01W__\xFD[Pa\x02Ka\x0B\xDAV[4\x80\x15a\x05\x15W__\xFD[Pa\x02Ka\x0CHV[4\x80\x15a\x05)W__\xFD[Pa\x02Ka\x03q6`\x04a/\\V[4\x80\x15a\x05CW__\xFD[Pa\x03\xCBa\x0CYV[4\x80\x15a\x05WW__\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15a\x03\x95V[4\x80\x15a\x05vW__\xFD[Pa\x05\x8Aa\x05\x856`\x04a)\tV[a\x0C~V[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x02\xA1V[4\x80\x15a\x05\xB2W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x04!V[4\x80\x15a\x05\xEEW__\xFD[Pa\x03\xCBa\x05\xFD6`\x04a/\xA0V[a\r\xA9V[4\x80\x15a\x06\rW__\xFD[Pa\x02Ka\x06\x1C6`\x04a.@V[a\x0E\x18V[4\x80\x15a\x06,W__\xFD[Pa\x04G`\x0FT\x81V[4\x80\x15a\x06AW__\xFD[Pa\x02Ka\x06P6`\x04a/\xC8V[a\x0E\xA1V[4\x80\x15a\x06`W__\xFD[P`\x06T`\x07Ta\x06\x84\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x02\xA1V[4\x80\x15a\x06\xB5W__\xFD[Pa\x02Ka\x06\xC46`\x04a0\x1DV[a\x0F\xB0V[4\x80\x15a\x06\xD4W__\xFD[Pa\x06\xF9`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02\xA1\x91\x90a0aV[4\x80\x15a\x07\x11W__\xFD[Pa\x02Ka\x07 6`\x04a/\xA0V[a\x12\xD5V[4\x80\x15a\x070W__\xFD[P`\x08Ta\x07H\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x07hW__\xFD[P`\x04T`\x05Ta\x06\x84\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x07\x97W__\xFD[Pa\x03\x95a\x07\xA66`\x04a0\x96V[a\x14.V[4\x80\x15a\x07\xB6W__\xFD[P`\nTa\x03\xCB\x90`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x07\xD5W__\xFD[Pa\x02Ka\x07\xE46`\x04a(\xF0V[a\x15\x86V[4\x80\x15a\x07\xF4W__\xFD[P`\tTa\x04GV[a\x08\x05a\x15\xC5V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x08,W`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\x08[W`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\x08\xBFW_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[a\t\x01a&WV[a\t\ta\x16 V[\x90P\x90V[a\t\x16a\x15\xC5V[`\n\x80To\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\x19\x81\x16`\x01`@\x1B`\x01`\x01`@\x1B\x03\x85\x81\x16\x82\x02\x92\x83\x17\x94\x85\x90Ua\t\\\x94\x91\x90\x91\x04\x81\x16\x92\x81\x16\x91\x16\x17a\r\xA9V[`\n`\x10a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPPV[`@QcN@\\\x8D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\t\xBEWP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\t\xCAWP_\x91\x90PV[`\nT`\x01`\x01`@\x1B\x03\x16a\t\xE1\x83`\x05a0\xCAV[a\t\xEB\x91\x90a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x15\x92\x91PPV[\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\n WP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\n,WP_\x91\x90PV[`\nTa\nB\x90`\x01`\x01`@\x1B\x03\x16\x83a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x15\x80a\n\x8FWP`\nTa\nj\x90`\x05\x90`\x01`\x01`@\x1B\x03\x16a1*V[`\nT`\x01`\x01`@\x1B\x03\x91\x82\x16\x91a\n\x84\x91\x16\x84a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x11[\x92\x91PPV[a\n\x9Da\x15\xC5V[_Q` a6\x8C_9_Q\x90_R\x80T`\x03\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\n\xD3WP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\n\xF1W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x90\x81\x17\x83U`\nTa\x0B+\x92\x91\x81\x04\x82\x16\x91\x16a\r\xA9V[`\x10\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x83\x16\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPV[a\x0B\x94a\x15\xC5V[a\x0B\x9D\x81a\x0E\x18V[PV[a\x0B\xA8a\x1CPV[a\x0B\xB1\x82a\x1C\xF4V[a\x0B\xBB\x82\x82a\x1D5V[PPV[_a\x0B\xC8a\x1D\xF6V[P_Q` a6l_9_Q\x90_R\x90V[a\x0B\xE2a\x15\xC5V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x0C-W`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x0CPa\x15\xC5V[a\x0CF_a\x1E?V[`\x06T`\nT_\x91a\t\t\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16a\r\xA9V[`\t\x80T_\x91\x82\x91\x90a\x0C\x92`\x01\x83a1IV[\x81T\x81\x10a\x0C\xA2Wa\x0C\xA2a1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x0C\xE1W`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\r\xA2W\x84`\t\x82\x81T\x81\x10a\r\x11Wa\r\x11a1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\r\x9AW`\t\x81\x81T\x81\x10a\rJWa\rJa1\\V[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\rmWa\rma1\\V[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x0C\xF5V[PP\x91P\x91V[_\x81`\x01`\x01`@\x1B\x03\x16_\x03a\r\xC1WP_a\n\x8FV[\x82`\x01`\x01`@\x1B\x03\x16_\x03a\r\xD9WP`\x01a\n\x8FV[a\r\xE3\x82\x84a0\xFDV[`\x01`\x01`@\x1B\x03\x16_\x03a\x0E\x03Wa\r\xFC\x82\x84a1pV[\x90Pa\n\x8FV[a\x0E\r\x82\x84a1pV[a\r\xFC\x90`\x01a0\xCAV[a\x0E a\x15\xC5V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\x0E?WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\x0E]WP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\x0E{W`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[_Q` a6\x8C_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x0E\xD2WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x0E\xEDWP0;\x15[\x90P\x81\x15\x80\x15a\x0E\xFBWP\x80\x15[\x15a\x0F\x19W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x0FCW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x0FL\x86a\x1E\xAFV[a\x0FTa\x1E\xC0V[a\x0F_\x89\x89\x89a\x1E\xC8V[\x83\x15a\x0F\xA5W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x0F\xD5WP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x0F\xF3W`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x84Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x10,WP`\x06T` \x85\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x10JW`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x10W\x84`@\x01Qa\x1F\xF4V[a\x10d\x83` \x01Qa\x1F\xF4V[a\x10q\x83`@\x01Qa\x1F\xF4V[a\x10~\x83``\x01Qa\x1F\xF4V[_a\x10\x87a\x0CYV[` \x86\x01Q`\nT\x91\x92P_\x91a\x10\xA7\x91\x90`\x01`\x01`@\x1B\x03\x16a\r\xA9V[`\x10T\x90\x91P`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x10a\x10\xEBWa\x10\xCD\x86` \x01Qa\n\0V[\x15a\x10\xEBW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x10T`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x11\x15a\x11\x97W`\x02a\x11\x0E\x83\x83a1*V[`\x01`\x01`@\x1B\x03\x16\x10a\x115W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11@\x82`\x01a0\xCAV[`\x01`\x01`@\x1B\x03\x16\x81`\x01`\x01`@\x1B\x03\x16\x14\x80\x15a\x11yWP`\x06Ta\x11w\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16a\t\x9EV[\x15[\x15a\x11\x97W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11\xA3\x86\x86\x86\x86a 5V[\x85Q`\x06\x80T` \x89\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x81\x16\x93\x90\x93\x17\x17\x90U`@\x87\x01Q`\x07U`\x0F\x85\x90U`\x10T\x81\x16\x90\x82\x16\x10\x80\x15\x90a\x12\0WPa\x12\0\x86` \x01Qa\t\x9EV[\x15a\x12jW\x84Q`\x0BU` \x85\x01Q`\x0CU`@\x85\x01Q`\rU``\x85\x01Q`\x0EU\x7F1\xEA\xBD\x90\x99\xFD\xB2]\xAC\xDD\xD2\x06\xAB\xFF\x871\x1EU4A\xFC\x9D\x0F\xCD\xEF \x10b\xD7\xE7\x07\x1Ba\x12N\x82`\x01a0\xCAV[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xA1[a\x12uCB\x88a\"_V[\x85` \x01Q`\x01`\x01`@\x1B\x03\x16\x86_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x88`@\x01Q`@Qa\x12\xC5\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[a\x12\xDDa\x15\xC5V[_Q` a6\x8C_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x13\x13WP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x131W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x17\x82U`\x05\x90\x85\x16\x11a\x13yW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x0BU`\x01T`\x0CU`\x02T`\rU`\x03T`\x0EU`\n\x80T`\x01`\x01`@\x1B\x03\x85\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x90\x87\x16\x17\x17\x90Ua\x13\xC2\x83\x85a\r\xA9V[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x16`\x01`\x80\x1B`\x01`\x01`@\x1B\x03\x93\x84\x16\x02\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[`\tT_\x90C\x84\x11\x80a\x14?WP\x80\x15[\x80a\x14\x89WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x14mWa\x14ma1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a\x14\xA7W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a\x14\xB5`\x01\x85a1IV[\x90P[\x81a\x15QW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a\x15QW\x86`\t\x82\x81T\x81\x10a\x14\xEAWa\x14\xEAa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a\x15?W`\x01\x91P`\t\x81\x81T\x81\x10a\x15\x1FWa\x15\x1Fa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa\x15QV[\x80a\x15I\x81a1\x9DV[\x91PPa\x14\xB8V[\x81a\x15oW`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a\x15z\x84\x89a1IV[\x11\x97\x96PPPPPPPV[a\x15\x8Ea\x15\xC5V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x15\xBCW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x0B\x9D\x81a\x1E?V[3a\x15\xF7\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0CFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x15\xB3V[a\x16(a&WV[b\x10\0\0\x81R`\x05` \x82\x01R\x7F)I&\r\xC9\xE9b\x1B\xB4\x1D\xCB\x96\xBApT\xB4\xBD^~#\x0F\xDB\xA5\xF3A\x12`@\x1CU\xF5\x9D`@\x82\x01QR\x7F\x05\xD06\x978E\xE2\xE9\xD2\xAD\x9Ay[5\x155\xA2WmQ\xD2\x7F!\xFF\x83r\xBE\x92\xBDo9F` `@\x83\x01Q\x01R\x7F\x0B\xA2\xC5\xAE\x93`\xEF\xEC\x9E9h\xE3?W\xFD3\x05\x9EW8\\\x1E\xA7\xDBd0Bk\x82\xE0\x87\x1A``\x82\x01QR\x7F\x1E3;S\x98\xC9S\x19@vw*\x86\x1B{\xF6\xA4\xC8\x0CJL.T\xEB\x9C\xA6z\xEC_\xF1\x9F\xC9` ``\x83\x01Q\x01R\x7F\r\x9E\x9B\x9F8\xDD\x9F\xBB\xD5\xCD\x8BZ\x1D\x1C\x8A\xA4\xE7w\xE5&\xE0n\xFE94[\xF3\xCEL[\xB4\xAA`\x80\x82\x01QR\x7F\x10A~\xAF\x9B\xA30\xBB\xF5l\xAF3\x1A6!\x14\x15:\x9C\x95\xAE\x91O\xBD\x1F\x99\xCB\x84\xD5\x9F\xBFV` `\x80\x83\x01Q\x01R\x7F\x15]\xFC:\x03\x9F\x16\xAB\x99\xFA\x96cV\x9F\xF0n[\xFD\xA9\x17H\xA7\x98!\xD8\r\xAF\xC7\xF1\xD9.^`\xA0\x82\x01QR\x7F\x15\xDA\xEE\x81\xE8\xFF\xCA\xC8\x86\xBF\x9C\xC7E=e\x9A\x98}\xA1\xFE\xB8\x93\xC1\xFE\x9A\x94X?3\x7Fm\xFA` `\xA0\x83\x01Q\x01R\x7F\x1Co\x99W'\x08?VsJHc\xC3\xBFD3\xB55:\xD8\xD2\x0F\x15\xD5T\xA8\xCD+\xE2\x8E\xF9-`\xC0\x82\x01QR\x7F\x076\xEB\xBF\rs\xD4,B\x8D]\xD6k\xA4\xD9\xD9Q:d-\x94\x14}\xB6)\x96Mm\x03'v\xA8` `\xC0\x83\x01Q\x01R\x7F,J\xA1\xA4-\x17\xF2&S'B\xB7\xDA!\xED\x90\x8E\xE6\xA1\xC1=\x82K&\x9D!\xAB\xCDY\xC8g#`\xE0\x82\x01QR\x7F\x05\xC4\x16<\xA9\xCA\xB2\xE6Z\xBB\xB4\x1BauY\x1C\xF9$`\0\x0C\x96\xFB\x9D\xAA\x1F\x01\xD5\n\xF4\x93l` `\xE0\x83\x01Q\x01R\x7F!^\xCFh<e\xEE=\xCA</\xC0KHd\xB1\xF2\xA58\xEF\x92:\xF68\rB\x0F\xA6\xB5\xA9\xF4\x96a\x01\0\x82\x01QR\x7F\x1D\x03\xC3x\xF3\xD7\x06=\x12\xC4Y\xACe\x9C\xE7\xA2|C\x9C\xD6\xAD\x18L\x17#R\x81_:8\r7` a\x01\0\x83\x01Q\x01R\x7F \xBC)T\x8F\x10\xBD\x07\xFD\xE4\x18\xD4\x9AV\x92\xF8\x91\x96\x94W\x1A\xB6L\x90\xF5\x83\xDCCJ_\xEC\x0Ca\x01 \x82\x01QR\x7F$N_\xCBQ\xC7G\xA5o\xE6\xFD\xB3/\x0B\x01\xEF;\xC5V'\xF6\xF9\xAF\xCD\x98\xDD\xDB\xED\xE5\x03\x08\xA3` a\x01 \x83\x01Q\x01R\x7F\x0E6F\xB3R\xD0\n4\x82\xE8\x98\x11\xF4\x96o\xB6F\x88\x9D\xAD\xB5a\xEB\xB7\xBB|\">\x81\x96\xD5\xB3a\x01@\x82\x01QR\x7F\x1B\x10!\x9Ab\x93\xAB\xAF08\x8F9\xE4\xC7\xB9%\xF8\x9BoW\xCB\x81eN\x1A\xD7U)Ny\x0F\t` a\x01@\x83\x01Q\x01R\x7F+)\xB3l\xD6\xD30b\xA9\xA8n$\xBD\x17\x8Di\xB1\xCE\xBD\xC1\xA3\x9Cyw\xD5G\xE7a{WG\xF9a\x01`\x82\x01QR\x7F\x17\x06!a\xC0\xA6<\xD1|\xEE[\x14\x82\x1Dx \xE7\xFAC##\xB1\"\xBAY\xC4M\xD0\x1Fj\x928` a\x01`\x83\x01Q\x01R\x7F\x11\x98\xDB<\xEC\x1Af\xCC\xDB\x90\x88k\xB9o\xCF\x17S\x16\xC6\xEAx\xF7?#\xF4\xA1\x1B\xCFC \xE1\x1Aa\x01\x80\x82\x01QR\x7F\x06;\x1F\x96>s+\xD2\r\x86\xE1\xFE\xF8Ux\x8C\x1A\xAC\xF2k\xAB\xB5&\xD8N0c:+Z\x94i` a\x01\x80\x83\x01Q\x01R\x7F#\x80\x9Aj[\xB0\xBF\x08\x8F\x97\xEF\xE1Qh\xA3\x94q\xA3\xA4\xE4\x1B\x8Dm\xB0\x10\x0E\x15\xFAh\xB0\x9Fca\x01\xA0\x82\x01QR\x7F\n\xBA{i\xAB\x7F\xDD\xA6\x8D\xAC\x90e\xA5\xEE\x9F\xB5\n\xBF\xE5{\xDBZ\xB3Y\xCC[V\xDF\xF6\\\xBE\xA1` a\x01\xA0\x83\x01Q\x01R\x7F\x1F\x03\x80d\xD3\xCA\x1F7\xC5n\xCF\xE4\x17\x01\xF1ZA,c\xD3\xC9\xADR\xFC\xFD?\xD4\xC6M\xA8\xB5\xF2a\x01\xC0\x82\x01QR\x7F&\x89\xFE\\\xC5\x9EK\xE1\x12\xC2G\x99i\xC2Z\x7F`:]q\xA2\xE7\x92D\x80\xC9\xF4\xEA\xFC,)\x8F` a\x01\xC0\x83\x01Q\x01R\x7F\x110!\xE93(\xA9\x151\xE4\x08qH\x1CG\x14\xE0\xB9\x9Aj\xFB\x10\xC7y\xEE\xB2\xB0zz\xE6\xF4\xE7a\x01\xE0\x82\x01QR\x7F\x1A6\xBB& \xCD\xB4\x0CM\xAD%%w\x16\xA9\xD8\xEB\x1EE\xF7\x15\xAD\xA9\x8EBF\x97\xAA\xF4\xD9\\\x86` a\x01\xE0\x83\x01Q\x01R\x7F\x08\xF3\xF8\x8F\xFB\x9EC&\x12\x94\xB7\xFA\xF5\x82\xC5\x13\xF9\xC7\xD0t\x9D\xB6\xDC\xC44\xD7I;\x8C\x97[/a\x02\0\x82\x01QR\x7F.>\x04Xt\x11\x19\xAD\x14\"\x07+h\x15\xFD\xA8\n8\x96d\x0F\x01\x8D(,\x88\xF1PkT\xE0\xE6` a\x02\0\x83\x01Q\x01R\x7F\x10\n\\\nN\x1A\xC2y\x1D\x1Fh\xBC\x9C%\xB3\x9C\xCF\xBB]b\x8CS\xD5T\x7F\x89\xAA\x0C\xAB\x83$\xD2a\x02 \x82\x01QR\x7F\x05\xBF\x9E\x97B\x8C8\x7F\xBB\xC5\xF9\xCB\xF6\xEF\xFB3\xB5vUIL*\xB9\xF7\xCC]DZ\x0E\xA5k\xEA` a\x02 \x83\x01Q\x01R\x7F\x06\x7F>\x0C\xE6\x9C\xBB\xE3#7\xF0S\x8B\xF6\x11\x9Cr\xF7\xFDM\x92\x85{x\\\xAF\x04\xA2%\xB9MFa\x02@\x82\x01QR\x7F!\x1A\x07bq\x06\x9F\xB1\xFA\xE1R*\xB8\xA4w\x94\x80\xB5\x0E\xD8\xC4d\x8D \x13A\xE4D\xE8\xEE-\x15` a\x02@\x83\x01Q\x01R\x7F\x0B\x93\x1B\x96\x99}\x9D\xB8\xBC\x19\x8Cu\0\x98\xCA\xD2\x96\r\xF4\x07\x88\x0F{,\xB5\x1C\x857m_\xC8Ia\x02`\x82\x01QR\x7F\x0E\x91!\xAFv\xD7\xD9ad2\xDE\xD6\xA4\xDE\x93\xCF\x14o[sS\xA7O\x8Are\xD67\x7F\xD4\xED\xC7` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1C\xD6WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1C\xCA_Q` a6l_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0CFW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1C\xFCa\x15\xC5V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x08\xA5V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1D\x8FWP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1D\x8C\x91\x81\x01\x90a1\xB2V[`\x01[a\x1D\xB7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x15\xB3V[_Q` a6l_9_Q\x90_R\x81\x14a\x1D\xE7W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x15\xB3V[a\x1D\xF1\x83\x83a$HV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0CFW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1E\xB7a$\x9DV[a\x0B\x9D\x81a$\xD3V[a\x0CFa$\x9DV[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a\x1E\xECWP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a\x1E\xF9WP` \x82\x01Q\x15[\x80a\x1F\x06WP`@\x82\x01Q\x15[\x80a\x1F\x13WP``\x82\x01Q\x15[\x80a\x1F\x1DWP\x81Q\x15[\x80a\x1F/WPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a\x1FCWPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a\x1FaW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x0B\xBBW`@Qc\x01l\x173`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a >a\x08\xF9V[\x90Pa Ha(\xBCV[`\x0CT\x81R`\rT` \x80\x83\x01\x91\x90\x91R`\x0ET`@\x83\x01R`\x0BT``\x80\x84\x01\x91\x90\x91R`\nT\x91\x88\x01Q\x90\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x91\x04\x81\x16\x91\x16\x10\x80\x15\x90a \xA0WPa \xA0\x87` \x01Qa\t\x9EV[\x15a \xE9W`@\x80Q\x87Q` \x80\x83\x01\x91\x90\x91R\x88\x01Q\x81\x83\x01R\x90\x87\x01Q``\x80\x83\x01\x91\x90\x91R\x87\x01Q`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa!$V[`@\x80Q`\x0BT` \x82\x01R`\x0CT\x91\x81\x01\x91\x90\x91R`\rT``\x82\x01R`\x0ET`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P[`@\x80Q\x88Q`\x01`\x01`@\x1B\x03\x90\x81\x16` \x80\x84\x01\x91\x90\x91R\x8A\x01Q\x16\x81\x83\x01R\x90\x88\x01Q``\x82\x01R_\x90`\x80\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra!u\x91\x84\x90\x89\x90` \x01a1\xE0V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x80Q` \x90\x91\x01 \x90Pa!\xB9\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x82a2\x02V[`\x80\x84\x01R`@QcT\xE8\xBDg`\xE0\x1B\x81Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90cT\xE8\xBDg\x90a!\xF9\x90\x87\x90\x87\x90\x8A\x90`\x04\x01a3\xF7V[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a\"\x14W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\"8\x91\x90a6\x17V[a\"UW`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPPPV[`\tT\x15\x80\x15\x90a\"\xD4WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\"\x9FWa\"\x9Fa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta\"\xC9\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a1*V[`\x01`\x01`@\x1B\x03\x16\x11[\x15a#gW`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a#\x01Wa#\x01a1\\V[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a#A\x83a66V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[a$Q\x82a$\xDBV[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a$\x95Wa\x1D\xF1\x82\x82a%>V[a\x0B\xBBa%\xB0V[_Q` a6\x8C_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\x0CFW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x15\x8Ea$\x9DV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a%\x10W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x15\xB3V[_Q` a6l_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa%Z\x91\x90a6`V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a%\x92W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a%\x97V[``\x91P[P\x91P\x91Pa%\xA7\x85\x83\x83a%\xCFV[\x95\x94PPPPPV[4\x15a\x0CFW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a%\xE4Wa%\xDF\x82a&.V[a&'V[\x81Q\x15\x80\x15a%\xFBWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a&$W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x15\xB3V[P\x80[\x93\x92PPPV[\x80Q\x15a&>W\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a&\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xCA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xEA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\n`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'*`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'J`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'j`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xCA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xEA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\n`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(*`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(J`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(j`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\xA0\x01`@R\x80`\x05\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a)\0W__\xFD[a&'\x82a(\xDAV[_` \x82\x84\x03\x12\x15a)\x19W__\xFD[P5\x91\x90PV[_a\x05\0\x82\x01\x90P\x82Q\x82R` \x83\x01Q` \x83\x01R`@\x83\x01Qa)R`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x83\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x83\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x83\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x83\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x83\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x83\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x83\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x83\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x83\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x83\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x83\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x83\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x83\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x83\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x83\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x83\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x83\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x83\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x90\x92\x01Qa\x04\xE0\x90\x91\x01R\x90V[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a+GW__\xFD[a&'\x82a+!V[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\x87Wa+\x87a+PV[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\xB5Wa+\xB5a+PV[`@R\x91\x90PV[_``\x82\x84\x03\x12\x15a+\xCDW__\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\xEFWa+\xEFa+PV[`@R\x90P\x80a+\xFE\x83a+!V[\x81Ra,\x0C` \x84\x01a+!V[` \x82\x01R`@\x92\x83\x015\x92\x01\x91\x90\x91R\x91\x90PV[_`@\x82\x84\x03\x12\x15a,2W__\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a,TWa,Ta+PV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a,\x7FW__\xFD[a,\x87a+dV[\x90Pa,\x93\x83\x83a,\"V[\x81Ra,\xA2\x83`@\x84\x01a,\"V[` \x82\x01Ra,\xB4\x83`\x80\x84\x01a,\"V[`@\x82\x01Ra,\xC6\x83`\xC0\x84\x01a,\"V[``\x82\x01Ra,\xD9\x83a\x01\0\x84\x01a,\"V[`\x80\x82\x01Ra,\xEC\x83a\x01@\x84\x01a,\"V[`\xA0\x82\x01Ra,\xFF\x83a\x01\x80\x84\x01a,\"V[`\xC0\x82\x01Ra-\x12\x83a\x01\xC0\x84\x01a,\"V[`\xE0\x82\x01Ra-%\x83a\x02\0\x84\x01a,\"V[a\x01\0\x82\x01Ra-9\x83a\x02@\x84\x01a,\"V[a\x01 \x82\x01Ra-M\x83a\x02\x80\x84\x01a,\"V[a\x01@\x82\x01Ra-a\x83a\x02\xC0\x84\x01a,\"V[a\x01`\x82\x01Ra-u\x83a\x03\0\x84\x01a,\"V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[__a\x04\xE0\x83\x85\x03\x12\x15a.\x0BW__\xFD[a.\x15\x84\x84a+\xBDV[\x91Pa.$\x84``\x85\x01a,nV[\x90P\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a.PW__\xFD[a&'\x82a.-V[__`@\x83\x85\x03\x12\x15a.jW__\xFD[a.s\x83a(\xDAV[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a.\x8DW__\xFD[\x83\x01`\x1F\x81\x01\x85\x13a.\x9DW__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a.\xB6Wa.\xB6a+PV[a.\xC9`\x1F\x82\x01`\x1F\x19\x16` \x01a+\x8DV[\x81\x81R\x86` \x83\x85\x01\x01\x11\x15a.\xDDW__\xFD[\x81` \x84\x01` \x83\x017_` \x83\x83\x01\x01R\x80\x93PPPP\x92P\x92\x90PV[_`\x80\x82\x84\x03\x12\x15a/\x0CW__\xFD[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/.Wa/.a+PV[`@\x90\x81R\x835\x82R` \x80\x85\x015\x90\x83\x01R\x83\x81\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[___a\x05`\x84\x86\x03\x12\x15a/oW__\xFD[a/y\x85\x85a+\xBDV[\x92Pa/\x88\x85``\x86\x01a.\xFCV[\x91Pa/\x97\x85`\xE0\x86\x01a,nV[\x90P\x92P\x92P\x92V[__`@\x83\x85\x03\x12\x15a/\xB1W__\xFD[a/\xBA\x83a+!V[\x91Pa.$` \x84\x01a+!V[____a\x01 \x85\x87\x03\x12\x15a/\xDCW__\xFD[a/\xE6\x86\x86a+\xBDV[\x93Pa/\xF5\x86``\x87\x01a.\xFCV[\x92Pa0\x03`\xE0\x86\x01a.-V[\x91Pa0\x12a\x01\0\x86\x01a(\xDAV[\x90P\x92\x95\x91\x94P\x92PV[____a\x05\x80\x85\x87\x03\x12\x15a01W__\xFD[a0;\x86\x86a+\xBDV[\x93Pa0J\x86``\x87\x01a.\xFCV[\x92P`\xE0\x85\x015\x91Pa0\x12\x86a\x01\0\x87\x01a,nV[` \x81R_\x82Q\x80` \x84\x01R\x80` \x85\x01`@\x85\x01^_`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[__`@\x83\x85\x03\x12\x15a0\xA7W__\xFD[PP\x805\x92` \x90\x91\x015\x91PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a1\x15Wa1\x15a0\xE9V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x06\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[\x81\x81\x03\x81\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a1\x88Wa1\x88a0\xE9V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x04\x91PP\x92\x91PPV[_\x81a1\xABWa1\xABa0\xB6V[P_\x19\x01\x90V[_` \x82\x84\x03\x12\x15a1\xC2W__\xFD[PQ\x91\x90PV[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a1\xF4a1\xEE\x83\x87a1\xC9V[\x85a1\xC9V[\x92\x83RPP` \x01\x92\x91PPV[_\x82a2\x10Wa2\x10a0\xE9V[P\x06\x90V[\x80_[`\x05\x81\x10\x15a27W\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a2\x18V[PPPPV[a2R\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xA0\x81\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xC0\x81\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RP`\xE0\x81\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01\0\x81\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01 \x81\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01@\x81\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01`\x81\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\x80\x81\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xA0\x81\x01Qa\x03@\x83\x01Ra\x01\xC0\x81\x01Qa\x03`\x83\x01Ra\x01\xE0\x81\x01Qa\x03\x80\x83\x01Ra\x02\0\x81\x01Qa\x03\xA0\x83\x01Ra\x02 \x81\x01Qa\x03\xC0\x83\x01Ra\x02@\x81\x01Qa\x03\xE0\x83\x01Ra\x02`\x81\x01Qa\x04\0\x83\x01Ra\x02\x80\x81\x01Qa\x04 \x83\x01Ra\x02\xA0\x81\x01Qa\x04@\x83\x01Ra\x02\xC0\x01Qa\x04`\x90\x91\x01RV[_a\n \x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa4)`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x85\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x85\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x85\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x85\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x85\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x85\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x85\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x85\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x85\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x85\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x85\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x85\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x85\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x85\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x85\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra6\x01a\x05\0\x83\x01\x85a2\x15V[a6\x0Fa\x05\xA0\x83\x01\x84a2=V[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a6'W__\xFD[\x81Q\x80\x15\x15\x81\x14a&'W__\xFD[_`\x01`\x01`@\x1B\x03\x82\x16`\x01`\x01`@\x1B\x03\x81\x03a6WWa6Wa0\xB6V[`\x01\x01\x92\x91PPV[_a&'\x82\x84a1\xC9V\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405260043610610228575f3560e01c8063715018a6116101295780639fdb54a7116100a8578063d24d933d1161006d578063d24d933d1461075d578063e03033011461078c578063f0682054146107ab578063f2fde38b146107ca578063f9e50d19146107e9575f5ffd5b80639fdb54a714610655578063aabd5db3146106aa578063ad3cb1cc146106c9578063b33bc49114610706578063c23b9e9e14610725575f5ffd5b80638da5cb5b116100ee5780638da5cb5b146105a757806390c14390146105e357806396c1ca6114610602578063998328e8146106215780639baa3cc914610636575f5ffd5b8063715018a61461050a578063757c37ad1461051e5780637667180814610538578063826e41fc1461054c5780638584d23f1461056b575f5ffd5b8063300c89dd116101b5578063426d31941161017a578063426d31941461048f578063433dba9f146104b05780634f1ef286146104cf57806352d1902d146104e257806369cc6a04146104f6575f5ffd5b8063300c89dd146103e3578063313df7b114610402578063378ec23b1461043957806338e454b1146104555780633ed55b7b14610469575f5ffd5b806312173c2c116101fb57806312173c2c14610317578063167ac618146103385780632063d4f71461035757806325297427146103765780632f79889d146103a5575f5ffd5b8063013fa5fc1461022c57806302b592f31461024d5780630625e19b146102aa5780630d8e6e2c146102ec575b5f5ffd5b348015610237575f5ffd5b5061024b6102463660046128f0565b6107fd565b005b348015610258575f5ffd5b5061026c610267366004612909565b6108b0565b6040516102a194939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b3480156102b5575f5ffd5b50600b54600c54600d54600e546102cc9392919084565b6040805194855260208501939093529183015260608201526080016102a1565b3480156102f7575f5ffd5b5060408051600381525f60208201819052918101919091526060016102a1565b348015610322575f5ffd5b5061032b6108f9565b6040516102a19190612920565b348015610343575f5ffd5b5061024b610352366004612b37565b61090e565b348015610362575f5ffd5b5061024b610371366004612df9565b610985565b348015610381575f5ffd5b50610395610390366004612b37565b61099e565b60405190151581526020016102a1565b3480156103b0575f5ffd5b506008546103cb90600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016102a1565b3480156103ee575f5ffd5b506103956103fd366004612b37565b610a00565b34801561040d575f5ffd5b50600854610421906001600160a01b031681565b6040516001600160a01b0390911681526020016102a1565b348015610444575f5ffd5b50435b6040519081526020016102a1565b348015610460575f5ffd5b5061024b610a95565b348015610474575f5ffd5b50600a546103cb90600160401b90046001600160401b031681565b34801561049a575f5ffd5b505f546001546002546003546102cc9392919084565b3480156104bb575f5ffd5b5061024b6104ca366004612e40565b610b8c565b61024b6104dd366004612e59565b610ba0565b3480156104ed575f5ffd5b50610447610bbf565b348015610501575f5ffd5b5061024b610bda565b348015610515575f5ffd5b5061024b610c48565b348015610529575f5ffd5b5061024b610371366004612f5c565b348015610543575f5ffd5b506103cb610c59565b348015610557575f5ffd5b506008546001600160a01b03161515610395565b348015610576575f5ffd5b5061058a610585366004612909565b610c7e565b604080519283526001600160401b039091166020830152016102a1565b3480156105b2575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b0316610421565b3480156105ee575f5ffd5b506103cb6105fd366004612fa0565b610da9565b34801561060d575f5ffd5b5061024b61061c366004612e40565b610e18565b34801561062c575f5ffd5b50610447600f5481565b348015610641575f5ffd5b5061024b610650366004612fc8565b610ea1565b348015610660575f5ffd5b50600654600754610684916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016102a1565b3480156106b5575f5ffd5b5061024b6106c436600461301d565b610fb0565b3480156106d4575f5ffd5b506106f9604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516102a19190613061565b348015610711575f5ffd5b5061024b610720366004612fa0565b6112d5565b348015610730575f5ffd5b5060085461074890600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016102a1565b348015610768575f5ffd5b50600454600554610684916001600160401b0380821692600160401b909204169083565b348015610797575f5ffd5b506103956107a6366004613096565b61142e565b3480156107b6575f5ffd5b50600a546103cb906001600160401b031681565b3480156107d5575f5ffd5b5061024b6107e43660046128f0565b611586565b3480156107f4575f5ffd5b50600954610447565b6108056115c5565b6001600160a01b03811661082c5760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b039081169082160361085b5760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b600981815481106108bf575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b610901612657565b610909611620565b905090565b6109166115c5565b600a80546fffffffffffffffff0000000000000000198116600160401b6001600160401b0385811682029283179485905561095c94919091048116928116911617610da9565b600a60106101000a8154816001600160401b0302191690836001600160401b0316021790555050565b604051634e405c8d60e01b815260040160405180910390fd5b5f6001600160401b03821615806109be5750600a546001600160401b0316155b156109ca57505f919050565b600a546001600160401b03166109e18360056130ca565b6109eb91906130fd565b6001600160401b03161592915050565b919050565b5f6001600160401b0382161580610a205750600a546001600160401b0316155b15610a2c57505f919050565b600a54610a42906001600160401b0316836130fd565b6001600160401b03161580610a8f5750600a54610a6a906005906001600160401b031661312a565b600a546001600160401b0391821691610a849116846130fd565b6001600160401b0316115b92915050565b610a9d6115c5565b5f51602061368c5f395f51905f52805460039190600160401b900460ff1680610ad3575080546001600160401b03808416911610155b15610af15760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b9081178355600a54610b2b9291810482169116610da9565b6010805467ffffffffffffffff19166001600160401b03928316179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050565b610b946115c5565b610b9d81610e18565b50565b610ba8611c50565b610bb182611cf4565b610bbb8282611d35565b5050565b5f610bc8611df6565b505f51602061366c5f395f51905f5290565b610be26115c5565b6008546001600160a01b031615610c2d57600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b610c506115c5565b610c465f611e3f565b600654600a545f91610909916001600160401b03600160401b90920482169116610da9565b600980545f91829190610c92600183613149565b81548110610ca257610ca261315c565b5f918252602090912060029091020154600160801b90046001600160401b03168410610ce157604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b81811015610da2578460098281548110610d1157610d1161315c565b5f918252602090912060029091020154600160801b90046001600160401b03161115610d9a5760098181548110610d4a57610d4a61315c565b905f5260205f2090600202016001015460098281548110610d6d57610d6d61315c565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b600101610cf5565b5050915091565b5f816001600160401b03165f03610dc157505f610a8f565b826001600160401b03165f03610dd957506001610a8f565b610de382846130fd565b6001600160401b03165f03610e0357610dfc8284613170565b9050610a8f565b610e0d8284613170565b610dfc9060016130ca565b610e206115c5565b610e108163ffffffff161080610e3f57506301e133808163ffffffff16115b80610e5d575060085463ffffffff600160a01b909104811690821611155b15610e7b576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b5f51602061368c5f395f51905f528054600160401b810460ff1615906001600160401b03165f81158015610ed25750825b90505f826001600160401b03166001148015610eed5750303b155b905081158015610efb575080155b15610f195760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff191660011785558315610f4357845460ff60401b1916600160401b1785555b610f4c86611eaf565b610f54611ec0565b610f5f898989611ec8565b8315610fa557845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6008546001600160a01b031615158015610fd557506008546001600160a01b03163314155b15610ff3576040516301474c8f60e71b815260040160405180910390fd5b60065484516001600160401b03918216911611158061102c575060065460208501516001600160401b03600160401b9092048216911611155b1561104a5760405163051c46ef60e01b815260040160405180910390fd5b6110578460400151611ff4565b6110648360200151611ff4565b6110718360400151611ff4565b61107e8360600151611ff4565b5f611087610c59565b6020860151600a549192505f916110a791906001600160401b0316610da9565b6010549091506001600160401b03908116908216106110eb576110cd8660200151610a00565b156110eb5760405163080ae8d960e01b815260040160405180910390fd5b6010546001600160401b03908116908216111561119757600261110e838361312a565b6001600160401b0316106111355760405163080ae8d960e01b815260040160405180910390fd5b6111408260016130ca565b6001600160401b0316816001600160401b0316148015611179575060065461117790600160401b90046001600160401b031661099e565b155b156111975760405163080ae8d960e01b815260040160405180910390fd5b6111a386868686612035565b85516006805460208901516001600160401b03908116600160401b026001600160801b0319909216938116939093171790556040870151600755600f8590556010548116908216108015906112005750611200866020015161099e565b1561126a578451600b556020850151600c556040850151600d556060850151600e557f31eabd9099fdb25dacddd206abff87311e553441fc9d0fcdef201062d7e7071b61124e8260016130ca565b6040516001600160401b03909116815260200160405180910390a15b61127543428861225f565b85602001516001600160401b0316865f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae688604001516040516112c591815260200190565b60405180910390a3505050505050565b6112dd6115c5565b5f51602061368c5f395f51905f52805460029190600160401b900460ff1680611313575080546001600160401b03808416911610155b156113315760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b178255600590851611611379576040516350dd03f760e11b815260040160405180910390fd5b5f54600b55600154600c55600254600d55600354600e55600a80546001600160401b03858116600160401b026001600160801b0319909216908716171790556113c28385610da9565b600a805467ffffffffffffffff60801b1916600160801b6001600160401b0393841602179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b6009545f904384118061143f575080155b806114895750600854600980549091600160c01b90046001600160401b031690811061146d5761146d61315c565b5f9182526020909120600290910201546001600160401b031684105b156114a75760405163b0b4387760e01b815260040160405180910390fd5b5f80806114b5600185613149565b90505b8161155157600854600160c01b90046001600160401b031681106115515786600982815481106114ea576114ea61315c565b5f9182526020909120600290910201546001600160401b03161161153f57600191506009818154811061151f5761151f61315c565b5f9182526020909120600290910201546001600160401b03169250611551565b806115498161319d565b9150506114b8565b8161156f5760405163b0b4387760e01b815260040160405180910390fd5b8561157a8489613149565b11979650505050505050565b61158e6115c5565b6001600160a01b0381166115bc57604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b610b9d81611e3f565b336115f77f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b031614610c465760405163118cdaa760e01b81523360048201526024016115b3565b611628612657565b621000008152600560208201527f2949260dc9e9621bb41dcb96ba7054b4bd5e7e230fdba5f3411260401c55f59d6040820151527f05d036973845e2e9d2ad9a795b351535a2576d51d27f21ff8372be92bd6f39466020604083015101527f0ba2c5ae9360efec9e3968e33f57fd33059e57385c1ea7db6430426b82e0871a6060820151527f1e333b5398c953194076772a861b7bf6a4c80c4a4c2e54eb9ca67aec5ff19fc96020606083015101527f0d9e9b9f38dd9fbbd5cd8b5a1d1c8aa4e777e526e06efe39345bf3ce4c5bb4aa6080820151527f10417eaf9ba330bbf56caf331a362114153a9c95ae914fbd1f99cb84d59fbf566020608083015101527f155dfc3a039f16ab99fa9663569ff06e5bfda91748a79821d80dafc7f1d92e5e60a0820151527f15daee81e8ffcac886bf9cc7453d659a987da1feb893c1fe9a94583f337f6dfa602060a083015101527f1c6f995727083f56734a4863c3bf4433b5353ad8d20f15d554a8cd2be28ef92d60c0820151527f0736ebbf0d73d42c428d5dd66ba4d9d9513a642d94147db629964d6d032776a8602060c083015101527f2c4aa1a42d17f226532742b7da21ed908ee6a1c13d824b269d21abcd59c8672360e0820151527f05c4163ca9cab2e65abbb41b6175591cf92460000c96fb9daa1f01d50af4936c602060e083015101527f215ecf683c65ee3dca3c2fc04b4864b1f2a538ef923af6380d420fa6b5a9f496610100820151527f1d03c378f3d7063d12c459ac659ce7a27c439cd6ad184c172352815f3a380d37602061010083015101527f20bc29548f10bd07fde418d49a5692f8919694571ab64c90f583dc434a5fec0c610120820151527f244e5fcb51c747a56fe6fdb32f0b01ef3bc55627f6f9afcd98dddbede50308a3602061012083015101527f0e3646b352d00a3482e89811f4966fb646889dadb561ebb7bb7c223e8196d5b3610140820151527f1b10219a6293abaf30388f39e4c7b925f89b6f57cb81654e1ad755294e790f09602061014083015101527f2b29b36cd6d33062a9a86e24bd178d69b1cebdc1a39c7977d547e7617b5747f9610160820151527f17062161c0a63cd17cee5b14821d7820e7fa432323b122ba59c44dd01f6a9238602061016083015101527f1198db3cec1a66ccdb90886bb96fcf175316c6ea78f73f23f4a11bcf4320e11a610180820151527f063b1f963e732bd20d86e1fef855788c1aacf26babb526d84e30633a2b5a9469602061018083015101527f23809a6a5bb0bf088f97efe15168a39471a3a4e41b8d6db0100e15fa68b09f636101a0820151527f0aba7b69ab7fdda68dac9065a5ee9fb50abfe57bdb5ab359cc5b56dff65cbea160206101a083015101527f1f038064d3ca1f37c56ecfe41701f15a412c63d3c9ad52fcfd3fd4c64da8b5f26101c0820151527f2689fe5cc59e4be112c2479969c25a7f603a5d71a2e7924480c9f4eafc2c298f60206101c083015101527f113021e93328a91531e40871481c4714e0b99a6afb10c779eeb2b07a7ae6f4e76101e0820151527f1a36bb2620cdb40c4dad25257716a9d8eb1e45f715ada98e424697aaf4d95c8660206101e083015101527f08f3f88ffb9e43261294b7faf582c513f9c7d0749db6dcc434d7493b8c975b2f610200820151527f2e3e0458741119ad1422072b6815fda80a3896640f018d282c88f1506b54e0e6602061020083015101527f100a5c0a4e1ac2791d1f68bc9c25b39ccfbb5d628c53d5547f89aa0cab8324d2610220820151527f05bf9e97428c387fbbc5f9cbf6effb33b57655494c2ab9f7cc5d445a0ea56bea602061022083015101527f067f3e0ce69cbbe32337f0538bf6119c72f7fd4d92857b785caf04a225b94d46610240820151527f211a076271069fb1fae1522ab8a4779480b50ed8c4648d201341e444e8ee2d15602061024083015101527f0b931b96997d9db8bc198c750098cad2960df407880f7b2cb51c85376d5fc849610260820151527f0e9121af76d7d9616432ded6a4de93cf146f5b7353a74f8a7265d6377fd4edc7602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611cd657507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611cca5f51602061366c5f395f51905f52546001600160a01b031690565b6001600160a01b031614155b15610c465760405163703e46dd60e11b815260040160405180910390fd5b611cfc6115c5565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d906020016108a5565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611d8f575060408051601f3d908101601f19168201909252611d8c918101906131b2565b60015b611db757604051634c9c8ce360e01b81526001600160a01b03831660048201526024016115b3565b5f51602061366c5f395f51905f528114611de757604051632a87526960e21b8152600481018290526024016115b3565b611df18383612448565b505050565b306001600160a01b037f00000000000000000000000000000000000000000000000000000000000000001614610c465760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611eb761249d565b610b9d816124d3565b610c4661249d565b82516001600160401b0316151580611eec575060208301516001600160401b031615155b80611ef957506020820151155b80611f0657506040820151155b80611f1357506060820151155b80611f1d57508151155b80611f2f5750610e108163ffffffff16105b80611f4357506301e133808163ffffffff16115b15611f61576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001811080610bbb5760405163016c173360e21b815260040160405180910390fd5b5f61203e6108f9565b90506120486128bc565b600c548152600d54602080830191909152600e546040830152600b54606080840191909152600a549188015190916001600160401b03600160401b90910481169116108015906120a057506120a0876020015161099e565b156120e9576040805187516020808301919091528801518183015290870151606080830191909152870151608082015260a0016040516020818303038152906040529050612124565b60408051600b546020820152600c5491810191909152600d546060820152600e54608082015260a00160405160208183030381529060405290505b6040805188516001600160401b039081166020808401919091528a015116818301529088015160608201525f9060800160408051601f198184030181529082905261217591849089906020016131e0565b60408051601f19818403018152919052805160209091012090506121b97f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000182613202565b60808401526040516354e8bd6760e01b815273ffffffffffffffffffffffffffffffffffffffff906354e8bd67906121f990879087908a906004016133f7565b602060405180830381865af4158015612214573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906122389190613617565b612255576040516309bde33960e01b815260040160405180910390fd5b5050505050505050565b600954158015906122d4575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b031690811061229f5761229f61315c565b5f9182526020909120600290910201546122c990600160401b90046001600160401b03168461312a565b6001600160401b0316115b1561236757600854600980549091600160c01b90046001600160401b03169081106123015761230161315c565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b031690601861234183613636565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b612451826124db565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561249557611df1828261253e565b610bbb6125b0565b5f51602061368c5f395f51905f5254600160401b900460ff16610c4657604051631afcd79f60e31b815260040160405180910390fd5b61158e61249d565b806001600160a01b03163b5f0361251057604051634c9c8ce360e01b81526001600160a01b03821660048201526024016115b3565b5f51602061366c5f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b03168460405161255a9190613660565b5f60405180830381855af49150503d805f8114612592576040519150601f19603f3d011682016040523d82523d5f602084013e612597565b606091505b50915091506125a78583836125cf565b95945050505050565b3415610c465760405163b398979f60e01b815260040160405180910390fd5b6060826125e4576125df8261262e565b612627565b81511580156125fb57506001600160a01b0384163b155b1561262457604051639996b31560e01b81526001600160a01b03851660048201526024016115b3565b50805b9392505050565b80511561263e5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b604051806102c001604052805f81526020015f815260200161268a60405180604001604052805f81526020015f81525090565b81526020016126aa60405180604001604052805f81526020015f81525090565b81526020016126ca60405180604001604052805f81526020015f81525090565b81526020016126ea60405180604001604052805f81526020015f81525090565b815260200161270a60405180604001604052805f81526020015f81525090565b815260200161272a60405180604001604052805f81526020015f81525090565b815260200161274a60405180604001604052805f81526020015f81525090565b815260200161276a60405180604001604052805f81526020015f81525090565b815260200161278a60405180604001604052805f81526020015f81525090565b81526020016127aa60405180604001604052805f81526020015f81525090565b81526020016127ca60405180604001604052805f81526020015f81525090565b81526020016127ea60405180604001604052805f81526020015f81525090565b815260200161280a60405180604001604052805f81526020015f81525090565b815260200161282a60405180604001604052805f81526020015f81525090565b815260200161284a60405180604001604052805f81526020015f81525090565b815260200161286a60405180604001604052805f81526020015f81525090565b815260200161288a60405180604001604052805f81526020015f81525090565b81526020016128aa60405180604001604052805f81526020015f81525090565b81526020015f81526020015f81525090565b6040518060a001604052806005906020820280368337509192915050565b80356001600160a01b03811681146109fb575f5ffd5b5f60208284031215612900575f5ffd5b612627826128da565b5f60208284031215612919575f5ffd5b5035919050565b5f6105008201905082518252602083015160208301526040830151612952604084018280518252602090810151910152565b50606083015180516080840152602081015160a0840152506080830151805160c0840152602081015160e08401525060a0830151805161010084015260208101516101208401525060c0830151805161014084015260208101516101608401525060e0830151805161018084015260208101516101a08401525061010083015180516101c084015260208101516101e08401525061012083015180516102008401526020810151610220840152506101408301518051610240840152602081015161026084015250610160830151805161028084015260208101516102a08401525061018083015180516102c084015260208101516102e0840152506101a083015180516103008401526020810151610320840152506101c083015180516103408401526020810151610360840152506101e0830151805161038084015260208101516103a08401525061020083015180516103c084015260208101516103e08401525061022083015180516104008401526020810151610420840152506102408301518051610440840152602081015161046084015250610260830151805161048084015260208101516104a0840152506102808301516104c08301526102a0909201516104e09091015290565b80356001600160401b03811681146109fb575f5ffd5b5f60208284031215612b47575f5ffd5b61262782612b21565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715612b8757612b87612b50565b60405290565b604051601f8201601f191681016001600160401b0381118282101715612bb557612bb5612b50565b604052919050565b5f60608284031215612bcd575f5ffd5b604051606081016001600160401b0381118282101715612bef57612bef612b50565b604052905080612bfe83612b21565b8152612c0c60208401612b21565b6020820152604092830135920191909152919050565b5f60408284031215612c32575f5ffd5b604080519081016001600160401b0381118282101715612c5457612c54612b50565b604052823581526020928301359281019290925250919050565b5f6104808284031215612c7f575f5ffd5b612c87612b64565b9050612c938383612c22565b8152612ca28360408401612c22565b6020820152612cb48360808401612c22565b6040820152612cc68360c08401612c22565b6060820152612cd9836101008401612c22565b6080820152612cec836101408401612c22565b60a0820152612cff836101808401612c22565b60c0820152612d12836101c08401612c22565b60e0820152612d25836102008401612c22565b610100820152612d39836102408401612c22565b610120820152612d4d836102808401612c22565b610140820152612d61836102c08401612c22565b610160820152612d75836103008401612c22565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f6104e08385031215612e0b575f5ffd5b612e158484612bbd565b9150612e248460608501612c6e565b90509250929050565b803563ffffffff811681146109fb575f5ffd5b5f60208284031215612e50575f5ffd5b61262782612e2d565b5f5f60408385031215612e6a575f5ffd5b612e73836128da565b915060208301356001600160401b03811115612e8d575f5ffd5b8301601f81018513612e9d575f5ffd5b80356001600160401b03811115612eb657612eb6612b50565b612ec9601f8201601f1916602001612b8d565b818152866020838501011115612edd575f5ffd5b816020840160208301375f602083830101528093505050509250929050565b5f60808284031215612f0c575f5ffd5b604051608081016001600160401b0381118282101715612f2e57612f2e612b50565b6040908152833582526020808501359083015283810135908201526060928301359281019290925250919050565b5f5f5f6105608486031215612f6f575f5ffd5b612f798585612bbd565b9250612f888560608601612efc565b9150612f978560e08601612c6e565b90509250925092565b5f5f60408385031215612fb1575f5ffd5b612fba83612b21565b9150612e2460208401612b21565b5f5f5f5f6101208587031215612fdc575f5ffd5b612fe68686612bbd565b9350612ff58660608701612efc565b925061300360e08601612e2d565b915061301261010086016128da565b905092959194509250565b5f5f5f5f6105808587031215613031575f5ffd5b61303b8686612bbd565b935061304a8660608701612efc565b925060e08501359150613012866101008701612c6e565b602081525f82518060208401528060208501604085015e5f604082850101526040601f19601f83011684010191505092915050565b5f5f604083850312156130a7575f5ffd5b50508035926020909101359150565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b038181168382160190811115610a8f57610a8f6130b6565b634e487b7160e01b5f52601260045260245ffd5b5f6001600160401b03831680613115576131156130e9565b806001600160401b0384160691505092915050565b6001600160401b038281168282160390811115610a8f57610a8f6130b6565b81810381811115610a8f57610a8f6130b6565b634e487b7160e01b5f52603260045260245ffd5b5f6001600160401b03831680613188576131886130e9565b806001600160401b0384160491505092915050565b5f816131ab576131ab6130b6565b505f190190565b5f602082840312156131c2575f5ffd5b5051919050565b5f81518060208401855e5f93019283525090919050565b5f6131f46131ee83876131c9565b856131c9565b928352505060200192915050565b5f82613210576132106130e9565b500690565b805f5b6005811015613237578151845260209384019390910190600101613218565b50505050565b61325282825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e0840152506080810151805161010084015260208101516101208401525060a0810151805161014084015260208101516101608401525060c0810151805161018084015260208101516101a08401525060e081015180516101c084015260208101516101e08401525061010081015180516102008401526020810151610220840152506101208101518051610240840152602081015161026084015250610140810151805161028084015260208101516102a08401525061016081015180516102c084015260208101516102e08401525061018081015180516103008401526020810151610320840152506101a08101516103408301526101c08101516103608301526101e08101516103808301526102008101516103a08301526102208101516103c08301526102408101516103e08301526102608101516104008301526102808101516104208301526102a08101516104408301526102c0015161046090910152565b5f610a208201905084518252602085015160208301526040850151613429604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a0850151805161010084015260208101516101208401525060c0850151805161014084015260208101516101608401525060e0850151805161018084015260208101516101a08401525061010085015180516101c084015260208101516101e08401525061012085015180516102008401526020810151610220840152506101408501518051610240840152602081015161026084015250610160850151805161028084015260208101516102a08401525061018085015180516102c084015260208101516102e0840152506101a085015180516103008401526020810151610320840152506101c085015180516103408401526020810151610360840152506101e0850151805161038084015260208101516103a08401525061020085015180516103c084015260208101516103e08401525061022085015180516104008401526020810151610420840152506102408501518051610440840152602081015161046084015250610260850151805161048084015260208101516104a0840152506102808501516104c08301526102a08501516104e0830152613601610500830185613215565b61360f6105a083018461323d565b949350505050565b5f60208284031215613627575f5ffd5b81518015158114612627575f5ffd5b5f6001600160401b0382166001600160401b038103613657576136576130b6565b60010192915050565b5f61262782846131c956fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbcf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x02(W_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x01)W\x80c\x9F\xDBT\xA7\x11a\0\xA8W\x80c\xD2M\x93=\x11a\0mW\x80c\xD2M\x93=\x14a\x07]W\x80c\xE003\x01\x14a\x07\x8CW\x80c\xF0h T\x14a\x07\xABW\x80c\xF2\xFD\xE3\x8B\x14a\x07\xCAW\x80c\xF9\xE5\r\x19\x14a\x07\xE9W__\xFD[\x80c\x9F\xDBT\xA7\x14a\x06UW\x80c\xAA\xBD]\xB3\x14a\x06\xAAW\x80c\xAD<\xB1\xCC\x14a\x06\xC9W\x80c\xB3;\xC4\x91\x14a\x07\x06W\x80c\xC2;\x9E\x9E\x14a\x07%W__\xFD[\x80c\x8D\xA5\xCB[\x11a\0\xEEW\x80c\x8D\xA5\xCB[\x14a\x05\xA7W\x80c\x90\xC1C\x90\x14a\x05\xE3W\x80c\x96\xC1\xCAa\x14a\x06\x02W\x80c\x99\x83(\xE8\x14a\x06!W\x80c\x9B\xAA<\xC9\x14a\x066W__\xFD[\x80cqP\x18\xA6\x14a\x05\nW\x80cu|7\xAD\x14a\x05\x1EW\x80cvg\x18\x08\x14a\x058W\x80c\x82nA\xFC\x14a\x05LW\x80c\x85\x84\xD2?\x14a\x05kW__\xFD[\x80c0\x0C\x89\xDD\x11a\x01\xB5W\x80cBm1\x94\x11a\x01zW\x80cBm1\x94\x14a\x04\x8FW\x80cC=\xBA\x9F\x14a\x04\xB0W\x80cO\x1E\xF2\x86\x14a\x04\xCFW\x80cR\xD1\x90-\x14a\x04\xE2W\x80ci\xCCj\x04\x14a\x04\xF6W__\xFD[\x80c0\x0C\x89\xDD\x14a\x03\xE3W\x80c1=\xF7\xB1\x14a\x04\x02W\x80c7\x8E\xC2;\x14a\x049W\x80c8\xE4T\xB1\x14a\x04UW\x80c>\xD5[{\x14a\x04iW__\xFD[\x80c\x12\x17<,\x11a\x01\xFBW\x80c\x12\x17<,\x14a\x03\x17W\x80c\x16z\xC6\x18\x14a\x038W\x80c c\xD4\xF7\x14a\x03WW\x80c%)t'\x14a\x03vW\x80c/y\x88\x9D\x14a\x03\xA5W__\xFD[\x80c\x01?\xA5\xFC\x14a\x02,W\x80c\x02\xB5\x92\xF3\x14a\x02MW\x80c\x06%\xE1\x9B\x14a\x02\xAAW\x80c\r\x8En,\x14a\x02\xECW[__\xFD[4\x80\x15a\x027W__\xFD[Pa\x02Ka\x02F6`\x04a(\xF0V[a\x07\xFDV[\0[4\x80\x15a\x02XW__\xFD[Pa\x02la\x02g6`\x04a)\tV[a\x08\xB0V[`@Qa\x02\xA1\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x02\xB5W__\xFD[P`\x0BT`\x0CT`\rT`\x0ETa\x02\xCC\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x02\xA1V[4\x80\x15a\x02\xF7W__\xFD[P`@\x80Q`\x03\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02\xA1V[4\x80\x15a\x03\"W__\xFD[Pa\x03+a\x08\xF9V[`@Qa\x02\xA1\x91\x90a) V[4\x80\x15a\x03CW__\xFD[Pa\x02Ka\x03R6`\x04a+7V[a\t\x0EV[4\x80\x15a\x03bW__\xFD[Pa\x02Ka\x03q6`\x04a-\xF9V[a\t\x85V[4\x80\x15a\x03\x81W__\xFD[Pa\x03\x95a\x03\x906`\x04a+7V[a\t\x9EV[`@Q\x90\x15\x15\x81R` \x01a\x02\xA1V[4\x80\x15a\x03\xB0W__\xFD[P`\x08Ta\x03\xCB\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x03\xEEW__\xFD[Pa\x03\x95a\x03\xFD6`\x04a+7V[a\n\0V[4\x80\x15a\x04\rW__\xFD[P`\x08Ta\x04!\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x04DW__\xFD[PC[`@Q\x90\x81R` \x01a\x02\xA1V[4\x80\x15a\x04`W__\xFD[Pa\x02Ka\n\x95V[4\x80\x15a\x04tW__\xFD[P`\nTa\x03\xCB\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x04\x9AW__\xFD[P_T`\x01T`\x02T`\x03Ta\x02\xCC\x93\x92\x91\x90\x84V[4\x80\x15a\x04\xBBW__\xFD[Pa\x02Ka\x04\xCA6`\x04a.@V[a\x0B\x8CV[a\x02Ka\x04\xDD6`\x04a.YV[a\x0B\xA0V[4\x80\x15a\x04\xEDW__\xFD[Pa\x04Ga\x0B\xBFV[4\x80\x15a\x05\x01W__\xFD[Pa\x02Ka\x0B\xDAV[4\x80\x15a\x05\x15W__\xFD[Pa\x02Ka\x0CHV[4\x80\x15a\x05)W__\xFD[Pa\x02Ka\x03q6`\x04a/\\V[4\x80\x15a\x05CW__\xFD[Pa\x03\xCBa\x0CYV[4\x80\x15a\x05WW__\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15a\x03\x95V[4\x80\x15a\x05vW__\xFD[Pa\x05\x8Aa\x05\x856`\x04a)\tV[a\x0C~V[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x02\xA1V[4\x80\x15a\x05\xB2W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x04!V[4\x80\x15a\x05\xEEW__\xFD[Pa\x03\xCBa\x05\xFD6`\x04a/\xA0V[a\r\xA9V[4\x80\x15a\x06\rW__\xFD[Pa\x02Ka\x06\x1C6`\x04a.@V[a\x0E\x18V[4\x80\x15a\x06,W__\xFD[Pa\x04G`\x0FT\x81V[4\x80\x15a\x06AW__\xFD[Pa\x02Ka\x06P6`\x04a/\xC8V[a\x0E\xA1V[4\x80\x15a\x06`W__\xFD[P`\x06T`\x07Ta\x06\x84\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x02\xA1V[4\x80\x15a\x06\xB5W__\xFD[Pa\x02Ka\x06\xC46`\x04a0\x1DV[a\x0F\xB0V[4\x80\x15a\x06\xD4W__\xFD[Pa\x06\xF9`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02\xA1\x91\x90a0aV[4\x80\x15a\x07\x11W__\xFD[Pa\x02Ka\x07 6`\x04a/\xA0V[a\x12\xD5V[4\x80\x15a\x070W__\xFD[P`\x08Ta\x07H\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02\xA1V[4\x80\x15a\x07hW__\xFD[P`\x04T`\x05Ta\x06\x84\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x07\x97W__\xFD[Pa\x03\x95a\x07\xA66`\x04a0\x96V[a\x14.V[4\x80\x15a\x07\xB6W__\xFD[P`\nTa\x03\xCB\x90`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x07\xD5W__\xFD[Pa\x02Ka\x07\xE46`\x04a(\xF0V[a\x15\x86V[4\x80\x15a\x07\xF4W__\xFD[P`\tTa\x04GV[a\x08\x05a\x15\xC5V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x08,W`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\x08[W`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\x08\xBFW_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[a\t\x01a&WV[a\t\ta\x16 V[\x90P\x90V[a\t\x16a\x15\xC5V[`\n\x80To\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\x19\x81\x16`\x01`@\x1B`\x01`\x01`@\x1B\x03\x85\x81\x16\x82\x02\x92\x83\x17\x94\x85\x90Ua\t\\\x94\x91\x90\x91\x04\x81\x16\x92\x81\x16\x91\x16\x17a\r\xA9V[`\n`\x10a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPPV[`@QcN@\\\x8D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\t\xBEWP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\t\xCAWP_\x91\x90PV[`\nT`\x01`\x01`@\x1B\x03\x16a\t\xE1\x83`\x05a0\xCAV[a\t\xEB\x91\x90a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x15\x92\x91PPV[\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\n WP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\n,WP_\x91\x90PV[`\nTa\nB\x90`\x01`\x01`@\x1B\x03\x16\x83a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x15\x80a\n\x8FWP`\nTa\nj\x90`\x05\x90`\x01`\x01`@\x1B\x03\x16a1*V[`\nT`\x01`\x01`@\x1B\x03\x91\x82\x16\x91a\n\x84\x91\x16\x84a0\xFDV[`\x01`\x01`@\x1B\x03\x16\x11[\x92\x91PPV[a\n\x9Da\x15\xC5V[_Q` a6\x8C_9_Q\x90_R\x80T`\x03\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\n\xD3WP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\n\xF1W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x90\x81\x17\x83U`\nTa\x0B+\x92\x91\x81\x04\x82\x16\x91\x16a\r\xA9V[`\x10\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x83\x16\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPV[a\x0B\x94a\x15\xC5V[a\x0B\x9D\x81a\x0E\x18V[PV[a\x0B\xA8a\x1CPV[a\x0B\xB1\x82a\x1C\xF4V[a\x0B\xBB\x82\x82a\x1D5V[PPV[_a\x0B\xC8a\x1D\xF6V[P_Q` a6l_9_Q\x90_R\x90V[a\x0B\xE2a\x15\xC5V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x0C-W`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x0CPa\x15\xC5V[a\x0CF_a\x1E?V[`\x06T`\nT_\x91a\t\t\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16a\r\xA9V[`\t\x80T_\x91\x82\x91\x90a\x0C\x92`\x01\x83a1IV[\x81T\x81\x10a\x0C\xA2Wa\x0C\xA2a1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x0C\xE1W`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\r\xA2W\x84`\t\x82\x81T\x81\x10a\r\x11Wa\r\x11a1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\r\x9AW`\t\x81\x81T\x81\x10a\rJWa\rJa1\\V[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\rmWa\rma1\\V[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x0C\xF5V[PP\x91P\x91V[_\x81`\x01`\x01`@\x1B\x03\x16_\x03a\r\xC1WP_a\n\x8FV[\x82`\x01`\x01`@\x1B\x03\x16_\x03a\r\xD9WP`\x01a\n\x8FV[a\r\xE3\x82\x84a0\xFDV[`\x01`\x01`@\x1B\x03\x16_\x03a\x0E\x03Wa\r\xFC\x82\x84a1pV[\x90Pa\n\x8FV[a\x0E\r\x82\x84a1pV[a\r\xFC\x90`\x01a0\xCAV[a\x0E a\x15\xC5V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\x0E?WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\x0E]WP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\x0E{W`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[_Q` a6\x8C_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x0E\xD2WP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x0E\xEDWP0;\x15[\x90P\x81\x15\x80\x15a\x0E\xFBWP\x80\x15[\x15a\x0F\x19W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x0FCW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x0FL\x86a\x1E\xAFV[a\x0FTa\x1E\xC0V[a\x0F_\x89\x89\x89a\x1E\xC8V[\x83\x15a\x0F\xA5W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x0F\xD5WP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x0F\xF3W`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x84Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x10,WP`\x06T` \x85\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x10JW`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x10W\x84`@\x01Qa\x1F\xF4V[a\x10d\x83` \x01Qa\x1F\xF4V[a\x10q\x83`@\x01Qa\x1F\xF4V[a\x10~\x83``\x01Qa\x1F\xF4V[_a\x10\x87a\x0CYV[` \x86\x01Q`\nT\x91\x92P_\x91a\x10\xA7\x91\x90`\x01`\x01`@\x1B\x03\x16a\r\xA9V[`\x10T\x90\x91P`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x10a\x10\xEBWa\x10\xCD\x86` \x01Qa\n\0V[\x15a\x10\xEBW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x10T`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x11\x15a\x11\x97W`\x02a\x11\x0E\x83\x83a1*V[`\x01`\x01`@\x1B\x03\x16\x10a\x115W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11@\x82`\x01a0\xCAV[`\x01`\x01`@\x1B\x03\x16\x81`\x01`\x01`@\x1B\x03\x16\x14\x80\x15a\x11yWP`\x06Ta\x11w\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16a\t\x9EV[\x15[\x15a\x11\x97W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x11\xA3\x86\x86\x86\x86a 5V[\x85Q`\x06\x80T` \x89\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x81\x16\x93\x90\x93\x17\x17\x90U`@\x87\x01Q`\x07U`\x0F\x85\x90U`\x10T\x81\x16\x90\x82\x16\x10\x80\x15\x90a\x12\0WPa\x12\0\x86` \x01Qa\t\x9EV[\x15a\x12jW\x84Q`\x0BU` \x85\x01Q`\x0CU`@\x85\x01Q`\rU``\x85\x01Q`\x0EU\x7F1\xEA\xBD\x90\x99\xFD\xB2]\xAC\xDD\xD2\x06\xAB\xFF\x871\x1EU4A\xFC\x9D\x0F\xCD\xEF \x10b\xD7\xE7\x07\x1Ba\x12N\x82`\x01a0\xCAV[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xA1[a\x12uCB\x88a\"_V[\x85` \x01Q`\x01`\x01`@\x1B\x03\x16\x86_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x88`@\x01Q`@Qa\x12\xC5\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[a\x12\xDDa\x15\xC5V[_Q` a6\x8C_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x13\x13WP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x131W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x17\x82U`\x05\x90\x85\x16\x11a\x13yW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x0BU`\x01T`\x0CU`\x02T`\rU`\x03T`\x0EU`\n\x80T`\x01`\x01`@\x1B\x03\x85\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x90\x87\x16\x17\x17\x90Ua\x13\xC2\x83\x85a\r\xA9V[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x16`\x01`\x80\x1B`\x01`\x01`@\x1B\x03\x93\x84\x16\x02\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[`\tT_\x90C\x84\x11\x80a\x14?WP\x80\x15[\x80a\x14\x89WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\x14mWa\x14ma1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a\x14\xA7W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a\x14\xB5`\x01\x85a1IV[\x90P[\x81a\x15QW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a\x15QW\x86`\t\x82\x81T\x81\x10a\x14\xEAWa\x14\xEAa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a\x15?W`\x01\x91P`\t\x81\x81T\x81\x10a\x15\x1FWa\x15\x1Fa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa\x15QV[\x80a\x15I\x81a1\x9DV[\x91PPa\x14\xB8V[\x81a\x15oW`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a\x15z\x84\x89a1IV[\x11\x97\x96PPPPPPPV[a\x15\x8Ea\x15\xC5V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x15\xBCW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x0B\x9D\x81a\x1E?V[3a\x15\xF7\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x0CFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x15\xB3V[a\x16(a&WV[b\x10\0\0\x81R`\x05` \x82\x01R\x7F)I&\r\xC9\xE9b\x1B\xB4\x1D\xCB\x96\xBApT\xB4\xBD^~#\x0F\xDB\xA5\xF3A\x12`@\x1CU\xF5\x9D`@\x82\x01QR\x7F\x05\xD06\x978E\xE2\xE9\xD2\xAD\x9Ay[5\x155\xA2WmQ\xD2\x7F!\xFF\x83r\xBE\x92\xBDo9F` `@\x83\x01Q\x01R\x7F\x0B\xA2\xC5\xAE\x93`\xEF\xEC\x9E9h\xE3?W\xFD3\x05\x9EW8\\\x1E\xA7\xDBd0Bk\x82\xE0\x87\x1A``\x82\x01QR\x7F\x1E3;S\x98\xC9S\x19@vw*\x86\x1B{\xF6\xA4\xC8\x0CJL.T\xEB\x9C\xA6z\xEC_\xF1\x9F\xC9` ``\x83\x01Q\x01R\x7F\r\x9E\x9B\x9F8\xDD\x9F\xBB\xD5\xCD\x8BZ\x1D\x1C\x8A\xA4\xE7w\xE5&\xE0n\xFE94[\xF3\xCEL[\xB4\xAA`\x80\x82\x01QR\x7F\x10A~\xAF\x9B\xA30\xBB\xF5l\xAF3\x1A6!\x14\x15:\x9C\x95\xAE\x91O\xBD\x1F\x99\xCB\x84\xD5\x9F\xBFV` `\x80\x83\x01Q\x01R\x7F\x15]\xFC:\x03\x9F\x16\xAB\x99\xFA\x96cV\x9F\xF0n[\xFD\xA9\x17H\xA7\x98!\xD8\r\xAF\xC7\xF1\xD9.^`\xA0\x82\x01QR\x7F\x15\xDA\xEE\x81\xE8\xFF\xCA\xC8\x86\xBF\x9C\xC7E=e\x9A\x98}\xA1\xFE\xB8\x93\xC1\xFE\x9A\x94X?3\x7Fm\xFA` `\xA0\x83\x01Q\x01R\x7F\x1Co\x99W'\x08?VsJHc\xC3\xBFD3\xB55:\xD8\xD2\x0F\x15\xD5T\xA8\xCD+\xE2\x8E\xF9-`\xC0\x82\x01QR\x7F\x076\xEB\xBF\rs\xD4,B\x8D]\xD6k\xA4\xD9\xD9Q:d-\x94\x14}\xB6)\x96Mm\x03'v\xA8` `\xC0\x83\x01Q\x01R\x7F,J\xA1\xA4-\x17\xF2&S'B\xB7\xDA!\xED\x90\x8E\xE6\xA1\xC1=\x82K&\x9D!\xAB\xCDY\xC8g#`\xE0\x82\x01QR\x7F\x05\xC4\x16<\xA9\xCA\xB2\xE6Z\xBB\xB4\x1BauY\x1C\xF9$`\0\x0C\x96\xFB\x9D\xAA\x1F\x01\xD5\n\xF4\x93l` `\xE0\x83\x01Q\x01R\x7F!^\xCFh<e\xEE=\xCA</\xC0KHd\xB1\xF2\xA58\xEF\x92:\xF68\rB\x0F\xA6\xB5\xA9\xF4\x96a\x01\0\x82\x01QR\x7F\x1D\x03\xC3x\xF3\xD7\x06=\x12\xC4Y\xACe\x9C\xE7\xA2|C\x9C\xD6\xAD\x18L\x17#R\x81_:8\r7` a\x01\0\x83\x01Q\x01R\x7F \xBC)T\x8F\x10\xBD\x07\xFD\xE4\x18\xD4\x9AV\x92\xF8\x91\x96\x94W\x1A\xB6L\x90\xF5\x83\xDCCJ_\xEC\x0Ca\x01 \x82\x01QR\x7F$N_\xCBQ\xC7G\xA5o\xE6\xFD\xB3/\x0B\x01\xEF;\xC5V'\xF6\xF9\xAF\xCD\x98\xDD\xDB\xED\xE5\x03\x08\xA3` a\x01 \x83\x01Q\x01R\x7F\x0E6F\xB3R\xD0\n4\x82\xE8\x98\x11\xF4\x96o\xB6F\x88\x9D\xAD\xB5a\xEB\xB7\xBB|\">\x81\x96\xD5\xB3a\x01@\x82\x01QR\x7F\x1B\x10!\x9Ab\x93\xAB\xAF08\x8F9\xE4\xC7\xB9%\xF8\x9BoW\xCB\x81eN\x1A\xD7U)Ny\x0F\t` a\x01@\x83\x01Q\x01R\x7F+)\xB3l\xD6\xD30b\xA9\xA8n$\xBD\x17\x8Di\xB1\xCE\xBD\xC1\xA3\x9Cyw\xD5G\xE7a{WG\xF9a\x01`\x82\x01QR\x7F\x17\x06!a\xC0\xA6<\xD1|\xEE[\x14\x82\x1Dx \xE7\xFAC##\xB1\"\xBAY\xC4M\xD0\x1Fj\x928` a\x01`\x83\x01Q\x01R\x7F\x11\x98\xDB<\xEC\x1Af\xCC\xDB\x90\x88k\xB9o\xCF\x17S\x16\xC6\xEAx\xF7?#\xF4\xA1\x1B\xCFC \xE1\x1Aa\x01\x80\x82\x01QR\x7F\x06;\x1F\x96>s+\xD2\r\x86\xE1\xFE\xF8Ux\x8C\x1A\xAC\xF2k\xAB\xB5&\xD8N0c:+Z\x94i` a\x01\x80\x83\x01Q\x01R\x7F#\x80\x9Aj[\xB0\xBF\x08\x8F\x97\xEF\xE1Qh\xA3\x94q\xA3\xA4\xE4\x1B\x8Dm\xB0\x10\x0E\x15\xFAh\xB0\x9Fca\x01\xA0\x82\x01QR\x7F\n\xBA{i\xAB\x7F\xDD\xA6\x8D\xAC\x90e\xA5\xEE\x9F\xB5\n\xBF\xE5{\xDBZ\xB3Y\xCC[V\xDF\xF6\\\xBE\xA1` a\x01\xA0\x83\x01Q\x01R\x7F\x1F\x03\x80d\xD3\xCA\x1F7\xC5n\xCF\xE4\x17\x01\xF1ZA,c\xD3\xC9\xADR\xFC\xFD?\xD4\xC6M\xA8\xB5\xF2a\x01\xC0\x82\x01QR\x7F&\x89\xFE\\\xC5\x9EK\xE1\x12\xC2G\x99i\xC2Z\x7F`:]q\xA2\xE7\x92D\x80\xC9\xF4\xEA\xFC,)\x8F` a\x01\xC0\x83\x01Q\x01R\x7F\x110!\xE93(\xA9\x151\xE4\x08qH\x1CG\x14\xE0\xB9\x9Aj\xFB\x10\xC7y\xEE\xB2\xB0zz\xE6\xF4\xE7a\x01\xE0\x82\x01QR\x7F\x1A6\xBB& \xCD\xB4\x0CM\xAD%%w\x16\xA9\xD8\xEB\x1EE\xF7\x15\xAD\xA9\x8EBF\x97\xAA\xF4\xD9\\\x86` a\x01\xE0\x83\x01Q\x01R\x7F\x08\xF3\xF8\x8F\xFB\x9EC&\x12\x94\xB7\xFA\xF5\x82\xC5\x13\xF9\xC7\xD0t\x9D\xB6\xDC\xC44\xD7I;\x8C\x97[/a\x02\0\x82\x01QR\x7F.>\x04Xt\x11\x19\xAD\x14\"\x07+h\x15\xFD\xA8\n8\x96d\x0F\x01\x8D(,\x88\xF1PkT\xE0\xE6` a\x02\0\x83\x01Q\x01R\x7F\x10\n\\\nN\x1A\xC2y\x1D\x1Fh\xBC\x9C%\xB3\x9C\xCF\xBB]b\x8CS\xD5T\x7F\x89\xAA\x0C\xAB\x83$\xD2a\x02 \x82\x01QR\x7F\x05\xBF\x9E\x97B\x8C8\x7F\xBB\xC5\xF9\xCB\xF6\xEF\xFB3\xB5vUIL*\xB9\xF7\xCC]DZ\x0E\xA5k\xEA` a\x02 \x83\x01Q\x01R\x7F\x06\x7F>\x0C\xE6\x9C\xBB\xE3#7\xF0S\x8B\xF6\x11\x9Cr\xF7\xFDM\x92\x85{x\\\xAF\x04\xA2%\xB9MFa\x02@\x82\x01QR\x7F!\x1A\x07bq\x06\x9F\xB1\xFA\xE1R*\xB8\xA4w\x94\x80\xB5\x0E\xD8\xC4d\x8D \x13A\xE4D\xE8\xEE-\x15` a\x02@\x83\x01Q\x01R\x7F\x0B\x93\x1B\x96\x99}\x9D\xB8\xBC\x19\x8Cu\0\x98\xCA\xD2\x96\r\xF4\x07\x88\x0F{,\xB5\x1C\x857m_\xC8Ia\x02`\x82\x01QR\x7F\x0E\x91!\xAFv\xD7\xD9ad2\xDE\xD6\xA4\xDE\x93\xCF\x14o[sS\xA7O\x8Are\xD67\x7F\xD4\xED\xC7` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1C\xD6WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1C\xCA_Q` a6l_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x0CFW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1C\xFCa\x15\xC5V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\x08\xA5V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1D\x8FWP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1D\x8C\x91\x81\x01\x90a1\xB2V[`\x01[a\x1D\xB7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x15\xB3V[_Q` a6l_9_Q\x90_R\x81\x14a\x1D\xE7W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x15\xB3V[a\x1D\xF1\x83\x83a$HV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x0CFW`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1E\xB7a$\x9DV[a\x0B\x9D\x81a$\xD3V[a\x0CFa$\x9DV[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a\x1E\xECWP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a\x1E\xF9WP` \x82\x01Q\x15[\x80a\x1F\x06WP`@\x82\x01Q\x15[\x80a\x1F\x13WP``\x82\x01Q\x15[\x80a\x1F\x1DWP\x81Q\x15[\x80a\x1F/WPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a\x1FCWPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a\x1FaW`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x0B\xBBW`@Qc\x01l\x173`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a >a\x08\xF9V[\x90Pa Ha(\xBCV[`\x0CT\x81R`\rT` \x80\x83\x01\x91\x90\x91R`\x0ET`@\x83\x01R`\x0BT``\x80\x84\x01\x91\x90\x91R`\nT\x91\x88\x01Q\x90\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x91\x04\x81\x16\x91\x16\x10\x80\x15\x90a \xA0WPa \xA0\x87` \x01Qa\t\x9EV[\x15a \xE9W`@\x80Q\x87Q` \x80\x83\x01\x91\x90\x91R\x88\x01Q\x81\x83\x01R\x90\x87\x01Q``\x80\x83\x01\x91\x90\x91R\x87\x01Q`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa!$V[`@\x80Q`\x0BT` \x82\x01R`\x0CT\x91\x81\x01\x91\x90\x91R`\rT``\x82\x01R`\x0ET`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P[`@\x80Q\x88Q`\x01`\x01`@\x1B\x03\x90\x81\x16` \x80\x84\x01\x91\x90\x91R\x8A\x01Q\x16\x81\x83\x01R\x90\x88\x01Q``\x82\x01R_\x90`\x80\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra!u\x91\x84\x90\x89\x90` \x01a1\xE0V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x80Q` \x90\x91\x01 \x90Pa!\xB9\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x82a2\x02V[`\x80\x84\x01R`@QcT\xE8\xBDg`\xE0\x1B\x81Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90cT\xE8\xBDg\x90a!\xF9\x90\x87\x90\x87\x90\x8A\x90`\x04\x01a3\xF7V[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a\"\x14W=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\"8\x91\x90a6\x17V[a\"UW`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPPPV[`\tT\x15\x80\x15\x90a\"\xD4WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a\"\x9FWa\"\x9Fa1\\V[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta\"\xC9\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a1*V[`\x01`\x01`@\x1B\x03\x16\x11[\x15a#gW`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a#\x01Wa#\x01a1\\V[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a#A\x83a66V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[a$Q\x82a$\xDBV[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a$\x95Wa\x1D\xF1\x82\x82a%>V[a\x0B\xBBa%\xB0V[_Q` a6\x8C_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\x0CFW`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x15\x8Ea$\x9DV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a%\x10W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x15\xB3V[_Q` a6l_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa%Z\x91\x90a6`V[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a%\x92W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a%\x97V[``\x91P[P\x91P\x91Pa%\xA7\x85\x83\x83a%\xCFV[\x95\x94PPPPPV[4\x15a\x0CFW`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a%\xE4Wa%\xDF\x82a&.V[a&'V[\x81Q\x15\x80\x15a%\xFBWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a&$W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x15\xB3V[P\x80[\x93\x92PPPV[\x80Q\x15a&>W\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a&\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xCA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a&\xEA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\n`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'*`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'J`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'j`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xCA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a'\xEA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\n`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(*`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(J`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(j`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\x8A`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a(\xAA`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\xA0\x01`@R\x80`\x05\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a)\0W__\xFD[a&'\x82a(\xDAV[_` \x82\x84\x03\x12\x15a)\x19W__\xFD[P5\x91\x90PV[_a\x05\0\x82\x01\x90P\x82Q\x82R` \x83\x01Q` \x83\x01R`@\x83\x01Qa)R`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x83\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x83\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x83\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x83\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x83\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x83\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x83\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x83\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x83\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x83\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x83\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x83\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x83\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x83\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x83\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x83\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x83\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x83\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x90\x92\x01Qa\x04\xE0\x90\x91\x01R\x90V[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a+GW__\xFD[a&'\x82a+!V[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\x87Wa+\x87a+PV[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\xB5Wa+\xB5a+PV[`@R\x91\x90PV[_``\x82\x84\x03\x12\x15a+\xCDW__\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a+\xEFWa+\xEFa+PV[`@R\x90P\x80a+\xFE\x83a+!V[\x81Ra,\x0C` \x84\x01a+!V[` \x82\x01R`@\x92\x83\x015\x92\x01\x91\x90\x91R\x91\x90PV[_`@\x82\x84\x03\x12\x15a,2W__\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a,TWa,Ta+PV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a,\x7FW__\xFD[a,\x87a+dV[\x90Pa,\x93\x83\x83a,\"V[\x81Ra,\xA2\x83`@\x84\x01a,\"V[` \x82\x01Ra,\xB4\x83`\x80\x84\x01a,\"V[`@\x82\x01Ra,\xC6\x83`\xC0\x84\x01a,\"V[``\x82\x01Ra,\xD9\x83a\x01\0\x84\x01a,\"V[`\x80\x82\x01Ra,\xEC\x83a\x01@\x84\x01a,\"V[`\xA0\x82\x01Ra,\xFF\x83a\x01\x80\x84\x01a,\"V[`\xC0\x82\x01Ra-\x12\x83a\x01\xC0\x84\x01a,\"V[`\xE0\x82\x01Ra-%\x83a\x02\0\x84\x01a,\"V[a\x01\0\x82\x01Ra-9\x83a\x02@\x84\x01a,\"V[a\x01 \x82\x01Ra-M\x83a\x02\x80\x84\x01a,\"V[a\x01@\x82\x01Ra-a\x83a\x02\xC0\x84\x01a,\"V[a\x01`\x82\x01Ra-u\x83a\x03\0\x84\x01a,\"V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[__a\x04\xE0\x83\x85\x03\x12\x15a.\x0BW__\xFD[a.\x15\x84\x84a+\xBDV[\x91Pa.$\x84``\x85\x01a,nV[\x90P\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\t\xFBW__\xFD[_` \x82\x84\x03\x12\x15a.PW__\xFD[a&'\x82a.-V[__`@\x83\x85\x03\x12\x15a.jW__\xFD[a.s\x83a(\xDAV[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a.\x8DW__\xFD[\x83\x01`\x1F\x81\x01\x85\x13a.\x9DW__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a.\xB6Wa.\xB6a+PV[a.\xC9`\x1F\x82\x01`\x1F\x19\x16` \x01a+\x8DV[\x81\x81R\x86` \x83\x85\x01\x01\x11\x15a.\xDDW__\xFD[\x81` \x84\x01` \x83\x017_` \x83\x83\x01\x01R\x80\x93PPPP\x92P\x92\x90PV[_`\x80\x82\x84\x03\x12\x15a/\x0CW__\xFD[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/.Wa/.a+PV[`@\x90\x81R\x835\x82R` \x80\x85\x015\x90\x83\x01R\x83\x81\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[___a\x05`\x84\x86\x03\x12\x15a/oW__\xFD[a/y\x85\x85a+\xBDV[\x92Pa/\x88\x85``\x86\x01a.\xFCV[\x91Pa/\x97\x85`\xE0\x86\x01a,nV[\x90P\x92P\x92P\x92V[__`@\x83\x85\x03\x12\x15a/\xB1W__\xFD[a/\xBA\x83a+!V[\x91Pa.$` \x84\x01a+!V[____a\x01 \x85\x87\x03\x12\x15a/\xDCW__\xFD[a/\xE6\x86\x86a+\xBDV[\x93Pa/\xF5\x86``\x87\x01a.\xFCV[\x92Pa0\x03`\xE0\x86\x01a.-V[\x91Pa0\x12a\x01\0\x86\x01a(\xDAV[\x90P\x92\x95\x91\x94P\x92PV[____a\x05\x80\x85\x87\x03\x12\x15a01W__\xFD[a0;\x86\x86a+\xBDV[\x93Pa0J\x86``\x87\x01a.\xFCV[\x92P`\xE0\x85\x015\x91Pa0\x12\x86a\x01\0\x87\x01a,nV[` \x81R_\x82Q\x80` \x84\x01R\x80` \x85\x01`@\x85\x01^_`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[__`@\x83\x85\x03\x12\x15a0\xA7W__\xFD[PP\x805\x92` \x90\x91\x015\x91PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a1\x15Wa1\x15a0\xE9V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x06\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[\x81\x81\x03\x81\x81\x11\x15a\n\x8FWa\n\x8Fa0\xB6V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a1\x88Wa1\x88a0\xE9V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x04\x91PP\x92\x91PPV[_\x81a1\xABWa1\xABa0\xB6V[P_\x19\x01\x90V[_` \x82\x84\x03\x12\x15a1\xC2W__\xFD[PQ\x91\x90PV[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a1\xF4a1\xEE\x83\x87a1\xC9V[\x85a1\xC9V[\x92\x83RPP` \x01\x92\x91PPV[_\x82a2\x10Wa2\x10a0\xE9V[P\x06\x90V[\x80_[`\x05\x81\x10\x15a27W\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a2\x18V[PPPPV[a2R\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xA0\x81\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xC0\x81\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RP`\xE0\x81\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01\0\x81\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01 \x81\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01@\x81\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01`\x81\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\x80\x81\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xA0\x81\x01Qa\x03@\x83\x01Ra\x01\xC0\x81\x01Qa\x03`\x83\x01Ra\x01\xE0\x81\x01Qa\x03\x80\x83\x01Ra\x02\0\x81\x01Qa\x03\xA0\x83\x01Ra\x02 \x81\x01Qa\x03\xC0\x83\x01Ra\x02@\x81\x01Qa\x03\xE0\x83\x01Ra\x02`\x81\x01Qa\x04\0\x83\x01Ra\x02\x80\x81\x01Qa\x04 \x83\x01Ra\x02\xA0\x81\x01Qa\x04@\x83\x01Ra\x02\xC0\x01Qa\x04`\x90\x91\x01RV[_a\n \x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa4)`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x85\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x85\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x85\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x85\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x85\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x85\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x85\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x85\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x85\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x85\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x85\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x85\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x85\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x85\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x85\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra6\x01a\x05\0\x83\x01\x85a2\x15V[a6\x0Fa\x05\xA0\x83\x01\x84a2=V[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a6'W__\xFD[\x81Q\x80\x15\x15\x81\x14a&'W__\xFD[_`\x01`\x01`@\x1B\x03\x82\x16`\x01`\x01`@\x1B\x03\x81\x03a6WWa6Wa0\xB6V[`\x01\x01\x92\x91PPV[_a&'\x82\x84a1\xC9V\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `DeprecatedApi()` and selector `0x4e405c8d`.
    ```solidity
    error DeprecatedApi();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DeprecatedApi;
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
        impl ::core::convert::From<DeprecatedApi> for UnderlyingRustTuple<'_> {
            fn from(value: DeprecatedApi) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for DeprecatedApi {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for DeprecatedApi {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "DeprecatedApi()";
            const SELECTOR: [u8; 4] = [78u8, 64u8, 92u8, 141u8];
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
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ERC1967NonPayable()` and selector `0xb398979f`.
    ```solidity
    error ERC1967NonPayable();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ERC1967NonPayable;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `FailedInnerCall()` and selector `0x1425ea42`.
    ```solidity
    error FailedInnerCall();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct FailedInnerCall;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InsufficientSnapshotHistory()` and selector `0xb0b43877`.
    ```solidity
    error InsufficientSnapshotHistory();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InsufficientSnapshotHistory;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidAddress()` and selector `0xe6c4247b`.
    ```solidity
    error InvalidAddress();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidAddress;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidArgs()` and selector `0xa1ba07ee`.
    ```solidity
    error InvalidArgs();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidArgs;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidHotShotBlockForCommitmentCheck()` and selector `0x615a9264`.
    ```solidity
    error InvalidHotShotBlockForCommitmentCheck();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidHotShotBlockForCommitmentCheck;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidInitialization()` and selector `0xf92ee8a9`.
    ```solidity
    error InvalidInitialization();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidInitialization;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidMaxStateHistory()` and selector `0xf4a0eee0`.
    ```solidity
    error InvalidMaxStateHistory();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidMaxStateHistory;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidProof()` and selector `0x09bde339`.
    ```solidity
    error InvalidProof();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidProof;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidScalar()` and selector `0x05b05ccc`.
    ```solidity
    error InvalidScalar();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidScalar;
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
        impl ::core::convert::From<InvalidScalar> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidScalar) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidScalar {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidScalar {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidScalar()";
            const SELECTOR: [u8; 4] = [5u8, 176u8, 92u8, 204u8];
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
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `MissingEpochRootUpdate()` and selector `0x080ae8d9`.
    ```solidity
    error MissingEpochRootUpdate();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct MissingEpochRootUpdate;
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
        impl ::core::convert::From<MissingEpochRootUpdate> for UnderlyingRustTuple<'_> {
            fn from(value: MissingEpochRootUpdate) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for MissingEpochRootUpdate {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for MissingEpochRootUpdate {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "MissingEpochRootUpdate()";
            const SELECTOR: [u8; 4] = [8u8, 10u8, 232u8, 217u8];
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
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `NoChangeRequired()` and selector `0xa863aec9`.
    ```solidity
    error NoChangeRequired();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct NoChangeRequired;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `NotInitializing()` and selector `0xd7e6bcf8`.
    ```solidity
    error NotInitializing();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct NotInitializing;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `OutdatedState()` and selector `0x051c46ef`.
    ```solidity
    error OutdatedState();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct OutdatedState;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `ProverNotPermissioned()` and selector `0xa3a64780`.
    ```solidity
    error ProverNotPermissioned();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ProverNotPermissioned;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `UUPSUnauthorizedCallContext()` and selector `0xe07c8dba`.
    ```solidity
    error UUPSUnauthorizedCallContext();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UUPSUnauthorizedCallContext;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `WrongStakeTableUsed()` and selector `0x51618089`.
    ```solidity
    error WrongStakeTableUsed();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct WrongStakeTableUsed;
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
                Self
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
            #[inline]
            fn abi_decode_raw_validate(data: &[u8]) -> alloy_sol_types::Result<Self> {
                <Self::Parameters<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Event with signature `NewEpoch(uint64)` and selector `0x31eabd9099fdb25dacddd206abff87311e553441fc9d0fcdef201062d7e7071b`.
    ```solidity
    event NewEpoch(uint64 epoch);
    ```*/
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    #[derive(Clone)]
    pub struct NewEpoch {
        #[allow(missing_docs)]
        pub epoch: u64,
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
        impl alloy_sol_types::SolEvent for NewEpoch {
            type DataTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type DataToken<'a> = <Self::DataTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "NewEpoch(uint64)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 =
                alloy_sol_types::private::B256::new([
                    49u8, 234u8, 189u8, 144u8, 153u8, 253u8, 178u8, 93u8, 172u8, 221u8, 210u8, 6u8,
                    171u8, 255u8, 135u8, 49u8, 30u8, 85u8, 52u8, 65u8, 252u8, 157u8, 15u8, 205u8,
                    239u8, 32u8, 16u8, 98u8, 215u8, 231u8, 7u8, 27u8,
                ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { epoch: data.0 }
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
                        &self.epoch,
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
        impl alloy_sol_types::private::IntoLogData for NewEpoch {
            fn to_log_data(&self) -> alloy_sol_types::private::LogData {
                From::from(self)
            }
            fn into_log_data(self) -> alloy_sol_types::private::LogData {
                From::from(&self)
            }
        }
        #[automatically_derived]
        impl From<&NewEpoch> for alloy_sol_types::private::LogData {
            #[inline]
            fn from(this: &NewEpoch) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    pub struct PermissionedProverNotRequired;
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `UPGRADE_INTERFACE_VERSION()` and selector `0xad3cb1cc`.
    ```solidity
    function UPGRADE_INTERFACE_VERSION() external view returns (string memory);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UPGRADE_INTERFACE_VERSIONCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::String;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<alloy::sol_types::sol_data::String as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: UPGRADE_INTERFACE_VERSIONReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: UPGRADE_INTERFACE_VERSIONReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `_getVk()` and selector `0x12173c2c`.
    ```solidity
    function _getVk() external pure returns (IPlonkVerifier.VerifyingKey memory vk);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _getVkCall;
    #[derive(serde::Serialize, serde::Deserialize)]
    ///Container type for the return parameters of the [`_getVk()`](_getVkCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _getVkReturn {
        #[allow(missing_docs)]
        pub vk: <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<_getVkCall> for UnderlyingRustTuple<'_> {
                fn from(value: _getVkCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _getVkCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (IPlonkVerifier::VerifyingKey,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<_getVkReturn> for UnderlyingRustTuple<'_> {
                fn from(value: _getVkReturn) -> Self {
                    (value.vk,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _getVkReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { vk: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for _getVkCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType;
            type ReturnTuple<'a> = (IPlonkVerifier::VerifyingKey,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "_getVk()";
            const SELECTOR: [u8; 4] = [18u8, 23u8, 60u8, 44u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<IPlonkVerifier::VerifyingKey as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: _getVkReturn = r.into();
                        r.vk
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: _getVkReturn = r.into();
                    r.vk
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `authRoot()` and selector `0x998328e8`.
    ```solidity
    function authRoot() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct authRootCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`authRoot()`](authRootCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct authRootReturn {
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
            impl ::core::convert::From<authRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: authRootCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for authRootCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<authRootReturn> for UnderlyingRustTuple<'_> {
                fn from(value: authRootReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for authRootReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for authRootCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = alloy::sol_types::private::primitives::aliases::U256;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "authRoot()";
            const SELECTOR: [u8; 4] = [153u8, 131u8, 40u8, 232u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: authRootReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: authRootReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `blocksPerEpoch()` and selector `0xf0682054`.
    ```solidity
    function blocksPerEpoch() external view returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct blocksPerEpochCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`blocksPerEpoch()`](blocksPerEpochCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct blocksPerEpochReturn {
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
            impl ::core::convert::From<blocksPerEpochCall> for UnderlyingRustTuple<'_> {
                fn from(value: blocksPerEpochCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for blocksPerEpochCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<blocksPerEpochReturn> for UnderlyingRustTuple<'_> {
                fn from(value: blocksPerEpochReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for blocksPerEpochReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for blocksPerEpochCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = u64;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "blocksPerEpoch()";
            const SELECTOR: [u8; 4] = [240u8, 104u8, 32u8, 84u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: blocksPerEpochReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: blocksPerEpochReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `currentBlockNumber()` and selector `0x378ec23b`.
    ```solidity
    function currentBlockNumber() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentBlockNumberCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::primitives::aliases::U256;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: currentBlockNumberReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: currentBlockNumberReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `currentEpoch()` and selector `0x76671808`.
    ```solidity
    function currentEpoch() external view returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentEpochCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`currentEpoch()`](currentEpochCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentEpochReturn {
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
            impl ::core::convert::From<currentEpochCall> for UnderlyingRustTuple<'_> {
                fn from(value: currentEpochCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for currentEpochCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<currentEpochReturn> for UnderlyingRustTuple<'_> {
                fn from(value: currentEpochReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for currentEpochReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for currentEpochCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = u64;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "currentEpoch()";
            const SELECTOR: [u8; 4] = [118u8, 103u8, 24u8, 8u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: currentEpochReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: currentEpochReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `disablePermissionedProverMode()` and selector `0x69cc6a04`.
    ```solidity
    function disablePermissionedProverMode() external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct disablePermissionedProverModeCall;
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
                    Self
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
        impl disablePermissionedProverModeReturn {
            fn _tokenize(
                &self,
            ) -> <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                disablePermissionedProverModeReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `epochFromBlockNumber(uint64,uint64)` and selector `0x90c14390`.
    ```solidity
    function epochFromBlockNumber(uint64 _blockNum, uint64 _blocksPerEpoch) external pure returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct epochFromBlockNumberCall {
        #[allow(missing_docs)]
        pub _blockNum: u64,
        #[allow(missing_docs)]
        pub _blocksPerEpoch: u64,
    }
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`epochFromBlockNumber(uint64,uint64)`](epochFromBlockNumberCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct epochFromBlockNumberReturn {
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
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u64, u64);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<epochFromBlockNumberCall> for UnderlyingRustTuple<'_> {
                fn from(value: epochFromBlockNumberCall) -> Self {
                    (value._blockNum, value._blocksPerEpoch)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for epochFromBlockNumberCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _blockNum: tuple.0,
                        _blocksPerEpoch: tuple.1,
                    }
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
            impl ::core::convert::From<epochFromBlockNumberReturn> for UnderlyingRustTuple<'_> {
                fn from(value: epochFromBlockNumberReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for epochFromBlockNumberReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for epochFromBlockNumberCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = u64;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "epochFromBlockNumber(uint64,uint64)";
            const SELECTOR: [u8; 4] = [144u8, 193u8, 67u8, 144u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self._blockNum,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self._blocksPerEpoch,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: epochFromBlockNumberReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: epochFromBlockNumberReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `epochStartBlock()` and selector `0x3ed55b7b`.
    ```solidity
    function epochStartBlock() external view returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct epochStartBlockCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`epochStartBlock()`](epochStartBlockCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct epochStartBlockReturn {
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
            impl ::core::convert::From<epochStartBlockCall> for UnderlyingRustTuple<'_> {
                fn from(value: epochStartBlockCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for epochStartBlockCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<epochStartBlockReturn> for UnderlyingRustTuple<'_> {
                fn from(value: epochStartBlockReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for epochStartBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for epochStartBlockCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = u64;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "epochStartBlock()";
            const SELECTOR: [u8; 4] = [62u8, 213u8, 91u8, 123u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: epochStartBlockReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: epochStartBlockReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `finalizedState()` and selector `0x9fdb54a7`.
    ```solidity
    function finalizedState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct finalizedStateCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
        impl finalizedStateReturn {
            fn _tokenize(
                &self,
            ) -> <finalizedStateCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                finalizedStateReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `genesisStakeTableState()` and selector `0x426d3194`.
    ```solidity
    function genesisStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStakeTableStateCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
        impl genesisStakeTableStateReturn {
            fn _tokenize(
                &self,
            ) -> <genesisStakeTableStateCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                genesisStakeTableStateReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `genesisState()` and selector `0xd24d933d`.
    ```solidity
    function genesisState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStateCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
        impl genesisStateReturn {
            fn _tokenize(&self) -> <genesisStateCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                genesisStateReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        impl getHotShotCommitmentReturn {
            fn _tokenize(
                &self,
            ) -> <getHotShotCommitmentCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                (
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.hotShotBlockCommRoot,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.hotshotBlockHeight,
                    ),
                )
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                getHotShotCommitmentReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getStateHistoryCount()` and selector `0xf9e50d19`.
    ```solidity
    function getStateHistoryCount() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getStateHistoryCountCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::primitives::aliases::U256;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: getStateHistoryCountReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: getStateHistoryCountReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `getVersion()` and selector `0x0d8e6e2c`.
    ```solidity
    function getVersion() external pure returns (uint8 majorVersion, uint8 minorVersion, uint8 patchVersion);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getVersionCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
        impl getVersionReturn {
            fn _tokenize(&self) -> <getVersionCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(
                        &self.majorVersion,
                    ),
                    <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(
                        &self.minorVersion,
                    ),
                    <alloy::sol_types::sol_data::Uint<8> as alloy_sol_types::SolType>::tokenize(
                        &self.patchVersion,
                    ),
                )
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                getVersionReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initialize((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint32,address)` and selector `0x9baa3cc9`.
    ```solidity
    function initialize(LightClient.LightClientState memory _genesis, LightClient.StakeTableState memory _genesisStakeTableState, uint32 _stateHistoryRetentionPeriod, address owner) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeCall {
        #[allow(missing_docs)]
        pub _genesis: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _genesisStakeTableState:
            <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
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
                LightClient::LightClientState,
                LightClient::StakeTableState,
                alloy::sol_types::sol_data::Uint<32>,
                alloy::sol_types::sol_data::Address,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
                <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
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
        impl initializeReturn {
            fn _tokenize(&self) -> <initializeCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeCall {
            type Parameters<'a> = (
                LightClient::LightClientState,
                LightClient::StakeTableState,
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
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(
                        &self._genesis,
                    ),
                    <LightClient::StakeTableState as alloy_sol_types::SolType>::tokenize(
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                initializeReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initializeV2(uint64,uint64)` and selector `0xb33bc491`.
    ```solidity
    function initializeV2(uint64 _blocksPerEpoch, uint64 _epochStartBlock) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV2Call {
        #[allow(missing_docs)]
        pub _blocksPerEpoch: u64,
        #[allow(missing_docs)]
        pub _epochStartBlock: u64,
    }
    ///Container type for the return parameters of the [`initializeV2(uint64,uint64)`](initializeV2Call) function.
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
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (u64, u64);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<initializeV2Call> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV2Call) -> Self {
                    (value._blocksPerEpoch, value._epochStartBlock)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV2Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _blocksPerEpoch: tuple.0,
                        _epochStartBlock: tuple.1,
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
        impl initializeV2Return {
            fn _tokenize(&self) -> <initializeV2Call as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeV2Call {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeV2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initializeV2(uint64,uint64)";
            const SELECTOR: [u8; 4] = [179u8, 59u8, 196u8, 145u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self._blocksPerEpoch,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self._epochStartBlock,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                initializeV2Return::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `initializeV3()` and selector `0x38e454b1`.
    ```solidity
    function initializeV3() external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV3Call;
    ///Container type for the return parameters of the [`initializeV3()`](initializeV3Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV3Return {}
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
            impl ::core::convert::From<initializeV3Call> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV3Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV3Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<initializeV3Return> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV3Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV3Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl initializeV3Return {
            fn _tokenize(&self) -> <initializeV3Call as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeV3Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeV3Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "initializeV3()";
            const SELECTOR: [u8; 4] = [56u8, 228u8, 84u8, 177u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                initializeV3Return::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isEpochRoot(uint64)` and selector `0x25297427`.
    ```solidity
    function isEpochRoot(uint64 blockHeight) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isEpochRootCall {
        #[allow(missing_docs)]
        pub blockHeight: u64,
    }
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isEpochRoot(uint64)`](isEpochRootCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isEpochRootReturn {
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
            impl ::core::convert::From<isEpochRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: isEpochRootCall) -> Self {
                    (value.blockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isEpochRootCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blockHeight: tuple.0,
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
            impl ::core::convert::From<isEpochRootReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isEpochRootReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isEpochRootReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isEpochRootCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = bool;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isEpochRoot(uint64)";
            const SELECTOR: [u8; 4] = [37u8, 41u8, 116u8, 39u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.blockHeight,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<alloy::sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: isEpochRootReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: isEpochRootReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isGtEpochRoot(uint64)` and selector `0x300c89dd`.
    ```solidity
    function isGtEpochRoot(uint64 blockHeight) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isGtEpochRootCall {
        #[allow(missing_docs)]
        pub blockHeight: u64,
    }
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`isGtEpochRoot(uint64)`](isGtEpochRootCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isGtEpochRootReturn {
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
            impl ::core::convert::From<isGtEpochRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: isGtEpochRootCall) -> Self {
                    (value.blockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isGtEpochRootCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blockHeight: tuple.0,
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
            impl ::core::convert::From<isGtEpochRootReturn> for UnderlyingRustTuple<'_> {
                fn from(value: isGtEpochRootReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isGtEpochRootReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isGtEpochRootCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = bool;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "isGtEpochRoot(uint64)";
            const SELECTOR: [u8; 4] = [48u8, 12u8, 137u8, 221u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.blockHeight,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<alloy::sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: isGtEpochRootReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: isGtEpochRootReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `isPermissionedProverEnabled()` and selector `0x826e41fc`.
    ```solidity
    function isPermissionedProverEnabled() external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPermissionedProverEnabledCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = bool;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<alloy::sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: isPermissionedProverEnabledReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: isPermissionedProverEnabledReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
            type Return = bool;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (<alloy::sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(ret),)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: lagOverEscapeHatchThresholdReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: lagOverEscapeHatchThresholdReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize)]
    /**Function with signature `newFinalizedState((uint64,uint64,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0x2063d4f7`.
    ```solidity
    function newFinalizedState(LightClient.LightClientState memory, IPlonkVerifier.PlonkProof memory) external pure;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_0Call {
        #[allow(missing_docs)]
        pub _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _1: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`newFinalizedState((uint64,uint64,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](newFinalizedState_0Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_0Return {}
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
                (LightClient::LightClientState, IPlonkVerifier::PlonkProof);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<newFinalizedState_0Call> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_0Call) -> Self {
                    (value._0, value._1)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_0Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _0: tuple.0,
                        _1: tuple.1,
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
            impl ::core::convert::From<newFinalizedState_0Return> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_0Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl newFinalizedState_0Return {
            fn _tokenize(
                &self,
            ) -> <newFinalizedState_0Call as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedState_0Call {
            type Parameters<'a> = (LightClient::LightClientState, IPlonkVerifier::PlonkProof);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_0Return;
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
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(&self._0),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(&self._1),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                newFinalizedState_0Return::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize)]
    /**Function with signature `newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0x757c37ad`.
    ```solidity
    function newFinalizedState(LightClient.LightClientState memory, LightClient.StakeTableState memory, IPlonkVerifier.PlonkProof memory) external pure;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_1Call {
        #[allow(missing_docs)]
        pub _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _1: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub _2: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](newFinalizedState_1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_1Return {}
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
                LightClient::LightClientState,
                LightClient::StakeTableState,
                IPlonkVerifier::PlonkProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
                <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<newFinalizedState_1Call> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_1Call) -> Self {
                    (value._0, value._1, value._2)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_1Call {
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<newFinalizedState_1Return> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_1Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl newFinalizedState_1Return {
            fn _tokenize(
                &self,
            ) -> <newFinalizedState_1Call as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedState_1Call {
            type Parameters<'a> = (
                LightClient::LightClientState,
                LightClient::StakeTableState,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_1Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [117u8, 124u8, 55u8, 173u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(&self._0),
                    <LightClient::StakeTableState as alloy_sol_types::SolType>::tokenize(&self._1),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(&self._2),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                newFinalizedState_1Return::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize)]
    /**Function with signature `newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint256,((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0xaabd5db3`.
    ```solidity
    function newFinalizedState(LightClient.LightClientState memory newState, LightClient.StakeTableState memory nextStakeTable, uint256 newAuthRoot, IPlonkVerifier.PlonkProof memory proof) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_2Call {
        #[allow(missing_docs)]
        pub newState: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub nextStakeTable: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub newAuthRoot: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint256,((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](newFinalizedState_2Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newFinalizedState_2Return {}
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
                LightClient::LightClientState,
                LightClient::StakeTableState,
                alloy::sol_types::sol_data::Uint<256>,
                IPlonkVerifier::PlonkProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
                <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
                alloy::sol_types::private::primitives::aliases::U256,
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
            impl ::core::convert::From<newFinalizedState_2Call> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_2Call) -> Self {
                    (
                        value.newState,
                        value.nextStakeTable,
                        value.newAuthRoot,
                        value.proof,
                    )
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_2Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newState: tuple.0,
                        nextStakeTable: tuple.1,
                        newAuthRoot: tuple.2,
                        proof: tuple.3,
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
            impl ::core::convert::From<newFinalizedState_2Return> for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_2Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newFinalizedState_2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl newFinalizedState_2Return {
            fn _tokenize(
                &self,
            ) -> <newFinalizedState_2Call as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedState_2Call {
            type Parameters<'a> = (
                LightClient::LightClientState,
                LightClient::StakeTableState,
                alloy::sol_types::sol_data::Uint<256>,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "newFinalizedState((uint64,uint64,uint256),(uint256,uint256,uint256,uint256),uint256,((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [170u8, 189u8, 93u8, 179u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(
                        &self.newState,
                    ),
                    <LightClient::StakeTableState as alloy_sol_types::SolType>::tokenize(
                        &self.nextStakeTable,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.newAuthRoot,
                    ),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(&self.proof),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                newFinalizedState_2Return::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `owner()` and selector `0x8da5cb5b`.
    ```solidity
    function owner() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ownerCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::Address;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: ownerReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: ownerReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `permissionedProver()` and selector `0x313df7b1`.
    ```solidity
    function permissionedProver() external view returns (address);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct permissionedProverCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::Address;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: permissionedProverReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: permissionedProverReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `proxiableUUID()` and selector `0x52d1902d`.
    ```solidity
    function proxiableUUID() external view returns (bytes32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct proxiableUUIDCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = alloy::sol_types::private::FixedBytes<32>;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::FixedBytes<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(ret),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: proxiableUUIDReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: proxiableUUIDReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `renounceOwnership()` and selector `0x715018a6`.
    ```solidity
    function renounceOwnership() external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct renounceOwnershipCall;
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
                    Self
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
        impl renounceOwnershipReturn {
            fn _tokenize(
                &self,
            ) -> <renounceOwnershipCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                renounceOwnershipReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        impl setPermissionedProverReturn {
            fn _tokenize(
                &self,
            ) -> <setPermissionedProverCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                setPermissionedProverReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `setStateHistoryRetentionPeriod(uint32)` and selector `0x433dba9f`.
    ```solidity
    function setStateHistoryRetentionPeriod(uint32 historySeconds) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setStateHistoryRetentionPeriodCall {
        #[allow(missing_docs)]
        pub historySeconds: u32,
    }
    ///Container type for the return parameters of the [`setStateHistoryRetentionPeriod(uint32)`](setStateHistoryRetentionPeriodCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setStateHistoryRetentionPeriodReturn {}
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
            impl ::core::convert::From<setStateHistoryRetentionPeriodCall> for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryRetentionPeriodCall) -> Self {
                    (value.historySeconds,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setStateHistoryRetentionPeriodCall {
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
            impl ::core::convert::From<setStateHistoryRetentionPeriodReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryRetentionPeriodReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setStateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl setStateHistoryRetentionPeriodReturn {
            fn _tokenize(
                &self,
            ) -> <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setStateHistoryRetentionPeriodCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = setStateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setStateHistoryRetentionPeriod(uint32)";
            const SELECTOR: [u8; 4] = [67u8, 61u8, 186u8, 159u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                setStateHistoryRetentionPeriodReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        impl setstateHistoryRetentionPeriodReturn {
            fn _tokenize(
                &self,
            ) -> <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                setstateHistoryRetentionPeriodReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `stateHistoryCommitments(uint256)` and selector `0x02b592f3`.
    ```solidity
    function stateHistoryCommitments(uint256) external view returns (uint64 l1BlockHeight, uint64 l1BlockTimestamp, uint64 hotShotBlockHeight, BN254.ScalarField hotShotBlockCommRoot);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryCommitmentsCall(
        pub alloy::sol_types::private::primitives::aliases::U256,
    );
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    (value.0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for stateHistoryCommitmentsCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self(tuple.0)
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
        impl stateHistoryCommitmentsReturn {
            fn _tokenize(
                &self,
            ) -> <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.l1BlockHeight,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.l1BlockTimestamp,
                    ),
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.hotShotBlockHeight,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.hotShotBlockCommRoot,
                    ),
                )
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
                        &self.0,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                stateHistoryCommitmentsReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `stateHistoryFirstIndex()` and selector `0x2f79889d`.
    ```solidity
    function stateHistoryFirstIndex() external view returns (uint64);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryFirstIndexCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = u64;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: stateHistoryFirstIndexReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: stateHistoryFirstIndexReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `stateHistoryRetentionPeriod()` and selector `0xc23b9e9e`.
    ```solidity
    function stateHistoryRetentionPeriod() external view returns (uint32);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryRetentionPeriodCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
                    Self
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
            type Return = u32;
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<32> as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data).map(
                    |r| {
                        let r: stateHistoryRetentionPeriodReturn = r.into();
                        r._0
                    },
                )
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(|r| {
                    let r: stateHistoryRetentionPeriodReturn = r.into();
                    r._0
                })
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        impl transferOwnershipReturn {
            fn _tokenize(
                &self,
            ) -> <transferOwnershipCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                transferOwnershipReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `updateEpochStartBlock(uint64)` and selector `0x167ac618`.
    ```solidity
    function updateEpochStartBlock(uint64 newEpochStartBlock) external;
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateEpochStartBlockCall {
        #[allow(missing_docs)]
        pub newEpochStartBlock: u64,
    }
    ///Container type for the return parameters of the [`updateEpochStartBlock(uint64)`](updateEpochStartBlockCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct updateEpochStartBlockReturn {}
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
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<updateEpochStartBlockCall> for UnderlyingRustTuple<'_> {
                fn from(value: updateEpochStartBlockCall) -> Self {
                    (value.newEpochStartBlock,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateEpochStartBlockCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        newEpochStartBlock: tuple.0,
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
            impl ::core::convert::From<updateEpochStartBlockReturn> for UnderlyingRustTuple<'_> {
                fn from(value: updateEpochStartBlockReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for updateEpochStartBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        impl updateEpochStartBlockReturn {
            fn _tokenize(
                &self,
            ) -> <updateEpochStartBlockCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
                ()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateEpochStartBlockCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateEpochStartBlockReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "updateEpochStartBlock(uint64)";
            const SELECTOR: [u8; 4] = [22u8, 122u8, 198u8, 24u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<64> as alloy_sol_types::SolType>::tokenize(
                        &self.newEpochStartBlock,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                updateEpochStartBlockReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
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
        impl upgradeToAndCallReturn {
            fn _tokenize(
                &self,
            ) -> <upgradeToAndCallCall as alloy_sol_types::SolCall>::ReturnToken<'_> {
                ()
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                upgradeToAndCallReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `votingStakeTableState()` and selector `0x0625e19b`.
    ```solidity
    function votingStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct votingStakeTableStateCall;
    #[derive(serde::Serialize, serde::Deserialize, Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`votingStakeTableState()`](votingStakeTableStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct votingStakeTableStateReturn {
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
            impl ::core::convert::From<votingStakeTableStateCall> for UnderlyingRustTuple<'_> {
                fn from(value: votingStakeTableStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for votingStakeTableStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self
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
            impl ::core::convert::From<votingStakeTableStateReturn> for UnderlyingRustTuple<'_> {
                fn from(value: votingStakeTableStateReturn) -> Self {
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
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for votingStakeTableStateReturn {
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
        impl votingStakeTableStateReturn {
            fn _tokenize(
                &self,
            ) -> <votingStakeTableStateCall as alloy_sol_types::SolCall>::ReturnToken<'_>
            {
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
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for votingStakeTableStateCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = votingStakeTableStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                BN254::ScalarField,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "votingStakeTableState()";
            const SELECTOR: [u8; 4] = [6u8, 37u8, 225u8, 155u8];
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
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                votingStakeTableStateReturn::_tokenize(ret)
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(Into::into)
            }
            #[inline]
            fn abi_decode_returns_validate(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<'_> as alloy_sol_types::SolType>::abi_decode_sequence_validate(
                    data,
                )
                .map(Into::into)
            }
        }
    };
    ///Container for all the [`LightClientV3`](self) function calls.
    #[derive(serde::Serialize, serde::Deserialize)]
    pub enum LightClientV3Calls {
        #[allow(missing_docs)]
        UPGRADE_INTERFACE_VERSION(UPGRADE_INTERFACE_VERSIONCall),
        #[allow(missing_docs)]
        _getVk(_getVkCall),
        #[allow(missing_docs)]
        authRoot(authRootCall),
        #[allow(missing_docs)]
        blocksPerEpoch(blocksPerEpochCall),
        #[allow(missing_docs)]
        currentBlockNumber(currentBlockNumberCall),
        #[allow(missing_docs)]
        currentEpoch(currentEpochCall),
        #[allow(missing_docs)]
        disablePermissionedProverMode(disablePermissionedProverModeCall),
        #[allow(missing_docs)]
        epochFromBlockNumber(epochFromBlockNumberCall),
        #[allow(missing_docs)]
        epochStartBlock(epochStartBlockCall),
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
        initializeV2(initializeV2Call),
        #[allow(missing_docs)]
        initializeV3(initializeV3Call),
        #[allow(missing_docs)]
        isEpochRoot(isEpochRootCall),
        #[allow(missing_docs)]
        isGtEpochRoot(isGtEpochRootCall),
        #[allow(missing_docs)]
        isPermissionedProverEnabled(isPermissionedProverEnabledCall),
        #[allow(missing_docs)]
        lagOverEscapeHatchThreshold(lagOverEscapeHatchThresholdCall),
        #[allow(missing_docs)]
        newFinalizedState_0(newFinalizedState_0Call),
        #[allow(missing_docs)]
        newFinalizedState_1(newFinalizedState_1Call),
        #[allow(missing_docs)]
        newFinalizedState_2(newFinalizedState_2Call),
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
        setStateHistoryRetentionPeriod(setStateHistoryRetentionPeriodCall),
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
        updateEpochStartBlock(updateEpochStartBlockCall),
        #[allow(missing_docs)]
        upgradeToAndCall(upgradeToAndCallCall),
        #[allow(missing_docs)]
        votingStakeTableState(votingStakeTableStateCall),
    }
    #[automatically_derived]
    impl LightClientV3Calls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [1u8, 63u8, 165u8, 252u8],
            [2u8, 181u8, 146u8, 243u8],
            [6u8, 37u8, 225u8, 155u8],
            [13u8, 142u8, 110u8, 44u8],
            [18u8, 23u8, 60u8, 44u8],
            [22u8, 122u8, 198u8, 24u8],
            [32u8, 99u8, 212u8, 247u8],
            [37u8, 41u8, 116u8, 39u8],
            [47u8, 121u8, 136u8, 157u8],
            [48u8, 12u8, 137u8, 221u8],
            [49u8, 61u8, 247u8, 177u8],
            [55u8, 142u8, 194u8, 59u8],
            [56u8, 228u8, 84u8, 177u8],
            [62u8, 213u8, 91u8, 123u8],
            [66u8, 109u8, 49u8, 148u8],
            [67u8, 61u8, 186u8, 159u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [105u8, 204u8, 106u8, 4u8],
            [113u8, 80u8, 24u8, 166u8],
            [117u8, 124u8, 55u8, 173u8],
            [118u8, 103u8, 24u8, 8u8],
            [130u8, 110u8, 65u8, 252u8],
            [133u8, 132u8, 210u8, 63u8],
            [141u8, 165u8, 203u8, 91u8],
            [144u8, 193u8, 67u8, 144u8],
            [150u8, 193u8, 202u8, 97u8],
            [153u8, 131u8, 40u8, 232u8],
            [155u8, 170u8, 60u8, 201u8],
            [159u8, 219u8, 84u8, 167u8],
            [170u8, 189u8, 93u8, 179u8],
            [173u8, 60u8, 177u8, 204u8],
            [179u8, 59u8, 196u8, 145u8],
            [194u8, 59u8, 158u8, 158u8],
            [210u8, 77u8, 147u8, 61u8],
            [224u8, 48u8, 51u8, 1u8],
            [240u8, 104u8, 32u8, 84u8],
            [242u8, 253u8, 227u8, 139u8],
            [249u8, 229u8, 13u8, 25u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for LightClientV3Calls {
        const NAME: &'static str = "LightClientV3Calls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 39usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::_getVk(_) => <_getVkCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::authRoot(_) => <authRootCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::blocksPerEpoch(_) => {
                    <blocksPerEpochCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::currentBlockNumber(_) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::currentEpoch(_) => <currentEpochCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::disablePermissionedProverMode(_) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::epochFromBlockNumber(_) => {
                    <epochFromBlockNumberCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::epochStartBlock(_) => {
                    <epochStartBlockCall as alloy_sol_types::SolCall>::SELECTOR
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
                Self::initializeV2(_) => <initializeV2Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::initializeV3(_) => <initializeV3Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::isEpochRoot(_) => <isEpochRootCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::isGtEpochRoot(_) => <isGtEpochRootCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::isPermissionedProverEnabled(_) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::lagOverEscapeHatchThreshold(_) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::newFinalizedState_0(_) => {
                    <newFinalizedState_0Call as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::newFinalizedState_1(_) => {
                    <newFinalizedState_1Call as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::newFinalizedState_2(_) => {
                    <newFinalizedState_2Call as alloy_sol_types::SolCall>::SELECTOR
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
                Self::setStateHistoryRetentionPeriod(_) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
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
                Self::updateEpochStartBlock(_) => {
                    <updateEpochStartBlockCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::upgradeToAndCall(_) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::votingStakeTableState(_) => {
                    <votingStakeTableStateCall as alloy_sol_types::SolCall>::SELECTOR
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
        fn abi_decode_raw(selector: [u8; 4], data: &[u8]) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(&[u8]) -> alloy_sol_types::Result<LightClientV3Calls>] = &[
                {
                    fn setPermissionedProver(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::setPermissionedProver)
                    }
                    setPermissionedProver
                },
                {
                    fn stateHistoryCommitments(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::stateHistoryCommitments)
                    }
                    stateHistoryCommitments
                },
                {
                    fn votingStakeTableState(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::votingStakeTableState)
                    }
                    votingStakeTableState
                },
                {
                    fn getVersion(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::getVersion)
                    }
                    getVersion
                },
                {
                    fn _getVk(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <_getVkCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::_getVk)
                    }
                    _getVk
                },
                {
                    fn updateEpochStartBlock(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::updateEpochStartBlock)
                    }
                    updateEpochStartBlock
                },
                {
                    fn newFinalizedState_0(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::newFinalizedState_0)
                    }
                    newFinalizedState_0
                },
                {
                    fn isEpochRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::isEpochRoot)
                    }
                    isEpochRoot
                },
                {
                    fn stateHistoryFirstIndex(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::stateHistoryFirstIndex)
                    }
                    stateHistoryFirstIndex
                },
                {
                    fn isGtEpochRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::isGtEpochRoot)
                    }
                    isGtEpochRoot
                },
                {
                    fn permissionedProver(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <permissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::permissionedProver)
                    }
                    permissionedProver
                },
                {
                    fn currentBlockNumber(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::currentBlockNumber)
                    }
                    currentBlockNumber
                },
                {
                    fn initializeV3(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeV3Call as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::initializeV3)
                    }
                    initializeV3
                },
                {
                    fn epochStartBlock(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <epochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::epochStartBlock)
                    }
                    epochStartBlock
                },
                {
                    fn genesisStakeTableState(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::genesisStakeTableState)
                    }
                    genesisStakeTableState
                },
                {
                    fn setStateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::setStateHistoryRetentionPeriod)
                    }
                    setStateHistoryRetentionPeriod
                },
                {
                    fn upgradeToAndCall(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::upgradeToAndCall)
                    }
                    upgradeToAndCall
                },
                {
                    fn proxiableUUID(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::proxiableUUID)
                    }
                    proxiableUUID
                },
                {
                    fn disablePermissionedProverMode(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::disablePermissionedProverMode)
                    }
                    disablePermissionedProverMode
                },
                {
                    fn renounceOwnership(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::renounceOwnership)
                    }
                    renounceOwnership
                },
                {
                    fn newFinalizedState_1(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::newFinalizedState_1)
                    }
                    newFinalizedState_1
                },
                {
                    fn currentEpoch(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <currentEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::currentEpoch)
                    }
                    currentEpoch
                },
                {
                    fn isPermissionedProverEnabled(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::isPermissionedProverEnabled)
                    }
                    isPermissionedProverEnabled
                },
                {
                    fn getHotShotCommitment(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::getHotShotCommitment)
                    }
                    getHotShotCommitment
                },
                {
                    fn owner(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::owner)
                    }
                    owner
                },
                {
                    fn epochFromBlockNumber(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::epochFromBlockNumber)
                    }
                    epochFromBlockNumber
                },
                {
                    fn setstateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::setstateHistoryRetentionPeriod)
                    }
                    setstateHistoryRetentionPeriod
                },
                {
                    fn authRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <authRootCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::authRoot)
                    }
                    authRoot
                },
                {
                    fn initialize(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::initialize)
                    }
                    initialize
                },
                {
                    fn finalizedState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <finalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::finalizedState)
                    }
                    finalizedState
                },
                {
                    fn newFinalizedState_2(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::newFinalizedState_2)
                    }
                    newFinalizedState_2
                },
                {
                    fn UPGRADE_INTERFACE_VERSION(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Calls::UPGRADE_INTERFACE_VERSION)
                    }
                    UPGRADE_INTERFACE_VERSION
                },
                {
                    fn initializeV2(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeV2Call as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::initializeV2)
                    }
                    initializeV2
                },
                {
                    fn stateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::stateHistoryRetentionPeriod)
                    }
                    stateHistoryRetentionPeriod
                },
                {
                    fn genesisState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <genesisStateCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::genesisState)
                    }
                    genesisState
                },
                {
                    fn lagOverEscapeHatchThreshold(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(LightClientV3Calls::lagOverEscapeHatchThreshold)
                    }
                    lagOverEscapeHatchThreshold
                },
                {
                    fn blocksPerEpoch(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::blocksPerEpoch)
                    }
                    blocksPerEpoch
                },
                {
                    fn transferOwnership(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::transferOwnership)
                    }
                    transferOwnership
                },
                {
                    fn getStateHistoryCount(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_decode_raw(data)
                            .map(LightClientV3Calls::getStateHistoryCount)
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
            DECODE_SHIMS[idx](data)
        }
        #[inline]
        #[allow(non_snake_case)]
        fn abi_decode_raw_validate(
            selector: [u8; 4],
            data: &[u8],
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_VALIDATE_SHIMS: &[fn(
                &[u8],
            )
                -> alloy_sol_types::Result<LightClientV3Calls>] = &[
                {
                    fn setPermissionedProver(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::setPermissionedProver)
                    }
                    setPermissionedProver
                },
                {
                    fn stateHistoryCommitments(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::stateHistoryCommitments)
                    }
                    stateHistoryCommitments
                },
                {
                    fn votingStakeTableState(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::votingStakeTableState)
                    }
                    votingStakeTableState
                },
                {
                    fn getVersion(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::getVersion)
                    }
                    getVersion
                },
                {
                    fn _getVk(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <_getVkCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::_getVk)
                    }
                    _getVk
                },
                {
                    fn updateEpochStartBlock(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::updateEpochStartBlock)
                    }
                    updateEpochStartBlock
                },
                {
                    fn newFinalizedState_0(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::newFinalizedState_0)
                    }
                    newFinalizedState_0
                },
                {
                    fn isEpochRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::isEpochRoot)
                    }
                    isEpochRoot
                },
                {
                    fn stateHistoryFirstIndex(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::stateHistoryFirstIndex)
                    }
                    stateHistoryFirstIndex
                },
                {
                    fn isGtEpochRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::isGtEpochRoot)
                    }
                    isGtEpochRoot
                },
                {
                    fn permissionedProver(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <permissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::permissionedProver)
                    }
                    permissionedProver
                },
                {
                    fn currentBlockNumber(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::currentBlockNumber)
                    }
                    currentBlockNumber
                },
                {
                    fn initializeV3(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeV3Call as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::initializeV3)
                    }
                    initializeV3
                },
                {
                    fn epochStartBlock(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <epochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::epochStartBlock)
                    }
                    epochStartBlock
                },
                {
                    fn genesisStakeTableState(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::genesisStakeTableState)
                    }
                    genesisStakeTableState
                },
                {
                    fn setStateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::setStateHistoryRetentionPeriod)
                    }
                    setStateHistoryRetentionPeriod
                },
                {
                    fn upgradeToAndCall(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::upgradeToAndCall)
                    }
                    upgradeToAndCall
                },
                {
                    fn proxiableUUID(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::proxiableUUID)
                    }
                    proxiableUUID
                },
                {
                    fn disablePermissionedProverMode(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::disablePermissionedProverMode)
                    }
                    disablePermissionedProverMode
                },
                {
                    fn renounceOwnership(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::renounceOwnership)
                    }
                    renounceOwnership
                },
                {
                    fn newFinalizedState_1(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::newFinalizedState_1)
                    }
                    newFinalizedState_1
                },
                {
                    fn currentEpoch(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <currentEpochCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::currentEpoch)
                    }
                    currentEpoch
                },
                {
                    fn isPermissionedProverEnabled(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::isPermissionedProverEnabled)
                    }
                    isPermissionedProverEnabled
                },
                {
                    fn getHotShotCommitment(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::getHotShotCommitment)
                    }
                    getHotShotCommitment
                },
                {
                    fn owner(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::owner)
                    }
                    owner
                },
                {
                    fn epochFromBlockNumber(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::epochFromBlockNumber)
                    }
                    epochFromBlockNumber
                },
                {
                    fn setstateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::setstateHistoryRetentionPeriod)
                    }
                    setstateHistoryRetentionPeriod
                },
                {
                    fn authRoot(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <authRootCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::authRoot)
                    }
                    authRoot
                },
                {
                    fn initialize(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(data)
                            .map(LightClientV3Calls::initialize)
                    }
                    initialize
                },
                {
                    fn finalizedState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <finalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::finalizedState)
                    }
                    finalizedState
                },
                {
                    fn newFinalizedState_2(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::newFinalizedState_2)
                    }
                    newFinalizedState_2
                },
                {
                    fn UPGRADE_INTERFACE_VERSION(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::UPGRADE_INTERFACE_VERSION)
                    }
                    UPGRADE_INTERFACE_VERSION
                },
                {
                    fn initializeV2(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <initializeV2Call as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::initializeV2)
                    }
                    initializeV2
                },
                {
                    fn stateHistoryRetentionPeriod(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::stateHistoryRetentionPeriod)
                    }
                    stateHistoryRetentionPeriod
                },
                {
                    fn genesisState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <genesisStateCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::genesisState)
                    }
                    genesisState
                },
                {
                    fn lagOverEscapeHatchThreshold(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::lagOverEscapeHatchThreshold)
                    }
                    lagOverEscapeHatchThreshold
                },
                {
                    fn blocksPerEpoch(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Calls::blocksPerEpoch)
                    }
                    blocksPerEpoch
                },
                {
                    fn transferOwnership(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::transferOwnership)
                    }
                    transferOwnership
                },
                {
                    fn getStateHistoryCount(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Calls> {
                        <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Calls::getStateHistoryCount)
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
            DECODE_VALIDATE_SHIMS[idx](data)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::UPGRADE_INTERFACE_VERSION(inner) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::_getVk(inner) => {
                    <_getVkCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::authRoot(inner) => {
                    <authRootCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                }
                Self::blocksPerEpoch(inner) => {
                    <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::currentBlockNumber(inner) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::currentEpoch(inner) => {
                    <currentEpochCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::disablePermissionedProverMode(inner) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::epochFromBlockNumber(inner) => {
                    <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::epochStartBlock(inner) => {
                    <epochStartBlockCall as alloy_sol_types::SolCall>::abi_encoded_size(
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
                Self::initializeV2(inner) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::initializeV3(inner) => {
                    <initializeV3Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::isEpochRoot(inner) => {
                    <isEpochRootCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::isGtEpochRoot(inner) => {
                    <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
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
                Self::newFinalizedState_0(inner) => {
                    <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::newFinalizedState_1(inner) => {
                    <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::newFinalizedState_2(inner) => {
                    <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_encoded_size(
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
                Self::setStateHistoryRetentionPeriod(inner) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
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
                Self::updateEpochStartBlock(inner) => {
                    <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::votingStakeTableState(inner) => {
                    <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
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
                Self::_getVk(inner) => {
                    <_getVkCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::authRoot(inner) => {
                    <authRootCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::blocksPerEpoch(inner) => {
                    <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::currentBlockNumber(inner) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::currentEpoch(inner) => {
                    <currentEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::disablePermissionedProverMode(inner) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::epochFromBlockNumber(inner) => {
                    <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::epochStartBlock(inner) => {
                    <epochStartBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
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
                Self::initializeV2(inner) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::initializeV3(inner) => {
                    <initializeV3Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isEpochRoot(inner) => {
                    <isEpochRootCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::isGtEpochRoot(inner) => {
                    <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
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
                Self::newFinalizedState_0(inner) => {
                    <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::newFinalizedState_1(inner) => {
                    <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::newFinalizedState_2(inner) => {
                    <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
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
                Self::setStateHistoryRetentionPeriod(inner) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
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
                Self::updateEpochStartBlock(inner) => {
                    <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::upgradeToAndCall(inner) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::votingStakeTableState(inner) => {
                    <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
            }
        }
    }
    ///Container for all the [`LightClientV3`](self) custom errors.
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
    pub enum LightClientV3Errors {
        #[allow(missing_docs)]
        AddressEmptyCode(AddressEmptyCode),
        #[allow(missing_docs)]
        DeprecatedApi(DeprecatedApi),
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
        InvalidScalar(InvalidScalar),
        #[allow(missing_docs)]
        MissingEpochRootUpdate(MissingEpochRootUpdate),
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
    impl LightClientV3Errors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [5u8, 28u8, 70u8, 239u8],
            [5u8, 176u8, 92u8, 204u8],
            [8u8, 10u8, 232u8, 217u8],
            [9u8, 189u8, 227u8, 57u8],
            [17u8, 140u8, 218u8, 167u8],
            [20u8, 37u8, 234u8, 66u8],
            [30u8, 79u8, 189u8, 247u8],
            [76u8, 156u8, 140u8, 227u8],
            [78u8, 64u8, 92u8, 141u8],
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
    impl alloy_sol_types::SolInterface for LightClientV3Errors {
        const NAME: &'static str = "LightClientV3Errors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 23usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AddressEmptyCode(_) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::SELECTOR
                },
                Self::DeprecatedApi(_) => <DeprecatedApi as alloy_sol_types::SolError>::SELECTOR,
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
                Self::InvalidScalar(_) => <InvalidScalar as alloy_sol_types::SolError>::SELECTOR,
                Self::MissingEpochRootUpdate(_) => {
                    <MissingEpochRootUpdate as alloy_sol_types::SolError>::SELECTOR
                },
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
        fn abi_decode_raw(selector: [u8; 4], data: &[u8]) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(&[u8]) -> alloy_sol_types::Result<LightClientV3Errors>] = &[
                {
                    fn OutdatedState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OutdatedState as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::OutdatedState)
                    }
                    OutdatedState
                },
                {
                    fn InvalidScalar(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidScalar as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidScalar)
                    }
                    InvalidScalar
                },
                {
                    fn MissingEpochRootUpdate(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <MissingEpochRootUpdate as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::MissingEpochRootUpdate)
                    }
                    MissingEpochRootUpdate
                },
                {
                    fn InvalidProof(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidProof as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidProof)
                    }
                    InvalidProof
                },
                {
                    fn OwnableUnauthorizedAccount(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Errors::OwnableUnauthorizedAccount)
                    }
                    OwnableUnauthorizedAccount
                },
                {
                    fn FailedInnerCall(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn OwnableInvalidOwner(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::OwnableInvalidOwner)
                    }
                    OwnableInvalidOwner
                },
                {
                    fn ERC1967InvalidImplementation(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Errors::ERC1967InvalidImplementation)
                    }
                    ERC1967InvalidImplementation
                },
                {
                    fn DeprecatedApi(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <DeprecatedApi as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::DeprecatedApi)
                    }
                    DeprecatedApi
                },
                {
                    fn WrongStakeTableUsed(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::WrongStakeTableUsed)
                    }
                    WrongStakeTableUsed
                },
                {
                    fn InvalidHotShotBlockForCommitmentCheck(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                            )
                            .map(
                                LightClientV3Errors::InvalidHotShotBlockForCommitmentCheck,
                            )
                    }
                    InvalidHotShotBlockForCommitmentCheck
                },
                {
                    fn AddressEmptyCode(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::AddressEmptyCode)
                    }
                    AddressEmptyCode
                },
                {
                    fn InvalidArgs(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidArgs as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidArgs)
                    }
                    InvalidArgs
                },
                {
                    fn ProverNotPermissioned(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ProverNotPermissioned as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::ProverNotPermissioned)
                    }
                    ProverNotPermissioned
                },
                {
                    fn NoChangeRequired(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <NoChangeRequired as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::NoChangeRequired)
                    }
                    NoChangeRequired
                },
                {
                    fn UUPSUnsupportedProxiableUUID(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Errors::UUPSUnsupportedProxiableUUID)
                    }
                    UUPSUnsupportedProxiableUUID
                },
                {
                    fn InsufficientSnapshotHistory(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Errors::InsufficientSnapshotHistory)
                    }
                    InsufficientSnapshotHistory
                },
                {
                    fn ERC1967NonPayable(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::ERC1967NonPayable)
                    }
                    ERC1967NonPayable
                },
                {
                    fn NotInitializing(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::NotInitializing)
                    }
                    NotInitializing
                },
                {
                    fn UUPSUnauthorizedCallContext(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw(
                            data,
                        )
                        .map(LightClientV3Errors::UUPSUnauthorizedCallContext)
                    }
                    UUPSUnauthorizedCallContext
                },
                {
                    fn InvalidAddress(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidAddress as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidAddress)
                    }
                    InvalidAddress
                },
                {
                    fn InvalidMaxStateHistory(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidMaxStateHistory)
                    }
                    InvalidMaxStateHistory
                },
                {
                    fn InvalidInitialization(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw(data)
                            .map(LightClientV3Errors::InvalidInitialization)
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
            DECODE_SHIMS[idx](data)
        }
        #[inline]
        #[allow(non_snake_case)]
        fn abi_decode_raw_validate(
            selector: [u8; 4],
            data: &[u8],
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_VALIDATE_SHIMS: &[fn(
                &[u8],
            )
                -> alloy_sol_types::Result<LightClientV3Errors>] = &[
                {
                    fn OutdatedState(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OutdatedState as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::OutdatedState)
                    }
                    OutdatedState
                },
                {
                    fn InvalidScalar(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidScalar as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::InvalidScalar)
                    }
                    InvalidScalar
                },
                {
                    fn MissingEpochRootUpdate(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <MissingEpochRootUpdate as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::MissingEpochRootUpdate)
                    }
                    MissingEpochRootUpdate
                },
                {
                    fn InvalidProof(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidProof as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::InvalidProof)
                    }
                    InvalidProof
                },
                {
                    fn OwnableUnauthorizedAccount(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::OwnableUnauthorizedAccount)
                    }
                    OwnableUnauthorizedAccount
                },
                {
                    fn FailedInnerCall(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn OwnableInvalidOwner(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::OwnableInvalidOwner)
                    }
                    OwnableInvalidOwner
                },
                {
                    fn ERC1967InvalidImplementation(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::ERC1967InvalidImplementation)
                    }
                    ERC1967InvalidImplementation
                },
                {
                    fn DeprecatedApi(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <DeprecatedApi as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::DeprecatedApi)
                    }
                    DeprecatedApi
                },
                {
                    fn WrongStakeTableUsed(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::WrongStakeTableUsed)
                    }
                    WrongStakeTableUsed
                },
                {
                    fn InvalidHotShotBlockForCommitmentCheck(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(
                                LightClientV3Errors::InvalidHotShotBlockForCommitmentCheck,
                            )
                    }
                    InvalidHotShotBlockForCommitmentCheck
                },
                {
                    fn AddressEmptyCode(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::AddressEmptyCode)
                    }
                    AddressEmptyCode
                },
                {
                    fn InvalidArgs(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidArgs as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::InvalidArgs)
                    }
                    InvalidArgs
                },
                {
                    fn ProverNotPermissioned(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ProverNotPermissioned as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::ProverNotPermissioned)
                    }
                    ProverNotPermissioned
                },
                {
                    fn NoChangeRequired(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <NoChangeRequired as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::NoChangeRequired)
                    }
                    NoChangeRequired
                },
                {
                    fn UUPSUnsupportedProxiableUUID(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::UUPSUnsupportedProxiableUUID)
                    }
                    UUPSUnsupportedProxiableUUID
                },
                {
                    fn InsufficientSnapshotHistory(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::InsufficientSnapshotHistory)
                    }
                    InsufficientSnapshotHistory
                },
                {
                    fn ERC1967NonPayable(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::ERC1967NonPayable)
                    }
                    ERC1967NonPayable
                },
                {
                    fn NotInitializing(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw_validate(
                            data,
                        )
                        .map(LightClientV3Errors::NotInitializing)
                    }
                    NotInitializing
                },
                {
                    fn UUPSUnauthorizedCallContext(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::UUPSUnauthorizedCallContext)
                    }
                    UUPSUnauthorizedCallContext
                },
                {
                    fn InvalidAddress(data: &[u8]) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidAddress as alloy_sol_types::SolError>::abi_decode_raw_validate(data)
                            .map(LightClientV3Errors::InvalidAddress)
                    }
                    InvalidAddress
                },
                {
                    fn InvalidMaxStateHistory(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::InvalidMaxStateHistory)
                    }
                    InvalidMaxStateHistory
                },
                {
                    fn InvalidInitialization(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<LightClientV3Errors> {
                        <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(LightClientV3Errors::InvalidInitialization)
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
            DECODE_VALIDATE_SHIMS[idx](data)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::AddressEmptyCode(inner) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
                Self::DeprecatedApi(inner) => {
                    <DeprecatedApi as alloy_sol_types::SolError>::abi_encoded_size(inner)
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
                Self::InvalidScalar(inner) => {
                    <InvalidScalar as alloy_sol_types::SolError>::abi_encoded_size(inner)
                }
                Self::MissingEpochRootUpdate(inner) => {
                    <MissingEpochRootUpdate as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
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
                Self::DeprecatedApi(inner) => {
                    <DeprecatedApi as alloy_sol_types::SolError>::abi_encode_raw(
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
                Self::InvalidScalar(inner) => {
                    <InvalidScalar as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::MissingEpochRootUpdate(inner) => {
                    <MissingEpochRootUpdate as alloy_sol_types::SolError>::abi_encode_raw(
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
    ///Container for all the [`LightClientV3`](self) events.
    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash)]
    pub enum LightClientV3Events {
        #[allow(missing_docs)]
        Initialized(Initialized),
        #[allow(missing_docs)]
        NewEpoch(NewEpoch),
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
    impl LightClientV3Events {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                49u8, 234u8, 189u8, 144u8, 153u8, 253u8, 178u8, 93u8, 172u8, 221u8, 210u8, 6u8,
                171u8, 255u8, 135u8, 49u8, 30u8, 85u8, 52u8, 65u8, 252u8, 157u8, 15u8, 205u8,
                239u8, 32u8, 16u8, 98u8, 215u8, 231u8, 7u8, 27u8,
            ],
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
    impl alloy_sol_types::SolEventInterface for LightClientV3Events {
        const NAME: &'static str = "LightClientV3Events";
        const COUNT: usize = 8usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(topics, data)
                        .map(Self::Initialized)
                },
                Some(<NewEpoch as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <NewEpoch as alloy_sol_types::SolEvent>::decode_raw_log(topics, data)
                        .map(Self::NewEpoch)
                },
                Some(<NewState as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <NewState as alloy_sol_types::SolEvent>::decode_raw_log(topics, data)
                        .map(Self::NewState)
                },
                Some(<OwnershipTransferred as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <OwnershipTransferred as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data,
                    )
                    .map(Self::OwnershipTransferred)
                },
                Some(
                    <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                    topics, data,
                )
                .map(Self::PermissionedProverNotRequired),
                Some(<PermissionedProverRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <PermissionedProverRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                        topics, data,
                    )
                    .map(Self::PermissionedProverRequired)
                },
                Some(<Upgrade as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgrade as alloy_sol_types::SolEvent>::decode_raw_log(topics, data)
                        .map(Self::Upgrade)
                },
                Some(<Upgraded as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Upgraded as alloy_sol_types::SolEvent>::decode_raw_log(topics, data)
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
    impl alloy_sol_types::private::IntoLogData for LightClientV3Events {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                },
                Self::NewEpoch(inner) => alloy_sol_types::private::IntoLogData::to_log_data(inner),
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
                Self::NewEpoch(inner) => {
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
    /**Creates a new wrapper around an on-chain [`LightClientV3`](self) contract instance.

    See the [wrapper's documentation](`LightClientV3Instance`) for more details.*/
    #[inline]
    pub const fn new<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> LightClientV3Instance<P, N> {
        LightClientV3Instance::<P, N>::new(address, provider)
    }
    /**Deploys this contract using the given `provider` and constructor arguments, if any.

    Returns a new instance of the contract, if the deployment was successful.

    For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
    #[inline]
    pub fn deploy<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>(
        provider: P,
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<LightClientV3Instance<P, N>>>
    {
        LightClientV3Instance::<P, N>::deploy(provider)
    }
    /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
    and constructor arguments, if any.

    This is a simple wrapper around creating a `RawCallBuilder` with the data set to
    the bytecode concatenated with the constructor's ABI-encoded arguments.*/
    #[inline]
    pub fn deploy_builder<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        provider: P,
    ) -> alloy_contract::RawCallBuilder<P, N> {
        LightClientV3Instance::<P, N>::deploy_builder(provider)
    }
    /**A [`LightClientV3`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`LightClientV3`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct LightClientV3Instance<P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for LightClientV3Instance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("LightClientV3Instance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientV3Instance<P, N>
    {
        /**Creates a new wrapper around an on-chain [`LightClientV3`](self) contract instance.

        See the [wrapper's documentation](`LightClientV3Instance`) for more details.*/
        #[inline]
        pub const fn new(address: alloy_sol_types::private::Address, provider: P) -> Self {
            Self {
                address,
                provider,
                _network: ::core::marker::PhantomData,
            }
        }
        /**Deploys this contract using the given `provider` and constructor arguments, if any.

        Returns a new instance of the contract, if the deployment was successful.

        For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
        #[inline]
        pub async fn deploy(provider: P) -> alloy_contract::Result<LightClientV3Instance<P, N>> {
            let call_builder = Self::deploy_builder(provider);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
        and constructor arguments, if any.

        This is a simple wrapper around creating a `RawCallBuilder` with the data set to
        the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(provider: P) -> alloy_contract::RawCallBuilder<P, N> {
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
    impl<P: ::core::clone::Clone, N> LightClientV3Instance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> LightClientV3Instance<P, N> {
            LightClientV3Instance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientV3Instance<P, N>
    {
        /// Creates a new call builder using this contract instance's provider and address.
        ///
        /// Note that the call can be any function call, not just those defined in this
        /// contract. Prefer using the other methods for building type-safe contract calls.
        pub fn call_builder<C: alloy_sol_types::SolCall>(
            &self,
            call: &C,
        ) -> alloy_contract::SolCallBuilder<&P, C, N> {
            alloy_contract::SolCallBuilder::new_sol(&self.provider, &self.address, call)
        }
        ///Creates a new call builder for the [`UPGRADE_INTERFACE_VERSION`] function.
        pub fn UPGRADE_INTERFACE_VERSION(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, UPGRADE_INTERFACE_VERSIONCall, N> {
            self.call_builder(&UPGRADE_INTERFACE_VERSIONCall)
        }
        ///Creates a new call builder for the [`_getVk`] function.
        pub fn _getVk(&self) -> alloy_contract::SolCallBuilder<&P, _getVkCall, N> {
            self.call_builder(&_getVkCall)
        }
        ///Creates a new call builder for the [`authRoot`] function.
        pub fn authRoot(&self) -> alloy_contract::SolCallBuilder<&P, authRootCall, N> {
            self.call_builder(&authRootCall)
        }
        ///Creates a new call builder for the [`blocksPerEpoch`] function.
        pub fn blocksPerEpoch(&self) -> alloy_contract::SolCallBuilder<&P, blocksPerEpochCall, N> {
            self.call_builder(&blocksPerEpochCall)
        }
        ///Creates a new call builder for the [`currentBlockNumber`] function.
        pub fn currentBlockNumber(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, currentBlockNumberCall, N> {
            self.call_builder(&currentBlockNumberCall)
        }
        ///Creates a new call builder for the [`currentEpoch`] function.
        pub fn currentEpoch(&self) -> alloy_contract::SolCallBuilder<&P, currentEpochCall, N> {
            self.call_builder(&currentEpochCall)
        }
        ///Creates a new call builder for the [`disablePermissionedProverMode`] function.
        pub fn disablePermissionedProverMode(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, disablePermissionedProverModeCall, N> {
            self.call_builder(&disablePermissionedProverModeCall)
        }
        ///Creates a new call builder for the [`epochFromBlockNumber`] function.
        pub fn epochFromBlockNumber(
            &self,
            _blockNum: u64,
            _blocksPerEpoch: u64,
        ) -> alloy_contract::SolCallBuilder<&P, epochFromBlockNumberCall, N> {
            self.call_builder(&epochFromBlockNumberCall {
                _blockNum,
                _blocksPerEpoch,
            })
        }
        ///Creates a new call builder for the [`epochStartBlock`] function.
        pub fn epochStartBlock(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, epochStartBlockCall, N> {
            self.call_builder(&epochStartBlockCall)
        }
        ///Creates a new call builder for the [`finalizedState`] function.
        pub fn finalizedState(&self) -> alloy_contract::SolCallBuilder<&P, finalizedStateCall, N> {
            self.call_builder(&finalizedStateCall)
        }
        ///Creates a new call builder for the [`genesisStakeTableState`] function.
        pub fn genesisStakeTableState(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, genesisStakeTableStateCall, N> {
            self.call_builder(&genesisStakeTableStateCall)
        }
        ///Creates a new call builder for the [`genesisState`] function.
        pub fn genesisState(&self) -> alloy_contract::SolCallBuilder<&P, genesisStateCall, N> {
            self.call_builder(&genesisStateCall)
        }
        ///Creates a new call builder for the [`getHotShotCommitment`] function.
        pub fn getHotShotCommitment(
            &self,
            hotShotBlockHeight: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<&P, getHotShotCommitmentCall, N> {
            self.call_builder(&getHotShotCommitmentCall { hotShotBlockHeight })
        }
        ///Creates a new call builder for the [`getStateHistoryCount`] function.
        pub fn getStateHistoryCount(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, getStateHistoryCountCall, N> {
            self.call_builder(&getStateHistoryCountCall)
        }
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(&self) -> alloy_contract::SolCallBuilder<&P, getVersionCall, N> {
            self.call_builder(&getVersionCall)
        }
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(
            &self,
            _genesis: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _genesisStakeTableState: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            _stateHistoryRetentionPeriod: u32,
            owner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<&P, initializeCall, N> {
            self.call_builder(&initializeCall {
                _genesis,
                _genesisStakeTableState,
                _stateHistoryRetentionPeriod,
                owner,
            })
        }
        ///Creates a new call builder for the [`initializeV2`] function.
        pub fn initializeV2(
            &self,
            _blocksPerEpoch: u64,
            _epochStartBlock: u64,
        ) -> alloy_contract::SolCallBuilder<&P, initializeV2Call, N> {
            self.call_builder(&initializeV2Call {
                _blocksPerEpoch,
                _epochStartBlock,
            })
        }
        ///Creates a new call builder for the [`initializeV3`] function.
        pub fn initializeV3(&self) -> alloy_contract::SolCallBuilder<&P, initializeV3Call, N> {
            self.call_builder(&initializeV3Call)
        }
        ///Creates a new call builder for the [`isEpochRoot`] function.
        pub fn isEpochRoot(
            &self,
            blockHeight: u64,
        ) -> alloy_contract::SolCallBuilder<&P, isEpochRootCall, N> {
            self.call_builder(&isEpochRootCall { blockHeight })
        }
        ///Creates a new call builder for the [`isGtEpochRoot`] function.
        pub fn isGtEpochRoot(
            &self,
            blockHeight: u64,
        ) -> alloy_contract::SolCallBuilder<&P, isGtEpochRootCall, N> {
            self.call_builder(&isGtEpochRootCall { blockHeight })
        }
        ///Creates a new call builder for the [`isPermissionedProverEnabled`] function.
        pub fn isPermissionedProverEnabled(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, isPermissionedProverEnabledCall, N> {
            self.call_builder(&isPermissionedProverEnabledCall)
        }
        ///Creates a new call builder for the [`lagOverEscapeHatchThreshold`] function.
        pub fn lagOverEscapeHatchThreshold(
            &self,
            blockNumber: alloy::sol_types::private::primitives::aliases::U256,
            blockThreshold: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<&P, lagOverEscapeHatchThresholdCall, N> {
            self.call_builder(&lagOverEscapeHatchThresholdCall {
                blockNumber,
                blockThreshold,
            })
        }
        ///Creates a new call builder for the [`newFinalizedState_0`] function.
        pub fn newFinalizedState_0(
            &self,
            _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _1: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<&P, newFinalizedState_0Call, N> {
            self.call_builder(&newFinalizedState_0Call { _0, _1 })
        }
        ///Creates a new call builder for the [`newFinalizedState_1`] function.
        pub fn newFinalizedState_1(
            &self,
            _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _1: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            _2: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<&P, newFinalizedState_1Call, N> {
            self.call_builder(&newFinalizedState_1Call { _0, _1, _2 })
        }
        ///Creates a new call builder for the [`newFinalizedState_2`] function.
        pub fn newFinalizedState_2(
            &self,
            newState: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            nextStakeTable: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            newAuthRoot: alloy::sol_types::private::primitives::aliases::U256,
            proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<&P, newFinalizedState_2Call, N> {
            self.call_builder(&newFinalizedState_2Call {
                newState,
                nextStakeTable,
                newAuthRoot,
                proof,
            })
        }
        ///Creates a new call builder for the [`owner`] function.
        pub fn owner(&self) -> alloy_contract::SolCallBuilder<&P, ownerCall, N> {
            self.call_builder(&ownerCall)
        }
        ///Creates a new call builder for the [`permissionedProver`] function.
        pub fn permissionedProver(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, permissionedProverCall, N> {
            self.call_builder(&permissionedProverCall)
        }
        ///Creates a new call builder for the [`proxiableUUID`] function.
        pub fn proxiableUUID(&self) -> alloy_contract::SolCallBuilder<&P, proxiableUUIDCall, N> {
            self.call_builder(&proxiableUUIDCall)
        }
        ///Creates a new call builder for the [`renounceOwnership`] function.
        pub fn renounceOwnership(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, renounceOwnershipCall, N> {
            self.call_builder(&renounceOwnershipCall)
        }
        ///Creates a new call builder for the [`setPermissionedProver`] function.
        pub fn setPermissionedProver(
            &self,
            prover: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<&P, setPermissionedProverCall, N> {
            self.call_builder(&setPermissionedProverCall { prover })
        }
        ///Creates a new call builder for the [`setStateHistoryRetentionPeriod`] function.
        pub fn setStateHistoryRetentionPeriod(
            &self,
            historySeconds: u32,
        ) -> alloy_contract::SolCallBuilder<&P, setStateHistoryRetentionPeriodCall, N> {
            self.call_builder(&setStateHistoryRetentionPeriodCall { historySeconds })
        }
        ///Creates a new call builder for the [`setstateHistoryRetentionPeriod`] function.
        pub fn setstateHistoryRetentionPeriod(
            &self,
            historySeconds: u32,
        ) -> alloy_contract::SolCallBuilder<&P, setstateHistoryRetentionPeriodCall, N> {
            self.call_builder(&setstateHistoryRetentionPeriodCall { historySeconds })
        }
        ///Creates a new call builder for the [`stateHistoryCommitments`] function.
        pub fn stateHistoryCommitments(
            &self,
            _0: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<&P, stateHistoryCommitmentsCall, N> {
            self.call_builder(&stateHistoryCommitmentsCall(_0))
        }
        ///Creates a new call builder for the [`stateHistoryFirstIndex`] function.
        pub fn stateHistoryFirstIndex(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, stateHistoryFirstIndexCall, N> {
            self.call_builder(&stateHistoryFirstIndexCall)
        }
        ///Creates a new call builder for the [`stateHistoryRetentionPeriod`] function.
        pub fn stateHistoryRetentionPeriod(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, stateHistoryRetentionPeriodCall, N> {
            self.call_builder(&stateHistoryRetentionPeriodCall)
        }
        ///Creates a new call builder for the [`transferOwnership`] function.
        pub fn transferOwnership(
            &self,
            newOwner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<&P, transferOwnershipCall, N> {
            self.call_builder(&transferOwnershipCall { newOwner })
        }
        ///Creates a new call builder for the [`updateEpochStartBlock`] function.
        pub fn updateEpochStartBlock(
            &self,
            newEpochStartBlock: u64,
        ) -> alloy_contract::SolCallBuilder<&P, updateEpochStartBlockCall, N> {
            self.call_builder(&updateEpochStartBlockCall { newEpochStartBlock })
        }
        ///Creates a new call builder for the [`upgradeToAndCall`] function.
        pub fn upgradeToAndCall(
            &self,
            newImplementation: alloy::sol_types::private::Address,
            data: alloy::sol_types::private::Bytes,
        ) -> alloy_contract::SolCallBuilder<&P, upgradeToAndCallCall, N> {
            self.call_builder(&upgradeToAndCallCall {
                newImplementation,
                data,
            })
        }
        ///Creates a new call builder for the [`votingStakeTableState`] function.
        pub fn votingStakeTableState(
            &self,
        ) -> alloy_contract::SolCallBuilder<&P, votingStakeTableStateCall, N> {
            self.call_builder(&votingStakeTableStateCall)
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<P: alloy_contract::private::Provider<N>, N: alloy_contract::private::Network>
        LightClientV3Instance<P, N>
    {
        /// Creates a new event filter using this contract instance's provider and address.
        ///
        /// Note that the type can be any event, not just those defined in this contract.
        /// Prefer using the other methods for building type-safe event filters.
        pub fn event_filter<E: alloy_sol_types::SolEvent>(
            &self,
        ) -> alloy_contract::Event<&P, E, N> {
            alloy_contract::Event::new_sol(&self.provider, &self.address)
        }
        ///Creates a new event filter for the [`Initialized`] event.
        pub fn Initialized_filter(&self) -> alloy_contract::Event<&P, Initialized, N> {
            self.event_filter::<Initialized>()
        }
        ///Creates a new event filter for the [`NewEpoch`] event.
        pub fn NewEpoch_filter(&self) -> alloy_contract::Event<&P, NewEpoch, N> {
            self.event_filter::<NewEpoch>()
        }
        ///Creates a new event filter for the [`NewState`] event.
        pub fn NewState_filter(&self) -> alloy_contract::Event<&P, NewState, N> {
            self.event_filter::<NewState>()
        }
        ///Creates a new event filter for the [`OwnershipTransferred`] event.
        pub fn OwnershipTransferred_filter(
            &self,
        ) -> alloy_contract::Event<&P, OwnershipTransferred, N> {
            self.event_filter::<OwnershipTransferred>()
        }
        ///Creates a new event filter for the [`PermissionedProverNotRequired`] event.
        pub fn PermissionedProverNotRequired_filter(
            &self,
        ) -> alloy_contract::Event<&P, PermissionedProverNotRequired, N> {
            self.event_filter::<PermissionedProverNotRequired>()
        }
        ///Creates a new event filter for the [`PermissionedProverRequired`] event.
        pub fn PermissionedProverRequired_filter(
            &self,
        ) -> alloy_contract::Event<&P, PermissionedProverRequired, N> {
            self.event_filter::<PermissionedProverRequired>()
        }
        ///Creates a new event filter for the [`Upgrade`] event.
        pub fn Upgrade_filter(&self) -> alloy_contract::Event<&P, Upgrade, N> {
            self.event_filter::<Upgrade>()
        }
        ///Creates a new event filter for the [`Upgraded`] event.
        pub fn Upgraded_filter(&self) -> alloy_contract::Event<&P, Upgraded, N> {
            self.event_filter::<Upgraded>()
        }
    }
}
