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
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct ScalarField(alloy::sol_types::private::primitives::aliases::U256);
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<ScalarField>
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
        impl ScalarField {
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
        impl alloy_sol_types::SolType for ScalarField {
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
    #[derive()]
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
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(
                        &self.prodPerm,
                    ),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split0),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split1),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split2),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split3),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.split4),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(&self.zeta),
                    <BN254::G1Point as alloy_sol_types::SolType>::tokenize(
                        &self.zetaOmega,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.wireEval0,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.wireEval1,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.wireEval2,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.wireEval3,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.wireEval4,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.sigmaEval0,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.sigmaEval1,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.sigmaEval2,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.sigmaEval3,
                    ),
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
        impl alloy_sol_types::SolType for PlonkProof {
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
        impl alloy_sol_types::SolStruct for PlonkProof {
            const NAME: &'static str = "PlonkProof";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "PlonkProof(BN254.G1Point wire0,BN254.G1Point wire1,BN254.G1Point wire2,BN254.G1Point wire3,BN254.G1Point wire4,BN254.G1Point prodPerm,BN254.G1Point split0,BN254.G1Point split1,BN254.G1Point split2,BN254.G1Point split3,BN254.G1Point split4,BN254.G1Point zeta,BN254.G1Point zetaOmega,uint256 wireEval0,uint256 wireEval1,uint256 wireEval2,uint256 wireEval3,uint256 wireEval4,uint256 sigmaEval0,uint256 sigmaEval1,uint256 sigmaEval2,uint256 sigmaEval3,uint256 prodPermZetaOmegaEval)",
                )
            }
            #[inline]
            fn eip712_components() -> alloy_sol_types::private::Vec<
                alloy_sol_types::private::Cow<'static, str>,
            > {
                let mut components = alloy_sol_types::private::Vec::with_capacity(13);
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
            }
            #[inline]
            fn eip712_encode_data(&self) -> alloy_sol_types::private::Vec<u8> {
                [
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.wire0,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.wire1,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.wire2,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.wire3,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.wire4,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.prodPerm,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.split0,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.split1,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.split2,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.split3,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.split4,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.zeta,
                        )
                        .0,
                    <BN254::G1Point as alloy_sol_types::SolType>::eip712_data_word(
                            &self.zetaOmega,
                        )
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
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
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
                    &rust.zeta,
                    out,
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
    #[derive()]
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
        impl alloy_sol_types::SolType for VerifyingKey {
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
        impl alloy_sol_types::SolStruct for VerifyingKey {
            const NAME: &'static str = "VerifyingKey";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "VerifyingKey(uint256 domainSize,uint256 numInputs,BN254.G1Point sigma0,BN254.G1Point sigma1,BN254.G1Point sigma2,BN254.G1Point sigma3,BN254.G1Point sigma4,BN254.G1Point q1,BN254.G1Point q2,BN254.G1Point q3,BN254.G1Point q4,BN254.G1Point qM12,BN254.G1Point qM34,BN254.G1Point qO,BN254.G1Point qC,BN254.G1Point qH1,BN254.G1Point qH2,BN254.G1Point qH3,BN254.G1Point qH4,BN254.G1Point qEcc,bytes32 g2LSB,bytes32 g2MSB)",
                )
            }
            #[inline]
            fn eip712_components() -> alloy_sol_types::private::Vec<
                alloy_sol_types::private::Cow<'static, str>,
            > {
                let mut components = alloy_sol_types::private::Vec::with_capacity(18);
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
                components
                    .push(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_root_type(),
                    );
                components
                    .extend(
                        <BN254::G1Point as alloy_sol_types::SolStruct>::eip712_components(),
                    );
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
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
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
                    &rust.q1,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q2,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q3,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.q4,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qM12,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qM34,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qO,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qC,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH1,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH2,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH3,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qH4,
                    out,
                );
                <BN254::G1Point as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.qEcc,
                    out,
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
            f.debug_tuple("IPlonkVerifierInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > IPlonkVerifierInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`IPlonkVerifier`](self) contract instance.

See the [wrapper's documentation](`IPlonkVerifierInstance`) for more details.*/
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
    > IPlonkVerifierInstance<T, P, N> {
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
    > IPlonkVerifierInstance<T, P, N> {
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
library LightClient {
    struct LightClientState { uint64 viewNum; uint64 blockHeight; BN254.ScalarField blockCommRoot; }
    struct StakeTableState { uint256 threshold; BN254.ScalarField blsKeyComm; BN254.ScalarField schnorrKeyComm; BN254.ScalarField amountComm; }
    struct StateHistoryCommitment { uint64 l1BlockHeight; uint64 l1BlockTimestamp; uint64 hotShotBlockHeight; BN254.ScalarField hotShotBlockCommRoot; }
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.viewNum),
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.blockHeight),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.blockCommRoot,
                    ),
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
        impl alloy_sol_types::SolType for LightClientState {
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
        impl alloy_sol_types::SolStruct for LightClientState {
            const NAME: &'static str = "LightClientState";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "LightClientState(uint64 viewNum,uint64 blockHeight,uint256 blockCommRoot)",
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
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.threshold),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.blsKeyComm,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.schnorrKeyComm,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.amountComm,
                    ),
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
        impl alloy_sol_types::SolType for StakeTableState {
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
        impl alloy_sol_types::SolStruct for StakeTableState {
            const NAME: &'static str = "StakeTableState";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "StakeTableState(uint256 threshold,uint256 blsKeyComm,uint256 schnorrKeyComm,uint256 amountComm)",
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
                out.reserve(
                    <Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust),
                );
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
struct StateHistoryCommitment { uint64 l1BlockHeight; uint64 l1BlockTimestamp; uint64 hotShotBlockHeight; BN254.ScalarField hotShotBlockCommRoot; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct StateHistoryCommitment {
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
        impl ::core::convert::From<StateHistoryCommitment> for UnderlyingRustTuple<'_> {
            fn from(value: StateHistoryCommitment) -> Self {
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
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for StateHistoryCommitment {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    l1BlockHeight: tuple.0,
                    l1BlockTimestamp: tuple.1,
                    hotShotBlockHeight: tuple.2,
                    hotShotBlockCommRoot: tuple.3,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for StateHistoryCommitment {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for StateHistoryCommitment {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.l1BlockHeight),
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.l1BlockTimestamp),
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.hotShotBlockHeight),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.hotShotBlockCommRoot,
                    ),
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
        impl alloy_sol_types::SolType for StateHistoryCommitment {
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
        impl alloy_sol_types::SolStruct for StateHistoryCommitment {
            const NAME: &'static str = "StateHistoryCommitment";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "StateHistoryCommitment(uint64 l1BlockHeight,uint64 l1BlockTimestamp,uint64 hotShotBlockHeight,uint256 hotShotBlockCommRoot)",
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
                        64,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.l1BlockHeight)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::eip712_data_word(
                            &self.l1BlockTimestamp,
                        )
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::eip712_data_word(
                            &self.hotShotBlockHeight,
                        )
                        .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                            &self.hotShotBlockCommRoot,
                        )
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for StateHistoryCommitment {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.l1BlockHeight,
                    )
                    + <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.l1BlockTimestamp,
                    )
                    + <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.hotShotBlockHeight,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.hotShotBlockCommRoot,
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
                <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.l1BlockHeight,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.l1BlockTimestamp,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    64,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.hotShotBlockHeight,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.hotShotBlockCommRoot,
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
            f.debug_tuple("LightClientInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > LightClientInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`LightClient`](self) contract instance.

See the [wrapper's documentation](`LightClientInstance`) for more details.*/
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
    > LightClientInstance<T, P, N> {
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
    > LightClientInstance<T, P, N> {
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
    struct StateHistoryCommitment {
        uint64 l1BlockHeight;
        uint64 l1BlockTimestamp;
        uint64 hotShotBlockHeight;
        BN254.ScalarField hotShotBlockCommRoot;
    }
}

interface LightClientV3Mock {
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
    function firstEpoch() external view returns (uint64);
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
    function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 threshold) external view returns (bool);
    function newFinalizedState(LightClient.LightClientState memory, IPlonkVerifier.PlonkProof memory) external pure;
    function newFinalizedState(LightClient.LightClientState memory, LightClient.StakeTableState memory, IPlonkVerifier.PlonkProof memory) external pure;
    function newFinalizedState(LightClient.LightClientState memory newState, LightClient.StakeTableState memory nextStakeTable, uint256 newAuthRoot, IPlonkVerifier.PlonkProof memory proof) external;
    function owner() external view returns (address);
    function permissionedProver() external view returns (address);
    function proxiableUUID() external view returns (bytes32);
    function renounceOwnership() external;
    function setBlocksPerEpoch(uint64 newBlocksPerEpoch) external;
    function setFinalizedState(LightClient.LightClientState memory state) external;
    function setHotShotDownSince(uint256 l1Height) external;
    function setHotShotUp() external;
    function setPermissionedProver(address prover) external;
    function setStateHistory(LightClient.StateHistoryCommitment[] memory _stateHistoryCommitments) external;
    function setStateHistoryRetentionPeriod(uint32 historySeconds) external;
    function setVotingStakeTableState(LightClient.StakeTableState memory stake) external;
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
    "name": "firstEpoch",
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
        "name": "threshold",
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
    "name": "setBlocksPerEpoch",
    "inputs": [
      {
        "name": "newBlocksPerEpoch",
        "type": "uint64",
        "internalType": "uint64"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setFinalizedState",
    "inputs": [
      {
        "name": "state",
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
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setHotShotDownSince",
    "inputs": [
      {
        "name": "l1Height",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [],
    "stateMutability": "nonpayable"
  },
  {
    "type": "function",
    "name": "setHotShotUp",
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
    "name": "setStateHistory",
    "inputs": [
      {
        "name": "_stateHistoryCommitments",
        "type": "tuple[]",
        "internalType": "struct LightClient.StateHistoryCommitment[]",
        "components": [
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
        ]
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
    "name": "setVotingStakeTableState",
    "inputs": [
      {
        "name": "stake",
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
pub mod LightClientV3Mock {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x60a060405230608052348015610013575f5ffd5b5061001c610021565b6100d3565b7ff0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00805468010000000000000000900460ff16156100715760405163f92ee8a960e01b815260040160405180910390fd5b80546001600160401b03908116146100d05780546001600160401b0319166001600160401b0390811782556040519081527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b50565b608051613acf6100f95f395f8181611d7b01528181611da40152611f210152613acf5ff3fe608060405260043610610275575f3560e01c8063715018a61161014a578063aabd5db3116100be578063d24d933d11610078578063d24d933d146108ab578063e0303301146108da578063f0682054146108f9578063f2fde38b14610918578063f567616014610937578063f9e50d1914610956575f5ffd5b8063aabd5db3146107bb578063ad3cb1cc146107da578063b33bc49114610817578063b5adea3c14610836578063c23b9e9e14610855578063c8e5e4981461088d575f5ffd5b80638da5cb5b1161010f5780638da5cb5b146106b857806390c14390146106f457806396c1ca6114610713578063998328e8146107325780639baa3cc9146107475780639fdb54a714610766575f5ffd5b8063715018a61461061b578063757c37ad1461062f5780637667180814610649578063826e41fc1461065d5780638584d23f1461067c575f5ffd5b8063313df7b1116101ec578063426d3194116101a6578063426d319414610568578063433dba9f146105895780634f1ef286146105a857806352d1902d146105bb578063623a1338146105cf57806369cc6a0414610607575f5ffd5b8063313df7b114610481578063378ec23b146104b857806338e454b1146104d45780633c23b6db146104e85780633ed55b7b14610525578063416827441461054b575f5ffd5b8063167ac6181161023d578063167ac618146103855780632063d4f7146103a457806325297427146103c35780632d52aad6146103f25780632f79889d14610424578063300c89dd14610462575f5ffd5b8063013fa5fc1461027957806302b592f31461029a5780630625e19b146102f75780630d8e6e2c1461033957806312173c2c14610364575b5f5ffd5b348015610284575f5ffd5b50610298610293366004612bd6565b61096a565b005b3480156102a5575f5ffd5b506102b96102b4366004612bef565b610a1d565b6040516102ee94939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b348015610302575f5ffd5b50600b54600c54600d54600e546103199392919084565b6040805194855260208501939093529183015260608201526080016102ee565b348015610344575f5ffd5b5060408051600381525f60208201819052918101919091526060016102ee565b34801561036f575f5ffd5b50610378610a66565b6040516102ee9190612c06565b348015610390575f5ffd5b5061029861039f366004612e1d565b611095565b3480156103af575f5ffd5b506102986103be366004613101565b61110c565b3480156103ce575f5ffd5b506103e26103dd366004612e1d565b611125565b60405190151581526020016102ee565b3480156103fd575f5ffd5b5061029861040c366004612bef565b6010805460ff60401b1916600160401b179055601155565b34801561042f575f5ffd5b5060085461044a90600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016102ee565b34801561046d575f5ffd5b506103e261047c366004612e1d565b611187565b34801561048c575f5ffd5b506008546104a0906001600160a01b031681565b6040516001600160a01b0390911681526020016102ee565b3480156104c3575f5ffd5b50435b6040519081526020016102ee565b3480156104df575f5ffd5b5061029861121c565b3480156104f3575f5ffd5b50610298610502366004612e1d565b600a805467ffffffffffffffff19166001600160401b0392909216919091179055565b348015610530575f5ffd5b50600a5461044a90600160401b90046001600160401b031681565b348015610556575f5ffd5b506010546001600160401b031661044a565b348015610573575f5ffd5b505f546001546002546003546103199392919084565b348015610594575f5ffd5b506102986105a3366004613148565b61130b565b6102986105b6366004613161565b61131f565b3480156105c6575f5ffd5b506104c661133e565b3480156105da575f5ffd5b506102986105e9366004613247565b8051600b556020810151600c556040810151600d5560600151600e55565b348015610612575f5ffd5b50610298611359565b348015610626575f5ffd5b506102986113c7565b34801561063a575f5ffd5b506102986103be366004613261565b348015610654575f5ffd5b5061044a6113d8565b348015610668575f5ffd5b506008546001600160a01b031615156103e2565b348015610687575f5ffd5b5061069b610696366004612bef565b611402565b604080519283526001600160401b039091166020830152016102ee565b3480156106c3575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03166104a0565b3480156106ff575f5ffd5b5061044a61070e3660046132a5565b61152d565b34801561071e575f5ffd5b5061029861072d366004613148565b61159c565b34801561073d575f5ffd5b506104c6600f5481565b348015610752575f5ffd5b506102986107613660046132cd565b611625565b348015610771575f5ffd5b50600654600754610795916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016102ee565b3480156107c6575f5ffd5b506102986107d5366004613322565b611734565b3480156107e5575f5ffd5b5061080a604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516102ee9190613366565b348015610822575f5ffd5b506102986108313660046132a5565b611a59565b348015610841575f5ffd5b5061029861085036600461339b565b611baa565b348015610860575f5ffd5b5060085461087890600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016102ee565b348015610898575f5ffd5b506102986010805460ff60401b19169055565b3480156108b6575f5ffd5b50600454600554610795916001600160401b0380821692600160401b909204169083565b3480156108e5575f5ffd5b506103e26108f43660046133b5565b611bf1565b348015610904575f5ffd5b50600a5461044a906001600160401b031681565b348015610923575f5ffd5b50610298610932366004612bd6565b611c2b565b348015610942575f5ffd5b506102986109513660046133d5565b611c6a565b348015610961575f5ffd5b506009546104c6565b610972611d15565b6001600160a01b0381166109995760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b03908116908216036109c85760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b60098181548110610a2c575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b610a6e6128f6565b620100008152600560208201527f0577cf9bf82e9f08a752d837709088c137f9cd88cd95b03b27e6c54be38e929f6040820151527f08ff03a27cbe6a980ec660379d28ee76975cef42ba5806b9362632f26a6dfb206020604083015101527f0f4fc36c1ca15b9185959287c9e9773283e15395a92701087d38704fdd0c17ea6060820151527f14b01587bb51202bae56dc992251795655dc44395193f22b39a68e45e501e3f36020606083015101527f0a4ce9176328422b484832155bb3333285a6e0e030667761c9576f502e8b1b1b6080820151527f0a16f5414614d648678c637a0931c14191b232bb1a5d23f6c1673af0596603a66020608083015101527f1c1c5ff424e5a4daef97061e02327a6b8e5bbecc8b55d7f51889c6c391c6fe4b60a0820151527f2bff3ce7c876f111de5e8a0caa063dce26d07b87c31220a7d739a0d88c39d8ea602060a083015101527f2d32dce8d080a1a62afd7360e6550218c76ed1ba7eefe29c0274f5dee372531260c0820151527f260f1d1a8ae6d936ae1f284076ad0a374334b1545cb8e1b04eb45539e36e932e602060c083015101527f25cc09d27d1f2758c7edabfc4ccb753a1e101f07cdaea612bb1bb9e66b57acb160e0820151527f2f87735fcbe11c70121edff1d520c62fe9988ddd15937c3f430aa1dfa467308f602060e083015101527f073aad1f4c59f4f2fffc5f489c7b81f8b6fd69b77328ec8b0093071a41a9b760610100820151527f25bbaae87750964f4a0eabced3363b4c120de7acde7ce3d619d42a98674bcc17602061010083015101527f140f1b0a71a0df100cf0af671bbecc506160ef2cbdc24a976faa459312c1425c610120820151527f0e6ca0733e1f4553bbcca3a7b314c408083834103b20ab3faa2cece9ef7aa980602061012083015101527f0c6cc1b3d364f405fc77865198c954edb87714765f879b057f4720fe139ce861610140820151527f0efad53ac051aee07f7f92c34a779b5f9bc5803a612611710f907bbda3630330602061014083015101527f299b5d26b5da318cfd6ee23542444fbd54a221ae40b043ee7b59e4be8a7cf0bc610160820151527f11bfb70de4b59c717cb73ba6d923f79447d10eb6e7ea91ab8a6d261184ab7ddd602061016083015101527f2e646df45071a61803f5e8e92b75c156cab05f38cf0f84df751cf2442f224482610180820151527eed98ef227ce627fe2e56a11e32e1b490d99dee481c2d812050d2ffbf1ca0df602061018083015101527f10e36bbdf42d56359d2e87932b781b62271c3e6e1b0287d13e5d2478cd96baab6101a0820151527f02d5037fc2543b02b2e2e41f6abca9137206f87b67549abd7d68951b8801aa8d60206101a083015101527f16e1122401b64f03c2da5ebd4d52021e1b86c9bb3d4a6be643ee057731be4efd6101c0820151527f1bf6cca2dc0b7aa23fecaf058d11bbf4c06c6d85b03b6b27e1f37ac870357dd860206101c083015101527f2dc5951c5e5f891003f77d9940ba8a458b9608a3cb3b926d1f0a226900964dca6101e0820151527f0d1148213d633db532d6ec4352ff41e521e016c8fadc8db42c4bf9eb166ad38160206101e083015101527f2d7fed4088b6e73aba7ae1c3be39ab9697137ac1c450aeb971878dcfda7e3281610200820151527f18dc0de1ef57df9c8f492adeba183c610ad5eae06996446e3478076360b29e76602061020083015101527f1b9cd9b879f242f331103d6ac1f52623937c0801a21ccdd23b4dc9d4caf77d9c610220820151527f1dea6a5f2cffff1c5576b73075a96237fdf95153ef68cbf4372b7f1f3806d84a602061022083015101527f012617672c1d38557f385f9ff0d9fce130e8c8cfde27f65b908db4fd30d30d72610240820151527f0ee4840561aa90b4f40257d51c71ed03e0a39069207a5927677cd14668d57477602061024083015101527f13596792a5ef6df4cdbb46effaa45f91d3cc0401a64583cdd74b792310e3fdbd610260820151527f192e2c61cbb5b730ef86a950633c5972c4f956a053c72572498a1e1212487039602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b61109d611d15565b600a80546fffffffffffffffff0000000000000000198116600160401b6001600160401b038581168202928317948590556110e39491909104811692811691161761152d565b600a60106101000a8154816001600160401b0302191690836001600160401b0316021790555050565b604051634e405c8d60e01b815260040160405180910390fd5b5f6001600160401b03821615806111455750600a546001600160401b0316155b1561115157505f919050565b600a546001600160401b03166111688360056134e1565b6111729190613514565b6001600160401b03161592915050565b919050565b5f6001600160401b03821615806111a75750600a546001600160401b0316155b156111b357505f919050565b600a546111c9906001600160401b031683613514565b6001600160401b031615806112165750600a546111f1906005906001600160401b0316613541565b600a546001600160401b039182169161120b911684613514565b6001600160401b0316115b92915050565b5f516020613aa35f395f51905f52805460039190600160401b900460ff1680611252575080546001600160401b03808416911610155b156112705760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b9081178355600a546112aa929181048216911661152d565b6010805467ffffffffffffffff19166001600160401b03928316179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050565b611313611d15565b61131c8161159c565b50565b611327611d70565b61133082611e14565b61133a8282611e55565b5050565b5f611347611f16565b505f516020613a835f395f51905f5290565b611361611d15565b6008546001600160a01b0316156113ac57600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b6113cf611d15565b6113c55f611f5f565b600654600a545f916113fd916001600160401b03600160401b9092048216911661152d565b905090565b600980545f91829190611416600183613560565b8154811061142657611426613573565b5f918252602090912060029091020154600160801b90046001600160401b0316841061146557604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b8181101561152657846009828154811061149557611495613573565b5f918252602090912060029091020154600160801b90046001600160401b0316111561151e57600981815481106114ce576114ce613573565b905f5260205f20906002020160010154600982815481106114f1576114f1613573565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b600101611479565b5050915091565b5f816001600160401b03165f0361154557505f611216565b826001600160401b03165f0361155d57506001611216565b6115678284613514565b6001600160401b03165f03611587576115808284613587565b9050611216565b6115918284613587565b6115809060016134e1565b6115a4611d15565b610e108163ffffffff1610806115c357506301e133808163ffffffff16115b806115e1575060085463ffffffff600160a01b909104811690821611155b156115ff576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b5f516020613aa35f395f51905f528054600160401b810460ff1615906001600160401b03165f811580156116565750825b90505f826001600160401b031660011480156116715750303b155b90508115801561167f575080155b1561169d5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff1916600117855583156116c757845460ff60401b1916600160401b1785555b6116d086611fcf565b6116d8611fe0565b6116e3898989611fe8565b831561172957845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6008546001600160a01b03161515801561175957506008546001600160a01b03163314155b15611777576040516301474c8f60e71b815260040160405180910390fd5b60065484516001600160401b0391821691161115806117b0575060065460208501516001600160401b03600160401b9092048216911611155b156117ce5760405163051c46ef60e01b815260040160405180910390fd5b6117db8460400151612114565b6117e88360200151612114565b6117f58360400151612114565b6118028360600151612114565b5f61180b6113d8565b6020860151600a549192505f9161182b91906001600160401b031661152d565b6010549091506001600160401b039081169082161061186f576118518660200151611187565b1561186f5760405163080ae8d960e01b815260040160405180910390fd5b6010546001600160401b03908116908216111561191b5760026118928383613541565b6001600160401b0316106118b95760405163080ae8d960e01b815260040160405180910390fd5b6118c48260016134e1565b6001600160401b0316816001600160401b03161480156118fd57506006546118fb90600160401b90046001600160401b0316611125565b155b1561191b5760405163080ae8d960e01b815260040160405180910390fd5b61192786868686612184565b85516006805460208901516001600160401b03908116600160401b026001600160801b0319909216938116939093171790556040870151600755600f85905560105481169082161080159061198457506119848660200151611125565b156119ee578451600b556020850151600c556040850151600d556060850151600e557f31eabd9099fdb25dacddd206abff87311e553441fc9d0fcdef201062d7e7071b6119d28260016134e1565b6040516001600160401b03909116815260200160405180910390a15b6119f94342886123ae565b85602001516001600160401b0316865f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae68860400151604051611a4991815260200190565b60405180910390a3505050505050565b5f516020613aa35f395f51905f52805460029190600160401b900460ff1680611a8f575080546001600160401b03808416911610155b15611aad5760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b178255600590851611611af5576040516350dd03f760e11b815260040160405180910390fd5b5f54600b55600154600c55600254600d55600354600e55600a80546001600160401b03858116600160401b026001600160801b031990921690871617179055611b3e838561152d565b600a805467ffffffffffffffff60801b1916600160801b6001600160401b0393841602179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b80516006805460208401516001600160401b03908116600160401b026001600160801b0319909216931692909217919091179055604081015160075561131c4342836123ae565b6010545f90600160401b900460ff16611c1357611c0e8383612597565b611c24565b8160115484611c229190613560565b115b9392505050565b611c33611d15565b6001600160a01b038116611c6157604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b61131c81611f5f565b611c7560095f612b5b565b5f5b815181101561133a576009828281518110611c9457611c94613573565b6020908102919091018101518254600181810185555f94855293839020825160029092020180549383015160408401516001600160401b03908116600160801b0267ffffffffffffffff60801b19928216600160401b026001600160801b031990971691909416179490941793909316178255606001519082015501611c77565b33611d477f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b0316146113c55760405163118cdaa760e01b8152336004820152602401611c58565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611df657507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611dea5f516020613a835f395f51905f52546001600160a01b031690565b6001600160a01b031614155b156113c55760405163703e46dd60e11b815260040160405180910390fd5b611e1c611d15565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d90602001610a12565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611eaf575060408051601f3d908101601f19168201909252611eac918101906135b4565b60015b611ed757604051634c9c8ce360e01b81526001600160a01b0383166004820152602401611c58565b5f516020613a835f395f51905f528114611f0757604051632a87526960e21b815260048101829052602401611c58565b611f1183836126ef565b505050565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016146113c55760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611fd7612744565b61131c8161277a565b6113c5612744565b82516001600160401b031615158061200c575060208301516001600160401b031615155b8061201957506020820151155b8061202657506040820151155b8061203357506060820151155b8061203d57508151155b8061204f5750610e108163ffffffff16105b8061206357506301e133808163ffffffff16115b15612081576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000181108061133a5760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401611c58565b5f61218d610a66565b9050612197612b79565b600c548152600d54602080830191909152600e546040830152600b54606080840191909152600a549188015190916001600160401b03600160401b90910481169116108015906121ef57506121ef8760200151611125565b15612238576040805187516020808301919091528801518183015290870151606080830191909152870151608082015260a0016040516020818303038152906040529050612273565b60408051600b546020820152600c5491810191909152600d546060820152600e54608082015260a00160405160208183030381529060405290505b6040805188516001600160401b039081166020808401919091528a015116818301529088015160608201525f9060800160408051601f19818403018152908290526122c491849089906020016135e2565b60408051601f19818403018152919052805160209091012090506123087f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000182613604565b60808401526040516354e8bd6760e01b815273ffffffffffffffffffffffffffffffffffffffff906354e8bd679061234890879087908a906004016137f9565b602060405180830381865af4158015612363573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906123879190613a19565b6123a4576040516309bde33960e01b815260040160405180910390fd5b5050505050505050565b60095415801590612423575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b03169081106123ee576123ee613573565b5f91825260209091206002909102015461241890600160401b90046001600160401b031684613541565b6001600160401b0316115b156124b657600854600980549091600160c01b90046001600160401b031690811061245057612450613573565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b031690601861249083613a38565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b6009545f90438411806125a8575080155b806125f25750600854600980549091600160c01b90046001600160401b03169081106125d6576125d6613573565b5f9182526020909120600290910201546001600160401b031684105b156126105760405163b0b4387760e01b815260040160405180910390fd5b5f808061261e600185613560565b90505b816126ba57600854600160c01b90046001600160401b031681106126ba57866009828154811061265357612653613573565b5f9182526020909120600290910201546001600160401b0316116126a857600191506009818154811061268857612688613573565b5f9182526020909120600290910201546001600160401b031692506126ba565b806126b281613a62565b915050612621565b816126d85760405163b0b4387760e01b815260040160405180910390fd5b856126e38489613560565b11979650505050505050565b6126f882612782565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561273c57611f1182826127e5565b61133a612857565b5f516020613aa35f395f51905f5254600160401b900460ff166113c557604051631afcd79f60e31b815260040160405180910390fd5b611c33612744565b806001600160a01b03163b5f036127b757604051634c9c8ce360e01b81526001600160a01b0382166004820152602401611c58565b5f516020613a835f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b0316846040516128019190613a77565b5f60405180830381855af49150503d805f8114612839576040519150601f19603f3d011682016040523d82523d5f602084013e61283e565b606091505b509150915061284e858383612876565b95945050505050565b34156113c55760405163b398979f60e01b815260040160405180910390fd5b60608261288657611c0e826128cd565b815115801561289d57506001600160a01b0384163b155b156128c657604051639996b31560e01b81526001600160a01b0385166004820152602401611c58565b5092915050565b8051156128dd5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b604051806102c001604052805f81526020015f815260200161292960405180604001604052805f81526020015f81525090565b815260200161294960405180604001604052805f81526020015f81525090565b815260200161296960405180604001604052805f81526020015f81525090565b815260200161298960405180604001604052805f81526020015f81525090565b81526020016129a960405180604001604052805f81526020015f81525090565b81526020016129c960405180604001604052805f81526020015f81525090565b81526020016129e960405180604001604052805f81526020015f81525090565b8152602001612a0960405180604001604052805f81526020015f81525090565b8152602001612a2960405180604001604052805f81526020015f81525090565b8152602001612a4960405180604001604052805f81526020015f81525090565b8152602001612a6960405180604001604052805f81526020015f81525090565b8152602001612a8960405180604001604052805f81526020015f81525090565b8152602001612aa960405180604001604052805f81526020015f81525090565b8152602001612ac960405180604001604052805f81526020015f81525090565b8152602001612ae960405180604001604052805f81526020015f81525090565b8152602001612b0960405180604001604052805f81526020015f81525090565b8152602001612b2960405180604001604052805f81526020015f81525090565b8152602001612b4960405180604001604052805f81526020015f81525090565b81526020015f81526020015f81525090565b5080545f8255600202905f5260205f209081019061131c9190612b97565b6040518060a001604052806005906020820280368337509192915050565b5b80821115612bbc5780546001600160c01b03191681555f6001820155600201612b98565b5090565b80356001600160a01b0381168114611182575f5ffd5b5f60208284031215612be6575f5ffd5b611c2482612bc0565b5f60208284031215612bff575f5ffd5b5035919050565b5f6105008201905082518252602083015160208301526040830151612c38604084018280518252602090810151910152565b50606083015180516080840152602081015160a0840152506080830151805160c0840152602081015160e08401525060a0830151805161010084015260208101516101208401525060c0830151805161014084015260208101516101608401525060e0830151805161018084015260208101516101a08401525061010083015180516101c084015260208101516101e08401525061012083015180516102008401526020810151610220840152506101408301518051610240840152602081015161026084015250610160830151805161028084015260208101516102a08401525061018083015180516102c084015260208101516102e0840152506101a083015180516103008401526020810151610320840152506101c083015180516103408401526020810151610360840152506101e0830151805161038084015260208101516103a08401525061020083015180516103c084015260208101516103e08401525061022083015180516104008401526020810151610420840152506102408301518051610440840152602081015161046084015250610260830151805161048084015260208101516104a0840152506102808301516104c08301526102a0909201516104e09091015290565b80356001600160401b0381168114611182575f5ffd5b5f60208284031215612e2d575f5ffd5b611c2482612e07565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715612e6d57612e6d612e36565b60405290565b604051608081016001600160401b0381118282101715612e6d57612e6d612e36565b604051601f8201601f191681016001600160401b0381118282101715612ebd57612ebd612e36565b604052919050565b5f60608284031215612ed5575f5ffd5b604051606081016001600160401b0381118282101715612ef757612ef7612e36565b604052905080612f0683612e07565b8152612f1460208401612e07565b6020820152604092830135920191909152919050565b5f60408284031215612f3a575f5ffd5b604080519081016001600160401b0381118282101715612f5c57612f5c612e36565b604052823581526020928301359281019290925250919050565b5f6104808284031215612f87575f5ffd5b612f8f612e4a565b9050612f9b8383612f2a565b8152612faa8360408401612f2a565b6020820152612fbc8360808401612f2a565b6040820152612fce8360c08401612f2a565b6060820152612fe1836101008401612f2a565b6080820152612ff4836101408401612f2a565b60a0820152613007836101808401612f2a565b60c082015261301a836101c08401612f2a565b60e082015261302d836102008401612f2a565b610100820152613041836102408401612f2a565b610120820152613055836102808401612f2a565b610140820152613069836102c08401612f2a565b61016082015261307d836103008401612f2a565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f6104e08385031215613113575f5ffd5b61311d8484612ec5565b915061312c8460608501612f76565b90509250929050565b803563ffffffff81168114611182575f5ffd5b5f60208284031215613158575f5ffd5b611c2482613135565b5f5f60408385031215613172575f5ffd5b61317b83612bc0565b915060208301356001600160401b03811115613195575f5ffd5b8301601f810185136131a5575f5ffd5b80356001600160401b038111156131be576131be612e36565b6131d1601f8201601f1916602001612e95565b8181528660208385010111156131e5575f5ffd5b816020840160208301375f602083830101528093505050509250929050565b5f60808284031215613214575f5ffd5b61321c612e73565b8235815260208084013590820152604080840135908201526060928301359281019290925250919050565b5f60808284031215613257575f5ffd5b611c248383613204565b5f5f5f6105608486031215613274575f5ffd5b61327e8585612ec5565b925061328d8560608601613204565b915061329c8560e08601612f76565b90509250925092565b5f5f604083850312156132b6575f5ffd5b6132bf83612e07565b915061312c60208401612e07565b5f5f5f5f61012085870312156132e1575f5ffd5b6132eb8686612ec5565b93506132fa8660608701613204565b925061330860e08601613135565b91506133176101008601612bc0565b905092959194509250565b5f5f5f5f6105808587031215613336575f5ffd5b6133408686612ec5565b935061334f8660608701613204565b925060e08501359150613317866101008701612f76565b602081525f82518060208401528060208501604085015e5f604082850101526040601f19601f83011684010191505092915050565b5f606082840312156133ab575f5ffd5b611c248383612ec5565b5f5f604083850312156133c6575f5ffd5b50508035926020909101359150565b5f602082840312156133e5575f5ffd5b81356001600160401b038111156133fa575f5ffd5b8201601f8101841361340a575f5ffd5b80356001600160401b0381111561342357613423612e36565b61343260208260051b01612e95565b8082825260208201915060208360071b850101925086831115613453575f5ffd5b6020840193505b828410156134c35760808488031215613471575f5ffd5b613479612e73565b61348285612e07565b815261349060208601612e07565b60208201526134a160408601612e07565b604082015260608581013590820152825260809093019260209091019061345a565b9695505050505050565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b038181168382160190811115611216576112166134cd565b634e487b7160e01b5f52601260045260245ffd5b5f6001600160401b0383168061352c5761352c613500565b806001600160401b0384160691505092915050565b6001600160401b038281168282160390811115611216576112166134cd565b81810381811115611216576112166134cd565b634e487b7160e01b5f52603260045260245ffd5b5f6001600160401b0383168061359f5761359f613500565b806001600160401b0384160491505092915050565b5f602082840312156135c4575f5ffd5b5051919050565b5f81518060208401855e5f93019283525090919050565b5f6135f66135f083876135cb565b856135cb565b928352505060200192915050565b5f8261361257613612613500565b500690565b805f5b600581101561363957815184526020938401939091019060010161361a565b50505050565b61365482825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e0840152506080810151805161010084015260208101516101208401525060a0810151805161014084015260208101516101608401525060c0810151805161018084015260208101516101a08401525060e081015180516101c084015260208101516101e08401525061010081015180516102008401526020810151610220840152506101208101518051610240840152602081015161026084015250610140810151805161028084015260208101516102a08401525061016081015180516102c084015260208101516102e08401525061018081015180516103008401526020810151610320840152506101a08101516103408301526101c08101516103608301526101e08101516103808301526102008101516103a08301526102208101516103c08301526102408101516103e08301526102608101516104008301526102808101516104208301526102a08101516104408301526102c0015161046090910152565b5f610a20820190508451825260208501516020830152604085015161382b604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a0850151805161010084015260208101516101208401525060c0850151805161014084015260208101516101608401525060e0850151805161018084015260208101516101a08401525061010085015180516101c084015260208101516101e08401525061012085015180516102008401526020810151610220840152506101408501518051610240840152602081015161026084015250610160850151805161028084015260208101516102a08401525061018085015180516102c084015260208101516102e0840152506101a085015180516103008401526020810151610320840152506101c085015180516103408401526020810151610360840152506101e0850151805161038084015260208101516103a08401525061020085015180516103c084015260208101516103e08401525061022085015180516104008401526020810151610420840152506102408501518051610440840152602081015161046084015250610260850151805161048084015260208101516104a0840152506102808501516104c08301526102a08501516104e0830152613a03610500830185613617565b613a116105a083018461363f565b949350505050565b5f60208284031215613a29575f5ffd5b81518015158114611c24575f5ffd5b5f6001600160401b0382166001600160401b038103613a5957613a596134cd565b60010192915050565b5f81613a7057613a706134cd565b505f190190565b5f611c2482846135cb56fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbcf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\xA0`@R0`\x80R4\x80\x15a\0\x13W__\xFD[Pa\0\x1Ca\0!V[a\0\xD3V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15a\0qW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14a\0\xD0W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[`\x80Qa:\xCFa\0\xF9_9_\x81\x81a\x1D{\x01R\x81\x81a\x1D\xA4\x01Ra\x1F!\x01Ra:\xCF_\xF3\xFE`\x80`@R`\x046\x10a\x02uW_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x01JW\x80c\xAA\xBD]\xB3\x11a\0\xBEW\x80c\xD2M\x93=\x11a\0xW\x80c\xD2M\x93=\x14a\x08\xABW\x80c\xE003\x01\x14a\x08\xDAW\x80c\xF0h T\x14a\x08\xF9W\x80c\xF2\xFD\xE3\x8B\x14a\t\x18W\x80c\xF5ga`\x14a\t7W\x80c\xF9\xE5\r\x19\x14a\tVW__\xFD[\x80c\xAA\xBD]\xB3\x14a\x07\xBBW\x80c\xAD<\xB1\xCC\x14a\x07\xDAW\x80c\xB3;\xC4\x91\x14a\x08\x17W\x80c\xB5\xAD\xEA<\x14a\x086W\x80c\xC2;\x9E\x9E\x14a\x08UW\x80c\xC8\xE5\xE4\x98\x14a\x08\x8DW__\xFD[\x80c\x8D\xA5\xCB[\x11a\x01\x0FW\x80c\x8D\xA5\xCB[\x14a\x06\xB8W\x80c\x90\xC1C\x90\x14a\x06\xF4W\x80c\x96\xC1\xCAa\x14a\x07\x13W\x80c\x99\x83(\xE8\x14a\x072W\x80c\x9B\xAA<\xC9\x14a\x07GW\x80c\x9F\xDBT\xA7\x14a\x07fW__\xFD[\x80cqP\x18\xA6\x14a\x06\x1BW\x80cu|7\xAD\x14a\x06/W\x80cvg\x18\x08\x14a\x06IW\x80c\x82nA\xFC\x14a\x06]W\x80c\x85\x84\xD2?\x14a\x06|W__\xFD[\x80c1=\xF7\xB1\x11a\x01\xECW\x80cBm1\x94\x11a\x01\xA6W\x80cBm1\x94\x14a\x05hW\x80cC=\xBA\x9F\x14a\x05\x89W\x80cO\x1E\xF2\x86\x14a\x05\xA8W\x80cR\xD1\x90-\x14a\x05\xBBW\x80cb:\x138\x14a\x05\xCFW\x80ci\xCCj\x04\x14a\x06\x07W__\xFD[\x80c1=\xF7\xB1\x14a\x04\x81W\x80c7\x8E\xC2;\x14a\x04\xB8W\x80c8\xE4T\xB1\x14a\x04\xD4W\x80c<#\xB6\xDB\x14a\x04\xE8W\x80c>\xD5[{\x14a\x05%W\x80cAh'D\x14a\x05KW__\xFD[\x80c\x16z\xC6\x18\x11a\x02=W\x80c\x16z\xC6\x18\x14a\x03\x85W\x80c c\xD4\xF7\x14a\x03\xA4W\x80c%)t'\x14a\x03\xC3W\x80c-R\xAA\xD6\x14a\x03\xF2W\x80c/y\x88\x9D\x14a\x04$W\x80c0\x0C\x89\xDD\x14a\x04bW__\xFD[\x80c\x01?\xA5\xFC\x14a\x02yW\x80c\x02\xB5\x92\xF3\x14a\x02\x9AW\x80c\x06%\xE1\x9B\x14a\x02\xF7W\x80c\r\x8En,\x14a\x039W\x80c\x12\x17<,\x14a\x03dW[__\xFD[4\x80\x15a\x02\x84W__\xFD[Pa\x02\x98a\x02\x936`\x04a+\xD6V[a\tjV[\0[4\x80\x15a\x02\xA5W__\xFD[Pa\x02\xB9a\x02\xB46`\x04a+\xEFV[a\n\x1DV[`@Qa\x02\xEE\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x03\x02W__\xFD[P`\x0BT`\x0CT`\rT`\x0ETa\x03\x19\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x02\xEEV[4\x80\x15a\x03DW__\xFD[P`@\x80Q`\x03\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02\xEEV[4\x80\x15a\x03oW__\xFD[Pa\x03xa\nfV[`@Qa\x02\xEE\x91\x90a,\x06V[4\x80\x15a\x03\x90W__\xFD[Pa\x02\x98a\x03\x9F6`\x04a.\x1DV[a\x10\x95V[4\x80\x15a\x03\xAFW__\xFD[Pa\x02\x98a\x03\xBE6`\x04a1\x01V[a\x11\x0CV[4\x80\x15a\x03\xCEW__\xFD[Pa\x03\xE2a\x03\xDD6`\x04a.\x1DV[a\x11%V[`@Q\x90\x15\x15\x81R` \x01a\x02\xEEV[4\x80\x15a\x03\xFDW__\xFD[Pa\x02\x98a\x04\x0C6`\x04a+\xEFV[`\x10\x80T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x90U`\x11UV[4\x80\x15a\x04/W__\xFD[P`\x08Ta\x04J\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x04mW__\xFD[Pa\x03\xE2a\x04|6`\x04a.\x1DV[a\x11\x87V[4\x80\x15a\x04\x8CW__\xFD[P`\x08Ta\x04\xA0\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x04\xC3W__\xFD[PC[`@Q\x90\x81R` \x01a\x02\xEEV[4\x80\x15a\x04\xDFW__\xFD[Pa\x02\x98a\x12\x1CV[4\x80\x15a\x04\xF3W__\xFD[Pa\x02\x98a\x05\x026`\x04a.\x1DV[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[4\x80\x15a\x050W__\xFD[P`\nTa\x04J\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x05VW__\xFD[P`\x10T`\x01`\x01`@\x1B\x03\x16a\x04JV[4\x80\x15a\x05sW__\xFD[P_T`\x01T`\x02T`\x03Ta\x03\x19\x93\x92\x91\x90\x84V[4\x80\x15a\x05\x94W__\xFD[Pa\x02\x98a\x05\xA36`\x04a1HV[a\x13\x0BV[a\x02\x98a\x05\xB66`\x04a1aV[a\x13\x1FV[4\x80\x15a\x05\xC6W__\xFD[Pa\x04\xC6a\x13>V[4\x80\x15a\x05\xDAW__\xFD[Pa\x02\x98a\x05\xE96`\x04a2GV[\x80Q`\x0BU` \x81\x01Q`\x0CU`@\x81\x01Q`\rU``\x01Q`\x0EUV[4\x80\x15a\x06\x12W__\xFD[Pa\x02\x98a\x13YV[4\x80\x15a\x06&W__\xFD[Pa\x02\x98a\x13\xC7V[4\x80\x15a\x06:W__\xFD[Pa\x02\x98a\x03\xBE6`\x04a2aV[4\x80\x15a\x06TW__\xFD[Pa\x04Ja\x13\xD8V[4\x80\x15a\x06hW__\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15a\x03\xE2V[4\x80\x15a\x06\x87W__\xFD[Pa\x06\x9Ba\x06\x966`\x04a+\xEFV[a\x14\x02V[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x02\xEEV[4\x80\x15a\x06\xC3W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x04\xA0V[4\x80\x15a\x06\xFFW__\xFD[Pa\x04Ja\x07\x0E6`\x04a2\xA5V[a\x15-V[4\x80\x15a\x07\x1EW__\xFD[Pa\x02\x98a\x07-6`\x04a1HV[a\x15\x9CV[4\x80\x15a\x07=W__\xFD[Pa\x04\xC6`\x0FT\x81V[4\x80\x15a\x07RW__\xFD[Pa\x02\x98a\x07a6`\x04a2\xCDV[a\x16%V[4\x80\x15a\x07qW__\xFD[P`\x06T`\x07Ta\x07\x95\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x02\xEEV[4\x80\x15a\x07\xC6W__\xFD[Pa\x02\x98a\x07\xD56`\x04a3\"V[a\x174V[4\x80\x15a\x07\xE5W__\xFD[Pa\x08\n`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02\xEE\x91\x90a3fV[4\x80\x15a\x08\"W__\xFD[Pa\x02\x98a\x0816`\x04a2\xA5V[a\x1AYV[4\x80\x15a\x08AW__\xFD[Pa\x02\x98a\x08P6`\x04a3\x9BV[a\x1B\xAAV[4\x80\x15a\x08`W__\xFD[P`\x08Ta\x08x\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x08\x98W__\xFD[Pa\x02\x98`\x10\x80T`\xFF`@\x1B\x19\x16\x90UV[4\x80\x15a\x08\xB6W__\xFD[P`\x04T`\x05Ta\x07\x95\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x08\xE5W__\xFD[Pa\x03\xE2a\x08\xF46`\x04a3\xB5V[a\x1B\xF1V[4\x80\x15a\t\x04W__\xFD[P`\nTa\x04J\x90`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\t#W__\xFD[Pa\x02\x98a\t26`\x04a+\xD6V[a\x1C+V[4\x80\x15a\tBW__\xFD[Pa\x02\x98a\tQ6`\x04a3\xD5V[a\x1CjV[4\x80\x15a\taW__\xFD[P`\tTa\x04\xC6V[a\tra\x1D\x15V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\t\x99W`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\t\xC8W`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\n,W_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[a\nna(\xF6V[b\x01\0\0\x81R`\x05` \x82\x01R\x7F\x05w\xCF\x9B\xF8.\x9F\x08\xA7R\xD87p\x90\x88\xC17\xF9\xCD\x88\xCD\x95\xB0;'\xE6\xC5K\xE3\x8E\x92\x9F`@\x82\x01QR\x7F\x08\xFF\x03\xA2|\xBEj\x98\x0E\xC6`7\x9D(\xEEv\x97\\\xEFB\xBAX\x06\xB96&2\xF2jm\xFB ` `@\x83\x01Q\x01R\x7F\x0FO\xC3l\x1C\xA1[\x91\x85\x95\x92\x87\xC9\xE9w2\x83\xE1S\x95\xA9'\x01\x08}8pO\xDD\x0C\x17\xEA``\x82\x01QR\x7F\x14\xB0\x15\x87\xBBQ +\xAEV\xDC\x99\"QyVU\xDCD9Q\x93\xF2+9\xA6\x8EE\xE5\x01\xE3\xF3` ``\x83\x01Q\x01R\x7F\nL\xE9\x17c(B+HH2\x15[\xB332\x85\xA6\xE0\xE00fwa\xC9WoP.\x8B\x1B\x1B`\x80\x82\x01QR\x7F\n\x16\xF5AF\x14\xD6Hg\x8Ccz\t1\xC1A\x91\xB22\xBB\x1A]#\xF6\xC1g:\xF0Yf\x03\xA6` `\x80\x83\x01Q\x01R\x7F\x1C\x1C_\xF4$\xE5\xA4\xDA\xEF\x97\x06\x1E\x022zk\x8E[\xBE\xCC\x8BU\xD7\xF5\x18\x89\xC6\xC3\x91\xC6\xFEK`\xA0\x82\x01QR\x7F+\xFF<\xE7\xC8v\xF1\x11\xDE^\x8A\x0C\xAA\x06=\xCE&\xD0{\x87\xC3\x12 \xA7\xD79\xA0\xD8\x8C9\xD8\xEA` `\xA0\x83\x01Q\x01R\x7F-2\xDC\xE8\xD0\x80\xA1\xA6*\xFDs`\xE6U\x02\x18\xC7n\xD1\xBA~\xEF\xE2\x9C\x02t\xF5\xDE\xE3rS\x12`\xC0\x82\x01QR\x7F&\x0F\x1D\x1A\x8A\xE6\xD96\xAE\x1F(@v\xAD\n7C4\xB1T\\\xB8\xE1\xB0N\xB4U9\xE3n\x93.` `\xC0\x83\x01Q\x01R\x7F%\xCC\t\xD2}\x1F'X\xC7\xED\xAB\xFCL\xCBu:\x1E\x10\x1F\x07\xCD\xAE\xA6\x12\xBB\x1B\xB9\xE6kW\xAC\xB1`\xE0\x82\x01QR\x7F/\x87s_\xCB\xE1\x1Cp\x12\x1E\xDF\xF1\xD5 \xC6/\xE9\x98\x8D\xDD\x15\x93|?C\n\xA1\xDF\xA4g0\x8F` `\xE0\x83\x01Q\x01R\x7F\x07:\xAD\x1FLY\xF4\xF2\xFF\xFC_H\x9C{\x81\xF8\xB6\xFDi\xB7s(\xEC\x8B\0\x93\x07\x1AA\xA9\xB7`a\x01\0\x82\x01QR\x7F%\xBB\xAA\xE8wP\x96OJ\x0E\xAB\xCE\xD36;L\x12\r\xE7\xAC\xDE|\xE3\xD6\x19\xD4*\x98gK\xCC\x17` a\x01\0\x83\x01Q\x01R\x7F\x14\x0F\x1B\nq\xA0\xDF\x10\x0C\xF0\xAFg\x1B\xBE\xCCPa`\xEF,\xBD\xC2J\x97o\xAAE\x93\x12\xC1B\\a\x01 \x82\x01QR\x7F\x0El\xA0s>\x1FES\xBB\xCC\xA3\xA7\xB3\x14\xC4\x08\x0884\x10; \xAB?\xAA,\xEC\xE9\xEFz\xA9\x80` a\x01 \x83\x01Q\x01R\x7F\x0Cl\xC1\xB3\xD3d\xF4\x05\xFCw\x86Q\x98\xC9T\xED\xB8w\x14v_\x87\x9B\x05\x7FG \xFE\x13\x9C\xE8aa\x01@\x82\x01QR\x7F\x0E\xFA\xD5:\xC0Q\xAE\xE0\x7F\x7F\x92\xC3Jw\x9B_\x9B\xC5\x80:a&\x11q\x0F\x90{\xBD\xA3c\x030` a\x01@\x83\x01Q\x01R\x7F)\x9B]&\xB5\xDA1\x8C\xFDn\xE25BDO\xBDT\xA2!\xAE@\xB0C\xEE{Y\xE4\xBE\x8A|\xF0\xBCa\x01`\x82\x01QR\x7F\x11\xBF\xB7\r\xE4\xB5\x9Cq|\xB7;\xA6\xD9#\xF7\x94G\xD1\x0E\xB6\xE7\xEA\x91\xAB\x8Am&\x11\x84\xAB}\xDD` a\x01`\x83\x01Q\x01R\x7F.dm\xF4Pq\xA6\x18\x03\xF5\xE8\xE9+u\xC1V\xCA\xB0_8\xCF\x0F\x84\xDFu\x1C\xF2D/\"D\x82a\x01\x80\x82\x01QR~\xED\x98\xEF\"|\xE6'\xFE.V\xA1\x1E2\xE1\xB4\x90\xD9\x9D\xEEH\x1C-\x81 P\xD2\xFF\xBF\x1C\xA0\xDF` a\x01\x80\x83\x01Q\x01R\x7F\x10\xE3k\xBD\xF4-V5\x9D.\x87\x93+x\x1Bb'\x1C>n\x1B\x02\x87\xD1>]$x\xCD\x96\xBA\xABa\x01\xA0\x82\x01QR\x7F\x02\xD5\x03\x7F\xC2T;\x02\xB2\xE2\xE4\x1Fj\xBC\xA9\x13r\x06\xF8{gT\x9A\xBD}h\x95\x1B\x88\x01\xAA\x8D` a\x01\xA0\x83\x01Q\x01R\x7F\x16\xE1\x12$\x01\xB6O\x03\xC2\xDA^\xBDMR\x02\x1E\x1B\x86\xC9\xBB=Jk\xE6C\xEE\x05w1\xBEN\xFDa\x01\xC0\x82\x01QR\x7F\x1B\xF6\xCC\xA2\xDC\x0Bz\xA2?\xEC\xAF\x05\x8D\x11\xBB\xF4\xC0lm\x85\xB0;k'\xE1\xF3z\xC8p5}\xD8` a\x01\xC0\x83\x01Q\x01R\x7F-\xC5\x95\x1C^_\x89\x10\x03\xF7}\x99@\xBA\x8AE\x8B\x96\x08\xA3\xCB;\x92m\x1F\n\"i\0\x96M\xCAa\x01\xE0\x82\x01QR\x7F\r\x11H!=c=\xB52\xD6\xECCR\xFFA\xE5!\xE0\x16\xC8\xFA\xDC\x8D\xB4,K\xF9\xEB\x16j\xD3\x81` a\x01\xE0\x83\x01Q\x01R\x7F-\x7F\xED@\x88\xB6\xE7:\xBAz\xE1\xC3\xBE9\xAB\x96\x97\x13z\xC1\xC4P\xAE\xB9q\x87\x8D\xCF\xDA~2\x81a\x02\0\x82\x01QR\x7F\x18\xDC\r\xE1\xEFW\xDF\x9C\x8FI*\xDE\xBA\x18<a\n\xD5\xEA\xE0i\x96Dn4x\x07c`\xB2\x9Ev` a\x02\0\x83\x01Q\x01R\x7F\x1B\x9C\xD9\xB8y\xF2B\xF31\x10=j\xC1\xF5&#\x93|\x08\x01\xA2\x1C\xCD\xD2;M\xC9\xD4\xCA\xF7}\x9Ca\x02 \x82\x01QR\x7F\x1D\xEAj_,\xFF\xFF\x1CUv\xB70u\xA9b7\xFD\xF9QS\xEFh\xCB\xF47+\x7F\x1F8\x06\xD8J` a\x02 \x83\x01Q\x01R\x7F\x01&\x17g,\x1D8U\x7F8_\x9F\xF0\xD9\xFC\xE10\xE8\xC8\xCF\xDE'\xF6[\x90\x8D\xB4\xFD0\xD3\rra\x02@\x82\x01QR\x7F\x0E\xE4\x84\x05a\xAA\x90\xB4\xF4\x02W\xD5\x1Cq\xED\x03\xE0\xA3\x90i zY'g|\xD1Fh\xD5tw` a\x02@\x83\x01Q\x01R\x7F\x13Yg\x92\xA5\xEFm\xF4\xCD\xBBF\xEF\xFA\xA4_\x91\xD3\xCC\x04\x01\xA6E\x83\xCD\xD7Ky#\x10\xE3\xFD\xBDa\x02`\x82\x01QR\x7F\x19.,a\xCB\xB5\xB70\xEF\x86\xA9Pc<Yr\xC4\xF9V\xA0S\xC7%rI\x8A\x1E\x12\x12Hp9` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[a\x10\x9Da\x1D\x15V[`\n\x80To\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\x19\x81\x16`\x01`@\x1B`\x01`\x01`@\x1B\x03\x85\x81\x16\x82\x02\x92\x83\x17\x94\x85\x90Ua\x10\xE3\x94\x91\x90\x91\x04\x81\x16\x92\x81\x16\x91\x16\x17a\x15-V[`\n`\x10a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPPV[`@QcN@\\\x8D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\x11EWP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\x11QWP_\x91\x90PV[`\nT`\x01`\x01`@\x1B\x03\x16a\x11h\x83`\x05a4\xE1V[a\x11r\x91\x90a5\x14V[`\x01`\x01`@\x1B\x03\x16\x15\x92\x91PPV[\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\x11\xA7WP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\x11\xB3WP_\x91\x90PV[`\nTa\x11\xC9\x90`\x01`\x01`@\x1B\x03\x16\x83a5\x14V[`\x01`\x01`@\x1B\x03\x16\x15\x80a\x12\x16WP`\nTa\x11\xF1\x90`\x05\x90`\x01`\x01`@\x1B\x03\x16a5AV[`\nT`\x01`\x01`@\x1B\x03\x91\x82\x16\x91a\x12\x0B\x91\x16\x84a5\x14V[`\x01`\x01`@\x1B\x03\x16\x11[\x92\x91PPV[_Q` a:\xA3_9_Q\x90_R\x80T`\x03\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x12RWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x12pW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x90\x81\x17\x83U`\nTa\x12\xAA\x92\x91\x81\x04\x82\x16\x91\x16a\x15-V[`\x10\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x83\x16\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPV[a\x13\x13a\x1D\x15V[a\x13\x1C\x81a\x15\x9CV[PV[a\x13'a\x1DpV[a\x130\x82a\x1E\x14V[a\x13:\x82\x82a\x1EUV[PPV[_a\x13Ga\x1F\x16V[P_Q` a:\x83_9_Q\x90_R\x90V[a\x13aa\x1D\x15V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x13\xACW`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x13\xCFa\x1D\x15V[a\x13\xC5_a\x1F_V[`\x06T`\nT_\x91a\x13\xFD\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16a\x15-V[\x90P\x90V[`\t\x80T_\x91\x82\x91\x90a\x14\x16`\x01\x83a5`V[\x81T\x81\x10a\x14&Wa\x14&a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x14eW`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\x15&W\x84`\t\x82\x81T\x81\x10a\x14\x95Wa\x14\x95a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\x15\x1EW`\t\x81\x81T\x81\x10a\x14\xCEWa\x14\xCEa5sV[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\x14\xF1Wa\x14\xF1a5sV[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x14yV[PP\x91P\x91V[_\x81`\x01`\x01`@\x1B\x03\x16_\x03a\x15EWP_a\x12\x16V[\x82`\x01`\x01`@\x1B\x03\x16_\x03a\x15]WP`\x01a\x12\x16V[a\x15g\x82\x84a5\x14V[`\x01`\x01`@\x1B\x03\x16_\x03a\x15\x87Wa\x15\x80\x82\x84a5\x87V[\x90Pa\x12\x16V[a\x15\x91\x82\x84a5\x87V[a\x15\x80\x90`\x01a4\xE1V[a\x15\xA4a\x1D\x15V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\x15\xC3WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\x15\xE1WP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\x15\xFFW`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[_Q` a:\xA3_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x16VWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x16qWP0;\x15[\x90P\x81\x15\x80\x15a\x16\x7FWP\x80\x15[\x15a\x16\x9DW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x16\xC7W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x16\xD0\x86a\x1F\xCFV[a\x16\xD8a\x1F\xE0V[a\x16\xE3\x89\x89\x89a\x1F\xE8V[\x83\x15a\x17)W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x17YWP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x17wW`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x84Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x17\xB0WP`\x06T` \x85\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x17\xCEW`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x17\xDB\x84`@\x01Qa!\x14V[a\x17\xE8\x83` \x01Qa!\x14V[a\x17\xF5\x83`@\x01Qa!\x14V[a\x18\x02\x83``\x01Qa!\x14V[_a\x18\x0Ba\x13\xD8V[` \x86\x01Q`\nT\x91\x92P_\x91a\x18+\x91\x90`\x01`\x01`@\x1B\x03\x16a\x15-V[`\x10T\x90\x91P`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x10a\x18oWa\x18Q\x86` \x01Qa\x11\x87V[\x15a\x18oW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x10T`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x11\x15a\x19\x1BW`\x02a\x18\x92\x83\x83a5AV[`\x01`\x01`@\x1B\x03\x16\x10a\x18\xB9W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x18\xC4\x82`\x01a4\xE1V[`\x01`\x01`@\x1B\x03\x16\x81`\x01`\x01`@\x1B\x03\x16\x14\x80\x15a\x18\xFDWP`\x06Ta\x18\xFB\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16a\x11%V[\x15[\x15a\x19\x1BW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x19'\x86\x86\x86\x86a!\x84V[\x85Q`\x06\x80T` \x89\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x81\x16\x93\x90\x93\x17\x17\x90U`@\x87\x01Q`\x07U`\x0F\x85\x90U`\x10T\x81\x16\x90\x82\x16\x10\x80\x15\x90a\x19\x84WPa\x19\x84\x86` \x01Qa\x11%V[\x15a\x19\xEEW\x84Q`\x0BU` \x85\x01Q`\x0CU`@\x85\x01Q`\rU``\x85\x01Q`\x0EU\x7F1\xEA\xBD\x90\x99\xFD\xB2]\xAC\xDD\xD2\x06\xAB\xFF\x871\x1EU4A\xFC\x9D\x0F\xCD\xEF \x10b\xD7\xE7\x07\x1Ba\x19\xD2\x82`\x01a4\xE1V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xA1[a\x19\xF9CB\x88a#\xAEV[\x85` \x01Q`\x01`\x01`@\x1B\x03\x16\x86_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x88`@\x01Q`@Qa\x1AI\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[_Q` a:\xA3_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x1A\x8FWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x1A\xADW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x17\x82U`\x05\x90\x85\x16\x11a\x1A\xF5W`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x0BU`\x01T`\x0CU`\x02T`\rU`\x03T`\x0EU`\n\x80T`\x01`\x01`@\x1B\x03\x85\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x90\x87\x16\x17\x17\x90Ua\x1B>\x83\x85a\x15-V[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x16`\x01`\x80\x1B`\x01`\x01`@\x1B\x03\x93\x84\x16\x02\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[\x80Q`\x06\x80T` \x84\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x16\x92\x90\x92\x17\x91\x90\x91\x17\x90U`@\x81\x01Q`\x07Ua\x13\x1CCB\x83a#\xAEV[`\x10T_\x90`\x01`@\x1B\x90\x04`\xFF\x16a\x1C\x13Wa\x1C\x0E\x83\x83a%\x97V[a\x1C$V[\x81`\x11T\x84a\x1C\"\x91\x90a5`V[\x11[\x93\x92PPPV[a\x1C3a\x1D\x15V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x1CaW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x13\x1C\x81a\x1F_V[a\x1Cu`\t_a+[V[_[\x81Q\x81\x10\x15a\x13:W`\t\x82\x82\x81Q\x81\x10a\x1C\x94Wa\x1C\x94a5sV[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q\x82T`\x01\x81\x81\x01\x85U_\x94\x85R\x93\x83\x90 \x82Q`\x02\x90\x92\x02\x01\x80T\x93\x83\x01Q`@\x84\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x92\x82\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x97\x16\x91\x90\x94\x16\x17\x94\x90\x94\x17\x93\x90\x93\x16\x17\x82U``\x01Q\x90\x82\x01U\x01a\x1CwV[3a\x1DG\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x13\xC5W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x1CXV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1D\xF6WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1D\xEA_Q` a:\x83_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x13\xC5W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1E\x1Ca\x1D\x15V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\n\x12V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1E\xAFWP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1E\xAC\x91\x81\x01\x90a5\xB4V[`\x01[a\x1E\xD7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x1CXV[_Q` a:\x83_9_Q\x90_R\x81\x14a\x1F\x07W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x1CXV[a\x1F\x11\x83\x83a&\xEFV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x13\xC5W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1F\xD7a'DV[a\x13\x1C\x81a'zV[a\x13\xC5a'DV[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a \x0CWP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a \x19WP` \x82\x01Q\x15[\x80a &WP`@\x82\x01Q\x15[\x80a 3WP``\x82\x01Q\x15[\x80a =WP\x81Q\x15[\x80a OWPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a cWPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a \x81W`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x13:W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x1CXV[_a!\x8Da\nfV[\x90Pa!\x97a+yV[`\x0CT\x81R`\rT` \x80\x83\x01\x91\x90\x91R`\x0ET`@\x83\x01R`\x0BT``\x80\x84\x01\x91\x90\x91R`\nT\x91\x88\x01Q\x90\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x91\x04\x81\x16\x91\x16\x10\x80\x15\x90a!\xEFWPa!\xEF\x87` \x01Qa\x11%V[\x15a\"8W`@\x80Q\x87Q` \x80\x83\x01\x91\x90\x91R\x88\x01Q\x81\x83\x01R\x90\x87\x01Q``\x80\x83\x01\x91\x90\x91R\x87\x01Q`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\"sV[`@\x80Q`\x0BT` \x82\x01R`\x0CT\x91\x81\x01\x91\x90\x91R`\rT``\x82\x01R`\x0ET`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P[`@\x80Q\x88Q`\x01`\x01`@\x1B\x03\x90\x81\x16` \x80\x84\x01\x91\x90\x91R\x8A\x01Q\x16\x81\x83\x01R\x90\x88\x01Q``\x82\x01R_\x90`\x80\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra\"\xC4\x91\x84\x90\x89\x90` \x01a5\xE2V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x80Q` \x90\x91\x01 \x90Pa#\x08\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x82a6\x04V[`\x80\x84\x01R`@QcT\xE8\xBDg`\xE0\x1B\x81Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90cT\xE8\xBDg\x90a#H\x90\x87\x90\x87\x90\x8A\x90`\x04\x01a7\xF9V[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a#cW=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a#\x87\x91\x90a:\x19V[a#\xA4W`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPPPV[`\tT\x15\x80\x15\x90a$#WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a#\xEEWa#\xEEa5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta$\x18\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a5AV[`\x01`\x01`@\x1B\x03\x16\x11[\x15a$\xB6W`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a$PWa$Pa5sV[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a$\x90\x83a:8V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[`\tT_\x90C\x84\x11\x80a%\xA8WP\x80\x15[\x80a%\xF2WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a%\xD6Wa%\xD6a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a&\x10W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a&\x1E`\x01\x85a5`V[\x90P[\x81a&\xBAW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a&\xBAW\x86`\t\x82\x81T\x81\x10a&SWa&Sa5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a&\xA8W`\x01\x91P`\t\x81\x81T\x81\x10a&\x88Wa&\x88a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa&\xBAV[\x80a&\xB2\x81a:bV[\x91PPa&!V[\x81a&\xD8W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a&\xE3\x84\x89a5`V[\x11\x97\x96PPPPPPPV[a&\xF8\x82a'\x82V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a'<Wa\x1F\x11\x82\x82a'\xE5V[a\x13:a(WV[_Q` a:\xA3_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\x13\xC5W`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1C3a'DV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a'\xB7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x1CXV[_Q` a:\x83_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa(\x01\x91\x90a:wV[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a(9W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a(>V[``\x91P[P\x91P\x91Pa(N\x85\x83\x83a(vV[\x95\x94PPPPPV[4\x15a\x13\xC5W`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a(\x86Wa\x1C\x0E\x82a(\xCDV[\x81Q\x15\x80\x15a(\x9DWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a(\xC6W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x1CXV[P\x92\x91PPV[\x80Q\x15a(\xDDW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a))`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)i`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\x89`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xA9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xC9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xE9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\t`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*)`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*i`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\x89`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xA9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xC9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xE9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+\t`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+)`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01_\x81R` \x01_\x81RP\x90V[P\x80T_\x82U`\x02\x02\x90_R` _ \x90\x81\x01\x90a\x13\x1C\x91\x90a+\x97V[`@Q\x80`\xA0\x01`@R\x80`\x05\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[[\x80\x82\x11\x15a+\xBCW\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U_`\x01\x82\x01U`\x02\x01a+\x98V[P\x90V[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a+\xE6W__\xFD[a\x1C$\x82a+\xC0V[_` \x82\x84\x03\x12\x15a+\xFFW__\xFD[P5\x91\x90PV[_a\x05\0\x82\x01\x90P\x82Q\x82R` \x83\x01Q` \x83\x01R`@\x83\x01Qa,8`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x83\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x83\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x83\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x83\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x83\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x83\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x83\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x83\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x83\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x83\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x83\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x83\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x83\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x83\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x83\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x83\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x83\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x83\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x90\x92\x01Qa\x04\xE0\x90\x91\x01R\x90V[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a.-W__\xFD[a\x1C$\x82a.\x07V[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.mWa.ma.6V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.mWa.ma.6V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.\xBDWa.\xBDa.6V[`@R\x91\x90PV[_``\x82\x84\x03\x12\x15a.\xD5W__\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.\xF7Wa.\xF7a.6V[`@R\x90P\x80a/\x06\x83a.\x07V[\x81Ra/\x14` \x84\x01a.\x07V[` \x82\x01R`@\x92\x83\x015\x92\x01\x91\x90\x91R\x91\x90PV[_`@\x82\x84\x03\x12\x15a/:W__\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/\\Wa/\\a.6V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a/\x87W__\xFD[a/\x8Fa.JV[\x90Pa/\x9B\x83\x83a/*V[\x81Ra/\xAA\x83`@\x84\x01a/*V[` \x82\x01Ra/\xBC\x83`\x80\x84\x01a/*V[`@\x82\x01Ra/\xCE\x83`\xC0\x84\x01a/*V[``\x82\x01Ra/\xE1\x83a\x01\0\x84\x01a/*V[`\x80\x82\x01Ra/\xF4\x83a\x01@\x84\x01a/*V[`\xA0\x82\x01Ra0\x07\x83a\x01\x80\x84\x01a/*V[`\xC0\x82\x01Ra0\x1A\x83a\x01\xC0\x84\x01a/*V[`\xE0\x82\x01Ra0-\x83a\x02\0\x84\x01a/*V[a\x01\0\x82\x01Ra0A\x83a\x02@\x84\x01a/*V[a\x01 \x82\x01Ra0U\x83a\x02\x80\x84\x01a/*V[a\x01@\x82\x01Ra0i\x83a\x02\xC0\x84\x01a/*V[a\x01`\x82\x01Ra0}\x83a\x03\0\x84\x01a/*V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[__a\x04\xE0\x83\x85\x03\x12\x15a1\x13W__\xFD[a1\x1D\x84\x84a.\xC5V[\x91Pa1,\x84``\x85\x01a/vV[\x90P\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a1XW__\xFD[a\x1C$\x82a15V[__`@\x83\x85\x03\x12\x15a1rW__\xFD[a1{\x83a+\xC0V[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a1\x95W__\xFD[\x83\x01`\x1F\x81\x01\x85\x13a1\xA5W__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a1\xBEWa1\xBEa.6V[a1\xD1`\x1F\x82\x01`\x1F\x19\x16` \x01a.\x95V[\x81\x81R\x86` \x83\x85\x01\x01\x11\x15a1\xE5W__\xFD[\x81` \x84\x01` \x83\x017_` \x83\x83\x01\x01R\x80\x93PPPP\x92P\x92\x90PV[_`\x80\x82\x84\x03\x12\x15a2\x14W__\xFD[a2\x1Ca.sV[\x825\x81R` \x80\x84\x015\x90\x82\x01R`@\x80\x84\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a2WW__\xFD[a\x1C$\x83\x83a2\x04V[___a\x05`\x84\x86\x03\x12\x15a2tW__\xFD[a2~\x85\x85a.\xC5V[\x92Pa2\x8D\x85``\x86\x01a2\x04V[\x91Pa2\x9C\x85`\xE0\x86\x01a/vV[\x90P\x92P\x92P\x92V[__`@\x83\x85\x03\x12\x15a2\xB6W__\xFD[a2\xBF\x83a.\x07V[\x91Pa1,` \x84\x01a.\x07V[____a\x01 \x85\x87\x03\x12\x15a2\xE1W__\xFD[a2\xEB\x86\x86a.\xC5V[\x93Pa2\xFA\x86``\x87\x01a2\x04V[\x92Pa3\x08`\xE0\x86\x01a15V[\x91Pa3\x17a\x01\0\x86\x01a+\xC0V[\x90P\x92\x95\x91\x94P\x92PV[____a\x05\x80\x85\x87\x03\x12\x15a36W__\xFD[a3@\x86\x86a.\xC5V[\x93Pa3O\x86``\x87\x01a2\x04V[\x92P`\xE0\x85\x015\x91Pa3\x17\x86a\x01\0\x87\x01a/vV[` \x81R_\x82Q\x80` \x84\x01R\x80` \x85\x01`@\x85\x01^_`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[_``\x82\x84\x03\x12\x15a3\xABW__\xFD[a\x1C$\x83\x83a.\xC5V[__`@\x83\x85\x03\x12\x15a3\xC6W__\xFD[PP\x805\x92` \x90\x91\x015\x91PV[_` \x82\x84\x03\x12\x15a3\xE5W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a3\xFAW__\xFD[\x82\x01`\x1F\x81\x01\x84\x13a4\nW__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a4#Wa4#a.6V[a42` \x82`\x05\x1B\x01a.\x95V[\x80\x82\x82R` \x82\x01\x91P` \x83`\x07\x1B\x85\x01\x01\x92P\x86\x83\x11\x15a4SW__\xFD[` \x84\x01\x93P[\x82\x84\x10\x15a4\xC3W`\x80\x84\x88\x03\x12\x15a4qW__\xFD[a4ya.sV[a4\x82\x85a.\x07V[\x81Ra4\x90` \x86\x01a.\x07V[` \x82\x01Ra4\xA1`@\x86\x01a.\x07V[`@\x82\x01R``\x85\x81\x015\x90\x82\x01R\x82R`\x80\x90\x93\x01\x92` \x90\x91\x01\x90a4ZV[\x96\x95PPPPPPV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a5,Wa5,a5\0V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x06\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[\x81\x81\x03\x81\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a5\x9FWa5\x9Fa5\0V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x04\x91PP\x92\x91PPV[_` \x82\x84\x03\x12\x15a5\xC4W__\xFD[PQ\x91\x90PV[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a5\xF6a5\xF0\x83\x87a5\xCBV[\x85a5\xCBV[\x92\x83RPP` \x01\x92\x91PPV[_\x82a6\x12Wa6\x12a5\0V[P\x06\x90V[\x80_[`\x05\x81\x10\x15a69W\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a6\x1AV[PPPPV[a6T\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xA0\x81\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xC0\x81\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RP`\xE0\x81\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01\0\x81\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01 \x81\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01@\x81\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01`\x81\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\x80\x81\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xA0\x81\x01Qa\x03@\x83\x01Ra\x01\xC0\x81\x01Qa\x03`\x83\x01Ra\x01\xE0\x81\x01Qa\x03\x80\x83\x01Ra\x02\0\x81\x01Qa\x03\xA0\x83\x01Ra\x02 \x81\x01Qa\x03\xC0\x83\x01Ra\x02@\x81\x01Qa\x03\xE0\x83\x01Ra\x02`\x81\x01Qa\x04\0\x83\x01Ra\x02\x80\x81\x01Qa\x04 \x83\x01Ra\x02\xA0\x81\x01Qa\x04@\x83\x01Ra\x02\xC0\x01Qa\x04`\x90\x91\x01RV[_a\n \x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa8+`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x85\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x85\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x85\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x85\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x85\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x85\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x85\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x85\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x85\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x85\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x85\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x85\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x85\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x85\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x85\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra:\x03a\x05\0\x83\x01\x85a6\x17V[a:\x11a\x05\xA0\x83\x01\x84a6?V[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a:)W__\xFD[\x81Q\x80\x15\x15\x81\x14a\x1C$W__\xFD[_`\x01`\x01`@\x1B\x03\x82\x16`\x01`\x01`@\x1B\x03\x81\x03a:YWa:Ya4\xCDV[`\x01\x01\x92\x91PPV[_\x81a:pWa:pa4\xCDV[P_\x19\x01\x90V[_a\x1C$\x82\x84a5\xCBV\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405260043610610275575f3560e01c8063715018a61161014a578063aabd5db3116100be578063d24d933d11610078578063d24d933d146108ab578063e0303301146108da578063f0682054146108f9578063f2fde38b14610918578063f567616014610937578063f9e50d1914610956575f5ffd5b8063aabd5db3146107bb578063ad3cb1cc146107da578063b33bc49114610817578063b5adea3c14610836578063c23b9e9e14610855578063c8e5e4981461088d575f5ffd5b80638da5cb5b1161010f5780638da5cb5b146106b857806390c14390146106f457806396c1ca6114610713578063998328e8146107325780639baa3cc9146107475780639fdb54a714610766575f5ffd5b8063715018a61461061b578063757c37ad1461062f5780637667180814610649578063826e41fc1461065d5780638584d23f1461067c575f5ffd5b8063313df7b1116101ec578063426d3194116101a6578063426d319414610568578063433dba9f146105895780634f1ef286146105a857806352d1902d146105bb578063623a1338146105cf57806369cc6a0414610607575f5ffd5b8063313df7b114610481578063378ec23b146104b857806338e454b1146104d45780633c23b6db146104e85780633ed55b7b14610525578063416827441461054b575f5ffd5b8063167ac6181161023d578063167ac618146103855780632063d4f7146103a457806325297427146103c35780632d52aad6146103f25780632f79889d14610424578063300c89dd14610462575f5ffd5b8063013fa5fc1461027957806302b592f31461029a5780630625e19b146102f75780630d8e6e2c1461033957806312173c2c14610364575b5f5ffd5b348015610284575f5ffd5b50610298610293366004612bd6565b61096a565b005b3480156102a5575f5ffd5b506102b96102b4366004612bef565b610a1d565b6040516102ee94939291906001600160401b039485168152928416602084015292166040820152606081019190915260800190565b60405180910390f35b348015610302575f5ffd5b50600b54600c54600d54600e546103199392919084565b6040805194855260208501939093529183015260608201526080016102ee565b348015610344575f5ffd5b5060408051600381525f60208201819052918101919091526060016102ee565b34801561036f575f5ffd5b50610378610a66565b6040516102ee9190612c06565b348015610390575f5ffd5b5061029861039f366004612e1d565b611095565b3480156103af575f5ffd5b506102986103be366004613101565b61110c565b3480156103ce575f5ffd5b506103e26103dd366004612e1d565b611125565b60405190151581526020016102ee565b3480156103fd575f5ffd5b5061029861040c366004612bef565b6010805460ff60401b1916600160401b179055601155565b34801561042f575f5ffd5b5060085461044a90600160c01b90046001600160401b031681565b6040516001600160401b0390911681526020016102ee565b34801561046d575f5ffd5b506103e261047c366004612e1d565b611187565b34801561048c575f5ffd5b506008546104a0906001600160a01b031681565b6040516001600160a01b0390911681526020016102ee565b3480156104c3575f5ffd5b50435b6040519081526020016102ee565b3480156104df575f5ffd5b5061029861121c565b3480156104f3575f5ffd5b50610298610502366004612e1d565b600a805467ffffffffffffffff19166001600160401b0392909216919091179055565b348015610530575f5ffd5b50600a5461044a90600160401b90046001600160401b031681565b348015610556575f5ffd5b506010546001600160401b031661044a565b348015610573575f5ffd5b505f546001546002546003546103199392919084565b348015610594575f5ffd5b506102986105a3366004613148565b61130b565b6102986105b6366004613161565b61131f565b3480156105c6575f5ffd5b506104c661133e565b3480156105da575f5ffd5b506102986105e9366004613247565b8051600b556020810151600c556040810151600d5560600151600e55565b348015610612575f5ffd5b50610298611359565b348015610626575f5ffd5b506102986113c7565b34801561063a575f5ffd5b506102986103be366004613261565b348015610654575f5ffd5b5061044a6113d8565b348015610668575f5ffd5b506008546001600160a01b031615156103e2565b348015610687575f5ffd5b5061069b610696366004612bef565b611402565b604080519283526001600160401b039091166020830152016102ee565b3480156106c3575f5ffd5b507f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b03166104a0565b3480156106ff575f5ffd5b5061044a61070e3660046132a5565b61152d565b34801561071e575f5ffd5b5061029861072d366004613148565b61159c565b34801561073d575f5ffd5b506104c6600f5481565b348015610752575f5ffd5b506102986107613660046132cd565b611625565b348015610771575f5ffd5b50600654600754610795916001600160401b0380821692600160401b909204169083565b604080516001600160401b039485168152939092166020840152908201526060016102ee565b3480156107c6575f5ffd5b506102986107d5366004613322565b611734565b3480156107e5575f5ffd5b5061080a604051806040016040528060058152602001640352e302e360dc1b81525081565b6040516102ee9190613366565b348015610822575f5ffd5b506102986108313660046132a5565b611a59565b348015610841575f5ffd5b5061029861085036600461339b565b611baa565b348015610860575f5ffd5b5060085461087890600160a01b900463ffffffff1681565b60405163ffffffff90911681526020016102ee565b348015610898575f5ffd5b506102986010805460ff60401b19169055565b3480156108b6575f5ffd5b50600454600554610795916001600160401b0380821692600160401b909204169083565b3480156108e5575f5ffd5b506103e26108f43660046133b5565b611bf1565b348015610904575f5ffd5b50600a5461044a906001600160401b031681565b348015610923575f5ffd5b50610298610932366004612bd6565b611c2b565b348015610942575f5ffd5b506102986109513660046133d5565b611c6a565b348015610961575f5ffd5b506009546104c6565b610972611d15565b6001600160a01b0381166109995760405163e6c4247b60e01b815260040160405180910390fd5b6008546001600160a01b03908116908216036109c85760405163a863aec960e01b815260040160405180910390fd5b600880546001600160a01b0319166001600160a01b0383169081179091556040519081527f8017bb887fdf8fca4314a9d40f6e73b3b81002d67e5cfa85d88173af6aa46072906020015b60405180910390a150565b60098181548110610a2c575f80fd5b5f918252602090912060029091020180546001909101546001600160401b038083169350600160401b8304811692600160801b9004169084565b610a6e6128f6565b620100008152600560208201527f0577cf9bf82e9f08a752d837709088c137f9cd88cd95b03b27e6c54be38e929f6040820151527f08ff03a27cbe6a980ec660379d28ee76975cef42ba5806b9362632f26a6dfb206020604083015101527f0f4fc36c1ca15b9185959287c9e9773283e15395a92701087d38704fdd0c17ea6060820151527f14b01587bb51202bae56dc992251795655dc44395193f22b39a68e45e501e3f36020606083015101527f0a4ce9176328422b484832155bb3333285a6e0e030667761c9576f502e8b1b1b6080820151527f0a16f5414614d648678c637a0931c14191b232bb1a5d23f6c1673af0596603a66020608083015101527f1c1c5ff424e5a4daef97061e02327a6b8e5bbecc8b55d7f51889c6c391c6fe4b60a0820151527f2bff3ce7c876f111de5e8a0caa063dce26d07b87c31220a7d739a0d88c39d8ea602060a083015101527f2d32dce8d080a1a62afd7360e6550218c76ed1ba7eefe29c0274f5dee372531260c0820151527f260f1d1a8ae6d936ae1f284076ad0a374334b1545cb8e1b04eb45539e36e932e602060c083015101527f25cc09d27d1f2758c7edabfc4ccb753a1e101f07cdaea612bb1bb9e66b57acb160e0820151527f2f87735fcbe11c70121edff1d520c62fe9988ddd15937c3f430aa1dfa467308f602060e083015101527f073aad1f4c59f4f2fffc5f489c7b81f8b6fd69b77328ec8b0093071a41a9b760610100820151527f25bbaae87750964f4a0eabced3363b4c120de7acde7ce3d619d42a98674bcc17602061010083015101527f140f1b0a71a0df100cf0af671bbecc506160ef2cbdc24a976faa459312c1425c610120820151527f0e6ca0733e1f4553bbcca3a7b314c408083834103b20ab3faa2cece9ef7aa980602061012083015101527f0c6cc1b3d364f405fc77865198c954edb87714765f879b057f4720fe139ce861610140820151527f0efad53ac051aee07f7f92c34a779b5f9bc5803a612611710f907bbda3630330602061014083015101527f299b5d26b5da318cfd6ee23542444fbd54a221ae40b043ee7b59e4be8a7cf0bc610160820151527f11bfb70de4b59c717cb73ba6d923f79447d10eb6e7ea91ab8a6d261184ab7ddd602061016083015101527f2e646df45071a61803f5e8e92b75c156cab05f38cf0f84df751cf2442f224482610180820151527eed98ef227ce627fe2e56a11e32e1b490d99dee481c2d812050d2ffbf1ca0df602061018083015101527f10e36bbdf42d56359d2e87932b781b62271c3e6e1b0287d13e5d2478cd96baab6101a0820151527f02d5037fc2543b02b2e2e41f6abca9137206f87b67549abd7d68951b8801aa8d60206101a083015101527f16e1122401b64f03c2da5ebd4d52021e1b86c9bb3d4a6be643ee057731be4efd6101c0820151527f1bf6cca2dc0b7aa23fecaf058d11bbf4c06c6d85b03b6b27e1f37ac870357dd860206101c083015101527f2dc5951c5e5f891003f77d9940ba8a458b9608a3cb3b926d1f0a226900964dca6101e0820151527f0d1148213d633db532d6ec4352ff41e521e016c8fadc8db42c4bf9eb166ad38160206101e083015101527f2d7fed4088b6e73aba7ae1c3be39ab9697137ac1c450aeb971878dcfda7e3281610200820151527f18dc0de1ef57df9c8f492adeba183c610ad5eae06996446e3478076360b29e76602061020083015101527f1b9cd9b879f242f331103d6ac1f52623937c0801a21ccdd23b4dc9d4caf77d9c610220820151527f1dea6a5f2cffff1c5576b73075a96237fdf95153ef68cbf4372b7f1f3806d84a602061022083015101527f012617672c1d38557f385f9ff0d9fce130e8c8cfde27f65b908db4fd30d30d72610240820151527f0ee4840561aa90b4f40257d51c71ed03e0a39069207a5927677cd14668d57477602061024083015101527f13596792a5ef6df4cdbb46effaa45f91d3cc0401a64583cdd74b792310e3fdbd610260820151527f192e2c61cbb5b730ef86a950633c5972c4f956a053c72572498a1e1212487039602061026083015101527fb0838893ec1f237e8b07323b0744599f4e97b598b3b589bcc2bc37b8d5c418016102808201527fc18393c0fa30fe4e8b038e357ad851eae8de9107584effe7c7f1f651b2010e266102a082015290565b61109d611d15565b600a80546fffffffffffffffff0000000000000000198116600160401b6001600160401b038581168202928317948590556110e39491909104811692811691161761152d565b600a60106101000a8154816001600160401b0302191690836001600160401b0316021790555050565b604051634e405c8d60e01b815260040160405180910390fd5b5f6001600160401b03821615806111455750600a546001600160401b0316155b1561115157505f919050565b600a546001600160401b03166111688360056134e1565b6111729190613514565b6001600160401b03161592915050565b919050565b5f6001600160401b03821615806111a75750600a546001600160401b0316155b156111b357505f919050565b600a546111c9906001600160401b031683613514565b6001600160401b031615806112165750600a546111f1906005906001600160401b0316613541565b600a546001600160401b039182169161120b911684613514565b6001600160401b0316115b92915050565b5f516020613aa35f395f51905f52805460039190600160401b900460ff1680611252575080546001600160401b03808416911610155b156112705760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b9081178355600a546112aa929181048216911661152d565b6010805467ffffffffffffffff19166001600160401b03928316179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15050565b611313611d15565b61131c8161159c565b50565b611327611d70565b61133082611e14565b61133a8282611e55565b5050565b5f611347611f16565b505f516020613a835f395f51905f5290565b611361611d15565b6008546001600160a01b0316156113ac57600880546001600160a01b03191690556040517f9a5f57de856dd668c54dd95e5c55df93432171cbca49a8776d5620ea59c02450905f90a1565b60405163a863aec960e01b815260040160405180910390fd5b565b6113cf611d15565b6113c55f611f5f565b600654600a545f916113fd916001600160401b03600160401b9092048216911661152d565b905090565b600980545f91829190611416600183613560565b8154811061142657611426613573565b5f918252602090912060029091020154600160801b90046001600160401b0316841061146557604051631856a49960e21b815260040160405180910390fd5b600854600160c01b90046001600160401b03165b8181101561152657846009828154811061149557611495613573565b5f918252602090912060029091020154600160801b90046001600160401b0316111561151e57600981815481106114ce576114ce613573565b905f5260205f20906002020160010154600982815481106114f1576114f1613573565b905f5260205f2090600202015f0160109054906101000a90046001600160401b0316935093505050915091565b600101611479565b5050915091565b5f816001600160401b03165f0361154557505f611216565b826001600160401b03165f0361155d57506001611216565b6115678284613514565b6001600160401b03165f03611587576115808284613587565b9050611216565b6115918284613587565b6115809060016134e1565b6115a4611d15565b610e108163ffffffff1610806115c357506301e133808163ffffffff16115b806115e1575060085463ffffffff600160a01b909104811690821611155b156115ff576040516307a5077760e51b815260040160405180910390fd5b6008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b5f516020613aa35f395f51905f528054600160401b810460ff1615906001600160401b03165f811580156116565750825b90505f826001600160401b031660011480156116715750303b155b90508115801561167f575080155b1561169d5760405163f92ee8a960e01b815260040160405180910390fd5b845467ffffffffffffffff1916600117855583156116c757845460ff60401b1916600160401b1785555b6116d086611fcf565b6116d8611fe0565b6116e3898989611fe8565b831561172957845460ff60401b19168555604051600181527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a15b505050505050505050565b6008546001600160a01b03161515801561175957506008546001600160a01b03163314155b15611777576040516301474c8f60e71b815260040160405180910390fd5b60065484516001600160401b0391821691161115806117b0575060065460208501516001600160401b03600160401b9092048216911611155b156117ce5760405163051c46ef60e01b815260040160405180910390fd5b6117db8460400151612114565b6117e88360200151612114565b6117f58360400151612114565b6118028360600151612114565b5f61180b6113d8565b6020860151600a549192505f9161182b91906001600160401b031661152d565b6010549091506001600160401b039081169082161061186f576118518660200151611187565b1561186f5760405163080ae8d960e01b815260040160405180910390fd5b6010546001600160401b03908116908216111561191b5760026118928383613541565b6001600160401b0316106118b95760405163080ae8d960e01b815260040160405180910390fd5b6118c48260016134e1565b6001600160401b0316816001600160401b03161480156118fd57506006546118fb90600160401b90046001600160401b0316611125565b155b1561191b5760405163080ae8d960e01b815260040160405180910390fd5b61192786868686612184565b85516006805460208901516001600160401b03908116600160401b026001600160801b0319909216938116939093171790556040870151600755600f85905560105481169082161080159061198457506119848660200151611125565b156119ee578451600b556020850151600c556040850151600d556060850151600e557f31eabd9099fdb25dacddd206abff87311e553441fc9d0fcdef201062d7e7071b6119d28260016134e1565b6040516001600160401b03909116815260200160405180910390a15b6119f94342886123ae565b85602001516001600160401b0316865f01516001600160401b03167fa04a773924505a418564363725f56832f5772e6b8d0dbd6efce724dfe803dae68860400151604051611a4991815260200190565b60405180910390a3505050505050565b5f516020613aa35f395f51905f52805460029190600160401b900460ff1680611a8f575080546001600160401b03808416911610155b15611aad5760405163f92ee8a960e01b815260040160405180910390fd5b805468ffffffffffffffffff19166001600160401b0380841691909117600160401b178255600590851611611af5576040516350dd03f760e11b815260040160405180910390fd5b5f54600b55600154600c55600254600d55600354600e55600a80546001600160401b03858116600160401b026001600160801b031990921690871617179055611b3e838561152d565b600a805467ffffffffffffffff60801b1916600160801b6001600160401b0393841602179055815460ff60401b1916825560405190831681527fc7f505b2f371ae2175ee4913f4499e1f2633a7b5936321eed1cdaeb6115181d29060200160405180910390a150505050565b80516006805460208401516001600160401b03908116600160401b026001600160801b0319909216931692909217919091179055604081015160075561131c4342836123ae565b6010545f90600160401b900460ff16611c1357611c0e8383612597565b611c24565b8160115484611c229190613560565b115b9392505050565b611c33611d15565b6001600160a01b038116611c6157604051631e4fbdf760e01b81525f60048201526024015b60405180910390fd5b61131c81611f5f565b611c7560095f612b5b565b5f5b815181101561133a576009828281518110611c9457611c94613573565b6020908102919091018101518254600181810185555f94855293839020825160029092020180549383015160408401516001600160401b03908116600160801b0267ffffffffffffffff60801b19928216600160401b026001600160801b031990971691909416179490941793909316178255606001519082015501611c77565b33611d477f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c199300546001600160a01b031690565b6001600160a01b0316146113c55760405163118cdaa760e01b8152336004820152602401611c58565b306001600160a01b037f0000000000000000000000000000000000000000000000000000000000000000161480611df657507f00000000000000000000000000000000000000000000000000000000000000006001600160a01b0316611dea5f516020613a835f395f51905f52546001600160a01b031690565b6001600160a01b031614155b156113c55760405163703e46dd60e11b815260040160405180910390fd5b611e1c611d15565b6040516001600160a01b03821681527ff78721226efe9a1bb678189a16d1554928b9f2192e2cb93eeda83b79fa40007d90602001610a12565b816001600160a01b03166352d1902d6040518163ffffffff1660e01b8152600401602060405180830381865afa925050508015611eaf575060408051601f3d908101601f19168201909252611eac918101906135b4565b60015b611ed757604051634c9c8ce360e01b81526001600160a01b0383166004820152602401611c58565b5f516020613a835f395f51905f528114611f0757604051632a87526960e21b815260048101829052602401611c58565b611f1183836126ef565b505050565b306001600160a01b037f000000000000000000000000000000000000000000000000000000000000000016146113c55760405163703e46dd60e11b815260040160405180910390fd5b7f9016d09d72d40fdae2fd8ceac6b6234c7706214fd39c1cd1e609a0528c19930080546001600160a01b031981166001600160a01b03848116918217845560405192169182907f8be0079c531659141344cd1fd0a4f28419497f9722a3daafe3b4186f6b6457e0905f90a3505050565b611fd7612744565b61131c8161277a565b6113c5612744565b82516001600160401b031615158061200c575060208301516001600160401b031615155b8061201957506020820151155b8061202657506040820151155b8061203357506060820151155b8061203d57508151155b8061204f5750610e108163ffffffff16105b8061206357506301e133808163ffffffff16115b15612081576040516350dd03f760e11b815260040160405180910390fd5b8251600480546020808701516001600160401b03908116600160401b026001600160801b0319938416919095169081178517909355604096870151600581905586515f5590860151600155958501516002556060909401516003556006805490941617179091556007919091556008805463ffffffff909216600160a01b0263ffffffff60a01b19909216919091179055565b7f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000181108061133a5760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401611c58565b5f61218d610a66565b9050612197612b79565b600c548152600d54602080830191909152600e546040830152600b54606080840191909152600a549188015190916001600160401b03600160401b90910481169116108015906121ef57506121ef8760200151611125565b15612238576040805187516020808301919091528801518183015290870151606080830191909152870151608082015260a0016040516020818303038152906040529050612273565b60408051600b546020820152600c5491810191909152600d546060820152600e54608082015260a00160405160208183030381529060405290505b6040805188516001600160401b039081166020808401919091528a015116818301529088015160608201525f9060800160408051601f19818403018152908290526122c491849089906020016135e2565b60408051601f19818403018152919052805160209091012090506123087f30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f000000182613604565b60808401526040516354e8bd6760e01b815273ffffffffffffffffffffffffffffffffffffffff906354e8bd679061234890879087908a906004016137f9565b602060405180830381865af4158015612363573d5f5f3e3d5ffd5b505050506040513d601f19601f820116820180604052508101906123879190613a19565b6123a4576040516309bde33960e01b815260040160405180910390fd5b5050505050505050565b60095415801590612423575060085460098054600160a01b830463ffffffff1692600160c01b90046001600160401b03169081106123ee576123ee613573565b5f91825260209091206002909102015461241890600160401b90046001600160401b031684613541565b6001600160401b0316115b156124b657600854600980549091600160c01b90046001600160401b031690811061245057612450613573565b5f9182526020822060029091020180546001600160c01b03191681556001015560088054600160c01b90046001600160401b031690601861249083613a38565b91906101000a8154816001600160401b0302191690836001600160401b03160217905550505b604080516080810182526001600160401b03948516815292841660208085019182528301518516848301908152929091015160608401908152600980546001810182555f91909152935160029094027f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7af81018054935194518716600160801b0267ffffffffffffffff60801b19958816600160401b026001600160801b03199095169690971695909517929092179290921693909317909155517f6e1540171b6c0c960b71a7020d9f60077f6af931a8bbf590da0223dacf75c7b090910155565b6009545f90438411806125a8575080155b806125f25750600854600980549091600160c01b90046001600160401b03169081106125d6576125d6613573565b5f9182526020909120600290910201546001600160401b031684105b156126105760405163b0b4387760e01b815260040160405180910390fd5b5f808061261e600185613560565b90505b816126ba57600854600160c01b90046001600160401b031681106126ba57866009828154811061265357612653613573565b5f9182526020909120600290910201546001600160401b0316116126a857600191506009818154811061268857612688613573565b5f9182526020909120600290910201546001600160401b031692506126ba565b806126b281613a62565b915050612621565b816126d85760405163b0b4387760e01b815260040160405180910390fd5b856126e38489613560565b11979650505050505050565b6126f882612782565b6040516001600160a01b038316907fbc7cd75a20ee27fd9adebab32041f755214dbc6bffa90cc0225b39da2e5c2d3b905f90a280511561273c57611f1182826127e5565b61133a612857565b5f516020613aa35f395f51905f5254600160401b900460ff166113c557604051631afcd79f60e31b815260040160405180910390fd5b611c33612744565b806001600160a01b03163b5f036127b757604051634c9c8ce360e01b81526001600160a01b0382166004820152602401611c58565b5f516020613a835f395f51905f5280546001600160a01b0319166001600160a01b0392909216919091179055565b60605f5f846001600160a01b0316846040516128019190613a77565b5f60405180830381855af49150503d805f8114612839576040519150601f19603f3d011682016040523d82523d5f602084013e61283e565b606091505b509150915061284e858383612876565b95945050505050565b34156113c55760405163b398979f60e01b815260040160405180910390fd5b60608261288657611c0e826128cd565b815115801561289d57506001600160a01b0384163b155b156128c657604051639996b31560e01b81526001600160a01b0385166004820152602401611c58565b5092915050565b8051156128dd5780518082602001fd5b604051630a12f52160e11b815260040160405180910390fd5b604051806102c001604052805f81526020015f815260200161292960405180604001604052805f81526020015f81525090565b815260200161294960405180604001604052805f81526020015f81525090565b815260200161296960405180604001604052805f81526020015f81525090565b815260200161298960405180604001604052805f81526020015f81525090565b81526020016129a960405180604001604052805f81526020015f81525090565b81526020016129c960405180604001604052805f81526020015f81525090565b81526020016129e960405180604001604052805f81526020015f81525090565b8152602001612a0960405180604001604052805f81526020015f81525090565b8152602001612a2960405180604001604052805f81526020015f81525090565b8152602001612a4960405180604001604052805f81526020015f81525090565b8152602001612a6960405180604001604052805f81526020015f81525090565b8152602001612a8960405180604001604052805f81526020015f81525090565b8152602001612aa960405180604001604052805f81526020015f81525090565b8152602001612ac960405180604001604052805f81526020015f81525090565b8152602001612ae960405180604001604052805f81526020015f81525090565b8152602001612b0960405180604001604052805f81526020015f81525090565b8152602001612b2960405180604001604052805f81526020015f81525090565b8152602001612b4960405180604001604052805f81526020015f81525090565b81526020015f81526020015f81525090565b5080545f8255600202905f5260205f209081019061131c9190612b97565b6040518060a001604052806005906020820280368337509192915050565b5b80821115612bbc5780546001600160c01b03191681555f6001820155600201612b98565b5090565b80356001600160a01b0381168114611182575f5ffd5b5f60208284031215612be6575f5ffd5b611c2482612bc0565b5f60208284031215612bff575f5ffd5b5035919050565b5f6105008201905082518252602083015160208301526040830151612c38604084018280518252602090810151910152565b50606083015180516080840152602081015160a0840152506080830151805160c0840152602081015160e08401525060a0830151805161010084015260208101516101208401525060c0830151805161014084015260208101516101608401525060e0830151805161018084015260208101516101a08401525061010083015180516101c084015260208101516101e08401525061012083015180516102008401526020810151610220840152506101408301518051610240840152602081015161026084015250610160830151805161028084015260208101516102a08401525061018083015180516102c084015260208101516102e0840152506101a083015180516103008401526020810151610320840152506101c083015180516103408401526020810151610360840152506101e0830151805161038084015260208101516103a08401525061020083015180516103c084015260208101516103e08401525061022083015180516104008401526020810151610420840152506102408301518051610440840152602081015161046084015250610260830151805161048084015260208101516104a0840152506102808301516104c08301526102a0909201516104e09091015290565b80356001600160401b0381168114611182575f5ffd5b5f60208284031215612e2d575f5ffd5b611c2482612e07565b634e487b7160e01b5f52604160045260245ffd5b6040516102e081016001600160401b0381118282101715612e6d57612e6d612e36565b60405290565b604051608081016001600160401b0381118282101715612e6d57612e6d612e36565b604051601f8201601f191681016001600160401b0381118282101715612ebd57612ebd612e36565b604052919050565b5f60608284031215612ed5575f5ffd5b604051606081016001600160401b0381118282101715612ef757612ef7612e36565b604052905080612f0683612e07565b8152612f1460208401612e07565b6020820152604092830135920191909152919050565b5f60408284031215612f3a575f5ffd5b604080519081016001600160401b0381118282101715612f5c57612f5c612e36565b604052823581526020928301359281019290925250919050565b5f6104808284031215612f87575f5ffd5b612f8f612e4a565b9050612f9b8383612f2a565b8152612faa8360408401612f2a565b6020820152612fbc8360808401612f2a565b6040820152612fce8360c08401612f2a565b6060820152612fe1836101008401612f2a565b6080820152612ff4836101408401612f2a565b60a0820152613007836101808401612f2a565b60c082015261301a836101c08401612f2a565b60e082015261302d836102008401612f2a565b610100820152613041836102408401612f2a565b610120820152613055836102808401612f2a565b610140820152613069836102c08401612f2a565b61016082015261307d836103008401612f2a565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f6104e08385031215613113575f5ffd5b61311d8484612ec5565b915061312c8460608501612f76565b90509250929050565b803563ffffffff81168114611182575f5ffd5b5f60208284031215613158575f5ffd5b611c2482613135565b5f5f60408385031215613172575f5ffd5b61317b83612bc0565b915060208301356001600160401b03811115613195575f5ffd5b8301601f810185136131a5575f5ffd5b80356001600160401b038111156131be576131be612e36565b6131d1601f8201601f1916602001612e95565b8181528660208385010111156131e5575f5ffd5b816020840160208301375f602083830101528093505050509250929050565b5f60808284031215613214575f5ffd5b61321c612e73565b8235815260208084013590820152604080840135908201526060928301359281019290925250919050565b5f60808284031215613257575f5ffd5b611c248383613204565b5f5f5f6105608486031215613274575f5ffd5b61327e8585612ec5565b925061328d8560608601613204565b915061329c8560e08601612f76565b90509250925092565b5f5f604083850312156132b6575f5ffd5b6132bf83612e07565b915061312c60208401612e07565b5f5f5f5f61012085870312156132e1575f5ffd5b6132eb8686612ec5565b93506132fa8660608701613204565b925061330860e08601613135565b91506133176101008601612bc0565b905092959194509250565b5f5f5f5f6105808587031215613336575f5ffd5b6133408686612ec5565b935061334f8660608701613204565b925060e08501359150613317866101008701612f76565b602081525f82518060208401528060208501604085015e5f604082850101526040601f19601f83011684010191505092915050565b5f606082840312156133ab575f5ffd5b611c248383612ec5565b5f5f604083850312156133c6575f5ffd5b50508035926020909101359150565b5f602082840312156133e5575f5ffd5b81356001600160401b038111156133fa575f5ffd5b8201601f8101841361340a575f5ffd5b80356001600160401b0381111561342357613423612e36565b61343260208260051b01612e95565b8082825260208201915060208360071b850101925086831115613453575f5ffd5b6020840193505b828410156134c35760808488031215613471575f5ffd5b613479612e73565b61348285612e07565b815261349060208601612e07565b60208201526134a160408601612e07565b604082015260608581013590820152825260809093019260209091019061345a565b9695505050505050565b634e487b7160e01b5f52601160045260245ffd5b6001600160401b038181168382160190811115611216576112166134cd565b634e487b7160e01b5f52601260045260245ffd5b5f6001600160401b0383168061352c5761352c613500565b806001600160401b0384160691505092915050565b6001600160401b038281168282160390811115611216576112166134cd565b81810381811115611216576112166134cd565b634e487b7160e01b5f52603260045260245ffd5b5f6001600160401b0383168061359f5761359f613500565b806001600160401b0384160491505092915050565b5f602082840312156135c4575f5ffd5b5051919050565b5f81518060208401855e5f93019283525090919050565b5f6135f66135f083876135cb565b856135cb565b928352505060200192915050565b5f8261361257613612613500565b500690565b805f5b600581101561363957815184526020938401939091019060010161361a565b50505050565b61365482825180518252602090810151910152565b6020818101518051604085015290810151606084015250604081015180516080840152602081015160a0840152506060810151805160c0840152602081015160e0840152506080810151805161010084015260208101516101208401525060a0810151805161014084015260208101516101608401525060c0810151805161018084015260208101516101a08401525060e081015180516101c084015260208101516101e08401525061010081015180516102008401526020810151610220840152506101208101518051610240840152602081015161026084015250610140810151805161028084015260208101516102a08401525061016081015180516102c084015260208101516102e08401525061018081015180516103008401526020810151610320840152506101a08101516103408301526101c08101516103608301526101e08101516103808301526102008101516103a08301526102208101516103c08301526102408101516103e08301526102608101516104008301526102808101516104208301526102a08101516104408301526102c0015161046090910152565b5f610a20820190508451825260208501516020830152604085015161382b604084018280518252602090810151910152565b50606085015180516080840152602081015160a0840152506080850151805160c0840152602081015160e08401525060a0850151805161010084015260208101516101208401525060c0850151805161014084015260208101516101608401525060e0850151805161018084015260208101516101a08401525061010085015180516101c084015260208101516101e08401525061012085015180516102008401526020810151610220840152506101408501518051610240840152602081015161026084015250610160850151805161028084015260208101516102a08401525061018085015180516102c084015260208101516102e0840152506101a085015180516103008401526020810151610320840152506101c085015180516103408401526020810151610360840152506101e0850151805161038084015260208101516103a08401525061020085015180516103c084015260208101516103e08401525061022085015180516104008401526020810151610420840152506102408501518051610440840152602081015161046084015250610260850151805161048084015260208101516104a0840152506102808501516104c08301526102a08501516104e0830152613a03610500830185613617565b613a116105a083018461363f565b949350505050565b5f60208284031215613a29575f5ffd5b81518015158114611c24575f5ffd5b5f6001600160401b0382166001600160401b038103613a5957613a596134cd565b60010192915050565b5f81613a7057613a706134cd565b505f190190565b5f611c2482846135cb56fe360894a13ba1a3210667c828492db98dca3e2076cc3735a920a3ca505d382bbcf0c57e16840df040f15088dc2f81fe391c3923bec73e23a9662efc9c229c6a00a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R`\x046\x10a\x02uW_5`\xE0\x1C\x80cqP\x18\xA6\x11a\x01JW\x80c\xAA\xBD]\xB3\x11a\0\xBEW\x80c\xD2M\x93=\x11a\0xW\x80c\xD2M\x93=\x14a\x08\xABW\x80c\xE003\x01\x14a\x08\xDAW\x80c\xF0h T\x14a\x08\xF9W\x80c\xF2\xFD\xE3\x8B\x14a\t\x18W\x80c\xF5ga`\x14a\t7W\x80c\xF9\xE5\r\x19\x14a\tVW__\xFD[\x80c\xAA\xBD]\xB3\x14a\x07\xBBW\x80c\xAD<\xB1\xCC\x14a\x07\xDAW\x80c\xB3;\xC4\x91\x14a\x08\x17W\x80c\xB5\xAD\xEA<\x14a\x086W\x80c\xC2;\x9E\x9E\x14a\x08UW\x80c\xC8\xE5\xE4\x98\x14a\x08\x8DW__\xFD[\x80c\x8D\xA5\xCB[\x11a\x01\x0FW\x80c\x8D\xA5\xCB[\x14a\x06\xB8W\x80c\x90\xC1C\x90\x14a\x06\xF4W\x80c\x96\xC1\xCAa\x14a\x07\x13W\x80c\x99\x83(\xE8\x14a\x072W\x80c\x9B\xAA<\xC9\x14a\x07GW\x80c\x9F\xDBT\xA7\x14a\x07fW__\xFD[\x80cqP\x18\xA6\x14a\x06\x1BW\x80cu|7\xAD\x14a\x06/W\x80cvg\x18\x08\x14a\x06IW\x80c\x82nA\xFC\x14a\x06]W\x80c\x85\x84\xD2?\x14a\x06|W__\xFD[\x80c1=\xF7\xB1\x11a\x01\xECW\x80cBm1\x94\x11a\x01\xA6W\x80cBm1\x94\x14a\x05hW\x80cC=\xBA\x9F\x14a\x05\x89W\x80cO\x1E\xF2\x86\x14a\x05\xA8W\x80cR\xD1\x90-\x14a\x05\xBBW\x80cb:\x138\x14a\x05\xCFW\x80ci\xCCj\x04\x14a\x06\x07W__\xFD[\x80c1=\xF7\xB1\x14a\x04\x81W\x80c7\x8E\xC2;\x14a\x04\xB8W\x80c8\xE4T\xB1\x14a\x04\xD4W\x80c<#\xB6\xDB\x14a\x04\xE8W\x80c>\xD5[{\x14a\x05%W\x80cAh'D\x14a\x05KW__\xFD[\x80c\x16z\xC6\x18\x11a\x02=W\x80c\x16z\xC6\x18\x14a\x03\x85W\x80c c\xD4\xF7\x14a\x03\xA4W\x80c%)t'\x14a\x03\xC3W\x80c-R\xAA\xD6\x14a\x03\xF2W\x80c/y\x88\x9D\x14a\x04$W\x80c0\x0C\x89\xDD\x14a\x04bW__\xFD[\x80c\x01?\xA5\xFC\x14a\x02yW\x80c\x02\xB5\x92\xF3\x14a\x02\x9AW\x80c\x06%\xE1\x9B\x14a\x02\xF7W\x80c\r\x8En,\x14a\x039W\x80c\x12\x17<,\x14a\x03dW[__\xFD[4\x80\x15a\x02\x84W__\xFD[Pa\x02\x98a\x02\x936`\x04a+\xD6V[a\tjV[\0[4\x80\x15a\x02\xA5W__\xFD[Pa\x02\xB9a\x02\xB46`\x04a+\xEFV[a\n\x1DV[`@Qa\x02\xEE\x94\x93\x92\x91\x90`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x84\x01R\x92\x16`@\x82\x01R``\x81\x01\x91\x90\x91R`\x80\x01\x90V[`@Q\x80\x91\x03\x90\xF3[4\x80\x15a\x03\x02W__\xFD[P`\x0BT`\x0CT`\rT`\x0ETa\x03\x19\x93\x92\x91\x90\x84V[`@\x80Q\x94\x85R` \x85\x01\x93\x90\x93R\x91\x83\x01R``\x82\x01R`\x80\x01a\x02\xEEV[4\x80\x15a\x03DW__\xFD[P`@\x80Q`\x03\x81R_` \x82\x01\x81\x90R\x91\x81\x01\x91\x90\x91R``\x01a\x02\xEEV[4\x80\x15a\x03oW__\xFD[Pa\x03xa\nfV[`@Qa\x02\xEE\x91\x90a,\x06V[4\x80\x15a\x03\x90W__\xFD[Pa\x02\x98a\x03\x9F6`\x04a.\x1DV[a\x10\x95V[4\x80\x15a\x03\xAFW__\xFD[Pa\x02\x98a\x03\xBE6`\x04a1\x01V[a\x11\x0CV[4\x80\x15a\x03\xCEW__\xFD[Pa\x03\xE2a\x03\xDD6`\x04a.\x1DV[a\x11%V[`@Q\x90\x15\x15\x81R` \x01a\x02\xEEV[4\x80\x15a\x03\xFDW__\xFD[Pa\x02\x98a\x04\x0C6`\x04a+\xEFV[`\x10\x80T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x90U`\x11UV[4\x80\x15a\x04/W__\xFD[P`\x08Ta\x04J\x90`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x04mW__\xFD[Pa\x03\xE2a\x04|6`\x04a.\x1DV[a\x11\x87V[4\x80\x15a\x04\x8CW__\xFD[P`\x08Ta\x04\xA0\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x04\xC3W__\xFD[PC[`@Q\x90\x81R` \x01a\x02\xEEV[4\x80\x15a\x04\xDFW__\xFD[Pa\x02\x98a\x12\x1CV[4\x80\x15a\x04\xF3W__\xFD[Pa\x02\x98a\x05\x026`\x04a.\x1DV[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[4\x80\x15a\x050W__\xFD[P`\nTa\x04J\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\x05VW__\xFD[P`\x10T`\x01`\x01`@\x1B\x03\x16a\x04JV[4\x80\x15a\x05sW__\xFD[P_T`\x01T`\x02T`\x03Ta\x03\x19\x93\x92\x91\x90\x84V[4\x80\x15a\x05\x94W__\xFD[Pa\x02\x98a\x05\xA36`\x04a1HV[a\x13\x0BV[a\x02\x98a\x05\xB66`\x04a1aV[a\x13\x1FV[4\x80\x15a\x05\xC6W__\xFD[Pa\x04\xC6a\x13>V[4\x80\x15a\x05\xDAW__\xFD[Pa\x02\x98a\x05\xE96`\x04a2GV[\x80Q`\x0BU` \x81\x01Q`\x0CU`@\x81\x01Q`\rU``\x01Q`\x0EUV[4\x80\x15a\x06\x12W__\xFD[Pa\x02\x98a\x13YV[4\x80\x15a\x06&W__\xFD[Pa\x02\x98a\x13\xC7V[4\x80\x15a\x06:W__\xFD[Pa\x02\x98a\x03\xBE6`\x04a2aV[4\x80\x15a\x06TW__\xFD[Pa\x04Ja\x13\xD8V[4\x80\x15a\x06hW__\xFD[P`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15a\x03\xE2V[4\x80\x15a\x06\x87W__\xFD[Pa\x06\x9Ba\x06\x966`\x04a+\xEFV[a\x14\x02V[`@\x80Q\x92\x83R`\x01`\x01`@\x1B\x03\x90\x91\x16` \x83\x01R\x01a\x02\xEEV[4\x80\x15a\x06\xC3W__\xFD[P\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16a\x04\xA0V[4\x80\x15a\x06\xFFW__\xFD[Pa\x04Ja\x07\x0E6`\x04a2\xA5V[a\x15-V[4\x80\x15a\x07\x1EW__\xFD[Pa\x02\x98a\x07-6`\x04a1HV[a\x15\x9CV[4\x80\x15a\x07=W__\xFD[Pa\x04\xC6`\x0FT\x81V[4\x80\x15a\x07RW__\xFD[Pa\x02\x98a\x07a6`\x04a2\xCDV[a\x16%V[4\x80\x15a\x07qW__\xFD[P`\x06T`\x07Ta\x07\x95\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[`@\x80Q`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x93\x90\x92\x16` \x84\x01R\x90\x82\x01R``\x01a\x02\xEEV[4\x80\x15a\x07\xC6W__\xFD[Pa\x02\x98a\x07\xD56`\x04a3\"V[a\x174V[4\x80\x15a\x07\xE5W__\xFD[Pa\x08\n`@Q\x80`@\x01`@R\x80`\x05\x81R` \x01d\x03R\xE3\x02\xE3`\xDC\x1B\x81RP\x81V[`@Qa\x02\xEE\x91\x90a3fV[4\x80\x15a\x08\"W__\xFD[Pa\x02\x98a\x0816`\x04a2\xA5V[a\x1AYV[4\x80\x15a\x08AW__\xFD[Pa\x02\x98a\x08P6`\x04a3\x9BV[a\x1B\xAAV[4\x80\x15a\x08`W__\xFD[P`\x08Ta\x08x\x90`\x01`\xA0\x1B\x90\x04c\xFF\xFF\xFF\xFF\x16\x81V[`@Qc\xFF\xFF\xFF\xFF\x90\x91\x16\x81R` \x01a\x02\xEEV[4\x80\x15a\x08\x98W__\xFD[Pa\x02\x98`\x10\x80T`\xFF`@\x1B\x19\x16\x90UV[4\x80\x15a\x08\xB6W__\xFD[P`\x04T`\x05Ta\x07\x95\x91`\x01`\x01`@\x1B\x03\x80\x82\x16\x92`\x01`@\x1B\x90\x92\x04\x16\x90\x83V[4\x80\x15a\x08\xE5W__\xFD[Pa\x03\xE2a\x08\xF46`\x04a3\xB5V[a\x1B\xF1V[4\x80\x15a\t\x04W__\xFD[P`\nTa\x04J\x90`\x01`\x01`@\x1B\x03\x16\x81V[4\x80\x15a\t#W__\xFD[Pa\x02\x98a\t26`\x04a+\xD6V[a\x1C+V[4\x80\x15a\tBW__\xFD[Pa\x02\x98a\tQ6`\x04a3\xD5V[a\x1CjV[4\x80\x15a\taW__\xFD[P`\tTa\x04\xC6V[a\tra\x1D\x15V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\t\x99W`@Qc\xE6\xC4${`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\x01`\xA0\x1B\x03\x90\x81\x16\x90\x82\x16\x03a\t\xC8W`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x81\x17\x90\x91U`@Q\x90\x81R\x7F\x80\x17\xBB\x88\x7F\xDF\x8F\xCAC\x14\xA9\xD4\x0Fns\xB3\xB8\x10\x02\xD6~\\\xFA\x85\xD8\x81s\xAFj\xA4`r\x90` \x01[`@Q\x80\x91\x03\x90\xA1PV[`\t\x81\x81T\x81\x10a\n,W_\x80\xFD[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01\x80T`\x01\x90\x91\x01T`\x01`\x01`@\x1B\x03\x80\x83\x16\x93P`\x01`@\x1B\x83\x04\x81\x16\x92`\x01`\x80\x1B\x90\x04\x16\x90\x84V[a\nna(\xF6V[b\x01\0\0\x81R`\x05` \x82\x01R\x7F\x05w\xCF\x9B\xF8.\x9F\x08\xA7R\xD87p\x90\x88\xC17\xF9\xCD\x88\xCD\x95\xB0;'\xE6\xC5K\xE3\x8E\x92\x9F`@\x82\x01QR\x7F\x08\xFF\x03\xA2|\xBEj\x98\x0E\xC6`7\x9D(\xEEv\x97\\\xEFB\xBAX\x06\xB96&2\xF2jm\xFB ` `@\x83\x01Q\x01R\x7F\x0FO\xC3l\x1C\xA1[\x91\x85\x95\x92\x87\xC9\xE9w2\x83\xE1S\x95\xA9'\x01\x08}8pO\xDD\x0C\x17\xEA``\x82\x01QR\x7F\x14\xB0\x15\x87\xBBQ +\xAEV\xDC\x99\"QyVU\xDCD9Q\x93\xF2+9\xA6\x8EE\xE5\x01\xE3\xF3` ``\x83\x01Q\x01R\x7F\nL\xE9\x17c(B+HH2\x15[\xB332\x85\xA6\xE0\xE00fwa\xC9WoP.\x8B\x1B\x1B`\x80\x82\x01QR\x7F\n\x16\xF5AF\x14\xD6Hg\x8Ccz\t1\xC1A\x91\xB22\xBB\x1A]#\xF6\xC1g:\xF0Yf\x03\xA6` `\x80\x83\x01Q\x01R\x7F\x1C\x1C_\xF4$\xE5\xA4\xDA\xEF\x97\x06\x1E\x022zk\x8E[\xBE\xCC\x8BU\xD7\xF5\x18\x89\xC6\xC3\x91\xC6\xFEK`\xA0\x82\x01QR\x7F+\xFF<\xE7\xC8v\xF1\x11\xDE^\x8A\x0C\xAA\x06=\xCE&\xD0{\x87\xC3\x12 \xA7\xD79\xA0\xD8\x8C9\xD8\xEA` `\xA0\x83\x01Q\x01R\x7F-2\xDC\xE8\xD0\x80\xA1\xA6*\xFDs`\xE6U\x02\x18\xC7n\xD1\xBA~\xEF\xE2\x9C\x02t\xF5\xDE\xE3rS\x12`\xC0\x82\x01QR\x7F&\x0F\x1D\x1A\x8A\xE6\xD96\xAE\x1F(@v\xAD\n7C4\xB1T\\\xB8\xE1\xB0N\xB4U9\xE3n\x93.` `\xC0\x83\x01Q\x01R\x7F%\xCC\t\xD2}\x1F'X\xC7\xED\xAB\xFCL\xCBu:\x1E\x10\x1F\x07\xCD\xAE\xA6\x12\xBB\x1B\xB9\xE6kW\xAC\xB1`\xE0\x82\x01QR\x7F/\x87s_\xCB\xE1\x1Cp\x12\x1E\xDF\xF1\xD5 \xC6/\xE9\x98\x8D\xDD\x15\x93|?C\n\xA1\xDF\xA4g0\x8F` `\xE0\x83\x01Q\x01R\x7F\x07:\xAD\x1FLY\xF4\xF2\xFF\xFC_H\x9C{\x81\xF8\xB6\xFDi\xB7s(\xEC\x8B\0\x93\x07\x1AA\xA9\xB7`a\x01\0\x82\x01QR\x7F%\xBB\xAA\xE8wP\x96OJ\x0E\xAB\xCE\xD36;L\x12\r\xE7\xAC\xDE|\xE3\xD6\x19\xD4*\x98gK\xCC\x17` a\x01\0\x83\x01Q\x01R\x7F\x14\x0F\x1B\nq\xA0\xDF\x10\x0C\xF0\xAFg\x1B\xBE\xCCPa`\xEF,\xBD\xC2J\x97o\xAAE\x93\x12\xC1B\\a\x01 \x82\x01QR\x7F\x0El\xA0s>\x1FES\xBB\xCC\xA3\xA7\xB3\x14\xC4\x08\x0884\x10; \xAB?\xAA,\xEC\xE9\xEFz\xA9\x80` a\x01 \x83\x01Q\x01R\x7F\x0Cl\xC1\xB3\xD3d\xF4\x05\xFCw\x86Q\x98\xC9T\xED\xB8w\x14v_\x87\x9B\x05\x7FG \xFE\x13\x9C\xE8aa\x01@\x82\x01QR\x7F\x0E\xFA\xD5:\xC0Q\xAE\xE0\x7F\x7F\x92\xC3Jw\x9B_\x9B\xC5\x80:a&\x11q\x0F\x90{\xBD\xA3c\x030` a\x01@\x83\x01Q\x01R\x7F)\x9B]&\xB5\xDA1\x8C\xFDn\xE25BDO\xBDT\xA2!\xAE@\xB0C\xEE{Y\xE4\xBE\x8A|\xF0\xBCa\x01`\x82\x01QR\x7F\x11\xBF\xB7\r\xE4\xB5\x9Cq|\xB7;\xA6\xD9#\xF7\x94G\xD1\x0E\xB6\xE7\xEA\x91\xAB\x8Am&\x11\x84\xAB}\xDD` a\x01`\x83\x01Q\x01R\x7F.dm\xF4Pq\xA6\x18\x03\xF5\xE8\xE9+u\xC1V\xCA\xB0_8\xCF\x0F\x84\xDFu\x1C\xF2D/\"D\x82a\x01\x80\x82\x01QR~\xED\x98\xEF\"|\xE6'\xFE.V\xA1\x1E2\xE1\xB4\x90\xD9\x9D\xEEH\x1C-\x81 P\xD2\xFF\xBF\x1C\xA0\xDF` a\x01\x80\x83\x01Q\x01R\x7F\x10\xE3k\xBD\xF4-V5\x9D.\x87\x93+x\x1Bb'\x1C>n\x1B\x02\x87\xD1>]$x\xCD\x96\xBA\xABa\x01\xA0\x82\x01QR\x7F\x02\xD5\x03\x7F\xC2T;\x02\xB2\xE2\xE4\x1Fj\xBC\xA9\x13r\x06\xF8{gT\x9A\xBD}h\x95\x1B\x88\x01\xAA\x8D` a\x01\xA0\x83\x01Q\x01R\x7F\x16\xE1\x12$\x01\xB6O\x03\xC2\xDA^\xBDMR\x02\x1E\x1B\x86\xC9\xBB=Jk\xE6C\xEE\x05w1\xBEN\xFDa\x01\xC0\x82\x01QR\x7F\x1B\xF6\xCC\xA2\xDC\x0Bz\xA2?\xEC\xAF\x05\x8D\x11\xBB\xF4\xC0lm\x85\xB0;k'\xE1\xF3z\xC8p5}\xD8` a\x01\xC0\x83\x01Q\x01R\x7F-\xC5\x95\x1C^_\x89\x10\x03\xF7}\x99@\xBA\x8AE\x8B\x96\x08\xA3\xCB;\x92m\x1F\n\"i\0\x96M\xCAa\x01\xE0\x82\x01QR\x7F\r\x11H!=c=\xB52\xD6\xECCR\xFFA\xE5!\xE0\x16\xC8\xFA\xDC\x8D\xB4,K\xF9\xEB\x16j\xD3\x81` a\x01\xE0\x83\x01Q\x01R\x7F-\x7F\xED@\x88\xB6\xE7:\xBAz\xE1\xC3\xBE9\xAB\x96\x97\x13z\xC1\xC4P\xAE\xB9q\x87\x8D\xCF\xDA~2\x81a\x02\0\x82\x01QR\x7F\x18\xDC\r\xE1\xEFW\xDF\x9C\x8FI*\xDE\xBA\x18<a\n\xD5\xEA\xE0i\x96Dn4x\x07c`\xB2\x9Ev` a\x02\0\x83\x01Q\x01R\x7F\x1B\x9C\xD9\xB8y\xF2B\xF31\x10=j\xC1\xF5&#\x93|\x08\x01\xA2\x1C\xCD\xD2;M\xC9\xD4\xCA\xF7}\x9Ca\x02 \x82\x01QR\x7F\x1D\xEAj_,\xFF\xFF\x1CUv\xB70u\xA9b7\xFD\xF9QS\xEFh\xCB\xF47+\x7F\x1F8\x06\xD8J` a\x02 \x83\x01Q\x01R\x7F\x01&\x17g,\x1D8U\x7F8_\x9F\xF0\xD9\xFC\xE10\xE8\xC8\xCF\xDE'\xF6[\x90\x8D\xB4\xFD0\xD3\rra\x02@\x82\x01QR\x7F\x0E\xE4\x84\x05a\xAA\x90\xB4\xF4\x02W\xD5\x1Cq\xED\x03\xE0\xA3\x90i zY'g|\xD1Fh\xD5tw` a\x02@\x83\x01Q\x01R\x7F\x13Yg\x92\xA5\xEFm\xF4\xCD\xBBF\xEF\xFA\xA4_\x91\xD3\xCC\x04\x01\xA6E\x83\xCD\xD7Ky#\x10\xE3\xFD\xBDa\x02`\x82\x01QR\x7F\x19.,a\xCB\xB5\xB70\xEF\x86\xA9Pc<Yr\xC4\xF9V\xA0S\xC7%rI\x8A\x1E\x12\x12Hp9` a\x02`\x83\x01Q\x01R\x7F\xB0\x83\x88\x93\xEC\x1F#~\x8B\x072;\x07DY\x9FN\x97\xB5\x98\xB3\xB5\x89\xBC\xC2\xBC7\xB8\xD5\xC4\x18\x01a\x02\x80\x82\x01R\x7F\xC1\x83\x93\xC0\xFA0\xFEN\x8B\x03\x8E5z\xD8Q\xEA\xE8\xDE\x91\x07XN\xFF\xE7\xC7\xF1\xF6Q\xB2\x01\x0E&a\x02\xA0\x82\x01R\x90V[a\x10\x9Da\x1D\x15V[`\n\x80To\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\0\0\0\0\0\0\0\0\x19\x81\x16`\x01`@\x1B`\x01`\x01`@\x1B\x03\x85\x81\x16\x82\x02\x92\x83\x17\x94\x85\x90Ua\x10\xE3\x94\x91\x90\x91\x04\x81\x16\x92\x81\x16\x91\x16\x17a\x15-V[`\n`\x10a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPPV[`@QcN@\\\x8D`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\x11EWP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\x11QWP_\x91\x90PV[`\nT`\x01`\x01`@\x1B\x03\x16a\x11h\x83`\x05a4\xE1V[a\x11r\x91\x90a5\x14V[`\x01`\x01`@\x1B\x03\x16\x15\x92\x91PPV[\x91\x90PV[_`\x01`\x01`@\x1B\x03\x82\x16\x15\x80a\x11\xA7WP`\nT`\x01`\x01`@\x1B\x03\x16\x15[\x15a\x11\xB3WP_\x91\x90PV[`\nTa\x11\xC9\x90`\x01`\x01`@\x1B\x03\x16\x83a5\x14V[`\x01`\x01`@\x1B\x03\x16\x15\x80a\x12\x16WP`\nTa\x11\xF1\x90`\x05\x90`\x01`\x01`@\x1B\x03\x16a5AV[`\nT`\x01`\x01`@\x1B\x03\x91\x82\x16\x91a\x12\x0B\x91\x16\x84a5\x14V[`\x01`\x01`@\x1B\x03\x16\x11[\x92\x91PPV[_Q` a:\xA3_9_Q\x90_R\x80T`\x03\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x12RWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x12pW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x90\x81\x17\x83U`\nTa\x12\xAA\x92\x91\x81\x04\x82\x16\x91\x16a\x15-V[`\x10\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x92\x83\x16\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPV[a\x13\x13a\x1D\x15V[a\x13\x1C\x81a\x15\x9CV[PV[a\x13'a\x1DpV[a\x130\x82a\x1E\x14V[a\x13:\x82\x82a\x1EUV[PPV[_a\x13Ga\x1F\x16V[P_Q` a:\x83_9_Q\x90_R\x90V[a\x13aa\x1D\x15V[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15a\x13\xACW`\x08\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16\x90U`@Q\x7F\x9A_W\xDE\x85m\xD6h\xC5M\xD9^\\U\xDF\x93C!q\xCB\xCAI\xA8wmV \xEAY\xC0$P\x90_\x90\xA1V[`@Qc\xA8c\xAE\xC9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[V[a\x13\xCFa\x1D\x15V[a\x13\xC5_a\x1F_V[`\x06T`\nT_\x91a\x13\xFD\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16a\x15-V[\x90P\x90V[`\t\x80T_\x91\x82\x91\x90a\x14\x16`\x01\x83a5`V[\x81T\x81\x10a\x14&Wa\x14&a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84\x10a\x14eW`@Qc\x18V\xA4\x99`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16[\x81\x81\x10\x15a\x15&W\x84`\t\x82\x81T\x81\x10a\x14\x95Wa\x14\x95a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x80\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x11\x15a\x15\x1EW`\t\x81\x81T\x81\x10a\x14\xCEWa\x14\xCEa5sV[\x90_R` _ \x90`\x02\x02\x01`\x01\x01T`\t\x82\x81T\x81\x10a\x14\xF1Wa\x14\xF1a5sV[\x90_R` _ \x90`\x02\x02\x01_\x01`\x10\x90T\x90a\x01\0\n\x90\x04`\x01`\x01`@\x1B\x03\x16\x93P\x93PPP\x91P\x91V[`\x01\x01a\x14yV[PP\x91P\x91V[_\x81`\x01`\x01`@\x1B\x03\x16_\x03a\x15EWP_a\x12\x16V[\x82`\x01`\x01`@\x1B\x03\x16_\x03a\x15]WP`\x01a\x12\x16V[a\x15g\x82\x84a5\x14V[`\x01`\x01`@\x1B\x03\x16_\x03a\x15\x87Wa\x15\x80\x82\x84a5\x87V[\x90Pa\x12\x16V[a\x15\x91\x82\x84a5\x87V[a\x15\x80\x90`\x01a4\xE1V[a\x15\xA4a\x1D\x15V[a\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10\x80a\x15\xC3WPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x80a\x15\xE1WP`\x08Tc\xFF\xFF\xFF\xFF`\x01`\xA0\x1B\x90\x91\x04\x81\x16\x90\x82\x16\x11\x15[\x15a\x15\xFFW`@Qc\x07\xA5\x07w`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[_Q` a:\xA3_9_Q\x90_R\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15a\x16VWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15a\x16qWP0;\x15[\x90P\x81\x15\x80\x15a\x16\x7FWP\x80\x15[\x15a\x16\x9DW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x16\xC7W\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[a\x16\xD0\x86a\x1F\xCFV[a\x16\xD8a\x1F\xE0V[a\x16\xE3\x89\x89\x89a\x1F\xE8V[\x83\x15a\x17)W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPPPPPV[`\x08T`\x01`\x01`\xA0\x1B\x03\x16\x15\x15\x80\x15a\x17YWP`\x08T`\x01`\x01`\xA0\x1B\x03\x163\x14\x15[\x15a\x17wW`@Qc\x01GL\x8F`\xE7\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x06T\x84Q`\x01`\x01`@\x1B\x03\x91\x82\x16\x91\x16\x11\x15\x80a\x17\xB0WP`\x06T` \x85\x01Q`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x92\x04\x82\x16\x91\x16\x11\x15[\x15a\x17\xCEW`@Qc\x05\x1CF\xEF`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x17\xDB\x84`@\x01Qa!\x14V[a\x17\xE8\x83` \x01Qa!\x14V[a\x17\xF5\x83`@\x01Qa!\x14V[a\x18\x02\x83``\x01Qa!\x14V[_a\x18\x0Ba\x13\xD8V[` \x86\x01Q`\nT\x91\x92P_\x91a\x18+\x91\x90`\x01`\x01`@\x1B\x03\x16a\x15-V[`\x10T\x90\x91P`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x10a\x18oWa\x18Q\x86` \x01Qa\x11\x87V[\x15a\x18oW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x10T`\x01`\x01`@\x1B\x03\x90\x81\x16\x90\x82\x16\x11\x15a\x19\x1BW`\x02a\x18\x92\x83\x83a5AV[`\x01`\x01`@\x1B\x03\x16\x10a\x18\xB9W`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x18\xC4\x82`\x01a4\xE1V[`\x01`\x01`@\x1B\x03\x16\x81`\x01`\x01`@\x1B\x03\x16\x14\x80\x15a\x18\xFDWP`\x06Ta\x18\xFB\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16a\x11%V[\x15[\x15a\x19\x1BW`@Qc\x08\n\xE8\xD9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x19'\x86\x86\x86\x86a!\x84V[\x85Q`\x06\x80T` \x89\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x81\x16\x93\x90\x93\x17\x17\x90U`@\x87\x01Q`\x07U`\x0F\x85\x90U`\x10T\x81\x16\x90\x82\x16\x10\x80\x15\x90a\x19\x84WPa\x19\x84\x86` \x01Qa\x11%V[\x15a\x19\xEEW\x84Q`\x0BU` \x85\x01Q`\x0CU`@\x85\x01Q`\rU``\x85\x01Q`\x0EU\x7F1\xEA\xBD\x90\x99\xFD\xB2]\xAC\xDD\xD2\x06\xAB\xFF\x871\x1EU4A\xFC\x9D\x0F\xCD\xEF \x10b\xD7\xE7\x07\x1Ba\x19\xD2\x82`\x01a4\xE1V[`@Q`\x01`\x01`@\x1B\x03\x90\x91\x16\x81R` \x01`@Q\x80\x91\x03\x90\xA1[a\x19\xF9CB\x88a#\xAEV[\x85` \x01Q`\x01`\x01`@\x1B\x03\x16\x86_\x01Q`\x01`\x01`@\x1B\x03\x16\x7F\xA0Jw9$PZA\x85d67%\xF5h2\xF5w.k\x8D\r\xBDn\xFC\xE7$\xDF\xE8\x03\xDA\xE6\x88`@\x01Q`@Qa\x1AI\x91\x81R` \x01\x90V[`@Q\x80\x91\x03\x90\xA3PPPPPPV[_Q` a:\xA3_9_Q\x90_R\x80T`\x02\x91\x90`\x01`@\x1B\x90\x04`\xFF\x16\x80a\x1A\x8FWP\x80T`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x16\x10\x15[\x15a\x1A\xADW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80Th\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01`\x01`@\x1B\x03\x80\x84\x16\x91\x90\x91\x17`\x01`@\x1B\x17\x82U`\x05\x90\x85\x16\x11a\x1A\xF5W`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_T`\x0BU`\x01T`\x0CU`\x02T`\rU`\x03T`\x0EU`\n\x80T`\x01`\x01`@\x1B\x03\x85\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x90\x87\x16\x17\x17\x90Ua\x1B>\x83\x85a\x15-V[`\n\x80Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x16`\x01`\x80\x1B`\x01`\x01`@\x1B\x03\x93\x84\x16\x02\x17\x90U\x81T`\xFF`@\x1B\x19\x16\x82U`@Q\x90\x83\x16\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1PPPPV[\x80Q`\x06\x80T` \x84\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x92\x16\x93\x16\x92\x90\x92\x17\x91\x90\x91\x17\x90U`@\x81\x01Q`\x07Ua\x13\x1CCB\x83a#\xAEV[`\x10T_\x90`\x01`@\x1B\x90\x04`\xFF\x16a\x1C\x13Wa\x1C\x0E\x83\x83a%\x97V[a\x1C$V[\x81`\x11T\x84a\x1C\"\x91\x90a5`V[\x11[\x93\x92PPPV[a\x1C3a\x1D\x15V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x1CaW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x13\x1C\x81a\x1F_V[a\x1Cu`\t_a+[V[_[\x81Q\x81\x10\x15a\x13:W`\t\x82\x82\x81Q\x81\x10a\x1C\x94Wa\x1C\x94a5sV[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q\x82T`\x01\x81\x81\x01\x85U_\x94\x85R\x93\x83\x90 \x82Q`\x02\x90\x92\x02\x01\x80T\x93\x83\x01Q`@\x84\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x92\x82\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x97\x16\x91\x90\x94\x16\x17\x94\x90\x94\x17\x93\x90\x93\x16\x17\x82U``\x01Q\x90\x82\x01U\x01a\x1CwV[3a\x1DG\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0T`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14a\x13\xC5W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x1CXV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14\x80a\x1D\xF6WP\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0`\x01`\x01`\xA0\x1B\x03\x16a\x1D\xEA_Q` a:\x83_9_Q\x90_RT`\x01`\x01`\xA0\x1B\x03\x16\x90V[`\x01`\x01`\xA0\x1B\x03\x16\x14\x15[\x15a\x13\xC5W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1E\x1Ca\x1D\x15V[`@Q`\x01`\x01`\xA0\x1B\x03\x82\x16\x81R\x7F\xF7\x87!\"n\xFE\x9A\x1B\xB6x\x18\x9A\x16\xD1UI(\xB9\xF2\x19.,\xB9>\xED\xA8;y\xFA@\0}\x90` \x01a\n\x12V[\x81`\x01`\x01`\xA0\x1B\x03\x16cR\xD1\x90-`@Q\x81c\xFF\xFF\xFF\xFF\x16`\xE0\x1B\x81R`\x04\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x92PPP\x80\x15a\x1E\xAFWP`@\x80Q`\x1F=\x90\x81\x01`\x1F\x19\x16\x82\x01\x90\x92Ra\x1E\xAC\x91\x81\x01\x90a5\xB4V[`\x01[a\x1E\xD7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x83\x16`\x04\x82\x01R`$\x01a\x1CXV[_Q` a:\x83_9_Q\x90_R\x81\x14a\x1F\x07W`@Qc*\x87Ri`\xE2\x1B\x81R`\x04\x81\x01\x82\x90R`$\x01a\x1CXV[a\x1F\x11\x83\x83a&\xEFV[PPPV[0`\x01`\x01`\xA0\x1B\x03\x7F\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x16\x14a\x13\xC5W`@Qcp>F\xDD`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x7F\x90\x16\xD0\x9Dr\xD4\x0F\xDA\xE2\xFD\x8C\xEA\xC6\xB6#Lw\x06!O\xD3\x9C\x1C\xD1\xE6\t\xA0R\x8C\x19\x93\0\x80T`\x01`\x01`\xA0\x1B\x03\x19\x81\x16`\x01`\x01`\xA0\x1B\x03\x84\x81\x16\x91\x82\x17\x84U`@Q\x92\x16\x91\x82\x90\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x90_\x90\xA3PPPV[a\x1F\xD7a'DV[a\x13\x1C\x81a'zV[a\x13\xC5a'DV[\x82Q`\x01`\x01`@\x1B\x03\x16\x15\x15\x80a \x0CWP` \x83\x01Q`\x01`\x01`@\x1B\x03\x16\x15\x15[\x80a \x19WP` \x82\x01Q\x15[\x80a &WP`@\x82\x01Q\x15[\x80a 3WP``\x82\x01Q\x15[\x80a =WP\x81Q\x15[\x80a OWPa\x0E\x10\x81c\xFF\xFF\xFF\xFF\x16\x10[\x80a cWPc\x01\xE13\x80\x81c\xFF\xFF\xFF\xFF\x16\x11[\x15a \x81W`@QcP\xDD\x03\xF7`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x82Q`\x04\x80T` \x80\x87\x01Q`\x01`\x01`@\x1B\x03\x90\x81\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x93\x84\x16\x91\x90\x95\x16\x90\x81\x17\x85\x17\x90\x93U`@\x96\x87\x01Q`\x05\x81\x90U\x86Q_U\x90\x86\x01Q`\x01U\x95\x85\x01Q`\x02U``\x90\x94\x01Q`\x03U`\x06\x80T\x90\x94\x16\x17\x17\x90\x91U`\x07\x91\x90\x91U`\x08\x80Tc\xFF\xFF\xFF\xFF\x90\x92\x16`\x01`\xA0\x1B\x02c\xFF\xFF\xFF\xFF`\xA0\x1B\x19\x90\x92\x16\x91\x90\x91\x17\x90UV[\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x81\x10\x80a\x13:W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x1CXV[_a!\x8Da\nfV[\x90Pa!\x97a+yV[`\x0CT\x81R`\rT` \x80\x83\x01\x91\x90\x91R`\x0ET`@\x83\x01R`\x0BT``\x80\x84\x01\x91\x90\x91R`\nT\x91\x88\x01Q\x90\x91`\x01`\x01`@\x1B\x03`\x01`@\x1B\x90\x91\x04\x81\x16\x91\x16\x10\x80\x15\x90a!\xEFWPa!\xEF\x87` \x01Qa\x11%V[\x15a\"8W`@\x80Q\x87Q` \x80\x83\x01\x91\x90\x91R\x88\x01Q\x81\x83\x01R\x90\x87\x01Q``\x80\x83\x01\x91\x90\x91R\x87\x01Q`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\"sV[`@\x80Q`\x0BT` \x82\x01R`\x0CT\x91\x81\x01\x91\x90\x91R`\rT``\x82\x01R`\x0ET`\x80\x82\x01R`\xA0\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P[`@\x80Q\x88Q`\x01`\x01`@\x1B\x03\x90\x81\x16` \x80\x84\x01\x91\x90\x91R\x8A\x01Q\x16\x81\x83\x01R\x90\x88\x01Q``\x82\x01R_\x90`\x80\x01`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90Ra\"\xC4\x91\x84\x90\x89\x90` \x01a5\xE2V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x80Q` \x90\x91\x01 \x90Pa#\x08\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\x82a6\x04V[`\x80\x84\x01R`@QcT\xE8\xBDg`\xE0\x1B\x81Rs\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x90cT\xE8\xBDg\x90a#H\x90\x87\x90\x87\x90\x8A\x90`\x04\x01a7\xF9V[` `@Q\x80\x83\x03\x81\x86Z\xF4\x15\x80\x15a#cW=__>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a#\x87\x91\x90a:\x19V[a#\xA4W`@Qc\t\xBD\xE39`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPPPV[`\tT\x15\x80\x15\x90a$#WP`\x08T`\t\x80T`\x01`\xA0\x1B\x83\x04c\xFF\xFF\xFF\xFF\x16\x92`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a#\xEEWa#\xEEa5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01Ta$\x18\x90`\x01`@\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x84a5AV[`\x01`\x01`@\x1B\x03\x16\x11[\x15a$\xB6W`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a$PWa$Pa5sV[_\x91\x82R` \x82 `\x02\x90\x91\x02\x01\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U`\x01\x01U`\x08\x80T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90`\x18a$\x90\x83a:8V[\x91\x90a\x01\0\n\x81T\x81`\x01`\x01`@\x1B\x03\x02\x19\x16\x90\x83`\x01`\x01`@\x1B\x03\x16\x02\x17\x90UPP[`@\x80Q`\x80\x81\x01\x82R`\x01`\x01`@\x1B\x03\x94\x85\x16\x81R\x92\x84\x16` \x80\x85\x01\x91\x82R\x83\x01Q\x85\x16\x84\x83\x01\x90\x81R\x92\x90\x91\x01Q``\x84\x01\x90\x81R`\t\x80T`\x01\x81\x01\x82U_\x91\x90\x91R\x93Q`\x02\x90\x94\x02\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xAF\x81\x01\x80T\x93Q\x94Q\x87\x16`\x01`\x80\x1B\x02g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF`\x80\x1B\x19\x95\x88\x16`\x01`@\x1B\x02`\x01`\x01`\x80\x1B\x03\x19\x90\x95\x16\x96\x90\x97\x16\x95\x90\x95\x17\x92\x90\x92\x17\x92\x90\x92\x16\x93\x90\x93\x17\x90\x91UQ\x7Fn\x15@\x17\x1Bl\x0C\x96\x0Bq\xA7\x02\r\x9F`\x07\x7Fj\xF91\xA8\xBB\xF5\x90\xDA\x02#\xDA\xCFu\xC7\xB0\x90\x91\x01UV[`\tT_\x90C\x84\x11\x80a%\xA8WP\x80\x15[\x80a%\xF2WP`\x08T`\t\x80T\x90\x91`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x90\x81\x10a%\xD6Wa%\xD6a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x84\x10[\x15a&\x10W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_\x80\x80a&\x1E`\x01\x85a5`V[\x90P[\x81a&\xBAW`\x08T`\x01`\xC0\x1B\x90\x04`\x01`\x01`@\x1B\x03\x16\x81\x10a&\xBAW\x86`\t\x82\x81T\x81\x10a&SWa&Sa5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x11a&\xA8W`\x01\x91P`\t\x81\x81T\x81\x10a&\x88Wa&\x88a5sV[_\x91\x82R` \x90\x91 `\x02\x90\x91\x02\x01T`\x01`\x01`@\x1B\x03\x16\x92Pa&\xBAV[\x80a&\xB2\x81a:bV[\x91PPa&!V[\x81a&\xD8W`@Qc\xB0\xB48w`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x85a&\xE3\x84\x89a5`V[\x11\x97\x96PPPPPPPV[a&\xF8\x82a'\x82V[`@Q`\x01`\x01`\xA0\x1B\x03\x83\x16\x90\x7F\xBC|\xD7Z \xEE'\xFD\x9A\xDE\xBA\xB3 A\xF7U!M\xBCk\xFF\xA9\x0C\xC0\"[9\xDA.\\-;\x90_\x90\xA2\x80Q\x15a'<Wa\x1F\x11\x82\x82a'\xE5V[a\x13:a(WV[_Q` a:\xA3_9_Q\x90_RT`\x01`@\x1B\x90\x04`\xFF\x16a\x13\xC5W`@Qc\x1A\xFC\xD7\x9F`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1C3a'DV[\x80`\x01`\x01`\xA0\x1B\x03\x16;_\x03a'\xB7W`@QcL\x9C\x8C\xE3`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x82\x16`\x04\x82\x01R`$\x01a\x1CXV[_Q` a:\x83_9_Q\x90_R\x80T`\x01`\x01`\xA0\x1B\x03\x19\x16`\x01`\x01`\xA0\x1B\x03\x92\x90\x92\x16\x91\x90\x91\x17\x90UV[``__\x84`\x01`\x01`\xA0\x1B\x03\x16\x84`@Qa(\x01\x91\x90a:wV[_`@Q\x80\x83\x03\x81\x85Z\xF4\x91PP=\x80_\x81\x14a(9W`@Q\x91P`\x1F\x19`?=\x01\x16\x82\x01`@R=\x82R=_` \x84\x01>a(>V[``\x91P[P\x91P\x91Pa(N\x85\x83\x83a(vV[\x95\x94PPPPPV[4\x15a\x13\xC5W`@Qc\xB3\x98\x97\x9F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[``\x82a(\x86Wa\x1C\x0E\x82a(\xCDV[\x81Q\x15\x80\x15a(\x9DWP`\x01`\x01`\xA0\x1B\x03\x84\x16;\x15[\x15a(\xC6W`@Qc\x99\x96\xB3\x15`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`$\x01a\x1CXV[P\x92\x91PPV[\x80Q\x15a(\xDDW\x80Q\x80\x82` \x01\xFD[`@Qc\n\x12\xF5!`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@Q\x80a\x02\xC0\x01`@R\x80_\x81R` \x01_\x81R` \x01a))`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)i`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\x89`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xA9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xC9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a)\xE9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\t`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*)`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*i`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\x89`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xA9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xC9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a*\xE9`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+\t`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+)`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01a+I`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[\x81R` \x01_\x81R` \x01_\x81RP\x90V[P\x80T_\x82U`\x02\x02\x90_R` _ \x90\x81\x01\x90a\x13\x1C\x91\x90a+\x97V[`@Q\x80`\xA0\x01`@R\x80`\x05\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[[\x80\x82\x11\x15a+\xBCW\x80T`\x01`\x01`\xC0\x1B\x03\x19\x16\x81U_`\x01\x82\x01U`\x02\x01a+\x98V[P\x90V[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a+\xE6W__\xFD[a\x1C$\x82a+\xC0V[_` \x82\x84\x03\x12\x15a+\xFFW__\xFD[P5\x91\x90PV[_a\x05\0\x82\x01\x90P\x82Q\x82R` \x83\x01Q` \x83\x01R`@\x83\x01Qa,8`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x83\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x83\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x83\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x83\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x83\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x83\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x83\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x83\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x83\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x83\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x83\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x83\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x83\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x83\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x83\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x83\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x83\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x83\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x90\x92\x01Qa\x04\xE0\x90\x91\x01R\x90V[\x805`\x01`\x01`@\x1B\x03\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a.-W__\xFD[a\x1C$\x82a.\x07V[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.mWa.ma.6V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.mWa.ma.6V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.\xBDWa.\xBDa.6V[`@R\x91\x90PV[_``\x82\x84\x03\x12\x15a.\xD5W__\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a.\xF7Wa.\xF7a.6V[`@R\x90P\x80a/\x06\x83a.\x07V[\x81Ra/\x14` \x84\x01a.\x07V[` \x82\x01R`@\x92\x83\x015\x92\x01\x91\x90\x91R\x91\x90PV[_`@\x82\x84\x03\x12\x15a/:W__\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15a/\\Wa/\\a.6V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a/\x87W__\xFD[a/\x8Fa.JV[\x90Pa/\x9B\x83\x83a/*V[\x81Ra/\xAA\x83`@\x84\x01a/*V[` \x82\x01Ra/\xBC\x83`\x80\x84\x01a/*V[`@\x82\x01Ra/\xCE\x83`\xC0\x84\x01a/*V[``\x82\x01Ra/\xE1\x83a\x01\0\x84\x01a/*V[`\x80\x82\x01Ra/\xF4\x83a\x01@\x84\x01a/*V[`\xA0\x82\x01Ra0\x07\x83a\x01\x80\x84\x01a/*V[`\xC0\x82\x01Ra0\x1A\x83a\x01\xC0\x84\x01a/*V[`\xE0\x82\x01Ra0-\x83a\x02\0\x84\x01a/*V[a\x01\0\x82\x01Ra0A\x83a\x02@\x84\x01a/*V[a\x01 \x82\x01Ra0U\x83a\x02\x80\x84\x01a/*V[a\x01@\x82\x01Ra0i\x83a\x02\xC0\x84\x01a/*V[a\x01`\x82\x01Ra0}\x83a\x03\0\x84\x01a/*V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[__a\x04\xE0\x83\x85\x03\x12\x15a1\x13W__\xFD[a1\x1D\x84\x84a.\xC5V[\x91Pa1,\x84``\x85\x01a/vV[\x90P\x92P\x92\x90PV[\x805c\xFF\xFF\xFF\xFF\x81\x16\x81\x14a\x11\x82W__\xFD[_` \x82\x84\x03\x12\x15a1XW__\xFD[a\x1C$\x82a15V[__`@\x83\x85\x03\x12\x15a1rW__\xFD[a1{\x83a+\xC0V[\x91P` \x83\x015`\x01`\x01`@\x1B\x03\x81\x11\x15a1\x95W__\xFD[\x83\x01`\x1F\x81\x01\x85\x13a1\xA5W__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a1\xBEWa1\xBEa.6V[a1\xD1`\x1F\x82\x01`\x1F\x19\x16` \x01a.\x95V[\x81\x81R\x86` \x83\x85\x01\x01\x11\x15a1\xE5W__\xFD[\x81` \x84\x01` \x83\x017_` \x83\x83\x01\x01R\x80\x93PPPP\x92P\x92\x90PV[_`\x80\x82\x84\x03\x12\x15a2\x14W__\xFD[a2\x1Ca.sV[\x825\x81R` \x80\x84\x015\x90\x82\x01R`@\x80\x84\x015\x90\x82\x01R``\x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a2WW__\xFD[a\x1C$\x83\x83a2\x04V[___a\x05`\x84\x86\x03\x12\x15a2tW__\xFD[a2~\x85\x85a.\xC5V[\x92Pa2\x8D\x85``\x86\x01a2\x04V[\x91Pa2\x9C\x85`\xE0\x86\x01a/vV[\x90P\x92P\x92P\x92V[__`@\x83\x85\x03\x12\x15a2\xB6W__\xFD[a2\xBF\x83a.\x07V[\x91Pa1,` \x84\x01a.\x07V[____a\x01 \x85\x87\x03\x12\x15a2\xE1W__\xFD[a2\xEB\x86\x86a.\xC5V[\x93Pa2\xFA\x86``\x87\x01a2\x04V[\x92Pa3\x08`\xE0\x86\x01a15V[\x91Pa3\x17a\x01\0\x86\x01a+\xC0V[\x90P\x92\x95\x91\x94P\x92PV[____a\x05\x80\x85\x87\x03\x12\x15a36W__\xFD[a3@\x86\x86a.\xC5V[\x93Pa3O\x86``\x87\x01a2\x04V[\x92P`\xE0\x85\x015\x91Pa3\x17\x86a\x01\0\x87\x01a/vV[` \x81R_\x82Q\x80` \x84\x01R\x80` \x85\x01`@\x85\x01^_`@\x82\x85\x01\x01R`@`\x1F\x19`\x1F\x83\x01\x16\x84\x01\x01\x91PP\x92\x91PPV[_``\x82\x84\x03\x12\x15a3\xABW__\xFD[a\x1C$\x83\x83a.\xC5V[__`@\x83\x85\x03\x12\x15a3\xC6W__\xFD[PP\x805\x92` \x90\x91\x015\x91PV[_` \x82\x84\x03\x12\x15a3\xE5W__\xFD[\x815`\x01`\x01`@\x1B\x03\x81\x11\x15a3\xFAW__\xFD[\x82\x01`\x1F\x81\x01\x84\x13a4\nW__\xFD[\x805`\x01`\x01`@\x1B\x03\x81\x11\x15a4#Wa4#a.6V[a42` \x82`\x05\x1B\x01a.\x95V[\x80\x82\x82R` \x82\x01\x91P` \x83`\x07\x1B\x85\x01\x01\x92P\x86\x83\x11\x15a4SW__\xFD[` \x84\x01\x93P[\x82\x84\x10\x15a4\xC3W`\x80\x84\x88\x03\x12\x15a4qW__\xFD[a4ya.sV[a4\x82\x85a.\x07V[\x81Ra4\x90` \x86\x01a.\x07V[` \x82\x01Ra4\xA1`@\x86\x01a.\x07V[`@\x82\x01R``\x85\x81\x015\x90\x82\x01R\x82R`\x80\x90\x93\x01\x92` \x90\x91\x01\x90a4ZV[\x96\x95PPPPPPV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[`\x01`\x01`@\x1B\x03\x81\x81\x16\x83\x82\x16\x01\x90\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a5,Wa5,a5\0V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x06\x91PP\x92\x91PPV[`\x01`\x01`@\x1B\x03\x82\x81\x16\x82\x82\x16\x03\x90\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[\x81\x81\x03\x81\x81\x11\x15a\x12\x16Wa\x12\x16a4\xCDV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_`\x01`\x01`@\x1B\x03\x83\x16\x80a5\x9FWa5\x9Fa5\0V[\x80`\x01`\x01`@\x1B\x03\x84\x16\x04\x91PP\x92\x91PPV[_` \x82\x84\x03\x12\x15a5\xC4W__\xFD[PQ\x91\x90PV[_\x81Q\x80` \x84\x01\x85^_\x93\x01\x92\x83RP\x90\x91\x90PV[_a5\xF6a5\xF0\x83\x87a5\xCBV[\x85a5\xCBV[\x92\x83RPP` \x01\x92\x91PPV[_\x82a6\x12Wa6\x12a5\0V[P\x06\x90V[\x80_[`\x05\x81\x10\x15a69W\x81Q\x84R` \x93\x84\x01\x93\x90\x91\x01\x90`\x01\x01a6\x1AV[PPPPV[a6T\x82\x82Q\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[` \x81\x81\x01Q\x80Q`@\x85\x01R\x90\x81\x01Q``\x84\x01RP`@\x81\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP``\x81\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\x80\x81\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xA0\x81\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xC0\x81\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RP`\xE0\x81\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01\0\x81\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01 \x81\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01@\x81\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01`\x81\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\x80\x81\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xA0\x81\x01Qa\x03@\x83\x01Ra\x01\xC0\x81\x01Qa\x03`\x83\x01Ra\x01\xE0\x81\x01Qa\x03\x80\x83\x01Ra\x02\0\x81\x01Qa\x03\xA0\x83\x01Ra\x02 \x81\x01Qa\x03\xC0\x83\x01Ra\x02@\x81\x01Qa\x03\xE0\x83\x01Ra\x02`\x81\x01Qa\x04\0\x83\x01Ra\x02\x80\x81\x01Qa\x04 \x83\x01Ra\x02\xA0\x81\x01Qa\x04@\x83\x01Ra\x02\xC0\x01Qa\x04`\x90\x91\x01RV[_a\n \x82\x01\x90P\x84Q\x82R` \x85\x01Q` \x83\x01R`@\x85\x01Qa8+`@\x84\x01\x82\x80Q\x82R` \x90\x81\x01Q\x91\x01RV[P``\x85\x01Q\x80Q`\x80\x84\x01R` \x81\x01Q`\xA0\x84\x01RP`\x80\x85\x01Q\x80Q`\xC0\x84\x01R` \x81\x01Q`\xE0\x84\x01RP`\xA0\x85\x01Q\x80Qa\x01\0\x84\x01R` \x81\x01Qa\x01 \x84\x01RP`\xC0\x85\x01Q\x80Qa\x01@\x84\x01R` \x81\x01Qa\x01`\x84\x01RP`\xE0\x85\x01Q\x80Qa\x01\x80\x84\x01R` \x81\x01Qa\x01\xA0\x84\x01RPa\x01\0\x85\x01Q\x80Qa\x01\xC0\x84\x01R` \x81\x01Qa\x01\xE0\x84\x01RPa\x01 \x85\x01Q\x80Qa\x02\0\x84\x01R` \x81\x01Qa\x02 \x84\x01RPa\x01@\x85\x01Q\x80Qa\x02@\x84\x01R` \x81\x01Qa\x02`\x84\x01RPa\x01`\x85\x01Q\x80Qa\x02\x80\x84\x01R` \x81\x01Qa\x02\xA0\x84\x01RPa\x01\x80\x85\x01Q\x80Qa\x02\xC0\x84\x01R` \x81\x01Qa\x02\xE0\x84\x01RPa\x01\xA0\x85\x01Q\x80Qa\x03\0\x84\x01R` \x81\x01Qa\x03 \x84\x01RPa\x01\xC0\x85\x01Q\x80Qa\x03@\x84\x01R` \x81\x01Qa\x03`\x84\x01RPa\x01\xE0\x85\x01Q\x80Qa\x03\x80\x84\x01R` \x81\x01Qa\x03\xA0\x84\x01RPa\x02\0\x85\x01Q\x80Qa\x03\xC0\x84\x01R` \x81\x01Qa\x03\xE0\x84\x01RPa\x02 \x85\x01Q\x80Qa\x04\0\x84\x01R` \x81\x01Qa\x04 \x84\x01RPa\x02@\x85\x01Q\x80Qa\x04@\x84\x01R` \x81\x01Qa\x04`\x84\x01RPa\x02`\x85\x01Q\x80Qa\x04\x80\x84\x01R` \x81\x01Qa\x04\xA0\x84\x01RPa\x02\x80\x85\x01Qa\x04\xC0\x83\x01Ra\x02\xA0\x85\x01Qa\x04\xE0\x83\x01Ra:\x03a\x05\0\x83\x01\x85a6\x17V[a:\x11a\x05\xA0\x83\x01\x84a6?V[\x94\x93PPPPV[_` \x82\x84\x03\x12\x15a:)W__\xFD[\x81Q\x80\x15\x15\x81\x14a\x1C$W__\xFD[_`\x01`\x01`@\x1B\x03\x82\x16`\x01`\x01`@\x1B\x03\x81\x03a:YWa:Ya4\xCDV[`\x01\x01\x92\x91PPV[_\x81a:pWa:pa4\xCDV[P_\x19\x01\x90V[_a\x1C$\x82\x84a5\xCBV\xFE6\x08\x94\xA1;\xA1\xA3!\x06g\xC8(I-\xB9\x8D\xCA> v\xCC75\xA9 \xA3\xCAP]8+\xBC\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\xA1dsolcC\0\x08\x1C\0\n",
    );
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
    /**Custom error with signature `DeprecatedApi()` and selector `0x4e405c8d`.
```solidity
error DeprecatedApi();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct DeprecatedApi {}
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
        impl ::core::convert::From<DeprecatedApi> for UnderlyingRustTuple<'_> {
            fn from(value: DeprecatedApi) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for DeprecatedApi {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for DeprecatedApi {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
        impl ::core::convert::From<InsufficientSnapshotHistory>
        for UnderlyingRustTuple<'_> {
            fn from(value: InsufficientSnapshotHistory) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for InsufficientSnapshotHistory {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InsufficientSnapshotHistory {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
        impl ::core::convert::From<InvalidHotShotBlockForCommitmentCheck>
        for UnderlyingRustTuple<'_> {
            fn from(value: InvalidHotShotBlockForCommitmentCheck) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>>
        for InvalidHotShotBlockForCommitmentCheck {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidHotShotBlockForCommitmentCheck {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `MissingEpochRootUpdate()` and selector `0x080ae8d9`.
```solidity
error MissingEpochRootUpdate();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct MissingEpochRootUpdate {}
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
        impl ::core::convert::From<MissingEpochRootUpdate> for UnderlyingRustTuple<'_> {
            fn from(value: MissingEpochRootUpdate) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for MissingEpochRootUpdate {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for MissingEpochRootUpdate {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "NewEpoch(uint64)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                49u8, 234u8, 189u8, 144u8, 153u8, 253u8, 178u8, 93u8, 172u8, 221u8,
                210u8, 6u8, 171u8, 255u8, 135u8, 49u8, 30u8, 85u8, 52u8, 65u8, 252u8,
                157u8, 15u8, 205u8, 239u8, 32u8, 16u8, 98u8, 215u8, 231u8, 7u8, 27u8,
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
                    > as alloy_sol_types::SolType>::tokenize(&self.epoch),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (
                alloy_sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            const SIGNATURE: &'static str = "NewState(uint64,uint64,uint256)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                160u8, 74u8, 119u8, 57u8, 36u8, 80u8, 90u8, 65u8, 133u8, 100u8, 54u8,
                55u8, 37u8, 245u8, 104u8, 50u8, 245u8, 119u8, 46u8, 107u8, 141u8, 13u8,
                189u8, 110u8, 252u8, 231u8, 36u8, 223u8, 232u8, 3u8, 218u8, 230u8,
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
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(
                        &self.blockCommRoot,
                    ),
                )
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
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
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "PermissionedProverNotRequired()";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                154u8, 95u8, 87u8, 222u8, 133u8, 109u8, 214u8, 104u8, 197u8, 77u8, 217u8,
                94u8, 92u8, 85u8, 223u8, 147u8, 67u8, 33u8, 113u8, 203u8, 202u8, 73u8,
                168u8, 119u8, 109u8, 86u8, 32u8, 234u8, 89u8, 192u8, 36u8, 80u8,
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
            fn from(
                this: &PermissionedProverNotRequired,
            ) -> alloy_sol_types::private::LogData {
                alloy_sol_types::SolEvent::encode_log_data(this)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type DataToken<'a> = <Self::DataTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type TopicList = (alloy_sol_types::sol_data::FixedBytes<32>,);
            const SIGNATURE: &'static str = "PermissionedProverRequired(address)";
            const SIGNATURE_HASH: alloy_sol_types::private::B256 = alloy_sol_types::private::B256::new([
                128u8, 23u8, 187u8, 136u8, 127u8, 223u8, 143u8, 202u8, 67u8, 20u8, 169u8,
                212u8, 15u8, 110u8, 115u8, 179u8, 184u8, 16u8, 2u8, 214u8, 126u8, 92u8,
                250u8, 133u8, 216u8, 129u8, 115u8, 175u8, 106u8, 164u8, 96u8, 114u8,
            ]);
            const ANONYMOUS: bool = false;
            #[allow(unused_variables)]
            #[inline]
            fn new(
                topics: <Self::TopicList as alloy_sol_types::SolType>::RustType,
                data: <Self::DataTuple<'_> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                Self { permissionedProver: data.0 }
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
                out[0usize] = alloy_sol_types::abi::token::WordToken(
                    Self::SIGNATURE_HASH,
                );
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
            fn from(
                this: &PermissionedProverRequired,
            ) -> alloy_sol_types::private::LogData {
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `_getVk()` and selector `0x12173c2c`.
```solidity
function _getVk() external pure returns (IPlonkVerifier.VerifyingKey memory vk);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct _getVkCall {}
    #[derive()]
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
            impl ::core::convert::From<_getVkCall> for UnderlyingRustTuple<'_> {
                fn from(value: _getVkCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for _getVkCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (IPlonkVerifier::VerifyingKey,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = _getVkReturn;
            type ReturnTuple<'a> = (IPlonkVerifier::VerifyingKey,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `authRoot()` and selector `0x998328e8`.
```solidity
function authRoot() external view returns (uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct authRootCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<authRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: authRootCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for authRootCall {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = authRootReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `blocksPerEpoch()` and selector `0xf0682054`.
```solidity
function blocksPerEpoch() external view returns (uint64);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct blocksPerEpochCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<blocksPerEpochCall> for UnderlyingRustTuple<'_> {
                fn from(value: blocksPerEpochCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for blocksPerEpochCall {
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
            impl ::core::convert::From<blocksPerEpochReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: blocksPerEpochReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for blocksPerEpochReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for blocksPerEpochCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = blocksPerEpochReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `currentBlockNumber()` and selector `0x378ec23b`.
```solidity
function currentBlockNumber() external view returns (uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentBlockNumberCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<currentBlockNumberCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: currentBlockNumberCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for currentBlockNumberCall {
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
            impl ::core::convert::From<currentBlockNumberReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: currentBlockNumberReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for currentBlockNumberReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for currentBlockNumberCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = currentBlockNumberReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `currentEpoch()` and selector `0x76671808`.
```solidity
function currentEpoch() external view returns (uint64);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct currentEpochCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<currentEpochCall> for UnderlyingRustTuple<'_> {
                fn from(value: currentEpochCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for currentEpochCall {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = currentEpochReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            impl ::core::convert::From<disablePermissionedProverModeCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: disablePermissionedProverModeCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for disablePermissionedProverModeCall {
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
            impl ::core::convert::From<disablePermissionedProverModeReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: disablePermissionedProverModeReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for disablePermissionedProverModeReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for disablePermissionedProverModeCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = disablePermissionedProverModeReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<epochFromBlockNumberCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: epochFromBlockNumberCall) -> Self {
                    (value._blockNum, value._blocksPerEpoch)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for epochFromBlockNumberCall {
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
            impl ::core::convert::From<epochFromBlockNumberReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: epochFromBlockNumberReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for epochFromBlockNumberReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = epochFromBlockNumberReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self._blockNum),
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self._blocksPerEpoch),
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
    /**Function with signature `epochStartBlock()` and selector `0x3ed55b7b`.
```solidity
function epochStartBlock() external view returns (uint64);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct epochStartBlockCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<epochStartBlockCall> for UnderlyingRustTuple<'_> {
                fn from(value: epochStartBlockCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for epochStartBlockCall {
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
            impl ::core::convert::From<epochStartBlockReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: epochStartBlockReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for epochStartBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for epochStartBlockCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = epochStartBlockReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `finalizedState()` and selector `0x9fdb54a7`.
```solidity
function finalizedState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct finalizedStateCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<finalizedStateReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: finalizedStateReturn) -> Self {
                    (value.viewNum, value.blockHeight, value.blockCommRoot)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for finalizedStateReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = finalizedStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `firstEpoch()` and selector `0x41682744`.
```solidity
function firstEpoch() external view returns (uint64);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct firstEpochCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`firstEpoch()`](firstEpochCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct firstEpochReturn {
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
            impl ::core::convert::From<firstEpochCall> for UnderlyingRustTuple<'_> {
                fn from(value: firstEpochCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for firstEpochCall {
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
            impl ::core::convert::From<firstEpochReturn> for UnderlyingRustTuple<'_> {
                fn from(value: firstEpochReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for firstEpochReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for firstEpochCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = firstEpochReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "firstEpoch()";
            const SELECTOR: [u8; 4] = [65u8, 104u8, 39u8, 68u8];
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
    /**Function with signature `genesisStakeTableState()` and selector `0x426d3194`.
```solidity
function genesisStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStakeTableStateCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<genesisStakeTableStateCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: genesisStakeTableStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for genesisStakeTableStateCall {
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
            impl ::core::convert::From<genesisStakeTableStateReturn>
            for UnderlyingRustTuple<'_> {
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
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for genesisStakeTableStateReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = genesisStakeTableStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                BN254::ScalarField,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `genesisState()` and selector `0xd24d933d`.
```solidity
function genesisState() external view returns (uint64 viewNum, uint64 blockHeight, BN254.ScalarField blockCommRoot);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct genesisStateCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = genesisStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<getHotShotCommitmentCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: getHotShotCommitmentCall) -> Self {
                    (value.hotShotBlockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for getHotShotCommitmentCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        hotShotBlockHeight: tuple.0,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (
                BN254::ScalarField,
                alloy::sol_types::sol_data::Uint<64>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
                u64,
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
            impl ::core::convert::From<getHotShotCommitmentReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: getHotShotCommitmentReturn) -> Self {
                    (value.hotShotBlockCommRoot, value.hotshotBlockHeight)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for getHotShotCommitmentReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = getHotShotCommitmentReturn;
            type ReturnTuple<'a> = (
                BN254::ScalarField,
                alloy::sol_types::sol_data::Uint<64>,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.hotShotBlockHeight),
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
    /**Function with signature `getStateHistoryCount()` and selector `0xf9e50d19`.
```solidity
function getStateHistoryCount() external view returns (uint256);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct getStateHistoryCountCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<getStateHistoryCountCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: getStateHistoryCountCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for getStateHistoryCountCall {
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
            impl ::core::convert::From<getStateHistoryCountReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: getStateHistoryCountReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for getStateHistoryCountReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for getStateHistoryCountCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = getStateHistoryCountReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
        pub _genesisStakeTableState: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
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
                LightClient::LightClientState,
                LightClient::StakeTableState,
                alloy::sol_types::sol_data::Uint<32>,
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
                    <alloy::sol_types::sol_data::Uint<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeV2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self._blocksPerEpoch),
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self._epochStartBlock),
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
    /**Function with signature `initializeV3()` and selector `0x38e454b1`.
```solidity
function initializeV3() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct initializeV3Call {}
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
            impl ::core::convert::From<initializeV3Call> for UnderlyingRustTuple<'_> {
                fn from(value: initializeV3Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for initializeV3Call {
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
        #[automatically_derived]
        impl alloy_sol_types::SolCall for initializeV3Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = initializeV3Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<isEpochRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: isEpochRootCall) -> Self {
                    (value.blockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isEpochRootCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { blockHeight: tuple.0 }
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = isEpochRootReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.blockHeight),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<isGtEpochRootCall> for UnderlyingRustTuple<'_> {
                fn from(value: isGtEpochRootCall) -> Self {
                    (value.blockHeight,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for isGtEpochRootCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { blockHeight: tuple.0 }
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = isGtEpochRootReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.blockHeight),
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
    /**Function with signature `isPermissionedProverEnabled()` and selector `0x826e41fc`.
```solidity
function isPermissionedProverEnabled() external view returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct isPermissionedProverEnabledCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<isPermissionedProverEnabledCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: isPermissionedProverEnabledCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for isPermissionedProverEnabledCall {
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
            impl ::core::convert::From<isPermissionedProverEnabledReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: isPermissionedProverEnabledReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for isPermissionedProverEnabledReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for isPermissionedProverEnabledCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = isPermissionedProverEnabledReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `lagOverEscapeHatchThreshold(uint256,uint256)` and selector `0xe0303301`.
```solidity
function lagOverEscapeHatchThreshold(uint256 blockNumber, uint256 threshold) external view returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct lagOverEscapeHatchThresholdCall {
        #[allow(missing_docs)]
        pub blockNumber: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub threshold: alloy::sol_types::private::primitives::aliases::U256,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<lagOverEscapeHatchThresholdCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: lagOverEscapeHatchThresholdCall) -> Self {
                    (value.blockNumber, value.threshold)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for lagOverEscapeHatchThresholdCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        blockNumber: tuple.0,
                        threshold: tuple.1,
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
            impl ::core::convert::From<lagOverEscapeHatchThresholdReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: lagOverEscapeHatchThresholdReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for lagOverEscapeHatchThresholdReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = lagOverEscapeHatchThresholdReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.blockNumber),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.threshold),
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
            type UnderlyingSolTuple<'a> = (
                LightClient::LightClientState,
                IPlonkVerifier::PlonkProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
                <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<newFinalizedState_0Call>
            for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_0Call) -> Self {
                    (value._0, value._1)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_0Call {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0, _1: tuple.1 }
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
            impl ::core::convert::From<newFinalizedState_0Return>
            for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_0Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedState_0Call {
            type Parameters<'a> = (
                LightClient::LightClientState,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_0Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(
                        &self._0,
                    ),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(
                        &self._1,
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
    #[derive()]
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
            impl ::core::convert::From<newFinalizedState_1Call>
            for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_1Call) -> Self {
                    (value._0, value._1, value._2)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_1Call {
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
            impl ::core::convert::From<newFinalizedState_1Return>
            for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_1Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newFinalizedState_1Call {
            type Parameters<'a> = (
                LightClient::LightClientState,
                LightClient::StakeTableState,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_1Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <LightClient::LightClientState as alloy_sol_types::SolType>::tokenize(
                        &self._0,
                    ),
                    <LightClient::StakeTableState as alloy_sol_types::SolType>::tokenize(
                        &self._1,
                    ),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(
                        &self._2,
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
    #[derive()]
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
            impl ::core::convert::From<newFinalizedState_2Call>
            for UnderlyingRustTuple<'_> {
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
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_2Call {
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
            impl ::core::convert::From<newFinalizedState_2Return>
            for UnderlyingRustTuple<'_> {
                fn from(value: newFinalizedState_2Return) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for newFinalizedState_2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = newFinalizedState_2Return;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.newAuthRoot),
                    <IPlonkVerifier::PlonkProof as alloy_sol_types::SolType>::tokenize(
                        &self.proof,
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
    /**Function with signature `permissionedProver()` and selector `0x313df7b1`.
```solidity
function permissionedProver() external view returns (address);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct permissionedProverCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<permissionedProverCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: permissionedProverCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for permissionedProverCall {
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
            impl ::core::convert::From<permissionedProverReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: permissionedProverReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for permissionedProverReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for permissionedProverCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = permissionedProverReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Address,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    /**Function with signature `setBlocksPerEpoch(uint64)` and selector `0x3c23b6db`.
```solidity
function setBlocksPerEpoch(uint64 newBlocksPerEpoch) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setBlocksPerEpochCall {
        #[allow(missing_docs)]
        pub newBlocksPerEpoch: u64,
    }
    ///Container type for the return parameters of the [`setBlocksPerEpoch(uint64)`](setBlocksPerEpochCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setBlocksPerEpochReturn {}
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
            impl ::core::convert::From<setBlocksPerEpochCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setBlocksPerEpochCall) -> Self {
                    (value.newBlocksPerEpoch,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setBlocksPerEpochCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { newBlocksPerEpoch: tuple.0 }
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
            impl ::core::convert::From<setBlocksPerEpochReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setBlocksPerEpochReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setBlocksPerEpochReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setBlocksPerEpochCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setBlocksPerEpochReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setBlocksPerEpoch(uint64)";
            const SELECTOR: [u8; 4] = [60u8, 35u8, 182u8, 219u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.newBlocksPerEpoch),
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
    /**Function with signature `setFinalizedState((uint64,uint64,uint256))` and selector `0xb5adea3c`.
```solidity
function setFinalizedState(LightClient.LightClientState memory state) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setFinalizedStateCall {
        #[allow(missing_docs)]
        pub state: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`setFinalizedState((uint64,uint64,uint256))`](setFinalizedStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setFinalizedStateReturn {}
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
            type UnderlyingSolTuple<'a> = (LightClient::LightClientState,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<setFinalizedStateCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setFinalizedStateCall) -> Self {
                    (value.state,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setFinalizedStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { state: tuple.0 }
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
            impl ::core::convert::From<setFinalizedStateReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setFinalizedStateReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setFinalizedStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setFinalizedStateCall {
            type Parameters<'a> = (LightClient::LightClientState,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setFinalizedStateReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setFinalizedState((uint64,uint64,uint256))";
            const SELECTOR: [u8; 4] = [181u8, 173u8, 234u8, 60u8];
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
                        &self.state,
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
    /**Function with signature `setHotShotDownSince(uint256)` and selector `0x2d52aad6`.
```solidity
function setHotShotDownSince(uint256 l1Height) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setHotShotDownSinceCall {
        #[allow(missing_docs)]
        pub l1Height: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`setHotShotDownSince(uint256)`](setHotShotDownSinceCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setHotShotDownSinceReturn {}
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
            impl ::core::convert::From<setHotShotDownSinceCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setHotShotDownSinceCall) -> Self {
                    (value.l1Height,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setHotShotDownSinceCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { l1Height: tuple.0 }
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
            impl ::core::convert::From<setHotShotDownSinceReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setHotShotDownSinceReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setHotShotDownSinceReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setHotShotDownSinceCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setHotShotDownSinceReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setHotShotDownSince(uint256)";
            const SELECTOR: [u8; 4] = [45u8, 82u8, 170u8, 214u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.l1Height),
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
    /**Function with signature `setHotShotUp()` and selector `0xc8e5e498`.
```solidity
function setHotShotUp() external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setHotShotUpCall {}
    ///Container type for the return parameters of the [`setHotShotUp()`](setHotShotUpCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setHotShotUpReturn {}
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
            impl ::core::convert::From<setHotShotUpCall> for UnderlyingRustTuple<'_> {
                fn from(value: setHotShotUpCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setHotShotUpCall {
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
            impl ::core::convert::From<setHotShotUpReturn> for UnderlyingRustTuple<'_> {
                fn from(value: setHotShotUpReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setHotShotUpReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setHotShotUpCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setHotShotUpReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setHotShotUp()";
            const SELECTOR: [u8; 4] = [200u8, 229u8, 228u8, 152u8];
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
            impl ::core::convert::From<setPermissionedProverCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setPermissionedProverCall) -> Self {
                    (value.prover,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setPermissionedProverCall {
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
            impl ::core::convert::From<setPermissionedProverReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setPermissionedProverReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setPermissionedProverReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setPermissionedProverCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Address,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setPermissionedProverReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `setStateHistory((uint64,uint64,uint64,uint256)[])` and selector `0xf5676160`.
```solidity
function setStateHistory(LightClient.StateHistoryCommitment[] memory _stateHistoryCommitments) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setStateHistoryCall {
        #[allow(missing_docs)]
        pub _stateHistoryCommitments: alloy::sol_types::private::Vec<
            <LightClient::StateHistoryCommitment as alloy::sol_types::SolType>::RustType,
        >,
    }
    ///Container type for the return parameters of the [`setStateHistory((uint64,uint64,uint64,uint256)[])`](setStateHistoryCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setStateHistoryReturn {}
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
                alloy::sol_types::sol_data::Array<LightClient::StateHistoryCommitment>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::Vec<
                    <LightClient::StateHistoryCommitment as alloy::sol_types::SolType>::RustType,
                >,
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
            impl ::core::convert::From<setStateHistoryCall> for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryCall) -> Self {
                    (value._stateHistoryCommitments,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for setStateHistoryCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        _stateHistoryCommitments: tuple.0,
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
            impl ::core::convert::From<setStateHistoryReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setStateHistoryReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setStateHistoryCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::Array<LightClient::StateHistoryCommitment>,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setStateHistoryReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setStateHistory((uint64,uint64,uint64,uint256)[])";
            const SELECTOR: [u8; 4] = [245u8, 103u8, 97u8, 96u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Array<
                        LightClient::StateHistoryCommitment,
                    > as alloy_sol_types::SolType>::tokenize(
                        &self._stateHistoryCommitments,
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
            impl ::core::convert::From<setStateHistoryRetentionPeriodCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryRetentionPeriodCall) -> Self {
                    (value.historySeconds,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setStateHistoryRetentionPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { historySeconds: tuple.0 }
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
            impl ::core::convert::From<setStateHistoryRetentionPeriodReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setStateHistoryRetentionPeriodReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setStateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setStateHistoryRetentionPeriodCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setStateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.historySeconds),
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
    /**Function with signature `setVotingStakeTableState((uint256,uint256,uint256,uint256))` and selector `0x623a1338`.
```solidity
function setVotingStakeTableState(LightClient.StakeTableState memory stake) external;
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setVotingStakeTableStateCall {
        #[allow(missing_docs)]
        pub stake: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`setVotingStakeTableState((uint256,uint256,uint256,uint256))`](setVotingStakeTableStateCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct setVotingStakeTableStateReturn {}
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
            type UnderlyingSolTuple<'a> = (LightClient::StakeTableState,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<setVotingStakeTableStateCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setVotingStakeTableStateCall) -> Self {
                    (value.stake,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setVotingStakeTableStateCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { stake: tuple.0 }
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
            impl ::core::convert::From<setVotingStakeTableStateReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setVotingStakeTableStateReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setVotingStakeTableStateReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setVotingStakeTableStateCall {
            type Parameters<'a> = (LightClient::StakeTableState,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setVotingStakeTableStateReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "setVotingStakeTableState((uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [98u8, 58u8, 19u8, 56u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <LightClient::StakeTableState as alloy_sol_types::SolType>::tokenize(
                        &self.stake,
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
            impl ::core::convert::From<setstateHistoryRetentionPeriodCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: setstateHistoryRetentionPeriodCall) -> Self {
                    (value.historySeconds,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setstateHistoryRetentionPeriodCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { historySeconds: tuple.0 }
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
            impl ::core::convert::From<setstateHistoryRetentionPeriodReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: setstateHistoryRetentionPeriodReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for setstateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for setstateHistoryRetentionPeriodCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = setstateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        32,
                    > as alloy_sol_types::SolType>::tokenize(&self.historySeconds),
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<stateHistoryCommitmentsCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryCommitmentsCall) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryCommitmentsCall {
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
            impl ::core::convert::From<stateHistoryCommitmentsReturn>
            for UnderlyingRustTuple<'_> {
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
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryCommitmentsReturn {
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
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryCommitmentsReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                alloy::sol_types::sol_data::Uint<64>,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self._0),
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
    /**Function with signature `stateHistoryFirstIndex()` and selector `0x2f79889d`.
```solidity
function stateHistoryFirstIndex() external view returns (uint64);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryFirstIndexCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<stateHistoryFirstIndexCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryFirstIndexCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryFirstIndexCall {
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
            impl ::core::convert::From<stateHistoryFirstIndexReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryFirstIndexReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryFirstIndexReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for stateHistoryFirstIndexCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryFirstIndexReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data, validate)
                    .map(Into::into)
            }
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `stateHistoryRetentionPeriod()` and selector `0xc23b9e9e`.
```solidity
function stateHistoryRetentionPeriod() external view returns (uint32);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct stateHistoryRetentionPeriodCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<stateHistoryRetentionPeriodCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryRetentionPeriodCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryRetentionPeriodCall {
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
            impl ::core::convert::From<stateHistoryRetentionPeriodReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: stateHistoryRetentionPeriodReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for stateHistoryRetentionPeriodReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for stateHistoryRetentionPeriodCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = stateHistoryRetentionPeriodReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<32>,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
            impl ::core::convert::From<updateEpochStartBlockCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateEpochStartBlockCall) -> Self {
                    (value.newEpochStartBlock,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateEpochStartBlockCall {
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
            impl ::core::convert::From<updateEpochStartBlockReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: updateEpochStartBlockReturn) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for updateEpochStartBlockReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {}
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for updateEpochStartBlockCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<64>,);
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = updateEpochStartBlockReturn;
            type ReturnTuple<'a> = ();
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
                    <alloy::sol_types::sol_data::Uint<
                        64,
                    > as alloy_sol_types::SolType>::tokenize(&self.newEpochStartBlock),
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
    /**Function with signature `votingStakeTableState()` and selector `0x0625e19b`.
```solidity
function votingStakeTableState() external view returns (uint256 threshold, BN254.ScalarField blsKeyComm, BN254.ScalarField schnorrKeyComm, BN254.ScalarField amountComm);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct votingStakeTableStateCall {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
            impl ::core::convert::From<votingStakeTableStateCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: votingStakeTableStateCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for votingStakeTableStateCall {
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
            impl ::core::convert::From<votingStakeTableStateReturn>
            for UnderlyingRustTuple<'_> {
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
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for votingStakeTableStateReturn {
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
        impl alloy_sol_types::SolCall for votingStakeTableStateCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = votingStakeTableStateReturn;
            type ReturnTuple<'a> = (
                alloy::sol_types::sol_data::Uint<256>,
                BN254::ScalarField,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
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
    ///Container for all the [`LightClientV3Mock`](self) function calls.
    #[derive()]
    pub enum LightClientV3MockCalls {
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
        firstEpoch(firstEpochCall),
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
        setBlocksPerEpoch(setBlocksPerEpochCall),
        #[allow(missing_docs)]
        setFinalizedState(setFinalizedStateCall),
        #[allow(missing_docs)]
        setHotShotDownSince(setHotShotDownSinceCall),
        #[allow(missing_docs)]
        setHotShotUp(setHotShotUpCall),
        #[allow(missing_docs)]
        setPermissionedProver(setPermissionedProverCall),
        #[allow(missing_docs)]
        setStateHistory(setStateHistoryCall),
        #[allow(missing_docs)]
        setStateHistoryRetentionPeriod(setStateHistoryRetentionPeriodCall),
        #[allow(missing_docs)]
        setVotingStakeTableState(setVotingStakeTableStateCall),
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
    impl LightClientV3MockCalls {
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
            [45u8, 82u8, 170u8, 214u8],
            [47u8, 121u8, 136u8, 157u8],
            [48u8, 12u8, 137u8, 221u8],
            [49u8, 61u8, 247u8, 177u8],
            [55u8, 142u8, 194u8, 59u8],
            [56u8, 228u8, 84u8, 177u8],
            [60u8, 35u8, 182u8, 219u8],
            [62u8, 213u8, 91u8, 123u8],
            [65u8, 104u8, 39u8, 68u8],
            [66u8, 109u8, 49u8, 148u8],
            [67u8, 61u8, 186u8, 159u8],
            [79u8, 30u8, 242u8, 134u8],
            [82u8, 209u8, 144u8, 45u8],
            [98u8, 58u8, 19u8, 56u8],
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
            [181u8, 173u8, 234u8, 60u8],
            [194u8, 59u8, 158u8, 158u8],
            [200u8, 229u8, 228u8, 152u8],
            [210u8, 77u8, 147u8, 61u8],
            [224u8, 48u8, 51u8, 1u8],
            [240u8, 104u8, 32u8, 84u8],
            [242u8, 253u8, 227u8, 139u8],
            [245u8, 103u8, 97u8, 96u8],
            [249u8, 229u8, 13u8, 25u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for LightClientV3MockCalls {
        const NAME: &'static str = "LightClientV3MockCalls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 46usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::UPGRADE_INTERFACE_VERSION(_) => {
                    <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::_getVk(_) => <_getVkCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::authRoot(_) => <authRootCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::blocksPerEpoch(_) => {
                    <blocksPerEpochCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::currentBlockNumber(_) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::currentEpoch(_) => {
                    <currentEpochCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::disablePermissionedProverMode(_) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::epochFromBlockNumber(_) => {
                    <epochFromBlockNumberCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::epochStartBlock(_) => {
                    <epochStartBlockCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::finalizedState(_) => {
                    <finalizedStateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::firstEpoch(_) => {
                    <firstEpochCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::genesisStakeTableState(_) => {
                    <genesisStakeTableStateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::genesisState(_) => {
                    <genesisStateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::getHotShotCommitment(_) => {
                    <getHotShotCommitmentCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::getStateHistoryCount(_) => {
                    <getStateHistoryCountCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::getVersion(_) => {
                    <getVersionCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initialize(_) => {
                    <initializeCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initializeV2(_) => {
                    <initializeV2Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::initializeV3(_) => {
                    <initializeV3Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::isEpochRoot(_) => {
                    <isEpochRootCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::isGtEpochRoot(_) => {
                    <isGtEpochRootCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::isPermissionedProverEnabled(_) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::lagOverEscapeHatchThreshold(_) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::newFinalizedState_0(_) => {
                    <newFinalizedState_0Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::newFinalizedState_1(_) => {
                    <newFinalizedState_1Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::newFinalizedState_2(_) => {
                    <newFinalizedState_2Call as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::owner(_) => <ownerCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::permissionedProver(_) => {
                    <permissionedProverCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::proxiableUUID(_) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::renounceOwnership(_) => {
                    <renounceOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setBlocksPerEpoch(_) => {
                    <setBlocksPerEpochCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setFinalizedState(_) => {
                    <setFinalizedStateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setHotShotDownSince(_) => {
                    <setHotShotDownSinceCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setHotShotUp(_) => {
                    <setHotShotUpCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setPermissionedProver(_) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setStateHistory(_) => {
                    <setStateHistoryCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setStateHistoryRetentionPeriod(_) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setVotingStakeTableState(_) => {
                    <setVotingStakeTableStateCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::setstateHistoryRetentionPeriod(_) => {
                    <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::stateHistoryCommitments(_) => {
                    <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::stateHistoryFirstIndex(_) => {
                    <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::stateHistoryRetentionPeriod(_) => {
                    <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::transferOwnership(_) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::updateEpochStartBlock(_) => {
                    <updateEpochStartBlockCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::upgradeToAndCall(_) => {
                    <upgradeToAndCallCall as alloy_sol_types::SolCall>::SELECTOR
                }
                Self::votingStakeTableState(_) => {
                    <votingStakeTableStateCall as alloy_sol_types::SolCall>::SELECTOR
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
            ) -> alloy_sol_types::Result<LightClientV3MockCalls>] = &[
                {
                    fn setPermissionedProver(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setPermissionedProver)
                    }
                    setPermissionedProver
                },
                {
                    fn stateHistoryCommitments(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::stateHistoryCommitments)
                    }
                    stateHistoryCommitments
                },
                {
                    fn votingStakeTableState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::votingStakeTableState)
                    }
                    votingStakeTableState
                },
                {
                    fn getVersion(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <getVersionCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::getVersion)
                    }
                    getVersion
                },
                {
                    fn _getVk(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <_getVkCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::_getVk)
                    }
                    _getVk
                },
                {
                    fn updateEpochStartBlock(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::updateEpochStartBlock)
                    }
                    updateEpochStartBlock
                },
                {
                    fn newFinalizedState_0(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::newFinalizedState_0)
                    }
                    newFinalizedState_0
                },
                {
                    fn isEpochRoot(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <isEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::isEpochRoot)
                    }
                    isEpochRoot
                },
                {
                    fn setHotShotDownSince(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setHotShotDownSinceCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setHotShotDownSince)
                    }
                    setHotShotDownSince
                },
                {
                    fn stateHistoryFirstIndex(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::stateHistoryFirstIndex)
                    }
                    stateHistoryFirstIndex
                },
                {
                    fn isGtEpochRoot(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::isGtEpochRoot)
                    }
                    isGtEpochRoot
                },
                {
                    fn permissionedProver(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <permissionedProverCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::permissionedProver)
                    }
                    permissionedProver
                },
                {
                    fn currentBlockNumber(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::currentBlockNumber)
                    }
                    currentBlockNumber
                },
                {
                    fn initializeV3(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <initializeV3Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::initializeV3)
                    }
                    initializeV3
                },
                {
                    fn setBlocksPerEpoch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setBlocksPerEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setBlocksPerEpoch)
                    }
                    setBlocksPerEpoch
                },
                {
                    fn epochStartBlock(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <epochStartBlockCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::epochStartBlock)
                    }
                    epochStartBlock
                },
                {
                    fn firstEpoch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <firstEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::firstEpoch)
                    }
                    firstEpoch
                },
                {
                    fn genesisStakeTableState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::genesisStakeTableState)
                    }
                    genesisStakeTableState
                },
                {
                    fn setStateHistoryRetentionPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setStateHistoryRetentionPeriod)
                    }
                    setStateHistoryRetentionPeriod
                },
                {
                    fn upgradeToAndCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <upgradeToAndCallCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::upgradeToAndCall)
                    }
                    upgradeToAndCall
                },
                {
                    fn proxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::proxiableUUID)
                    }
                    proxiableUUID
                },
                {
                    fn setVotingStakeTableState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setVotingStakeTableStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setVotingStakeTableState)
                    }
                    setVotingStakeTableState
                },
                {
                    fn disablePermissionedProverMode(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::disablePermissionedProverMode)
                    }
                    disablePermissionedProverMode
                },
                {
                    fn renounceOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <renounceOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::renounceOwnership)
                    }
                    renounceOwnership
                },
                {
                    fn newFinalizedState_1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::newFinalizedState_1)
                    }
                    newFinalizedState_1
                },
                {
                    fn currentEpoch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <currentEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::currentEpoch)
                    }
                    currentEpoch
                },
                {
                    fn isPermissionedProverEnabled(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::isPermissionedProverEnabled)
                    }
                    isPermissionedProverEnabled
                },
                {
                    fn getHotShotCommitment(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::getHotShotCommitment)
                    }
                    getHotShotCommitment
                },
                {
                    fn owner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <ownerCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::owner)
                    }
                    owner
                },
                {
                    fn epochFromBlockNumber(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::epochFromBlockNumber)
                    }
                    epochFromBlockNumber
                },
                {
                    fn setstateHistoryRetentionPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setstateHistoryRetentionPeriod)
                    }
                    setstateHistoryRetentionPeriod
                },
                {
                    fn authRoot(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <authRootCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::authRoot)
                    }
                    authRoot
                },
                {
                    fn initialize(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <initializeCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::initialize)
                    }
                    initialize
                },
                {
                    fn finalizedState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <finalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::finalizedState)
                    }
                    finalizedState
                },
                {
                    fn newFinalizedState_2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::newFinalizedState_2)
                    }
                    newFinalizedState_2
                },
                {
                    fn UPGRADE_INTERFACE_VERSION(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <UPGRADE_INTERFACE_VERSIONCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::UPGRADE_INTERFACE_VERSION)
                    }
                    UPGRADE_INTERFACE_VERSION
                },
                {
                    fn initializeV2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <initializeV2Call as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::initializeV2)
                    }
                    initializeV2
                },
                {
                    fn setFinalizedState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setFinalizedStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setFinalizedState)
                    }
                    setFinalizedState
                },
                {
                    fn stateHistoryRetentionPeriod(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::stateHistoryRetentionPeriod)
                    }
                    stateHistoryRetentionPeriod
                },
                {
                    fn setHotShotUp(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setHotShotUpCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setHotShotUp)
                    }
                    setHotShotUp
                },
                {
                    fn genesisState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <genesisStateCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::genesisState)
                    }
                    genesisState
                },
                {
                    fn lagOverEscapeHatchThreshold(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::lagOverEscapeHatchThreshold)
                    }
                    lagOverEscapeHatchThreshold
                },
                {
                    fn blocksPerEpoch(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::blocksPerEpoch)
                    }
                    blocksPerEpoch
                },
                {
                    fn transferOwnership(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <transferOwnershipCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::transferOwnership)
                    }
                    transferOwnership
                },
                {
                    fn setStateHistory(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <setStateHistoryCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::setStateHistory)
                    }
                    setStateHistory
                },
                {
                    fn getStateHistoryCount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockCalls> {
                        <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockCalls::getStateHistoryCount)
                    }
                    getStateHistoryCount
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
                Self::firstEpoch(inner) => {
                    <firstEpochCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
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
                Self::setBlocksPerEpoch(inner) => {
                    <setBlocksPerEpochCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setFinalizedState(inner) => {
                    <setFinalizedStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setHotShotDownSince(inner) => {
                    <setHotShotDownSinceCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setHotShotUp(inner) => {
                    <setHotShotUpCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setPermissionedProver(inner) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setStateHistory(inner) => {
                    <setStateHistoryCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setStateHistoryRetentionPeriod(inner) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
                Self::setVotingStakeTableState(inner) => {
                    <setVotingStakeTableStateCall as alloy_sol_types::SolCall>::abi_encoded_size(
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
                        inner,
                        out,
                    )
                }
                Self::_getVk(inner) => {
                    <_getVkCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::authRoot(inner) => {
                    <authRootCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::blocksPerEpoch(inner) => {
                    <blocksPerEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::currentBlockNumber(inner) => {
                    <currentBlockNumberCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::currentEpoch(inner) => {
                    <currentEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::disablePermissionedProverMode(inner) => {
                    <disablePermissionedProverModeCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::epochFromBlockNumber(inner) => {
                    <epochFromBlockNumberCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::epochStartBlock(inner) => {
                    <epochStartBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::finalizedState(inner) => {
                    <finalizedStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::firstEpoch(inner) => {
                    <firstEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::genesisStakeTableState(inner) => {
                    <genesisStakeTableStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::genesisState(inner) => {
                    <genesisStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::getHotShotCommitment(inner) => {
                    <getHotShotCommitmentCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::getStateHistoryCount(inner) => {
                    <getStateHistoryCountCall as alloy_sol_types::SolCall>::abi_encode_raw(
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
                Self::initializeV3(inner) => {
                    <initializeV3Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::isEpochRoot(inner) => {
                    <isEpochRootCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::isGtEpochRoot(inner) => {
                    <isGtEpochRootCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::isPermissionedProverEnabled(inner) => {
                    <isPermissionedProverEnabledCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::lagOverEscapeHatchThreshold(inner) => {
                    <lagOverEscapeHatchThresholdCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::newFinalizedState_0(inner) => {
                    <newFinalizedState_0Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::newFinalizedState_1(inner) => {
                    <newFinalizedState_1Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::newFinalizedState_2(inner) => {
                    <newFinalizedState_2Call as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::owner(inner) => {
                    <ownerCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                }
                Self::permissionedProver(inner) => {
                    <permissionedProverCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::proxiableUUID(inner) => {
                    <proxiableUUIDCall as alloy_sol_types::SolCall>::abi_encode_raw(
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
                Self::setBlocksPerEpoch(inner) => {
                    <setBlocksPerEpochCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setFinalizedState(inner) => {
                    <setFinalizedStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setHotShotDownSince(inner) => {
                    <setHotShotDownSinceCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setHotShotUp(inner) => {
                    <setHotShotUpCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setPermissionedProver(inner) => {
                    <setPermissionedProverCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setStateHistory(inner) => {
                    <setStateHistoryCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setStateHistoryRetentionPeriod(inner) => {
                    <setStateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setVotingStakeTableState(inner) => {
                    <setVotingStakeTableStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::setstateHistoryRetentionPeriod(inner) => {
                    <setstateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::stateHistoryCommitments(inner) => {
                    <stateHistoryCommitmentsCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::stateHistoryFirstIndex(inner) => {
                    <stateHistoryFirstIndexCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::stateHistoryRetentionPeriod(inner) => {
                    <stateHistoryRetentionPeriodCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::transferOwnership(inner) => {
                    <transferOwnershipCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
                Self::updateEpochStartBlock(inner) => {
                    <updateEpochStartBlockCall as alloy_sol_types::SolCall>::abi_encode_raw(
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
                Self::votingStakeTableState(inner) => {
                    <votingStakeTableStateCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
            }
        }
    }
    ///Container for all the [`LightClientV3Mock`](self) custom errors.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum LightClientV3MockErrors {
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
    impl LightClientV3MockErrors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [5u8, 28u8, 70u8, 239u8],
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
    impl alloy_sol_types::SolInterface for LightClientV3MockErrors {
        const NAME: &'static str = "LightClientV3MockErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 22usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::AddressEmptyCode(_) => {
                    <AddressEmptyCode as alloy_sol_types::SolError>::SELECTOR
                }
                Self::DeprecatedApi(_) => {
                    <DeprecatedApi as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ERC1967InvalidImplementation(_) => {
                    <ERC1967InvalidImplementation as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ERC1967NonPayable(_) => {
                    <ERC1967NonPayable as alloy_sol_types::SolError>::SELECTOR
                }
                Self::FailedInnerCall(_) => {
                    <FailedInnerCall as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InsufficientSnapshotHistory(_) => {
                    <InsufficientSnapshotHistory as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidAddress(_) => {
                    <InvalidAddress as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidArgs(_) => {
                    <InvalidArgs as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidHotShotBlockForCommitmentCheck(_) => {
                    <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidInitialization(_) => {
                    <InvalidInitialization as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidMaxStateHistory(_) => {
                    <InvalidMaxStateHistory as alloy_sol_types::SolError>::SELECTOR
                }
                Self::InvalidProof(_) => {
                    <InvalidProof as alloy_sol_types::SolError>::SELECTOR
                }
                Self::MissingEpochRootUpdate(_) => {
                    <MissingEpochRootUpdate as alloy_sol_types::SolError>::SELECTOR
                }
                Self::NoChangeRequired(_) => {
                    <NoChangeRequired as alloy_sol_types::SolError>::SELECTOR
                }
                Self::NotInitializing(_) => {
                    <NotInitializing as alloy_sol_types::SolError>::SELECTOR
                }
                Self::OutdatedState(_) => {
                    <OutdatedState as alloy_sol_types::SolError>::SELECTOR
                }
                Self::OwnableInvalidOwner(_) => {
                    <OwnableInvalidOwner as alloy_sol_types::SolError>::SELECTOR
                }
                Self::OwnableUnauthorizedAccount(_) => {
                    <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::SELECTOR
                }
                Self::ProverNotPermissioned(_) => {
                    <ProverNotPermissioned as alloy_sol_types::SolError>::SELECTOR
                }
                Self::UUPSUnauthorizedCallContext(_) => {
                    <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::SELECTOR
                }
                Self::UUPSUnsupportedProxiableUUID(_) => {
                    <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::SELECTOR
                }
                Self::WrongStakeTableUsed(_) => {
                    <WrongStakeTableUsed as alloy_sol_types::SolError>::SELECTOR
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
            ) -> alloy_sol_types::Result<LightClientV3MockErrors>] = &[
                {
                    fn OutdatedState(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <OutdatedState as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::OutdatedState)
                    }
                    OutdatedState
                },
                {
                    fn MissingEpochRootUpdate(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <MissingEpochRootUpdate as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::MissingEpochRootUpdate)
                    }
                    MissingEpochRootUpdate
                },
                {
                    fn InvalidProof(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidProof as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InvalidProof)
                    }
                    InvalidProof
                },
                {
                    fn OwnableUnauthorizedAccount(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <OwnableUnauthorizedAccount as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::OwnableUnauthorizedAccount)
                    }
                    OwnableUnauthorizedAccount
                },
                {
                    fn FailedInnerCall(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <FailedInnerCall as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::FailedInnerCall)
                    }
                    FailedInnerCall
                },
                {
                    fn OwnableInvalidOwner(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <OwnableInvalidOwner as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::OwnableInvalidOwner)
                    }
                    OwnableInvalidOwner
                },
                {
                    fn ERC1967InvalidImplementation(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <ERC1967InvalidImplementation as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::ERC1967InvalidImplementation)
                    }
                    ERC1967InvalidImplementation
                },
                {
                    fn DeprecatedApi(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <DeprecatedApi as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::DeprecatedApi)
                    }
                    DeprecatedApi
                },
                {
                    fn WrongStakeTableUsed(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <WrongStakeTableUsed as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::WrongStakeTableUsed)
                    }
                    WrongStakeTableUsed
                },
                {
                    fn InvalidHotShotBlockForCommitmentCheck(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidHotShotBlockForCommitmentCheck as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(
                                LightClientV3MockErrors::InvalidHotShotBlockForCommitmentCheck,
                            )
                    }
                    InvalidHotShotBlockForCommitmentCheck
                },
                {
                    fn AddressEmptyCode(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <AddressEmptyCode as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::AddressEmptyCode)
                    }
                    AddressEmptyCode
                },
                {
                    fn InvalidArgs(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidArgs as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InvalidArgs)
                    }
                    InvalidArgs
                },
                {
                    fn ProverNotPermissioned(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <ProverNotPermissioned as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::ProverNotPermissioned)
                    }
                    ProverNotPermissioned
                },
                {
                    fn NoChangeRequired(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <NoChangeRequired as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::NoChangeRequired)
                    }
                    NoChangeRequired
                },
                {
                    fn UUPSUnsupportedProxiableUUID(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <UUPSUnsupportedProxiableUUID as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::UUPSUnsupportedProxiableUUID)
                    }
                    UUPSUnsupportedProxiableUUID
                },
                {
                    fn InsufficientSnapshotHistory(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InsufficientSnapshotHistory as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InsufficientSnapshotHistory)
                    }
                    InsufficientSnapshotHistory
                },
                {
                    fn ERC1967NonPayable(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <ERC1967NonPayable as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::ERC1967NonPayable)
                    }
                    ERC1967NonPayable
                },
                {
                    fn NotInitializing(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <NotInitializing as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::NotInitializing)
                    }
                    NotInitializing
                },
                {
                    fn UUPSUnauthorizedCallContext(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <UUPSUnauthorizedCallContext as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::UUPSUnauthorizedCallContext)
                    }
                    UUPSUnauthorizedCallContext
                },
                {
                    fn InvalidAddress(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidAddress as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InvalidAddress)
                    }
                    InvalidAddress
                },
                {
                    fn InvalidMaxStateHistory(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidMaxStateHistory as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InvalidMaxStateHistory)
                    }
                    InvalidMaxStateHistory
                },
                {
                    fn InvalidInitialization(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<LightClientV3MockErrors> {
                        <InvalidInitialization as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
                            )
                            .map(LightClientV3MockErrors::InvalidInitialization)
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
    ///Container for all the [`LightClientV3Mock`](self) events.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum LightClientV3MockEvents {
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
    impl LightClientV3MockEvents {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 32usize]] = &[
            [
                49u8, 234u8, 189u8, 144u8, 153u8, 253u8, 178u8, 93u8, 172u8, 221u8,
                210u8, 6u8, 171u8, 255u8, 135u8, 49u8, 30u8, 85u8, 52u8, 65u8, 252u8,
                157u8, 15u8, 205u8, 239u8, 32u8, 16u8, 98u8, 215u8, 231u8, 7u8, 27u8,
            ],
            [
                128u8, 23u8, 187u8, 136u8, 127u8, 223u8, 143u8, 202u8, 67u8, 20u8, 169u8,
                212u8, 15u8, 110u8, 115u8, 179u8, 184u8, 16u8, 2u8, 214u8, 126u8, 92u8,
                250u8, 133u8, 216u8, 129u8, 115u8, 175u8, 106u8, 164u8, 96u8, 114u8,
            ],
            [
                139u8, 224u8, 7u8, 156u8, 83u8, 22u8, 89u8, 20u8, 19u8, 68u8, 205u8,
                31u8, 208u8, 164u8, 242u8, 132u8, 25u8, 73u8, 127u8, 151u8, 34u8, 163u8,
                218u8, 175u8, 227u8, 180u8, 24u8, 111u8, 107u8, 100u8, 87u8, 224u8,
            ],
            [
                154u8, 95u8, 87u8, 222u8, 133u8, 109u8, 214u8, 104u8, 197u8, 77u8, 217u8,
                94u8, 92u8, 85u8, 223u8, 147u8, 67u8, 33u8, 113u8, 203u8, 202u8, 73u8,
                168u8, 119u8, 109u8, 86u8, 32u8, 234u8, 89u8, 192u8, 36u8, 80u8,
            ],
            [
                160u8, 74u8, 119u8, 57u8, 36u8, 80u8, 90u8, 65u8, 133u8, 100u8, 54u8,
                55u8, 37u8, 245u8, 104u8, 50u8, 245u8, 119u8, 46u8, 107u8, 141u8, 13u8,
                189u8, 110u8, 252u8, 231u8, 36u8, 223u8, 232u8, 3u8, 218u8, 230u8,
            ],
            [
                188u8, 124u8, 215u8, 90u8, 32u8, 238u8, 39u8, 253u8, 154u8, 222u8, 186u8,
                179u8, 32u8, 65u8, 247u8, 85u8, 33u8, 77u8, 188u8, 107u8, 255u8, 169u8,
                12u8, 192u8, 34u8, 91u8, 57u8, 218u8, 46u8, 92u8, 45u8, 59u8,
            ],
            [
                199u8, 245u8, 5u8, 178u8, 243u8, 113u8, 174u8, 33u8, 117u8, 238u8, 73u8,
                19u8, 244u8, 73u8, 158u8, 31u8, 38u8, 51u8, 167u8, 181u8, 147u8, 99u8,
                33u8, 238u8, 209u8, 205u8, 174u8, 182u8, 17u8, 81u8, 129u8, 210u8,
            ],
            [
                247u8, 135u8, 33u8, 34u8, 110u8, 254u8, 154u8, 27u8, 182u8, 120u8, 24u8,
                154u8, 22u8, 209u8, 85u8, 73u8, 40u8, 185u8, 242u8, 25u8, 46u8, 44u8,
                185u8, 62u8, 237u8, 168u8, 59u8, 121u8, 250u8, 64u8, 0u8, 125u8,
            ],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolEventInterface for LightClientV3MockEvents {
        const NAME: &'static str = "LightClientV3MockEvents";
        const COUNT: usize = 8usize;
        fn decode_raw_log(
            topics: &[alloy_sol_types::Word],
            data: &[u8],
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            match topics.first().copied() {
                Some(<Initialized as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <Initialized as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::Initialized)
                }
                Some(<NewEpoch as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <NewEpoch as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::NewEpoch)
                }
                Some(<NewState as alloy_sol_types::SolEvent>::SIGNATURE_HASH) => {
                    <NewState as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::NewState)
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
                Some(
                    <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <PermissionedProverNotRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::PermissionedProverNotRequired)
                }
                Some(
                    <PermissionedProverRequired as alloy_sol_types::SolEvent>::SIGNATURE_HASH,
                ) => {
                    <PermissionedProverRequired as alloy_sol_types::SolEvent>::decode_raw_log(
                            topics,
                            data,
                            validate,
                        )
                        .map(Self::PermissionedProverRequired)
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
    impl alloy_sol_types::private::IntoLogData for LightClientV3MockEvents {
        fn to_log_data(&self) -> alloy_sol_types::private::LogData {
            match self {
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::NewEpoch(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::NewState(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::PermissionedProverNotRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::PermissionedProverRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Upgrade(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::to_log_data(inner)
                }
            }
        }
        fn into_log_data(self) -> alloy_sol_types::private::LogData {
            match self {
                Self::Initialized(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::NewEpoch(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::NewState(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::OwnershipTransferred(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::PermissionedProverNotRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::PermissionedProverRequired(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Upgrade(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
                Self::Upgraded(inner) => {
                    alloy_sol_types::private::IntoLogData::into_log_data(inner)
                }
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`LightClientV3Mock`](self) contract instance.

See the [wrapper's documentation](`LightClientV3MockInstance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> LightClientV3MockInstance<T, P, N> {
        LightClientV3MockInstance::<T, P, N>::new(address, provider)
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
        Output = alloy_contract::Result<LightClientV3MockInstance<T, P, N>>,
    > {
        LightClientV3MockInstance::<T, P, N>::deploy(provider)
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
        LightClientV3MockInstance::<T, P, N>::deploy_builder(provider)
    }
    /**A [`LightClientV3Mock`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`LightClientV3Mock`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct LightClientV3MockInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for LightClientV3MockInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("LightClientV3MockInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > LightClientV3MockInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`LightClientV3Mock`](self) contract instance.

See the [wrapper's documentation](`LightClientV3MockInstance`) for more details.*/
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
        ) -> alloy_contract::Result<LightClientV3MockInstance<T, P, N>> {
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
    impl<T, P: ::core::clone::Clone, N> LightClientV3MockInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> LightClientV3MockInstance<T, P, N> {
            LightClientV3MockInstance {
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
    > LightClientV3MockInstance<T, P, N> {
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
        ///Creates a new call builder for the [`_getVk`] function.
        pub fn _getVk(&self) -> alloy_contract::SolCallBuilder<T, &P, _getVkCall, N> {
            self.call_builder(&_getVkCall {})
        }
        ///Creates a new call builder for the [`authRoot`] function.
        pub fn authRoot(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, authRootCall, N> {
            self.call_builder(&authRootCall {})
        }
        ///Creates a new call builder for the [`blocksPerEpoch`] function.
        pub fn blocksPerEpoch(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, blocksPerEpochCall, N> {
            self.call_builder(&blocksPerEpochCall {})
        }
        ///Creates a new call builder for the [`currentBlockNumber`] function.
        pub fn currentBlockNumber(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, currentBlockNumberCall, N> {
            self.call_builder(&currentBlockNumberCall {})
        }
        ///Creates a new call builder for the [`currentEpoch`] function.
        pub fn currentEpoch(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, currentEpochCall, N> {
            self.call_builder(&currentEpochCall {})
        }
        ///Creates a new call builder for the [`disablePermissionedProverMode`] function.
        pub fn disablePermissionedProverMode(
            &self,
        ) -> alloy_contract::SolCallBuilder<
            T,
            &P,
            disablePermissionedProverModeCall,
            N,
        > {
            self.call_builder(
                &disablePermissionedProverModeCall {
                },
            )
        }
        ///Creates a new call builder for the [`epochFromBlockNumber`] function.
        pub fn epochFromBlockNumber(
            &self,
            _blockNum: u64,
            _blocksPerEpoch: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, epochFromBlockNumberCall, N> {
            self.call_builder(
                &epochFromBlockNumberCall {
                    _blockNum,
                    _blocksPerEpoch,
                },
            )
        }
        ///Creates a new call builder for the [`epochStartBlock`] function.
        pub fn epochStartBlock(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, epochStartBlockCall, N> {
            self.call_builder(&epochStartBlockCall {})
        }
        ///Creates a new call builder for the [`finalizedState`] function.
        pub fn finalizedState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, finalizedStateCall, N> {
            self.call_builder(&finalizedStateCall {})
        }
        ///Creates a new call builder for the [`firstEpoch`] function.
        pub fn firstEpoch(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, firstEpochCall, N> {
            self.call_builder(&firstEpochCall {})
        }
        ///Creates a new call builder for the [`genesisStakeTableState`] function.
        pub fn genesisStakeTableState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, genesisStakeTableStateCall, N> {
            self.call_builder(&genesisStakeTableStateCall {})
        }
        ///Creates a new call builder for the [`genesisState`] function.
        pub fn genesisState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, genesisStateCall, N> {
            self.call_builder(&genesisStateCall {})
        }
        ///Creates a new call builder for the [`getHotShotCommitment`] function.
        pub fn getHotShotCommitment(
            &self,
            hotShotBlockHeight: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, getHotShotCommitmentCall, N> {
            self.call_builder(
                &getHotShotCommitmentCall {
                    hotShotBlockHeight,
                },
            )
        }
        ///Creates a new call builder for the [`getStateHistoryCount`] function.
        pub fn getStateHistoryCount(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, getStateHistoryCountCall, N> {
            self.call_builder(&getStateHistoryCountCall {})
        }
        ///Creates a new call builder for the [`getVersion`] function.
        pub fn getVersion(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, getVersionCall, N> {
            self.call_builder(&getVersionCall {})
        }
        ///Creates a new call builder for the [`initialize`] function.
        pub fn initialize(
            &self,
            _genesis: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _genesisStakeTableState: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            _stateHistoryRetentionPeriod: u32,
            owner: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeCall, N> {
            self.call_builder(
                &initializeCall {
                    _genesis,
                    _genesisStakeTableState,
                    _stateHistoryRetentionPeriod,
                    owner,
                },
            )
        }
        ///Creates a new call builder for the [`initializeV2`] function.
        pub fn initializeV2(
            &self,
            _blocksPerEpoch: u64,
            _epochStartBlock: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeV2Call, N> {
            self.call_builder(
                &initializeV2Call {
                    _blocksPerEpoch,
                    _epochStartBlock,
                },
            )
        }
        ///Creates a new call builder for the [`initializeV3`] function.
        pub fn initializeV3(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, initializeV3Call, N> {
            self.call_builder(&initializeV3Call {})
        }
        ///Creates a new call builder for the [`isEpochRoot`] function.
        pub fn isEpochRoot(
            &self,
            blockHeight: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, isEpochRootCall, N> {
            self.call_builder(&isEpochRootCall { blockHeight })
        }
        ///Creates a new call builder for the [`isGtEpochRoot`] function.
        pub fn isGtEpochRoot(
            &self,
            blockHeight: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, isGtEpochRootCall, N> {
            self.call_builder(&isGtEpochRootCall { blockHeight })
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
            threshold: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, lagOverEscapeHatchThresholdCall, N> {
            self.call_builder(
                &lagOverEscapeHatchThresholdCall {
                    blockNumber,
                    threshold,
                },
            )
        }
        ///Creates a new call builder for the [`newFinalizedState_0`] function.
        pub fn newFinalizedState_0(
            &self,
            _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _1: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, newFinalizedState_0Call, N> {
            self.call_builder(&newFinalizedState_0Call { _0, _1 })
        }
        ///Creates a new call builder for the [`newFinalizedState_1`] function.
        pub fn newFinalizedState_1(
            &self,
            _0: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            _1: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            _2: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, newFinalizedState_1Call, N> {
            self.call_builder(
                &newFinalizedState_1Call {
                    _0,
                    _1,
                    _2,
                },
            )
        }
        ///Creates a new call builder for the [`newFinalizedState_2`] function.
        pub fn newFinalizedState_2(
            &self,
            newState: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
            nextStakeTable: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
            newAuthRoot: alloy::sol_types::private::primitives::aliases::U256,
            proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, newFinalizedState_2Call, N> {
            self.call_builder(
                &newFinalizedState_2Call {
                    newState,
                    nextStakeTable,
                    newAuthRoot,
                    proof,
                },
            )
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
        pub fn proxiableUUID(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, proxiableUUIDCall, N> {
            self.call_builder(&proxiableUUIDCall {})
        }
        ///Creates a new call builder for the [`renounceOwnership`] function.
        pub fn renounceOwnership(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, renounceOwnershipCall, N> {
            self.call_builder(&renounceOwnershipCall {})
        }
        ///Creates a new call builder for the [`setBlocksPerEpoch`] function.
        pub fn setBlocksPerEpoch(
            &self,
            newBlocksPerEpoch: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, setBlocksPerEpochCall, N> {
            self.call_builder(
                &setBlocksPerEpochCall {
                    newBlocksPerEpoch,
                },
            )
        }
        ///Creates a new call builder for the [`setFinalizedState`] function.
        pub fn setFinalizedState(
            &self,
            state: <LightClient::LightClientState as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, setFinalizedStateCall, N> {
            self.call_builder(&setFinalizedStateCall { state })
        }
        ///Creates a new call builder for the [`setHotShotDownSince`] function.
        pub fn setHotShotDownSince(
            &self,
            l1Height: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, setHotShotDownSinceCall, N> {
            self.call_builder(
                &setHotShotDownSinceCall {
                    l1Height,
                },
            )
        }
        ///Creates a new call builder for the [`setHotShotUp`] function.
        pub fn setHotShotUp(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, setHotShotUpCall, N> {
            self.call_builder(&setHotShotUpCall {})
        }
        ///Creates a new call builder for the [`setPermissionedProver`] function.
        pub fn setPermissionedProver(
            &self,
            prover: alloy::sol_types::private::Address,
        ) -> alloy_contract::SolCallBuilder<T, &P, setPermissionedProverCall, N> {
            self.call_builder(
                &setPermissionedProverCall {
                    prover,
                },
            )
        }
        ///Creates a new call builder for the [`setStateHistory`] function.
        pub fn setStateHistory(
            &self,
            _stateHistoryCommitments: alloy::sol_types::private::Vec<
                <LightClient::StateHistoryCommitment as alloy::sol_types::SolType>::RustType,
            >,
        ) -> alloy_contract::SolCallBuilder<T, &P, setStateHistoryCall, N> {
            self.call_builder(
                &setStateHistoryCall {
                    _stateHistoryCommitments,
                },
            )
        }
        ///Creates a new call builder for the [`setStateHistoryRetentionPeriod`] function.
        pub fn setStateHistoryRetentionPeriod(
            &self,
            historySeconds: u32,
        ) -> alloy_contract::SolCallBuilder<
            T,
            &P,
            setStateHistoryRetentionPeriodCall,
            N,
        > {
            self.call_builder(
                &setStateHistoryRetentionPeriodCall {
                    historySeconds,
                },
            )
        }
        ///Creates a new call builder for the [`setVotingStakeTableState`] function.
        pub fn setVotingStakeTableState(
            &self,
            stake: <LightClient::StakeTableState as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, setVotingStakeTableStateCall, N> {
            self.call_builder(
                &setVotingStakeTableStateCall {
                    stake,
                },
            )
        }
        ///Creates a new call builder for the [`setstateHistoryRetentionPeriod`] function.
        pub fn setstateHistoryRetentionPeriod(
            &self,
            historySeconds: u32,
        ) -> alloy_contract::SolCallBuilder<
            T,
            &P,
            setstateHistoryRetentionPeriodCall,
            N,
        > {
            self.call_builder(
                &setstateHistoryRetentionPeriodCall {
                    historySeconds,
                },
            )
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
        ///Creates a new call builder for the [`updateEpochStartBlock`] function.
        pub fn updateEpochStartBlock(
            &self,
            newEpochStartBlock: u64,
        ) -> alloy_contract::SolCallBuilder<T, &P, updateEpochStartBlockCall, N> {
            self.call_builder(
                &updateEpochStartBlockCall {
                    newEpochStartBlock,
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
        ///Creates a new call builder for the [`votingStakeTableState`] function.
        pub fn votingStakeTableState(
            &self,
        ) -> alloy_contract::SolCallBuilder<T, &P, votingStakeTableStateCall, N> {
            self.call_builder(&votingStakeTableStateCall {})
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > LightClientV3MockInstance<T, P, N> {
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
        pub fn Initialized_filter(
            &self,
        ) -> alloy_contract::Event<T, &P, Initialized, N> {
            self.event_filter::<Initialized>()
        }
        ///Creates a new event filter for the [`NewEpoch`] event.
        pub fn NewEpoch_filter(&self) -> alloy_contract::Event<T, &P, NewEpoch, N> {
            self.event_filter::<NewEpoch>()
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
