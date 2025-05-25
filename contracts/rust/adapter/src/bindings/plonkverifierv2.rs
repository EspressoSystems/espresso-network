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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
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
    use alloy::sol_types as alloy_sol_types;

    use super::*;
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
                    "VerifyingKey(uint256 domainSize,uint256 numInputs,BN254.G1Point sigma0,BN254.G1Point sigma1,BN254.G1Point sigma2,BN254.G1Point sigma3,BN254.G1Point sigma4,BN254.G1Point q1,BN254.G1Point q2,BN254.G1Point q3,BN254.G1Point q4,BN254.G1Point qM12,BN254.G1Point qM34,BN254.G1Point qO,BN254.G1Point qC,BN254.G1Point qH1,BN254.G1Point qH2,BN254.G1Point qH3,BN254.G1Point qH4,BN254.G1Point qEcc,bytes32 g2LSB,bytes32 g2MSB)",
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

interface PlonkVerifierV2 {
    error InvalidPlonkArgs();
    error UnsupportedDegree();
    error WrongPlonkVK();

    function BETA_H_X0() external view returns (uint256);
    function BETA_H_X1() external view returns (uint256);
    function BETA_H_Y0() external view returns (uint256);
    function BETA_H_Y1() external view returns (uint256);
    function COSET_K1() external view returns (uint256);
    function COSET_K2() external view returns (uint256);
    function COSET_K3() external view returns (uint256);
    function COSET_K4() external view returns (uint256);
    function verify(IPlonkVerifier.VerifyingKey memory verifyingKey, uint256[11] memory publicInput, IPlonkVerifier.PlonkProof memory proof) external view returns (bool);
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "function",
    "name": "BETA_H_X0",
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
    "name": "BETA_H_X1",
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
    "name": "BETA_H_Y0",
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
    "name": "BETA_H_Y1",
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
    "name": "COSET_K1",
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
    "name": "COSET_K2",
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
    "name": "COSET_K3",
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
    "name": "COSET_K4",
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
    "name": "verify",
    "inputs": [
      {
        "name": "verifyingKey",
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
      },
      {
        "name": "publicInput",
        "type": "uint256[11]",
        "internalType": "uint256[11]"
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
    "type": "error",
    "name": "InvalidPlonkArgs",
    "inputs": []
  },
  {
    "type": "error",
    "name": "UnsupportedDegree",
    "inputs": []
  },
  {
    "type": "error",
    "name": "WrongPlonkVK",
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
pub mod PlonkVerifierV2 {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x612673610034600b8282823980515f1a607314602857634e487b7160e01b5f525f60045260245ffd5b305f52607381538281f3fe730000000000000000000000000000000000000000301460806040526004361061009b575f3560e01c8063af196ba21161006e578063af196ba21461014e578063de24ac0f14610175578063e3512d561461019c578063f5144326146101c3578063fc8660c7146101ea575f5ffd5b80630c551f3f1461009f5780634b4734e3146100d95780635a14c0fe14610100578063834c452a14610127575b5f5ffd5b6100c67f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02581565b6040519081526020015b60405180910390f35b6100c67f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581565b6100c67f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a81565b6100c67f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181565b6100c67f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081565b6100c67f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88181565b6100c67f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a81565b6100c67f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe481565b6101fd6101f8366004612407565b61020d565b60405190151581526020016100d0565b5f610217826102aa565b610227835f5b60200201516103e5565b61023283600161021d565b61023d83600261021d565b61024883600361021d565b61025383600461021d565b61025e83600561021d565b61026983600661021d565b61027483600761021d565b61027f83600861021d565b61028a83600961021d565b61029583600a61021d565b6102a084848461044b565b90505b9392505050565b80516102b59061063f565b6102c2816020015161063f565b6102cf816040015161063f565b6102dc816060015161063f565b6102e9816080015161063f565b6102f68160a0015161063f565b6103038160c0015161063f565b6103108160e0015161063f565b61031e81610100015161063f565b61032c81610120015161063f565b61033a81610140015161063f565b61034881610160015161063f565b61035681610180015161063f565b610364816101a001516103e5565b610372816101c001516103e5565b610380816101e001516103e5565b61038e8161020001516103e5565b61039c8161022001516103e5565b6103aa8161024001516103e5565b6103b88161026001516103e5565b6103c68161028001516103e5565b6103d4816102a001516103e5565b6103e2816102c001516103e5565b50565b5f5160206126475f395f51905f528110806104475760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c64000000000060448201526064015b60405180910390fd5b5050565b5f8360200151600b14610471576040516320fa9d8960e11b815260040160405180910390fd5b5f61047d8585856106ed565b90505f61048c865f0151610c7c565b90505f61049e828460a0015188611223565b90506104bb60405180604001604052805f81526020015f81525090565b604080518082019091525f80825260208201526104ef8761016001516104ea8961018001518860e00151611280565b611321565b91505f5f6104ff8b88878c6113c5565b91509150610510816104ea846115fd565b9250610529836104ea8b61016001518a60a00151611280565b60a08801516040880151602001519194505f5160206126475f395f51905f52918290820990508160e08a01518209905061056c856104ea8d610180015184611280565b94505f60405180608001604052807f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081526020017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181526020017f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581526020017f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe4815250905061062d8782610620896115fd565b61062861169a565b611767565b9e9d5050505050505050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4791159015161561067857505050565b8251602084015182600384858586098509088382830914838210848410161693505050816106e85760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e74000000000000000000604482015260640161043e565b505050565b61072d6040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b5f5f5160206126475f395f51905f529050604051602081015f815260fe60e01b8152865160c01b6004820152602087015160c01b600c82015261028087015160208201526102a08701516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e087015180516101008301526020810151610120830152506101008701518051610140830152602081015161016083015250610120870151805161018083015260208101516101a08301525061014087015180516101c083015260208101516101e083015250610160870151805161020083015260208101516102208301525061018087015180516102408301526020810151610260830152506101e0870151805161028083015260208101516102a08301525061020087015180516102c083015260208101516102e083015250610220870151805161030083015260208101516103208301525061024087015180516103408301526020810151610360830152506101a0870151805161038083015260208101516103a0830152506101c087015180516103c083015260208101516103e0830152506102608701518051610400830152602081015161042083015250604087015180516104408301526020810151610460830152506060870151805161048083015260208101516104a083015250608087015180516104c083015260208101516104e08301525060a0870151805161050083015260208101516105208301525060c08701518051610540830152602081015161056083015250855161058082015260208601516105a082015260408601516105c082015260608601516105e0820152608086015161060082015260a086015161062082015260c086015161064082015260e08601516106608201526101008601516106808201526101208601516106a08201526101408601516106c0820152845180516106e08301526020810151610700830152506020850151805161072083015260208101516107408301525060408501518051610760830152602081015161078083015250606085015180516107a083015260208101516107c083015250608085015180516107e08301526020810151610800830152505f82526108408220825282825106606085015260208220825282825106608085015260a085015180518252602081015160208301525060608220808352838106855283818209848282099150806020870152508060408601525060c085015180518252602081015160208301525060e085015180516040830152602081015160608301525061010085015180516080830152602081015160a083015250610120850151805160c0830152602081015160e0830152506101408501518051610100830152602081015161012083015250610160822082528282510660a08501526101a085015181526101c085015160208201526101e085015160408201526102008501516060820152610220850151608082015261024085015160a082015261026085015160c082015261028085015160e08201526102a08501516101008201526102c0850151610120820152610160822082528282510660c08501526101608501518051825260208101516020830152506101808501518051604083015260208101516060830152505060a0812082810660e08501525050509392505050565b610c846120e1565b816201000003610e5b576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c10018152602001604051806101600160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd81526020017f15ee2475bee517c4ee05e51fa1ee7312a8373a0b13db8c51baf04cb2e99bd2bd81526020017e6fab49b869ae62001deac878b2667bd31bf3e28e3a2d764aa49b8d9bbdd31081526020017f2e856bf6d037708ffa4c06d4d8820f45ccadce9c5a6d178cbd573f82e0f9701181526020017f1407eee35993f2b1ad5ec6d9b8950ca3af33135d06037f871c5e33bf566dd7b48152508152509050919050565b816210000003611034576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c1018152602001604051806101600160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c81526020017f29e84143f5870d4776a92df8da8c6c9303d59088f37ba85f40cf6fd14265b4bc81526020017f1bf82deba7d74902c3708cc6e70e61f30512eca95655210e276e5858ce8f58e581526020017f22b94b2e2b0043d04e662d5ec018ea1c8a99a23a62c9eb46f0318f6a194985f081526020017f29969d8d5363bef1101a68e446a14e1da7ba9294e142a146a980fddb4d4d41a58152508152509050919050565b8160200361120a576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e7508000018152602001604051806101600160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b03904381526020017ee14b6364a47e9c4284a9f80a5fc41cd212b0d4dbf8a5703770a40a9a34399081526020017f30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f70363681526020017f22399c34139bffada8de046aac50c9628e3517a3a452795364e777cd65bb9f4881526020017f2290ee31c482cf92b79b1944db1c0147635e9004db8c3b9d13644bef31ec3bd38152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b61124460405180606001604052805f81526020015f81526020015f81525090565b61124e8484611847565b80825261125e9085908590611898565b6020820152805161127490859084908690611907565b60408201529392505050565b604080518082019091525f808252602082015261129b612105565b8351815260208085015190820152604081018390525f60608360808460076107d05a03fa905080806112cb575f5ffd5b50806113195760405162461bcd60e51b815260206004820152601960248201527f426e3235343a207363616c6172206d756c206661696c65642100000000000000604482015260640161043e565b505092915050565b604080518082019091525f808252602082015261133c612123565b8351815260208085015181830152835160408301528301516060808301919091525f908360c08460066107d05a03fa90508080611377575f5ffd5b50806113195760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a2067726f7570206164646974696f6e206661696c656421000000604482015260640161043e565b604080518082019091525f8082526020820152604080518082019091525f80825260208201525f6113f887878787611a56565b90505f5160206126475f395f51905f525f611414888789611f20565b905061142081836125f4565b60c08901516101a08801519192509081908490819083098408925061144c856104ea8a5f015184611280565b955083828209905083846101c08a0151830984089250611474866104ea8a6020015184611280565b955083828209905083846101e08a015183098408925061149c866104ea8a6040015184611280565b955083828209905083846102008a01518309840892506114c4866104ea8a6060015184611280565b955083828209905083846102208a01518309840892506114ec866104ea8a6080015184611280565b955083828209905083846102408a0151830984089250611514866104ea8d6040015184611280565b955083828209905083846102608a015183098408925061153c866104ea8d6060015184611280565b955083828209905083846102808a0151830984089250611564866104ea8d6080015184611280565b955083828209905083846102a08a015183098408925061158c866104ea8d60a0015184611280565b95505f8a60e00151905084856102c08b01518309850893506115b6876104ea8b60a0015184611280565b96506115ec6115e66040805180820182525f80825260209182015281518083019092526001825260029082015290565b85611280565b975050505050505094509492505050565b604080518082019091525f8082526020820152815160208301511590151615611624575090565b6040518060400160405280835f015181526020017f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4784602001516116689190612627565b611692907f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd476125f4565b905292915050565b6116c160405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f5f5f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806118395760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c65642100000000604482015260640161043e565b50151590505b949350505050565b81515f905f5160206126475f395f51905f5290838015611888578493505f5b8281101561187c57838586099450600101611866565b5060018403935061188f565b6001830393505b50505092915050565b5f826001036118a9575060016102a3565b815f036118b757505f6102a3565b60208401515f5160206126475f395f51905f52905f908281860990508580156118e5576001870392506118ec565b6001840392505b506118f68261200b565b915082828209979650505050505050565b5f5f5160206126475f395f51905f528282036119805760015f5b600b81101561197557818603611952578681600b8110611943576119436125e0565b6020020151935050505061183f565b828061196057611960612613565b60408901516020015183099150600101611921565b505f9250505061183f565b611988612141565b60408701516001610140838101828152920190805b600b8110156119ca5760208403935085868a85518903088309808552601f1990930192915060010161199d565b505050505f5f5f90506001838960408c01515f5b600b811015611a1e578882518a85518c88518a0909098981880896505088898d84518c0308860994506020938401939283019291909101906001016119de565b50505050809250505f611a308361200b565b905060208a015185818909965050848187099550848287099a9950505050505050505050565b604080518082019091525f80825260208201525f5f5f5f5f5f5160206126475f395f51905f52905060808901518160208a015160208c0151099550895194508160a08b015160608c0151099350816101a089015185089250818184089250818584099450817f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a85099250816101c089015184089250818184089250818584099450817f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02585099250816101e089015184089250818184089250818584099450817f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992508161020089015184089250818184089250818584099450817f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88185099250816102208901518408925081818408925050808483099350808486089450611bc38760a0015186611280565b9550885160608a015160808b0151838284099750836102c08b015189099750836102408b015183099550836101a08b015187089550838187089550838689099750836102608b015183099550836101c08b015187089550838187089550838689099750836102808b015183099550836101e08b015187089550838187089550838689099750836102a08b015183099550836102008b015187089550838187089550505050808386099450611c8a866104ea8c60c001518885611c8591906125f4565b611280565b9550611ca3866104ea8c60e001518a6101a00151611280565b9550611cbd866104ea8c61010001518a6101c00151611280565b9550611cd7866104ea8c61012001518a6101e00151611280565b9550611cf1866104ea8c61014001518a6102000151611280565b9550806101c08801516101a0890151099250611d16866104ea8c610160015186611280565b9550806102008801516101e0890151099250611d3b866104ea8c610180015186611280565b95506101a08701519250808384099150808283099150808284099250611d6a866104ea8c6101e0015186611280565b95506101c08701519250808384099150808283099150808284099250611d99866104ea8c610200015186611280565b95506101e08701519250808384099150808283099150808284099250611dc8866104ea8c610220015186611280565b95506102008701519250808384099150808283099150808284099250611df7866104ea8c610240015186611280565b9550611e14866104ea8c6101a00151611c858b61022001516120ac565b9550611e25868b6101c00151611321565b9550806101c08801516101a0890151099250806101e08801518409925080610200880151840992508061022088015184099250611e6b866104ea8c610260015186611280565b9550611e79885f01516120ac565b9450611e8d866104ea8960c0015188611280565b955080600189510860a08a0151909350819080099150808284099250808386099450611ec1866104ea8960e0015188611280565b9550808386099450611edc866104ea89610100015188611280565b9550808386099450611ef7866104ea89610120015188611280565b9550808386099450611f12866104ea89610140015188611280565b9a9950505050505050505050565b5f5f5f5160206126475f395f51905f5290505f836020015190505f846040015190505f60019050606088015160808901516101a08901516102408a01518788898387098a868608088609945050506101c08901516102608a01518788898387098a868608088609945050506101e08901516102808a01518788898387098a868608088609945050506102008901516102a08a01518788898387098a8686080886099450505061022089015191506102c0890151868782898587080985099350505050875160208901518586868309870385089650508485838309860387089998505050505050505050565b5f5f5f5f5160206126475f395f51905f52905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f519250816120a55760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c656421000000604482015260640161043e565b5050919050565b5f6120c45f5160206126475f395f51905f5283612627565b6120db905f5160206126475f395f51905f526125f4565b92915050565b60405180606001604052805f81526020015f8152602001612100612141565b905290565b60405180606001604052806003906020820280368337509192915050565b60405180608001604052806004906020820280368337509192915050565b604051806101600160405280600b906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561219857612198612160565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561219857612198612160565b5f604082840312156121d2575f5ffd5b6040805190810167ffffffffffffffff811182821017156121f5576121f5612160565b604052823581526020928301359281019290925250919050565b5f82601f83011261221e575f5ffd5b604051610160810167ffffffffffffffff8111828210171561224257612242612160565b60405280610160840185811115612257575f5ffd5b845b81811015612271578035835260209283019201612259565b509195945050505050565b5f610480828403121561228d575f5ffd5b612295612174565b90506122a183836121c2565b81526122b083604084016121c2565b60208201526122c283608084016121c2565b60408201526122d48360c084016121c2565b60608201526122e78361010084016121c2565b60808201526122fa8361014084016121c2565b60a082015261230d8361018084016121c2565b60c0820152612320836101c084016121c2565b60e08201526123338361020084016121c2565b6101008201526123478361024084016121c2565b61012082015261235b8361028084016121c2565b61014082015261236f836102c084016121c2565b6101608201526123838361030084016121c2565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f5f838503610ae081121561241b575f5ffd5b610500811215612429575f5ffd5b5061243261219e565b843581526020808601359082015261244d86604087016121c2565b604082015261245f86608087016121c2565b60608201526124718660c087016121c2565b60808201526124848661010087016121c2565b60a08201526124978661014087016121c2565b60c08201526124aa8661018087016121c2565b60e08201526124bd866101c087016121c2565b6101008201526124d18661020087016121c2565b6101208201526124e58661024087016121c2565b6101408201526124f98661028087016121c2565b61016082015261250d866102c087016121c2565b6101808201526125218661030087016121c2565b6101a08201526125358661034087016121c2565b6101c08201526125498661038087016121c2565b6101e082015261255d866103c087016121c2565b6102008201526125718661040087016121c2565b6102208201526125858661044087016121c2565b6102408201526125998661048087016121c2565b6102608201526104c08501356102808201526104e08501356102a082015292506125c785610500860161220f565b91506125d785610660860161227c565b90509250925092565b634e487b7160e01b5f52603260045260245ffd5b818103818111156120db57634e487b7160e01b5f52601160045260245ffd5b634e487b7160e01b5f52601260045260245ffd5b5f8261264157634e487b7160e01b5f52601260045260245ffd5b50069056fe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"a&sa\x004`\x0B\x82\x82\x829\x80Q_\x1A`s\x14`(WcNH{q`\xE0\x1B_R_`\x04R`$_\xFD[0_R`s\x81S\x82\x81\xF3\xFEs\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x000\x14`\x80`@R`\x046\x10a\0\x9BW_5`\xE0\x1C\x80c\xAF\x19k\xA2\x11a\0nW\x80c\xAF\x19k\xA2\x14a\x01NW\x80c\xDE$\xAC\x0F\x14a\x01uW\x80c\xE3Q-V\x14a\x01\x9CW\x80c\xF5\x14C&\x14a\x01\xC3W\x80c\xFC\x86`\xC7\x14a\x01\xEAW__\xFD[\x80c\x0CU\x1F?\x14a\0\x9FW\x80cKG4\xE3\x14a\0\xD9W\x80cZ\x14\xC0\xFE\x14a\x01\0W\x80c\x83LE*\x14a\x01'W[__\xFD[a\0\xC6\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xC6\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81V[a\0\xC6\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x81V[a\0\xC6\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81V[a\0\xC6\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81V[a\0\xC6\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x81V[a\0\xC6\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x81V[a\0\xC6\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81V[a\x01\xFDa\x01\xF86`\x04a$\x07V[a\x02\rV[`@Q\x90\x15\x15\x81R` \x01a\0\xD0V[_a\x02\x17\x82a\x02\xAAV[a\x02'\x83_[` \x02\x01Qa\x03\xE5V[a\x022\x83`\x01a\x02\x1DV[a\x02=\x83`\x02a\x02\x1DV[a\x02H\x83`\x03a\x02\x1DV[a\x02S\x83`\x04a\x02\x1DV[a\x02^\x83`\x05a\x02\x1DV[a\x02i\x83`\x06a\x02\x1DV[a\x02t\x83`\x07a\x02\x1DV[a\x02\x7F\x83`\x08a\x02\x1DV[a\x02\x8A\x83`\ta\x02\x1DV[a\x02\x95\x83`\na\x02\x1DV[a\x02\xA0\x84\x84\x84a\x04KV[\x90P[\x93\x92PPPV[\x80Qa\x02\xB5\x90a\x06?V[a\x02\xC2\x81` \x01Qa\x06?V[a\x02\xCF\x81`@\x01Qa\x06?V[a\x02\xDC\x81``\x01Qa\x06?V[a\x02\xE9\x81`\x80\x01Qa\x06?V[a\x02\xF6\x81`\xA0\x01Qa\x06?V[a\x03\x03\x81`\xC0\x01Qa\x06?V[a\x03\x10\x81`\xE0\x01Qa\x06?V[a\x03\x1E\x81a\x01\0\x01Qa\x06?V[a\x03,\x81a\x01 \x01Qa\x06?V[a\x03:\x81a\x01@\x01Qa\x06?V[a\x03H\x81a\x01`\x01Qa\x06?V[a\x03V\x81a\x01\x80\x01Qa\x06?V[a\x03d\x81a\x01\xA0\x01Qa\x03\xE5V[a\x03r\x81a\x01\xC0\x01Qa\x03\xE5V[a\x03\x80\x81a\x01\xE0\x01Qa\x03\xE5V[a\x03\x8E\x81a\x02\0\x01Qa\x03\xE5V[a\x03\x9C\x81a\x02 \x01Qa\x03\xE5V[a\x03\xAA\x81a\x02@\x01Qa\x03\xE5V[a\x03\xB8\x81a\x02`\x01Qa\x03\xE5V[a\x03\xC6\x81a\x02\x80\x01Qa\x03\xE5V[a\x03\xD4\x81a\x02\xA0\x01Qa\x03\xE5V[a\x03\xE2\x81a\x02\xC0\x01Qa\x03\xE5V[PV[_Q` a&G_9_Q\x90_R\x81\x10\x80a\x04GW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[PPV[_\x83` \x01Q`\x0B\x14a\x04qW`@Qc \xFA\x9D\x89`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04}\x85\x85\x85a\x06\xEDV[\x90P_a\x04\x8C\x86_\x01Qa\x0C|V[\x90P_a\x04\x9E\x82\x84`\xA0\x01Q\x88a\x12#V[\x90Pa\x04\xBB`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x04\xEF\x87a\x01`\x01Qa\x04\xEA\x89a\x01\x80\x01Q\x88`\xE0\x01Qa\x12\x80V[a\x13!V[\x91P__a\x04\xFF\x8B\x88\x87\x8Ca\x13\xC5V[\x91P\x91Pa\x05\x10\x81a\x04\xEA\x84a\x15\xFDV[\x92Pa\x05)\x83a\x04\xEA\x8Ba\x01`\x01Q\x8A`\xA0\x01Qa\x12\x80V[`\xA0\x88\x01Q`@\x88\x01Q` \x01Q\x91\x94P_Q` a&G_9_Q\x90_R\x91\x82\x90\x82\t\x90P\x81`\xE0\x8A\x01Q\x82\t\x90Pa\x05l\x85a\x04\xEA\x8Da\x01\x80\x01Q\x84a\x12\x80V[\x94P_`@Q\x80`\x80\x01`@R\x80\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81R` \x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81R` \x01\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81R` \x01\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81RP\x90Pa\x06-\x87\x82a\x06 \x89a\x15\xFDV[a\x06(a\x16\x9AV[a\x17gV[\x9E\x9DPPPPPPPPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x06xWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x06\xE8W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x04>V[PPPV[a\x07-`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[__Q` a&G_9_Q\x90_R\x90P`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x86Q`\xC0\x1B`\x04\x82\x01R` \x87\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x87\x01Q` \x82\x01Ra\x02\xA0\x87\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x87\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x87\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x87\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x87\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x87\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x87\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x87\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x87\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x87\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x87\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x87\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x87\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x87\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x87\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x87\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x87\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x87\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x87\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x85Qa\x05\x80\x82\x01R` \x86\x01Qa\x05\xA0\x82\x01R`@\x86\x01Qa\x05\xC0\x82\x01R``\x86\x01Qa\x05\xE0\x82\x01R`\x80\x86\x01Qa\x06\0\x82\x01R`\xA0\x86\x01Qa\x06 \x82\x01R`\xC0\x86\x01Qa\x06@\x82\x01R`\xE0\x86\x01Qa\x06`\x82\x01Ra\x01\0\x86\x01Qa\x06\x80\x82\x01Ra\x01 \x86\x01Qa\x06\xA0\x82\x01Ra\x01@\x86\x01Qa\x06\xC0\x82\x01R\x84Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP` \x85\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`@\x85\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP``\x85\x01Q\x80Qa\x07\xA0\x83\x01R` \x81\x01Qa\x07\xC0\x83\x01RP`\x80\x85\x01Q\x80Qa\x07\xE0\x83\x01R` \x81\x01Qa\x08\0\x83\x01RP_\x82Ra\x08@\x82 \x82R\x82\x82Q\x06``\x85\x01R` \x82 \x82R\x82\x82Q\x06`\x80\x85\x01R`\xA0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 \x80\x83R\x83\x81\x06\x85R\x83\x81\x82\t\x84\x82\x82\t\x91P\x80` \x87\x01RP\x80`@\x86\x01RP`\xC0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x85\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x85\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x85\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 \x82R\x82\x82Q\x06`\xA0\x85\x01Ra\x01\xA0\x85\x01Q\x81Ra\x01\xC0\x85\x01Q` \x82\x01Ra\x01\xE0\x85\x01Q`@\x82\x01Ra\x02\0\x85\x01Q``\x82\x01Ra\x02 \x85\x01Q`\x80\x82\x01Ra\x02@\x85\x01Q`\xA0\x82\x01Ra\x02`\x85\x01Q`\xC0\x82\x01Ra\x02\x80\x85\x01Q`\xE0\x82\x01Ra\x02\xA0\x85\x01Qa\x01\0\x82\x01Ra\x02\xC0\x85\x01Qa\x01 \x82\x01Ra\x01`\x82 \x82R\x82\x82Q\x06`\xC0\x85\x01Ra\x01`\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 \x82\x81\x06`\xE0\x85\x01RPPP\x93\x92PPPV[a\x0C\x84a \xE1V[\x81b\x01\0\0\x03a\x0E[W`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81R` \x01\x7F\x15\xEE$u\xBE\xE5\x17\xC4\xEE\x05\xE5\x1F\xA1\xEEs\x12\xA87:\x0B\x13\xDB\x8CQ\xBA\xF0L\xB2\xE9\x9B\xD2\xBD\x81R` \x01~o\xABI\xB8i\xAEb\0\x1D\xEA\xC8x\xB2f{\xD3\x1B\xF3\xE2\x8E:-vJ\xA4\x9B\x8D\x9B\xBD\xD3\x10\x81R` \x01\x7F.\x85k\xF6\xD07p\x8F\xFAL\x06\xD4\xD8\x82\x0FE\xCC\xAD\xCE\x9CZm\x17\x8C\xBDW?\x82\xE0\xF9p\x11\x81R` \x01\x7F\x14\x07\xEE\xE3Y\x93\xF2\xB1\xAD^\xC6\xD9\xB8\x95\x0C\xA3\xAF3\x13]\x06\x03\x7F\x87\x1C^3\xBFVm\xD7\xB4\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x104W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81R` \x01\x7F)\xE8AC\xF5\x87\rGv\xA9-\xF8\xDA\x8Cl\x93\x03\xD5\x90\x88\xF3{\xA8_@\xCFo\xD1Be\xB4\xBC\x81R` \x01\x7F\x1B\xF8-\xEB\xA7\xD7I\x02\xC3p\x8C\xC6\xE7\x0Ea\xF3\x05\x12\xEC\xA9VU!\x0E'nXX\xCE\x8FX\xE5\x81R` \x01\x7F\"\xB9K.+\0C\xD0Nf-^\xC0\x18\xEA\x1C\x8A\x99\xA2:b\xC9\xEBF\xF01\x8Fj\x19I\x85\xF0\x81R` \x01\x7F)\x96\x9D\x8DSc\xBE\xF1\x10\x1Ah\xE4F\xA1N\x1D\xA7\xBA\x92\x94\xE1B\xA1F\xA9\x80\xFD\xDBMMA\xA5\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\x12\nW`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81R` \x01~\xE1Kcd\xA4~\x9CB\x84\xA9\xF8\n_\xC4\x1C\xD2\x12\xB0\xD4\xDB\xF8\xA5p7p\xA4\n\x9A49\x90\x81R` \x01\x7F0dNr\xE11\xA0)\x04\x8Bn\x19?\xD8A\x04\\\xEA$\xF6\xFDsk\xEC#\x12\x04p\x8Fp66\x81R` \x01\x7F\"9\x9C4\x13\x9B\xFF\xAD\xA8\xDE\x04j\xACP\xC9b\x8E5\x17\xA3\xA4RySd\xE7w\xCDe\xBB\x9FH\x81R` \x01\x7F\"\x90\xEE1\xC4\x82\xCF\x92\xB7\x9B\x19D\xDB\x1C\x01Gc^\x90\x04\xDB\x8C;\x9D\x13dK\xEF1\xEC;\xD3\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x12D`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\x12N\x84\x84a\x18GV[\x80\x82Ra\x12^\x90\x85\x90\x85\x90a\x18\x98V[` \x82\x01R\x80Qa\x12t\x90\x85\x90\x84\x90\x86\x90a\x19\x07V[`@\x82\x01R\x93\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x12\x9Ba!\x05V[\x83Q\x81R` \x80\x85\x01Q\x90\x82\x01R`@\x81\x01\x83\x90R_``\x83`\x80\x84`\x07a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x12\xCBW__\xFD[P\x80a\x13\x19W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x19`$\x82\x01R\x7FBn254: scalar mul failed!\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x04>V[PP\x92\x91PPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x13<a!#V[\x83Q\x81R` \x80\x85\x01Q\x81\x83\x01R\x83Q`@\x83\x01R\x83\x01Q``\x80\x83\x01\x91\x90\x91R_\x90\x83`\xC0\x84`\x06a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x13wW__\xFD[P\x80a\x13\x19W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: group addition failed!\0\0\0`D\x82\x01R`d\x01a\x04>V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x13\xF8\x87\x87\x87\x87a\x1AVV[\x90P_Q` a&G_9_Q\x90_R_a\x14\x14\x88\x87\x89a\x1F V[\x90Pa\x14 \x81\x83a%\xF4V[`\xC0\x89\x01Qa\x01\xA0\x88\x01Q\x91\x92P\x90\x81\x90\x84\x90\x81\x90\x83\t\x84\x08\x92Pa\x14L\x85a\x04\xEA\x8A_\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xC0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14t\x86a\x04\xEA\x8A` \x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xE0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14\x9C\x86a\x04\xEA\x8A`@\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14\xC4\x86a\x04\xEA\x8A``\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02 \x8A\x01Q\x83\t\x84\x08\x92Pa\x14\xEC\x86a\x04\xEA\x8A`\x80\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02@\x8A\x01Q\x83\t\x84\x08\x92Pa\x15\x14\x86a\x04\xEA\x8D`@\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02`\x8A\x01Q\x83\t\x84\x08\x92Pa\x15<\x86a\x04\xEA\x8D``\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\x80\x8A\x01Q\x83\t\x84\x08\x92Pa\x15d\x86a\x04\xEA\x8D`\x80\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\xA0\x8A\x01Q\x83\t\x84\x08\x92Pa\x15\x8C\x86a\x04\xEA\x8D`\xA0\x01Q\x84a\x12\x80V[\x95P_\x8A`\xE0\x01Q\x90P\x84\x85a\x02\xC0\x8B\x01Q\x83\t\x85\x08\x93Pa\x15\xB6\x87a\x04\xEA\x8B`\xA0\x01Q\x84a\x12\x80V[\x96Pa\x15\xECa\x15\xE6`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01R\x81Q\x80\x83\x01\x90\x92R`\x01\x82R`\x02\x90\x82\x01R\x90V[\x85a\x12\x80V[\x97PPPPPPP\x94P\x94\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x16$WP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x84` \x01Qa\x16h\x91\x90a&'V[a\x16\x92\x90\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDGa%\xF4V[\x90R\x92\x91PPV[a\x16\xC1`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[___`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x189W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x04>V[P\x15\x15\x90P[\x94\x93PPPPV[\x81Q_\x90_Q` a&G_9_Q\x90_R\x90\x83\x80\x15a\x18\x88W\x84\x93P_[\x82\x81\x10\x15a\x18|W\x83\x85\x86\t\x94P`\x01\x01a\x18fV[P`\x01\x84\x03\x93Pa\x18\x8FV[`\x01\x83\x03\x93P[PPP\x92\x91PPV[_\x82`\x01\x03a\x18\xA9WP`\x01a\x02\xA3V[\x81_\x03a\x18\xB7WP_a\x02\xA3V[` \x84\x01Q_Q` a&G_9_Q\x90_R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x18\xE5W`\x01\x87\x03\x92Pa\x18\xECV[`\x01\x84\x03\x92P[Pa\x18\xF6\x82a \x0BV[\x91P\x82\x82\x82\t\x97\x96PPPPPPPV[__Q` a&G_9_Q\x90_R\x82\x82\x03a\x19\x80W`\x01_[`\x0B\x81\x10\x15a\x19uW\x81\x86\x03a\x19RW\x86\x81`\x0B\x81\x10a\x19CWa\x19Ca%\xE0V[` \x02\x01Q\x93PPPPa\x18?V[\x82\x80a\x19`Wa\x19`a&\x13V[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\x19!V[P_\x92PPPa\x18?V[a\x19\x88a!AV[`@\x87\x01Q`\x01a\x01@\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x0B\x81\x10\x15a\x19\xCAW` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\x19\x9DV[PPPP___\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x0B\x81\x10\x15a\x1A\x1EW\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x19\xDEV[PPPP\x80\x92PP_a\x1A0\x83a \x0BV[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x9A\x99PPPPPPPPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R______Q` a&G_9_Q\x90_R\x90P`\x80\x89\x01Q\x81` \x8A\x01Q` \x8C\x01Q\t\x95P\x89Q\x94P\x81`\xA0\x8B\x01Q``\x8C\x01Q\t\x93P\x81a\x01\xA0\x89\x01Q\x85\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P\x81a\x01\xC0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P\x81a\x01\xE0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P\x81a\x02\0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P\x81a\x02 \x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92PP\x80\x84\x83\t\x93P\x80\x84\x86\x08\x94Pa\x1B\xC3\x87`\xA0\x01Q\x86a\x12\x80V[\x95P\x88Q``\x8A\x01Q`\x80\x8B\x01Q\x83\x82\x84\t\x97P\x83a\x02\xC0\x8B\x01Q\x89\t\x97P\x83a\x02@\x8B\x01Q\x83\t\x95P\x83a\x01\xA0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02`\x8B\x01Q\x83\t\x95P\x83a\x01\xC0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\x80\x8B\x01Q\x83\t\x95P\x83a\x01\xE0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\xA0\x8B\x01Q\x83\t\x95P\x83a\x02\0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95PPPP\x80\x83\x86\t\x94Pa\x1C\x8A\x86a\x04\xEA\x8C`\xC0\x01Q\x88\x85a\x1C\x85\x91\x90a%\xF4V[a\x12\x80V[\x95Pa\x1C\xA3\x86a\x04\xEA\x8C`\xE0\x01Q\x8Aa\x01\xA0\x01Qa\x12\x80V[\x95Pa\x1C\xBD\x86a\x04\xEA\x8Ca\x01\0\x01Q\x8Aa\x01\xC0\x01Qa\x12\x80V[\x95Pa\x1C\xD7\x86a\x04\xEA\x8Ca\x01 \x01Q\x8Aa\x01\xE0\x01Qa\x12\x80V[\x95Pa\x1C\xF1\x86a\x04\xEA\x8Ca\x01@\x01Q\x8Aa\x02\0\x01Qa\x12\x80V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92Pa\x1D\x16\x86a\x04\xEA\x8Ca\x01`\x01Q\x86a\x12\x80V[\x95P\x80a\x02\0\x88\x01Qa\x01\xE0\x89\x01Q\t\x92Pa\x1D;\x86a\x04\xEA\x8Ca\x01\x80\x01Q\x86a\x12\x80V[\x95Pa\x01\xA0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1Dj\x86a\x04\xEA\x8Ca\x01\xE0\x01Q\x86a\x12\x80V[\x95Pa\x01\xC0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\x99\x86a\x04\xEA\x8Ca\x02\0\x01Q\x86a\x12\x80V[\x95Pa\x01\xE0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\xC8\x86a\x04\xEA\x8Ca\x02 \x01Q\x86a\x12\x80V[\x95Pa\x02\0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\xF7\x86a\x04\xEA\x8Ca\x02@\x01Q\x86a\x12\x80V[\x95Pa\x1E\x14\x86a\x04\xEA\x8Ca\x01\xA0\x01Qa\x1C\x85\x8Ba\x02 \x01Qa \xACV[\x95Pa\x1E%\x86\x8Ba\x01\xC0\x01Qa\x13!V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92P\x80a\x01\xE0\x88\x01Q\x84\t\x92P\x80a\x02\0\x88\x01Q\x84\t\x92P\x80a\x02 \x88\x01Q\x84\t\x92Pa\x1Ek\x86a\x04\xEA\x8Ca\x02`\x01Q\x86a\x12\x80V[\x95Pa\x1Ey\x88_\x01Qa \xACV[\x94Pa\x1E\x8D\x86a\x04\xEA\x89`\xC0\x01Q\x88a\x12\x80V[\x95P\x80`\x01\x89Q\x08`\xA0\x8A\x01Q\x90\x93P\x81\x90\x80\t\x91P\x80\x82\x84\t\x92P\x80\x83\x86\t\x94Pa\x1E\xC1\x86a\x04\xEA\x89`\xE0\x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1E\xDC\x86a\x04\xEA\x89a\x01\0\x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1E\xF7\x86a\x04\xEA\x89a\x01 \x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1F\x12\x86a\x04\xEA\x89a\x01@\x01Q\x88a\x12\x80V[\x9A\x99PPPPPPPPPPV[___Q` a&G_9_Q\x90_R\x90P_\x83` \x01Q\x90P_\x84`@\x01Q\x90P_`\x01\x90P``\x88\x01Q`\x80\x89\x01Qa\x01\xA0\x89\x01Qa\x02@\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x89\x01Qa\x02`\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x89\x01Qa\x02\x80\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x89\x01Qa\x02\xA0\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x89\x01Q\x91Pa\x02\xC0\x89\x01Q\x86\x87\x82\x89\x85\x87\x08\t\x85\t\x93PPPP\x87Q` \x89\x01Q\x85\x86\x86\x83\t\x87\x03\x85\x08\x96PP\x84\x85\x83\x83\t\x86\x03\x87\x08\x99\x98PPPPPPPPPV[____Q` a&G_9_Q\x90_R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a \xA5W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01a\x04>V[PP\x91\x90PV[_a \xC4_Q` a&G_9_Q\x90_R\x83a&'V[a \xDB\x90_Q` a&G_9_Q\x90_Ra%\xF4V[\x92\x91PPV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!\0a!AV[\x90R\x90V[`@Q\x80``\x01`@R\x80`\x03\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80`\x80\x01`@R\x80`\x04\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80a\x01`\x01`@R\x80`\x0B\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\x98Wa!\x98a!`V[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\x98Wa!\x98a!`V[_`@\x82\x84\x03\x12\x15a!\xD2W__\xFD[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\xF5Wa!\xF5a!`V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x82`\x1F\x83\x01\x12a\"\x1EW__\xFD[`@Qa\x01`\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"BWa\"Ba!`V[`@R\x80a\x01`\x84\x01\x85\x81\x11\x15a\"WW__\xFD[\x84[\x81\x81\x10\x15a\"qW\x805\x83R` \x92\x83\x01\x92\x01a\"YV[P\x91\x95\x94PPPPPV[_a\x04\x80\x82\x84\x03\x12\x15a\"\x8DW__\xFD[a\"\x95a!tV[\x90Pa\"\xA1\x83\x83a!\xC2V[\x81Ra\"\xB0\x83`@\x84\x01a!\xC2V[` \x82\x01Ra\"\xC2\x83`\x80\x84\x01a!\xC2V[`@\x82\x01Ra\"\xD4\x83`\xC0\x84\x01a!\xC2V[``\x82\x01Ra\"\xE7\x83a\x01\0\x84\x01a!\xC2V[`\x80\x82\x01Ra\"\xFA\x83a\x01@\x84\x01a!\xC2V[`\xA0\x82\x01Ra#\r\x83a\x01\x80\x84\x01a!\xC2V[`\xC0\x82\x01Ra# \x83a\x01\xC0\x84\x01a!\xC2V[`\xE0\x82\x01Ra#3\x83a\x02\0\x84\x01a!\xC2V[a\x01\0\x82\x01Ra#G\x83a\x02@\x84\x01a!\xC2V[a\x01 \x82\x01Ra#[\x83a\x02\x80\x84\x01a!\xC2V[a\x01@\x82\x01Ra#o\x83a\x02\xC0\x84\x01a!\xC2V[a\x01`\x82\x01Ra#\x83\x83a\x03\0\x84\x01a!\xC2V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[___\x83\x85\x03a\n\xE0\x81\x12\x15a$\x1BW__\xFD[a\x05\0\x81\x12\x15a$)W__\xFD[Pa$2a!\x9EV[\x845\x81R` \x80\x86\x015\x90\x82\x01Ra$M\x86`@\x87\x01a!\xC2V[`@\x82\x01Ra$_\x86`\x80\x87\x01a!\xC2V[``\x82\x01Ra$q\x86`\xC0\x87\x01a!\xC2V[`\x80\x82\x01Ra$\x84\x86a\x01\0\x87\x01a!\xC2V[`\xA0\x82\x01Ra$\x97\x86a\x01@\x87\x01a!\xC2V[`\xC0\x82\x01Ra$\xAA\x86a\x01\x80\x87\x01a!\xC2V[`\xE0\x82\x01Ra$\xBD\x86a\x01\xC0\x87\x01a!\xC2V[a\x01\0\x82\x01Ra$\xD1\x86a\x02\0\x87\x01a!\xC2V[a\x01 \x82\x01Ra$\xE5\x86a\x02@\x87\x01a!\xC2V[a\x01@\x82\x01Ra$\xF9\x86a\x02\x80\x87\x01a!\xC2V[a\x01`\x82\x01Ra%\r\x86a\x02\xC0\x87\x01a!\xC2V[a\x01\x80\x82\x01Ra%!\x86a\x03\0\x87\x01a!\xC2V[a\x01\xA0\x82\x01Ra%5\x86a\x03@\x87\x01a!\xC2V[a\x01\xC0\x82\x01Ra%I\x86a\x03\x80\x87\x01a!\xC2V[a\x01\xE0\x82\x01Ra%]\x86a\x03\xC0\x87\x01a!\xC2V[a\x02\0\x82\x01Ra%q\x86a\x04\0\x87\x01a!\xC2V[a\x02 \x82\x01Ra%\x85\x86a\x04@\x87\x01a!\xC2V[a\x02@\x82\x01Ra%\x99\x86a\x04\x80\x87\x01a!\xC2V[a\x02`\x82\x01Ra\x04\xC0\x85\x015a\x02\x80\x82\x01Ra\x04\xE0\x85\x015a\x02\xA0\x82\x01R\x92Pa%\xC7\x85a\x05\0\x86\x01a\"\x0FV[\x91Pa%\xD7\x85a\x06`\x86\x01a\"|V[\x90P\x92P\x92P\x92V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a \xDBWcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a&AWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x730000000000000000000000000000000000000000301460806040526004361061009b575f3560e01c8063af196ba21161006e578063af196ba21461014e578063de24ac0f14610175578063e3512d561461019c578063f5144326146101c3578063fc8660c7146101ea575f5ffd5b80630c551f3f1461009f5780634b4734e3146100d95780635a14c0fe14610100578063834c452a14610127575b5f5ffd5b6100c67f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02581565b6040519081526020015b60405180910390f35b6100c67f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581565b6100c67f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a81565b6100c67f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181565b6100c67f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081565b6100c67f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88181565b6100c67f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a81565b6100c67f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe481565b6101fd6101f8366004612407565b61020d565b60405190151581526020016100d0565b5f610217826102aa565b610227835f5b60200201516103e5565b61023283600161021d565b61023d83600261021d565b61024883600361021d565b61025383600461021d565b61025e83600561021d565b61026983600661021d565b61027483600761021d565b61027f83600861021d565b61028a83600961021d565b61029583600a61021d565b6102a084848461044b565b90505b9392505050565b80516102b59061063f565b6102c2816020015161063f565b6102cf816040015161063f565b6102dc816060015161063f565b6102e9816080015161063f565b6102f68160a0015161063f565b6103038160c0015161063f565b6103108160e0015161063f565b61031e81610100015161063f565b61032c81610120015161063f565b61033a81610140015161063f565b61034881610160015161063f565b61035681610180015161063f565b610364816101a001516103e5565b610372816101c001516103e5565b610380816101e001516103e5565b61038e8161020001516103e5565b61039c8161022001516103e5565b6103aa8161024001516103e5565b6103b88161026001516103e5565b6103c68161028001516103e5565b6103d4816102a001516103e5565b6103e2816102c001516103e5565b50565b5f5160206126475f395f51905f528110806104475760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c64000000000060448201526064015b60405180910390fd5b5050565b5f8360200151600b14610471576040516320fa9d8960e11b815260040160405180910390fd5b5f61047d8585856106ed565b90505f61048c865f0151610c7c565b90505f61049e828460a0015188611223565b90506104bb60405180604001604052805f81526020015f81525090565b604080518082019091525f80825260208201526104ef8761016001516104ea8961018001518860e00151611280565b611321565b91505f5f6104ff8b88878c6113c5565b91509150610510816104ea846115fd565b9250610529836104ea8b61016001518a60a00151611280565b60a08801516040880151602001519194505f5160206126475f395f51905f52918290820990508160e08a01518209905061056c856104ea8d610180015184611280565b94505f60405180608001604052807f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081526020017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181526020017f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581526020017f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe4815250905061062d8782610620896115fd565b61062861169a565b611767565b9e9d5050505050505050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4791159015161561067857505050565b8251602084015182600384858586098509088382830914838210848410161693505050816106e85760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e74000000000000000000604482015260640161043e565b505050565b61072d6040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b5f5f5160206126475f395f51905f529050604051602081015f815260fe60e01b8152865160c01b6004820152602087015160c01b600c82015261028087015160208201526102a08701516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e087015180516101008301526020810151610120830152506101008701518051610140830152602081015161016083015250610120870151805161018083015260208101516101a08301525061014087015180516101c083015260208101516101e083015250610160870151805161020083015260208101516102208301525061018087015180516102408301526020810151610260830152506101e0870151805161028083015260208101516102a08301525061020087015180516102c083015260208101516102e083015250610220870151805161030083015260208101516103208301525061024087015180516103408301526020810151610360830152506101a0870151805161038083015260208101516103a0830152506101c087015180516103c083015260208101516103e0830152506102608701518051610400830152602081015161042083015250604087015180516104408301526020810151610460830152506060870151805161048083015260208101516104a083015250608087015180516104c083015260208101516104e08301525060a0870151805161050083015260208101516105208301525060c08701518051610540830152602081015161056083015250855161058082015260208601516105a082015260408601516105c082015260608601516105e0820152608086015161060082015260a086015161062082015260c086015161064082015260e08601516106608201526101008601516106808201526101208601516106a08201526101408601516106c0820152845180516106e08301526020810151610700830152506020850151805161072083015260208101516107408301525060408501518051610760830152602081015161078083015250606085015180516107a083015260208101516107c083015250608085015180516107e08301526020810151610800830152505f82526108408220825282825106606085015260208220825282825106608085015260a085015180518252602081015160208301525060608220808352838106855283818209848282099150806020870152508060408601525060c085015180518252602081015160208301525060e085015180516040830152602081015160608301525061010085015180516080830152602081015160a083015250610120850151805160c0830152602081015160e0830152506101408501518051610100830152602081015161012083015250610160822082528282510660a08501526101a085015181526101c085015160208201526101e085015160408201526102008501516060820152610220850151608082015261024085015160a082015261026085015160c082015261028085015160e08201526102a08501516101008201526102c0850151610120820152610160822082528282510660c08501526101608501518051825260208101516020830152506101808501518051604083015260208101516060830152505060a0812082810660e08501525050509392505050565b610c846120e1565b816201000003610e5b576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c10018152602001604051806101600160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd81526020017f15ee2475bee517c4ee05e51fa1ee7312a8373a0b13db8c51baf04cb2e99bd2bd81526020017e6fab49b869ae62001deac878b2667bd31bf3e28e3a2d764aa49b8d9bbdd31081526020017f2e856bf6d037708ffa4c06d4d8820f45ccadce9c5a6d178cbd573f82e0f9701181526020017f1407eee35993f2b1ad5ec6d9b8950ca3af33135d06037f871c5e33bf566dd7b48152508152509050919050565b816210000003611034576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c1018152602001604051806101600160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c81526020017f29e84143f5870d4776a92df8da8c6c9303d59088f37ba85f40cf6fd14265b4bc81526020017f1bf82deba7d74902c3708cc6e70e61f30512eca95655210e276e5858ce8f58e581526020017f22b94b2e2b0043d04e662d5ec018ea1c8a99a23a62c9eb46f0318f6a194985f081526020017f29969d8d5363bef1101a68e446a14e1da7ba9294e142a146a980fddb4d4d41a58152508152509050919050565b8160200361120a576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e7508000018152602001604051806101600160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b03904381526020017ee14b6364a47e9c4284a9f80a5fc41cd212b0d4dbf8a5703770a40a9a34399081526020017f30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f70363681526020017f22399c34139bffada8de046aac50c9628e3517a3a452795364e777cd65bb9f4881526020017f2290ee31c482cf92b79b1944db1c0147635e9004db8c3b9d13644bef31ec3bd38152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b61124460405180606001604052805f81526020015f81526020015f81525090565b61124e8484611847565b80825261125e9085908590611898565b6020820152805161127490859084908690611907565b60408201529392505050565b604080518082019091525f808252602082015261129b612105565b8351815260208085015190820152604081018390525f60608360808460076107d05a03fa905080806112cb575f5ffd5b50806113195760405162461bcd60e51b815260206004820152601960248201527f426e3235343a207363616c6172206d756c206661696c65642100000000000000604482015260640161043e565b505092915050565b604080518082019091525f808252602082015261133c612123565b8351815260208085015181830152835160408301528301516060808301919091525f908360c08460066107d05a03fa90508080611377575f5ffd5b50806113195760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a2067726f7570206164646974696f6e206661696c656421000000604482015260640161043e565b604080518082019091525f8082526020820152604080518082019091525f80825260208201525f6113f887878787611a56565b90505f5160206126475f395f51905f525f611414888789611f20565b905061142081836125f4565b60c08901516101a08801519192509081908490819083098408925061144c856104ea8a5f015184611280565b955083828209905083846101c08a0151830984089250611474866104ea8a6020015184611280565b955083828209905083846101e08a015183098408925061149c866104ea8a6040015184611280565b955083828209905083846102008a01518309840892506114c4866104ea8a6060015184611280565b955083828209905083846102208a01518309840892506114ec866104ea8a6080015184611280565b955083828209905083846102408a0151830984089250611514866104ea8d6040015184611280565b955083828209905083846102608a015183098408925061153c866104ea8d6060015184611280565b955083828209905083846102808a0151830984089250611564866104ea8d6080015184611280565b955083828209905083846102a08a015183098408925061158c866104ea8d60a0015184611280565b95505f8a60e00151905084856102c08b01518309850893506115b6876104ea8b60a0015184611280565b96506115ec6115e66040805180820182525f80825260209182015281518083019092526001825260029082015290565b85611280565b975050505050505094509492505050565b604080518082019091525f8082526020820152815160208301511590151615611624575090565b6040518060400160405280835f015181526020017f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4784602001516116689190612627565b611692907f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd476125f4565b905292915050565b6116c160405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f5f5f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f519150806118395760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c65642100000000604482015260640161043e565b50151590505b949350505050565b81515f905f5160206126475f395f51905f5290838015611888578493505f5b8281101561187c57838586099450600101611866565b5060018403935061188f565b6001830393505b50505092915050565b5f826001036118a9575060016102a3565b815f036118b757505f6102a3565b60208401515f5160206126475f395f51905f52905f908281860990508580156118e5576001870392506118ec565b6001840392505b506118f68261200b565b915082828209979650505050505050565b5f5f5160206126475f395f51905f528282036119805760015f5b600b81101561197557818603611952578681600b8110611943576119436125e0565b6020020151935050505061183f565b828061196057611960612613565b60408901516020015183099150600101611921565b505f9250505061183f565b611988612141565b60408701516001610140838101828152920190805b600b8110156119ca5760208403935085868a85518903088309808552601f1990930192915060010161199d565b505050505f5f5f90506001838960408c01515f5b600b811015611a1e578882518a85518c88518a0909098981880896505088898d84518c0308860994506020938401939283019291909101906001016119de565b50505050809250505f611a308361200b565b905060208a015185818909965050848187099550848287099a9950505050505050505050565b604080518082019091525f80825260208201525f5f5f5f5f5f5160206126475f395f51905f52905060808901518160208a015160208c0151099550895194508160a08b015160608c0151099350816101a089015185089250818184089250818584099450817f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a85099250816101c089015184089250818184089250818584099450817f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02585099250816101e089015184089250818184089250818584099450817f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992508161020089015184089250818184089250818584099450817f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88185099250816102208901518408925081818408925050808483099350808486089450611bc38760a0015186611280565b9550885160608a015160808b0151838284099750836102c08b015189099750836102408b015183099550836101a08b015187089550838187089550838689099750836102608b015183099550836101c08b015187089550838187089550838689099750836102808b015183099550836101e08b015187089550838187089550838689099750836102a08b015183099550836102008b015187089550838187089550505050808386099450611c8a866104ea8c60c001518885611c8591906125f4565b611280565b9550611ca3866104ea8c60e001518a6101a00151611280565b9550611cbd866104ea8c61010001518a6101c00151611280565b9550611cd7866104ea8c61012001518a6101e00151611280565b9550611cf1866104ea8c61014001518a6102000151611280565b9550806101c08801516101a0890151099250611d16866104ea8c610160015186611280565b9550806102008801516101e0890151099250611d3b866104ea8c610180015186611280565b95506101a08701519250808384099150808283099150808284099250611d6a866104ea8c6101e0015186611280565b95506101c08701519250808384099150808283099150808284099250611d99866104ea8c610200015186611280565b95506101e08701519250808384099150808283099150808284099250611dc8866104ea8c610220015186611280565b95506102008701519250808384099150808283099150808284099250611df7866104ea8c610240015186611280565b9550611e14866104ea8c6101a00151611c858b61022001516120ac565b9550611e25868b6101c00151611321565b9550806101c08801516101a0890151099250806101e08801518409925080610200880151840992508061022088015184099250611e6b866104ea8c610260015186611280565b9550611e79885f01516120ac565b9450611e8d866104ea8960c0015188611280565b955080600189510860a08a0151909350819080099150808284099250808386099450611ec1866104ea8960e0015188611280565b9550808386099450611edc866104ea89610100015188611280565b9550808386099450611ef7866104ea89610120015188611280565b9550808386099450611f12866104ea89610140015188611280565b9a9950505050505050505050565b5f5f5f5160206126475f395f51905f5290505f836020015190505f846040015190505f60019050606088015160808901516101a08901516102408a01518788898387098a868608088609945050506101c08901516102608a01518788898387098a868608088609945050506101e08901516102808a01518788898387098a868608088609945050506102008901516102a08a01518788898387098a8686080886099450505061022089015191506102c0890151868782898587080985099350505050875160208901518586868309870385089650508485838309860387089998505050505050505050565b5f5f5f5f5160206126475f395f51905f52905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f519250816120a55760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c656421000000604482015260640161043e565b5050919050565b5f6120c45f5160206126475f395f51905f5283612627565b6120db905f5160206126475f395f51905f526125f4565b92915050565b60405180606001604052805f81526020015f8152602001612100612141565b905290565b60405180606001604052806003906020820280368337509192915050565b60405180608001604052806004906020820280368337509192915050565b604051806101600160405280600b906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561219857612198612160565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561219857612198612160565b5f604082840312156121d2575f5ffd5b6040805190810167ffffffffffffffff811182821017156121f5576121f5612160565b604052823581526020928301359281019290925250919050565b5f82601f83011261221e575f5ffd5b604051610160810167ffffffffffffffff8111828210171561224257612242612160565b60405280610160840185811115612257575f5ffd5b845b81811015612271578035835260209283019201612259565b509195945050505050565b5f610480828403121561228d575f5ffd5b612295612174565b90506122a183836121c2565b81526122b083604084016121c2565b60208201526122c283608084016121c2565b60408201526122d48360c084016121c2565b60608201526122e78361010084016121c2565b60808201526122fa8361014084016121c2565b60a082015261230d8361018084016121c2565b60c0820152612320836101c084016121c2565b60e08201526123338361020084016121c2565b6101008201526123478361024084016121c2565b61012082015261235b8361028084016121c2565b61014082015261236f836102c084016121c2565b6101608201526123838361030084016121c2565b6101808201526103408201356101a08201526103608201356101c08201526103808201356101e08201526103a08201356102008201526103c08201356102208201526103e08201356102408201526104008201356102608201526104208201356102808201526104408201356102a0820152610460909101356102c0820152919050565b5f5f5f838503610ae081121561241b575f5ffd5b610500811215612429575f5ffd5b5061243261219e565b843581526020808601359082015261244d86604087016121c2565b604082015261245f86608087016121c2565b60608201526124718660c087016121c2565b60808201526124848661010087016121c2565b60a08201526124978661014087016121c2565b60c08201526124aa8661018087016121c2565b60e08201526124bd866101c087016121c2565b6101008201526124d18661020087016121c2565b6101208201526124e58661024087016121c2565b6101408201526124f98661028087016121c2565b61016082015261250d866102c087016121c2565b6101808201526125218661030087016121c2565b6101a08201526125358661034087016121c2565b6101c08201526125498661038087016121c2565b6101e082015261255d866103c087016121c2565b6102008201526125718661040087016121c2565b6102208201526125858661044087016121c2565b6102408201526125998661048087016121c2565b6102608201526104c08501356102808201526104e08501356102a082015292506125c785610500860161220f565b91506125d785610660860161227c565b90509250925092565b634e487b7160e01b5f52603260045260245ffd5b818103818111156120db57634e487b7160e01b5f52601160045260245ffd5b634e487b7160e01b5f52601260045260245ffd5b5f8261264157634e487b7160e01b5f52601260045260245ffd5b50069056fe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"s\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x000\x14`\x80`@R`\x046\x10a\0\x9BW_5`\xE0\x1C\x80c\xAF\x19k\xA2\x11a\0nW\x80c\xAF\x19k\xA2\x14a\x01NW\x80c\xDE$\xAC\x0F\x14a\x01uW\x80c\xE3Q-V\x14a\x01\x9CW\x80c\xF5\x14C&\x14a\x01\xC3W\x80c\xFC\x86`\xC7\x14a\x01\xEAW__\xFD[\x80c\x0CU\x1F?\x14a\0\x9FW\x80cKG4\xE3\x14a\0\xD9W\x80cZ\x14\xC0\xFE\x14a\x01\0W\x80c\x83LE*\x14a\x01'W[__\xFD[a\0\xC6\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xC6\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81V[a\0\xC6\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x81V[a\0\xC6\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81V[a\0\xC6\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81V[a\0\xC6\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x81V[a\0\xC6\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x81V[a\0\xC6\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81V[a\x01\xFDa\x01\xF86`\x04a$\x07V[a\x02\rV[`@Q\x90\x15\x15\x81R` \x01a\0\xD0V[_a\x02\x17\x82a\x02\xAAV[a\x02'\x83_[` \x02\x01Qa\x03\xE5V[a\x022\x83`\x01a\x02\x1DV[a\x02=\x83`\x02a\x02\x1DV[a\x02H\x83`\x03a\x02\x1DV[a\x02S\x83`\x04a\x02\x1DV[a\x02^\x83`\x05a\x02\x1DV[a\x02i\x83`\x06a\x02\x1DV[a\x02t\x83`\x07a\x02\x1DV[a\x02\x7F\x83`\x08a\x02\x1DV[a\x02\x8A\x83`\ta\x02\x1DV[a\x02\x95\x83`\na\x02\x1DV[a\x02\xA0\x84\x84\x84a\x04KV[\x90P[\x93\x92PPPV[\x80Qa\x02\xB5\x90a\x06?V[a\x02\xC2\x81` \x01Qa\x06?V[a\x02\xCF\x81`@\x01Qa\x06?V[a\x02\xDC\x81``\x01Qa\x06?V[a\x02\xE9\x81`\x80\x01Qa\x06?V[a\x02\xF6\x81`\xA0\x01Qa\x06?V[a\x03\x03\x81`\xC0\x01Qa\x06?V[a\x03\x10\x81`\xE0\x01Qa\x06?V[a\x03\x1E\x81a\x01\0\x01Qa\x06?V[a\x03,\x81a\x01 \x01Qa\x06?V[a\x03:\x81a\x01@\x01Qa\x06?V[a\x03H\x81a\x01`\x01Qa\x06?V[a\x03V\x81a\x01\x80\x01Qa\x06?V[a\x03d\x81a\x01\xA0\x01Qa\x03\xE5V[a\x03r\x81a\x01\xC0\x01Qa\x03\xE5V[a\x03\x80\x81a\x01\xE0\x01Qa\x03\xE5V[a\x03\x8E\x81a\x02\0\x01Qa\x03\xE5V[a\x03\x9C\x81a\x02 \x01Qa\x03\xE5V[a\x03\xAA\x81a\x02@\x01Qa\x03\xE5V[a\x03\xB8\x81a\x02`\x01Qa\x03\xE5V[a\x03\xC6\x81a\x02\x80\x01Qa\x03\xE5V[a\x03\xD4\x81a\x02\xA0\x01Qa\x03\xE5V[a\x03\xE2\x81a\x02\xC0\x01Qa\x03\xE5V[PV[_Q` a&G_9_Q\x90_R\x81\x10\x80a\x04GW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[PPV[_\x83` \x01Q`\x0B\x14a\x04qW`@Qc \xFA\x9D\x89`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x04}\x85\x85\x85a\x06\xEDV[\x90P_a\x04\x8C\x86_\x01Qa\x0C|V[\x90P_a\x04\x9E\x82\x84`\xA0\x01Q\x88a\x12#V[\x90Pa\x04\xBB`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x04\xEF\x87a\x01`\x01Qa\x04\xEA\x89a\x01\x80\x01Q\x88`\xE0\x01Qa\x12\x80V[a\x13!V[\x91P__a\x04\xFF\x8B\x88\x87\x8Ca\x13\xC5V[\x91P\x91Pa\x05\x10\x81a\x04\xEA\x84a\x15\xFDV[\x92Pa\x05)\x83a\x04\xEA\x8Ba\x01`\x01Q\x8A`\xA0\x01Qa\x12\x80V[`\xA0\x88\x01Q`@\x88\x01Q` \x01Q\x91\x94P_Q` a&G_9_Q\x90_R\x91\x82\x90\x82\t\x90P\x81`\xE0\x8A\x01Q\x82\t\x90Pa\x05l\x85a\x04\xEA\x8Da\x01\x80\x01Q\x84a\x12\x80V[\x94P_`@Q\x80`\x80\x01`@R\x80\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81R` \x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81R` \x01\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81R` \x01\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81RP\x90Pa\x06-\x87\x82a\x06 \x89a\x15\xFDV[a\x06(a\x16\x9AV[a\x17gV[\x9E\x9DPPPPPPPPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x06xWPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x06\xE8W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x04>V[PPPV[a\x07-`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[__Q` a&G_9_Q\x90_R\x90P`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x86Q`\xC0\x1B`\x04\x82\x01R` \x87\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x87\x01Q` \x82\x01Ra\x02\xA0\x87\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x87\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x87\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x87\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x87\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x87\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x87\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x87\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x87\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x87\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x87\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x87\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x87\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x87\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x87\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x87\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x87\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x87\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x87\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x85Qa\x05\x80\x82\x01R` \x86\x01Qa\x05\xA0\x82\x01R`@\x86\x01Qa\x05\xC0\x82\x01R``\x86\x01Qa\x05\xE0\x82\x01R`\x80\x86\x01Qa\x06\0\x82\x01R`\xA0\x86\x01Qa\x06 \x82\x01R`\xC0\x86\x01Qa\x06@\x82\x01R`\xE0\x86\x01Qa\x06`\x82\x01Ra\x01\0\x86\x01Qa\x06\x80\x82\x01Ra\x01 \x86\x01Qa\x06\xA0\x82\x01Ra\x01@\x86\x01Qa\x06\xC0\x82\x01R\x84Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP` \x85\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`@\x85\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP``\x85\x01Q\x80Qa\x07\xA0\x83\x01R` \x81\x01Qa\x07\xC0\x83\x01RP`\x80\x85\x01Q\x80Qa\x07\xE0\x83\x01R` \x81\x01Qa\x08\0\x83\x01RP_\x82Ra\x08@\x82 \x82R\x82\x82Q\x06``\x85\x01R` \x82 \x82R\x82\x82Q\x06`\x80\x85\x01R`\xA0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 \x80\x83R\x83\x81\x06\x85R\x83\x81\x82\t\x84\x82\x82\t\x91P\x80` \x87\x01RP\x80`@\x86\x01RP`\xC0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x85\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x85\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x85\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 \x82R\x82\x82Q\x06`\xA0\x85\x01Ra\x01\xA0\x85\x01Q\x81Ra\x01\xC0\x85\x01Q` \x82\x01Ra\x01\xE0\x85\x01Q`@\x82\x01Ra\x02\0\x85\x01Q``\x82\x01Ra\x02 \x85\x01Q`\x80\x82\x01Ra\x02@\x85\x01Q`\xA0\x82\x01Ra\x02`\x85\x01Q`\xC0\x82\x01Ra\x02\x80\x85\x01Q`\xE0\x82\x01Ra\x02\xA0\x85\x01Qa\x01\0\x82\x01Ra\x02\xC0\x85\x01Qa\x01 \x82\x01Ra\x01`\x82 \x82R\x82\x82Q\x06`\xC0\x85\x01Ra\x01`\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 \x82\x81\x06`\xE0\x85\x01RPPP\x93\x92PPPV[a\x0C\x84a \xE1V[\x81b\x01\0\0\x03a\x0E[W`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81R` \x01\x7F\x15\xEE$u\xBE\xE5\x17\xC4\xEE\x05\xE5\x1F\xA1\xEEs\x12\xA87:\x0B\x13\xDB\x8CQ\xBA\xF0L\xB2\xE9\x9B\xD2\xBD\x81R` \x01~o\xABI\xB8i\xAEb\0\x1D\xEA\xC8x\xB2f{\xD3\x1B\xF3\xE2\x8E:-vJ\xA4\x9B\x8D\x9B\xBD\xD3\x10\x81R` \x01\x7F.\x85k\xF6\xD07p\x8F\xFAL\x06\xD4\xD8\x82\x0FE\xCC\xAD\xCE\x9CZm\x17\x8C\xBDW?\x82\xE0\xF9p\x11\x81R` \x01\x7F\x14\x07\xEE\xE3Y\x93\xF2\xB1\xAD^\xC6\xD9\xB8\x95\x0C\xA3\xAF3\x13]\x06\x03\x7F\x87\x1C^3\xBFVm\xD7\xB4\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x104W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81R` \x01\x7F)\xE8AC\xF5\x87\rGv\xA9-\xF8\xDA\x8Cl\x93\x03\xD5\x90\x88\xF3{\xA8_@\xCFo\xD1Be\xB4\xBC\x81R` \x01\x7F\x1B\xF8-\xEB\xA7\xD7I\x02\xC3p\x8C\xC6\xE7\x0Ea\xF3\x05\x12\xEC\xA9VU!\x0E'nXX\xCE\x8FX\xE5\x81R` \x01\x7F\"\xB9K.+\0C\xD0Nf-^\xC0\x18\xEA\x1C\x8A\x99\xA2:b\xC9\xEBF\xF01\x8Fj\x19I\x85\xF0\x81R` \x01\x7F)\x96\x9D\x8DSc\xBE\xF1\x10\x1Ah\xE4F\xA1N\x1D\xA7\xBA\x92\x94\xE1B\xA1F\xA9\x80\xFD\xDBMMA\xA5\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\x12\nW`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81R` \x01~\xE1Kcd\xA4~\x9CB\x84\xA9\xF8\n_\xC4\x1C\xD2\x12\xB0\xD4\xDB\xF8\xA5p7p\xA4\n\x9A49\x90\x81R` \x01\x7F0dNr\xE11\xA0)\x04\x8Bn\x19?\xD8A\x04\\\xEA$\xF6\xFDsk\xEC#\x12\x04p\x8Fp66\x81R` \x01\x7F\"9\x9C4\x13\x9B\xFF\xAD\xA8\xDE\x04j\xACP\xC9b\x8E5\x17\xA3\xA4RySd\xE7w\xCDe\xBB\x9FH\x81R` \x01\x7F\"\x90\xEE1\xC4\x82\xCF\x92\xB7\x9B\x19D\xDB\x1C\x01Gc^\x90\x04\xDB\x8C;\x9D\x13dK\xEF1\xEC;\xD3\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x12D`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\x12N\x84\x84a\x18GV[\x80\x82Ra\x12^\x90\x85\x90\x85\x90a\x18\x98V[` \x82\x01R\x80Qa\x12t\x90\x85\x90\x84\x90\x86\x90a\x19\x07V[`@\x82\x01R\x93\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x12\x9Ba!\x05V[\x83Q\x81R` \x80\x85\x01Q\x90\x82\x01R`@\x81\x01\x83\x90R_``\x83`\x80\x84`\x07a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x12\xCBW__\xFD[P\x80a\x13\x19W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x19`$\x82\x01R\x7FBn254: scalar mul failed!\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x04>V[PP\x92\x91PPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x13<a!#V[\x83Q\x81R` \x80\x85\x01Q\x81\x83\x01R\x83Q`@\x83\x01R\x83\x01Q``\x80\x83\x01\x91\x90\x91R_\x90\x83`\xC0\x84`\x06a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x13wW__\xFD[P\x80a\x13\x19W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: group addition failed!\0\0\0`D\x82\x01R`d\x01a\x04>V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x13\xF8\x87\x87\x87\x87a\x1AVV[\x90P_Q` a&G_9_Q\x90_R_a\x14\x14\x88\x87\x89a\x1F V[\x90Pa\x14 \x81\x83a%\xF4V[`\xC0\x89\x01Qa\x01\xA0\x88\x01Q\x91\x92P\x90\x81\x90\x84\x90\x81\x90\x83\t\x84\x08\x92Pa\x14L\x85a\x04\xEA\x8A_\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xC0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14t\x86a\x04\xEA\x8A` \x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xE0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14\x9C\x86a\x04\xEA\x8A`@\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\0\x8A\x01Q\x83\t\x84\x08\x92Pa\x14\xC4\x86a\x04\xEA\x8A``\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02 \x8A\x01Q\x83\t\x84\x08\x92Pa\x14\xEC\x86a\x04\xEA\x8A`\x80\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02@\x8A\x01Q\x83\t\x84\x08\x92Pa\x15\x14\x86a\x04\xEA\x8D`@\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02`\x8A\x01Q\x83\t\x84\x08\x92Pa\x15<\x86a\x04\xEA\x8D``\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\x80\x8A\x01Q\x83\t\x84\x08\x92Pa\x15d\x86a\x04\xEA\x8D`\x80\x01Q\x84a\x12\x80V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\xA0\x8A\x01Q\x83\t\x84\x08\x92Pa\x15\x8C\x86a\x04\xEA\x8D`\xA0\x01Q\x84a\x12\x80V[\x95P_\x8A`\xE0\x01Q\x90P\x84\x85a\x02\xC0\x8B\x01Q\x83\t\x85\x08\x93Pa\x15\xB6\x87a\x04\xEA\x8B`\xA0\x01Q\x84a\x12\x80V[\x96Pa\x15\xECa\x15\xE6`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01R\x81Q\x80\x83\x01\x90\x92R`\x01\x82R`\x02\x90\x82\x01R\x90V[\x85a\x12\x80V[\x97PPPPPPP\x94P\x94\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x16$WP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x84` \x01Qa\x16h\x91\x90a&'V[a\x16\x92\x90\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDGa%\xF4V[\x90R\x92\x91PPV[a\x16\xC1`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[___`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x189W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x04>V[P\x15\x15\x90P[\x94\x93PPPPV[\x81Q_\x90_Q` a&G_9_Q\x90_R\x90\x83\x80\x15a\x18\x88W\x84\x93P_[\x82\x81\x10\x15a\x18|W\x83\x85\x86\t\x94P`\x01\x01a\x18fV[P`\x01\x84\x03\x93Pa\x18\x8FV[`\x01\x83\x03\x93P[PPP\x92\x91PPV[_\x82`\x01\x03a\x18\xA9WP`\x01a\x02\xA3V[\x81_\x03a\x18\xB7WP_a\x02\xA3V[` \x84\x01Q_Q` a&G_9_Q\x90_R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x18\xE5W`\x01\x87\x03\x92Pa\x18\xECV[`\x01\x84\x03\x92P[Pa\x18\xF6\x82a \x0BV[\x91P\x82\x82\x82\t\x97\x96PPPPPPPV[__Q` a&G_9_Q\x90_R\x82\x82\x03a\x19\x80W`\x01_[`\x0B\x81\x10\x15a\x19uW\x81\x86\x03a\x19RW\x86\x81`\x0B\x81\x10a\x19CWa\x19Ca%\xE0V[` \x02\x01Q\x93PPPPa\x18?V[\x82\x80a\x19`Wa\x19`a&\x13V[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\x19!V[P_\x92PPPa\x18?V[a\x19\x88a!AV[`@\x87\x01Q`\x01a\x01@\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x0B\x81\x10\x15a\x19\xCAW` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\x19\x9DV[PPPP___\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x0B\x81\x10\x15a\x1A\x1EW\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x19\xDEV[PPPP\x80\x92PP_a\x1A0\x83a \x0BV[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x9A\x99PPPPPPPPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R______Q` a&G_9_Q\x90_R\x90P`\x80\x89\x01Q\x81` \x8A\x01Q` \x8C\x01Q\t\x95P\x89Q\x94P\x81`\xA0\x8B\x01Q``\x8C\x01Q\t\x93P\x81a\x01\xA0\x89\x01Q\x85\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P\x81a\x01\xC0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P\x81a\x01\xE0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P\x81a\x02\0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P\x81a\x02 \x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92PP\x80\x84\x83\t\x93P\x80\x84\x86\x08\x94Pa\x1B\xC3\x87`\xA0\x01Q\x86a\x12\x80V[\x95P\x88Q``\x8A\x01Q`\x80\x8B\x01Q\x83\x82\x84\t\x97P\x83a\x02\xC0\x8B\x01Q\x89\t\x97P\x83a\x02@\x8B\x01Q\x83\t\x95P\x83a\x01\xA0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02`\x8B\x01Q\x83\t\x95P\x83a\x01\xC0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\x80\x8B\x01Q\x83\t\x95P\x83a\x01\xE0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\xA0\x8B\x01Q\x83\t\x95P\x83a\x02\0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95PPPP\x80\x83\x86\t\x94Pa\x1C\x8A\x86a\x04\xEA\x8C`\xC0\x01Q\x88\x85a\x1C\x85\x91\x90a%\xF4V[a\x12\x80V[\x95Pa\x1C\xA3\x86a\x04\xEA\x8C`\xE0\x01Q\x8Aa\x01\xA0\x01Qa\x12\x80V[\x95Pa\x1C\xBD\x86a\x04\xEA\x8Ca\x01\0\x01Q\x8Aa\x01\xC0\x01Qa\x12\x80V[\x95Pa\x1C\xD7\x86a\x04\xEA\x8Ca\x01 \x01Q\x8Aa\x01\xE0\x01Qa\x12\x80V[\x95Pa\x1C\xF1\x86a\x04\xEA\x8Ca\x01@\x01Q\x8Aa\x02\0\x01Qa\x12\x80V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92Pa\x1D\x16\x86a\x04\xEA\x8Ca\x01`\x01Q\x86a\x12\x80V[\x95P\x80a\x02\0\x88\x01Qa\x01\xE0\x89\x01Q\t\x92Pa\x1D;\x86a\x04\xEA\x8Ca\x01\x80\x01Q\x86a\x12\x80V[\x95Pa\x01\xA0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1Dj\x86a\x04\xEA\x8Ca\x01\xE0\x01Q\x86a\x12\x80V[\x95Pa\x01\xC0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\x99\x86a\x04\xEA\x8Ca\x02\0\x01Q\x86a\x12\x80V[\x95Pa\x01\xE0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\xC8\x86a\x04\xEA\x8Ca\x02 \x01Q\x86a\x12\x80V[\x95Pa\x02\0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1D\xF7\x86a\x04\xEA\x8Ca\x02@\x01Q\x86a\x12\x80V[\x95Pa\x1E\x14\x86a\x04\xEA\x8Ca\x01\xA0\x01Qa\x1C\x85\x8Ba\x02 \x01Qa \xACV[\x95Pa\x1E%\x86\x8Ba\x01\xC0\x01Qa\x13!V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92P\x80a\x01\xE0\x88\x01Q\x84\t\x92P\x80a\x02\0\x88\x01Q\x84\t\x92P\x80a\x02 \x88\x01Q\x84\t\x92Pa\x1Ek\x86a\x04\xEA\x8Ca\x02`\x01Q\x86a\x12\x80V[\x95Pa\x1Ey\x88_\x01Qa \xACV[\x94Pa\x1E\x8D\x86a\x04\xEA\x89`\xC0\x01Q\x88a\x12\x80V[\x95P\x80`\x01\x89Q\x08`\xA0\x8A\x01Q\x90\x93P\x81\x90\x80\t\x91P\x80\x82\x84\t\x92P\x80\x83\x86\t\x94Pa\x1E\xC1\x86a\x04\xEA\x89`\xE0\x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1E\xDC\x86a\x04\xEA\x89a\x01\0\x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1E\xF7\x86a\x04\xEA\x89a\x01 \x01Q\x88a\x12\x80V[\x95P\x80\x83\x86\t\x94Pa\x1F\x12\x86a\x04\xEA\x89a\x01@\x01Q\x88a\x12\x80V[\x9A\x99PPPPPPPPPPV[___Q` a&G_9_Q\x90_R\x90P_\x83` \x01Q\x90P_\x84`@\x01Q\x90P_`\x01\x90P``\x88\x01Q`\x80\x89\x01Qa\x01\xA0\x89\x01Qa\x02@\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x89\x01Qa\x02`\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x89\x01Qa\x02\x80\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x89\x01Qa\x02\xA0\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x89\x01Q\x91Pa\x02\xC0\x89\x01Q\x86\x87\x82\x89\x85\x87\x08\t\x85\t\x93PPPP\x87Q` \x89\x01Q\x85\x86\x86\x83\t\x87\x03\x85\x08\x96PP\x84\x85\x83\x83\t\x86\x03\x87\x08\x99\x98PPPPPPPPPV[____Q` a&G_9_Q\x90_R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a \xA5W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01a\x04>V[PP\x91\x90PV[_a \xC4_Q` a&G_9_Q\x90_R\x83a&'V[a \xDB\x90_Q` a&G_9_Q\x90_Ra%\xF4V[\x92\x91PPV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!\0a!AV[\x90R\x90V[`@Q\x80``\x01`@R\x80`\x03\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80`\x80\x01`@R\x80`\x04\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80a\x01`\x01`@R\x80`\x0B\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\x98Wa!\x98a!`V[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\x98Wa!\x98a!`V[_`@\x82\x84\x03\x12\x15a!\xD2W__\xFD[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!\xF5Wa!\xF5a!`V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x82`\x1F\x83\x01\x12a\"\x1EW__\xFD[`@Qa\x01`\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"BWa\"Ba!`V[`@R\x80a\x01`\x84\x01\x85\x81\x11\x15a\"WW__\xFD[\x84[\x81\x81\x10\x15a\"qW\x805\x83R` \x92\x83\x01\x92\x01a\"YV[P\x91\x95\x94PPPPPV[_a\x04\x80\x82\x84\x03\x12\x15a\"\x8DW__\xFD[a\"\x95a!tV[\x90Pa\"\xA1\x83\x83a!\xC2V[\x81Ra\"\xB0\x83`@\x84\x01a!\xC2V[` \x82\x01Ra\"\xC2\x83`\x80\x84\x01a!\xC2V[`@\x82\x01Ra\"\xD4\x83`\xC0\x84\x01a!\xC2V[``\x82\x01Ra\"\xE7\x83a\x01\0\x84\x01a!\xC2V[`\x80\x82\x01Ra\"\xFA\x83a\x01@\x84\x01a!\xC2V[`\xA0\x82\x01Ra#\r\x83a\x01\x80\x84\x01a!\xC2V[`\xC0\x82\x01Ra# \x83a\x01\xC0\x84\x01a!\xC2V[`\xE0\x82\x01Ra#3\x83a\x02\0\x84\x01a!\xC2V[a\x01\0\x82\x01Ra#G\x83a\x02@\x84\x01a!\xC2V[a\x01 \x82\x01Ra#[\x83a\x02\x80\x84\x01a!\xC2V[a\x01@\x82\x01Ra#o\x83a\x02\xC0\x84\x01a!\xC2V[a\x01`\x82\x01Ra#\x83\x83a\x03\0\x84\x01a!\xC2V[a\x01\x80\x82\x01Ra\x03@\x82\x015a\x01\xA0\x82\x01Ra\x03`\x82\x015a\x01\xC0\x82\x01Ra\x03\x80\x82\x015a\x01\xE0\x82\x01Ra\x03\xA0\x82\x015a\x02\0\x82\x01Ra\x03\xC0\x82\x015a\x02 \x82\x01Ra\x03\xE0\x82\x015a\x02@\x82\x01Ra\x04\0\x82\x015a\x02`\x82\x01Ra\x04 \x82\x015a\x02\x80\x82\x01Ra\x04@\x82\x015a\x02\xA0\x82\x01Ra\x04`\x90\x91\x015a\x02\xC0\x82\x01R\x91\x90PV[___\x83\x85\x03a\n\xE0\x81\x12\x15a$\x1BW__\xFD[a\x05\0\x81\x12\x15a$)W__\xFD[Pa$2a!\x9EV[\x845\x81R` \x80\x86\x015\x90\x82\x01Ra$M\x86`@\x87\x01a!\xC2V[`@\x82\x01Ra$_\x86`\x80\x87\x01a!\xC2V[``\x82\x01Ra$q\x86`\xC0\x87\x01a!\xC2V[`\x80\x82\x01Ra$\x84\x86a\x01\0\x87\x01a!\xC2V[`\xA0\x82\x01Ra$\x97\x86a\x01@\x87\x01a!\xC2V[`\xC0\x82\x01Ra$\xAA\x86a\x01\x80\x87\x01a!\xC2V[`\xE0\x82\x01Ra$\xBD\x86a\x01\xC0\x87\x01a!\xC2V[a\x01\0\x82\x01Ra$\xD1\x86a\x02\0\x87\x01a!\xC2V[a\x01 \x82\x01Ra$\xE5\x86a\x02@\x87\x01a!\xC2V[a\x01@\x82\x01Ra$\xF9\x86a\x02\x80\x87\x01a!\xC2V[a\x01`\x82\x01Ra%\r\x86a\x02\xC0\x87\x01a!\xC2V[a\x01\x80\x82\x01Ra%!\x86a\x03\0\x87\x01a!\xC2V[a\x01\xA0\x82\x01Ra%5\x86a\x03@\x87\x01a!\xC2V[a\x01\xC0\x82\x01Ra%I\x86a\x03\x80\x87\x01a!\xC2V[a\x01\xE0\x82\x01Ra%]\x86a\x03\xC0\x87\x01a!\xC2V[a\x02\0\x82\x01Ra%q\x86a\x04\0\x87\x01a!\xC2V[a\x02 \x82\x01Ra%\x85\x86a\x04@\x87\x01a!\xC2V[a\x02@\x82\x01Ra%\x99\x86a\x04\x80\x87\x01a!\xC2V[a\x02`\x82\x01Ra\x04\xC0\x85\x015a\x02\x80\x82\x01Ra\x04\xE0\x85\x015a\x02\xA0\x82\x01R\x92Pa%\xC7\x85a\x05\0\x86\x01a\"\x0FV[\x91Pa%\xD7\x85a\x06`\x86\x01a\"|V[\x90P\x92P\x92P\x92V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a \xDBWcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a&AWcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidPlonkArgs()` and selector `0xfd9a2d1b`.
    ```solidity
    error InvalidPlonkArgs();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidPlonkArgs {}
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
        impl ::core::convert::From<InvalidPlonkArgs> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidPlonkArgs) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidPlonkArgs {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidPlonkArgs {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidPlonkArgs()";
            const SELECTOR: [u8; 4] = [253u8, 154u8, 45u8, 27u8];
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
    /**Custom error with signature `UnsupportedDegree()` and selector `0xe2ef09e5`.
    ```solidity
    error UnsupportedDegree();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct UnsupportedDegree {}
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
        impl ::core::convert::From<UnsupportedDegree> for UnderlyingRustTuple<'_> {
            fn from(value: UnsupportedDegree) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for UnsupportedDegree {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for UnsupportedDegree {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "UnsupportedDegree()";
            const SELECTOR: [u8; 4] = [226u8, 239u8, 9u8, 229u8];
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
    /**Custom error with signature `WrongPlonkVK()` and selector `0x41f53b12`.
    ```solidity
    error WrongPlonkVK();
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct WrongPlonkVK {}
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
        impl ::core::convert::From<WrongPlonkVK> for UnderlyingRustTuple<'_> {
            fn from(value: WrongPlonkVK) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for WrongPlonkVK {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for WrongPlonkVK {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "WrongPlonkVK()";
            const SELECTOR: [u8; 4] = [65u8, 245u8, 59u8, 18u8];
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
    /**Function with signature `BETA_H_X0()` and selector `0x834c452a`.
    ```solidity
    function BETA_H_X0() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X0Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`BETA_H_X0()`](BETA_H_X0Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X0Return {
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
            impl ::core::convert::From<BETA_H_X0Call> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_X0Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_X0Call {
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
            impl ::core::convert::From<BETA_H_X0Return> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_X0Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_X0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for BETA_H_X0Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = BETA_H_X0Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BETA_H_X0()";
            const SELECTOR: [u8; 4] = [131u8, 76u8, 69u8, 42u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `BETA_H_X1()` and selector `0xaf196ba2`.
    ```solidity
    function BETA_H_X1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X1Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`BETA_H_X1()`](BETA_H_X1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X1Return {
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
            impl ::core::convert::From<BETA_H_X1Call> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_X1Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_X1Call {
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
            impl ::core::convert::From<BETA_H_X1Return> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_X1Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_X1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for BETA_H_X1Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = BETA_H_X1Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BETA_H_X1()";
            const SELECTOR: [u8; 4] = [175u8, 25u8, 107u8, 162u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `BETA_H_Y0()` and selector `0xf5144326`.
    ```solidity
    function BETA_H_Y0() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y0Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`BETA_H_Y0()`](BETA_H_Y0Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y0Return {
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
            impl ::core::convert::From<BETA_H_Y0Call> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_Y0Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_Y0Call {
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
            impl ::core::convert::From<BETA_H_Y0Return> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_Y0Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_Y0Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for BETA_H_Y0Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = BETA_H_Y0Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BETA_H_Y0()";
            const SELECTOR: [u8; 4] = [245u8, 20u8, 67u8, 38u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `BETA_H_Y1()` and selector `0x4b4734e3`.
    ```solidity
    function BETA_H_Y1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y1Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`BETA_H_Y1()`](BETA_H_Y1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y1Return {
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
            impl ::core::convert::From<BETA_H_Y1Call> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_Y1Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_Y1Call {
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
            impl ::core::convert::From<BETA_H_Y1Return> for UnderlyingRustTuple<'_> {
                fn from(value: BETA_H_Y1Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for BETA_H_Y1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for BETA_H_Y1Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = BETA_H_Y1Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "BETA_H_Y1()";
            const SELECTOR: [u8; 4] = [75u8, 71u8, 52u8, 227u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `COSET_K1()` and selector `0xe3512d56`.
    ```solidity
    function COSET_K1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K1Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`COSET_K1()`](COSET_K1Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K1Return {
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
            impl ::core::convert::From<COSET_K1Call> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K1Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K1Call {
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
            impl ::core::convert::From<COSET_K1Return> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K1Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K1Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for COSET_K1Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = COSET_K1Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "COSET_K1()";
            const SELECTOR: [u8; 4] = [227u8, 81u8, 45u8, 86u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `COSET_K2()` and selector `0x0c551f3f`.
    ```solidity
    function COSET_K2() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K2Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`COSET_K2()`](COSET_K2Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K2Return {
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
            impl ::core::convert::From<COSET_K2Call> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K2Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K2Call {
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
            impl ::core::convert::From<COSET_K2Return> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K2Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K2Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for COSET_K2Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = COSET_K2Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "COSET_K2()";
            const SELECTOR: [u8; 4] = [12u8, 85u8, 31u8, 63u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `COSET_K3()` and selector `0x5a14c0fe`.
    ```solidity
    function COSET_K3() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K3Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`COSET_K3()`](COSET_K3Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K3Return {
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
            impl ::core::convert::From<COSET_K3Call> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K3Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K3Call {
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
            impl ::core::convert::From<COSET_K3Return> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K3Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K3Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for COSET_K3Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = COSET_K3Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "COSET_K3()";
            const SELECTOR: [u8; 4] = [90u8, 20u8, 192u8, 254u8];
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `COSET_K4()` and selector `0xde24ac0f`.
    ```solidity
    function COSET_K4() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K4Call {}
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`COSET_K4()`](COSET_K4Call) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K4Return {
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
            impl ::core::convert::From<COSET_K4Call> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K4Call) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K4Call {
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
            impl ::core::convert::From<COSET_K4Return> for UnderlyingRustTuple<'_> {
                fn from(value: COSET_K4Return) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for COSET_K4Return {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for COSET_K4Call {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = COSET_K4Return;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "COSET_K4()";
            const SELECTOR: [u8; 4] = [222u8, 36u8, 172u8, 15u8];
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
    #[derive()]
    /**Function with signature `verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[11],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0xab959ee3`.
    ```solidity
    function verify(IPlonkVerifier.VerifyingKey memory verifyingKey, uint256[11] memory publicInput, IPlonkVerifier.PlonkProof memory proof) external view returns (bool);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyCall {
        #[allow(missing_docs)]
        pub verifyingKey: <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub publicInput: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
        #[allow(missing_docs)]
        pub proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[11],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](verifyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyReturn {
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
                IPlonkVerifier::VerifyingKey,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
                IPlonkVerifier::PlonkProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
                [alloy::sol_types::private::primitives::aliases::U256; 11usize],
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
            impl ::core::convert::From<verifyCall> for UnderlyingRustTuple<'_> {
                fn from(value: verifyCall) -> Self {
                    (value.verifyingKey, value.publicInput, value.proof)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for verifyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        verifyingKey: tuple.0,
                        publicInput: tuple.1,
                        proof: tuple.2,
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
            impl ::core::convert::From<verifyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: verifyReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for verifyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for verifyCall {
            type Parameters<'a> = (
                IPlonkVerifier::VerifyingKey,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = verifyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[11],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [171u8, 149u8, 158u8, 227u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <IPlonkVerifier::VerifyingKey as alloy_sol_types::SolType>::tokenize(
                        &self.verifyingKey,
                    ),
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::SolType>::tokenize(&self.publicInput),
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
    ///Container for all the [`PlonkVerifierV2`](self) function calls.
    #[derive()]
    pub enum PlonkVerifierV2Calls {
        #[allow(missing_docs)]
        BETA_H_X0(BETA_H_X0Call),
        #[allow(missing_docs)]
        BETA_H_X1(BETA_H_X1Call),
        #[allow(missing_docs)]
        BETA_H_Y0(BETA_H_Y0Call),
        #[allow(missing_docs)]
        BETA_H_Y1(BETA_H_Y1Call),
        #[allow(missing_docs)]
        COSET_K1(COSET_K1Call),
        #[allow(missing_docs)]
        COSET_K2(COSET_K2Call),
        #[allow(missing_docs)]
        COSET_K3(COSET_K3Call),
        #[allow(missing_docs)]
        COSET_K4(COSET_K4Call),
        #[allow(missing_docs)]
        verify(verifyCall),
    }
    #[automatically_derived]
    impl PlonkVerifierV2Calls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [12u8, 85u8, 31u8, 63u8],
            [75u8, 71u8, 52u8, 227u8],
            [90u8, 20u8, 192u8, 254u8],
            [131u8, 76u8, 69u8, 42u8],
            [171u8, 149u8, 158u8, 227u8],
            [175u8, 25u8, 107u8, 162u8],
            [222u8, 36u8, 172u8, 15u8],
            [227u8, 81u8, 45u8, 86u8],
            [245u8, 20u8, 67u8, 38u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PlonkVerifierV2Calls {
        const NAME: &'static str = "PlonkVerifierV2Calls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 9usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::BETA_H_X0(_) => <BETA_H_X0Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::BETA_H_X1(_) => <BETA_H_X1Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::BETA_H_Y0(_) => <BETA_H_Y0Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::BETA_H_Y1(_) => <BETA_H_Y1Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::COSET_K1(_) => <COSET_K1Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::COSET_K2(_) => <COSET_K2Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::COSET_K3(_) => <COSET_K3Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::COSET_K4(_) => <COSET_K4Call as alloy_sol_types::SolCall>::SELECTOR,
                Self::verify(_) => <verifyCall as alloy_sol_types::SolCall>::SELECTOR,
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
                -> alloy_sol_types::Result<PlonkVerifierV2Calls>] = &[
                {
                    fn COSET_K2(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <COSET_K2Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::COSET_K2)
                    }
                    COSET_K2
                },
                {
                    fn BETA_H_Y1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <BETA_H_Y1Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::BETA_H_Y1)
                    }
                    BETA_H_Y1
                },
                {
                    fn COSET_K3(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <COSET_K3Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::COSET_K3)
                    }
                    COSET_K3
                },
                {
                    fn BETA_H_X0(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <BETA_H_X0Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::BETA_H_X0)
                    }
                    BETA_H_X0
                },
                {
                    fn verify(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <verifyCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::verify)
                    }
                    verify
                },
                {
                    fn BETA_H_X1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <BETA_H_X1Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::BETA_H_X1)
                    }
                    BETA_H_X1
                },
                {
                    fn COSET_K4(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <COSET_K4Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::COSET_K4)
                    }
                    COSET_K4
                },
                {
                    fn COSET_K1(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <COSET_K1Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::COSET_K1)
                    }
                    COSET_K1
                },
                {
                    fn BETA_H_Y0(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <BETA_H_Y0Call as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Calls::BETA_H_Y0)
                    }
                    BETA_H_Y0
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
                Self::BETA_H_X0(inner) => {
                    <BETA_H_X0Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::BETA_H_X1(inner) => {
                    <BETA_H_X1Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::BETA_H_Y0(inner) => {
                    <BETA_H_Y0Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::BETA_H_Y1(inner) => {
                    <BETA_H_Y1Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::COSET_K1(inner) => {
                    <COSET_K1Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::COSET_K2(inner) => {
                    <COSET_K2Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::COSET_K3(inner) => {
                    <COSET_K3Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::COSET_K4(inner) => {
                    <COSET_K4Call as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::verify(inner) => {
                    <verifyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::BETA_H_X0(inner) => {
                    <BETA_H_X0Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::BETA_H_X1(inner) => {
                    <BETA_H_X1Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::BETA_H_Y0(inner) => {
                    <BETA_H_Y0Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::BETA_H_Y1(inner) => {
                    <BETA_H_Y1Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::COSET_K1(inner) => {
                    <COSET_K1Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::COSET_K2(inner) => {
                    <COSET_K2Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::COSET_K3(inner) => {
                    <COSET_K3Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::COSET_K4(inner) => {
                    <COSET_K4Call as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::verify(inner) => {
                    <verifyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`PlonkVerifierV2`](self) custom errors.
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum PlonkVerifierV2Errors {
        #[allow(missing_docs)]
        InvalidPlonkArgs(InvalidPlonkArgs),
        #[allow(missing_docs)]
        UnsupportedDegree(UnsupportedDegree),
        #[allow(missing_docs)]
        WrongPlonkVK(WrongPlonkVK),
    }
    #[automatically_derived]
    impl PlonkVerifierV2Errors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [65u8, 245u8, 59u8, 18u8],
            [226u8, 239u8, 9u8, 229u8],
            [253u8, 154u8, 45u8, 27u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PlonkVerifierV2Errors {
        const NAME: &'static str = "PlonkVerifierV2Errors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 3usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::InvalidPlonkArgs(_) => {
                    <InvalidPlonkArgs as alloy_sol_types::SolError>::SELECTOR
                },
                Self::UnsupportedDegree(_) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::SELECTOR
                },
                Self::WrongPlonkVK(_) => <WrongPlonkVK as alloy_sol_types::SolError>::SELECTOR,
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
                -> alloy_sol_types::Result<PlonkVerifierV2Errors>] = &[
                {
                    fn WrongPlonkVK(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Errors> {
                        <WrongPlonkVK as alloy_sol_types::SolError>::abi_decode_raw(data, validate)
                            .map(PlonkVerifierV2Errors::WrongPlonkVK)
                    }
                    WrongPlonkVK
                },
                {
                    fn UnsupportedDegree(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Errors> {
                        <UnsupportedDegree as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Errors::UnsupportedDegree)
                    }
                    UnsupportedDegree
                },
                {
                    fn InvalidPlonkArgs(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Errors> {
                        <InvalidPlonkArgs as alloy_sol_types::SolError>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Errors::InvalidPlonkArgs)
                    }
                    InvalidPlonkArgs
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
                Self::InvalidPlonkArgs(inner) => {
                    <InvalidPlonkArgs as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::UnsupportedDegree(inner) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
                Self::WrongPlonkVK(inner) => {
                    <WrongPlonkVK as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::InvalidPlonkArgs(inner) => {
                    <InvalidPlonkArgs as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::UnsupportedDegree(inner) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
                Self::WrongPlonkVK(inner) => {
                    <WrongPlonkVK as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`PlonkVerifierV2`](self) contract instance.

    See the [wrapper's documentation](`PlonkVerifierV2Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> PlonkVerifierV2Instance<T, P, N> {
        PlonkVerifierV2Instance::<T, P, N>::new(address, provider)
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
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<PlonkVerifierV2Instance<T, P, N>>>
    {
        PlonkVerifierV2Instance::<T, P, N>::deploy(provider)
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
        PlonkVerifierV2Instance::<T, P, N>::deploy_builder(provider)
    }
    /**A [`PlonkVerifierV2`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`PlonkVerifierV2`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct PlonkVerifierV2Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for PlonkVerifierV2Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("PlonkVerifierV2Instance")
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
        > PlonkVerifierV2Instance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`PlonkVerifierV2`](self) contract instance.

        See the [wrapper's documentation](`PlonkVerifierV2Instance`) for more details.*/
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
        ) -> alloy_contract::Result<PlonkVerifierV2Instance<T, P, N>> {
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
    impl<T, P: ::core::clone::Clone, N> PlonkVerifierV2Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> PlonkVerifierV2Instance<T, P, N> {
            PlonkVerifierV2Instance {
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
        > PlonkVerifierV2Instance<T, P, N>
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
        ///Creates a new call builder for the [`BETA_H_X0`] function.
        pub fn BETA_H_X0(&self) -> alloy_contract::SolCallBuilder<T, &P, BETA_H_X0Call, N> {
            self.call_builder(&BETA_H_X0Call {})
        }
        ///Creates a new call builder for the [`BETA_H_X1`] function.
        pub fn BETA_H_X1(&self) -> alloy_contract::SolCallBuilder<T, &P, BETA_H_X1Call, N> {
            self.call_builder(&BETA_H_X1Call {})
        }
        ///Creates a new call builder for the [`BETA_H_Y0`] function.
        pub fn BETA_H_Y0(&self) -> alloy_contract::SolCallBuilder<T, &P, BETA_H_Y0Call, N> {
            self.call_builder(&BETA_H_Y0Call {})
        }
        ///Creates a new call builder for the [`BETA_H_Y1`] function.
        pub fn BETA_H_Y1(&self) -> alloy_contract::SolCallBuilder<T, &P, BETA_H_Y1Call, N> {
            self.call_builder(&BETA_H_Y1Call {})
        }
        ///Creates a new call builder for the [`COSET_K1`] function.
        pub fn COSET_K1(&self) -> alloy_contract::SolCallBuilder<T, &P, COSET_K1Call, N> {
            self.call_builder(&COSET_K1Call {})
        }
        ///Creates a new call builder for the [`COSET_K2`] function.
        pub fn COSET_K2(&self) -> alloy_contract::SolCallBuilder<T, &P, COSET_K2Call, N> {
            self.call_builder(&COSET_K2Call {})
        }
        ///Creates a new call builder for the [`COSET_K3`] function.
        pub fn COSET_K3(&self) -> alloy_contract::SolCallBuilder<T, &P, COSET_K3Call, N> {
            self.call_builder(&COSET_K3Call {})
        }
        ///Creates a new call builder for the [`COSET_K4`] function.
        pub fn COSET_K4(&self) -> alloy_contract::SolCallBuilder<T, &P, COSET_K4Call, N> {
            self.call_builder(&COSET_K4Call {})
        }
        ///Creates a new call builder for the [`verify`] function.
        pub fn verify(
            &self,
            verifyingKey: <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
            publicInput: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
            proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, verifyCall, N> {
            self.call_builder(&verifyCall {
                verifyingKey,
                publicInput,
                proof,
            })
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
            T: alloy_contract::private::Transport + ::core::clone::Clone,
            P: alloy_contract::private::Provider<T, N>,
            N: alloy_contract::private::Network,
        > PlonkVerifierV2Instance<T, P, N>
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
