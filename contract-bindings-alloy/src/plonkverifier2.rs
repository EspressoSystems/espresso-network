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

interface PlonkVerifier2 {
    error UnsupportedDegree();

    function P_MOD() external view returns (uint256);
    function R_MOD() external view returns (uint256);
    function verify(IPlonkVerifier.VerifyingKey memory vk, uint256[7] memory publicInput, IPlonkVerifier.PlonkProof memory proof) external view returns (bool success);
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "function",
    "name": "P_MOD",
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
    "name": "R_MOD",
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
      },
      {
        "name": "publicInput",
        "type": "uint256[7]",
        "internalType": "uint256[7]"
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
        "name": "success",
        "type": "bool",
        "internalType": "bool"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "error",
    "name": "UnsupportedDegree",
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
pub mod PlonkVerifier2 {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x612620610035600b8282823980515f1a60731461002957634e487b7160e01b5f525f60045260245ffd5b305f52607381538281f3fe730000000000000000000000000000000000000000301460806040526004361061004a575f3560e01c80631d712e271461004e578063ce537a7714610088578063df6e6cb4146100ab575b5f80fd5b6100757f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4781565b6040519081526020015b60405180910390f35b61009b6100963660046123ed565b6100bf565b604051901515815260200161007f565b6100755f805160206125f483398151915281565b5f6100c982611012565b6100d9835f5b602002015161114d565b6100e48360016100cf565b6100ef8360026100cf565b6100fa8360036100cf565b6101058360046100cf565b6101108360056100cf565b61011b8360066100cf565b5f6101278585856111af565b90505f610136865f01516117b8565b90505f610148828460a0015188611b96565b90505f610156848784611bf3565b9050610235565b60405162461bcd60e51b815260206004820152600c60248201526b6572726f722076657269667960a01b6044820152606481fd5b60405162461bcd60e51b815260206004820152600d60248201526c6572726f722070616972696e6760981b6044820152606481fd5b604051815181526020820151602082015282604082015260405f60608360075afa9050806101f6576101f661015d565b505050565b6040805182518152602080840151818301525f5182840152516060820152908260808360065afa9050806102315761023161015d565b5050565b60405160c081017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c160408301527f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b060608301527f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe460808301527f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5560a08301527f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c26101008301527f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed6101208301527f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b6101408301527f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa61016083015261018082016040525f805f8060808a01515f805160206125f483398151915260208a015160208d01510993508a515f805160206125f483398151915260a08d015160608e01510993505f805160206125f48339815191526101a08f0151850892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a850992505f805160206125f48339815191526101c08f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb025850992505f805160206125f48339815191526101e08f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992505f805160206125f48339815191526102008f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e881850992505f805160206125f48339815191526102208f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f483398151915281860894505060a08d015194506105e084866101c6565b5f51865260205160208701525f805160206125f483398151915260608c01518c510993505f805160206125f48339815191526102c08e0151850993505f805160206125f48339815191526102408e015160608d01510992505f805160206125f48339815191526101a08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102608e015160608d01510992505f805160206125f48339815191526101c08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102808e015160608d01510992505f805160206125f48339815191526101e08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102a08e015160608d01510992505f805160206125f48339815191526102008e0151840892505f805160206125f48339815191528184089250505f805160206125f48339815191528284095f805160206125f483398151915203925060c08e015193506107a583856101c6565b6107ae856101fb565b6101a08c0151925060e08e015193506107c783856101c6565b6107d0856101fb565b6101c08c015192506101008e015193506107ea83856101c6565b6107f3856101fb565b6101e08c015192506101208e0151935061080d83856101c6565b610816856101fb565b6102008c015192506101408e0151935061083083856101c6565b610839856101fb565b5f805160206125f48339815191526101c08d01516101a08e01510992506101608e0151935061086883856101c6565b610871856101fb565b5f805160206125f48339815191526102008d01516101e08e01510992506101808e015193506108a083856101c6565b6108a9856101fb565b6101a08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506101e08e015193506108fc83856101c6565b610905856101fb565b6101c08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102008e0151935061095883856101c6565b610961856101fb565b6101e08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102208e015193506109b483856101c6565b6109bd856101fb565b6102008c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102408e01519350610a1083856101c6565b610a19856101fb565b6102208c01515f805160206125f48339815191520392506101a08e01519350610a4283856101c6565b610a4b856101fb565b600192506101c08e01519350610a6183856101c6565b610a6a856101fb565b5f805160206125f48339815191526101c08d01516101a08e01510991505f805160206125f48339815191526101e08d0151830991505f805160206125f48339815191526102008d0151830991505f805160206125f48339815191526102208d0151830992506102608e01519350610ae183856101c6565b610aea856101fb565b87515f805160206125f483398151915203925060c08c01519350610b0e83856101c6565b610b17856101fb565b5f805160206125f48339815191526001895108915060a08a015190505f805160206125f483398151915281820990505f805160206125f48339815191528183099150505f805160206125f4833981519152818309915060e08b01519250610b7e82846101c6565b610b87846101fb565b5f805160206125f483398151915281830991506101008b01519250610bac82846101c6565b610bb5846101fb565b5f805160206125f483398151915281830991506101208b01519250610bda82846101c6565b610be3846101fb565b5f805160206125f483398151915281830991506101408b01519250610c0882846101c6565b610c11846101fb565b50505060c0860151885190805f805160206125f4833981519152869003610c3882856101c6565b610c41856101fb565b5f805160206125f4833981519152806101a08e01518509820890505f805160206125f4833981519152828409925060208c01519350610c8083856101c6565b610c89856101fb565b5f805160206125f4833981519152806101c08e01518509820890505f805160206125f4833981519152828409925060408c01519350610cc883856101c6565b610cd1856101fb565b5f805160206125f4833981519152806101e08e01518509820890505f805160206125f4833981519152828409925060608c01519350610d1083856101c6565b610d19856101fb565b5f805160206125f4833981519152806102008e01518509820890505f805160206125f4833981519152828409925060808c01519350610d5883856101c6565b610d61856101fb565b5f805160206125f4833981519152806102208e01518509820890505f805160206125f4833981519152828409925060408e01519350610da083856101c6565b610da9856101fb565b5f805160206125f4833981519152806102408e01518509820890505f805160206125f4833981519152828409925060608e01519350610de883856101c6565b610df1856101fb565b5f805160206125f4833981519152806102608e01518509820890505f805160206125f4833981519152828409925060808e01519350610e3083856101c6565b610e39856101fb565b5f805160206125f4833981519152806102808e01518509820890505f805160206125f4833981519152828409925060a08e01519350610e7883856101c6565b610e81856101fb565b5f805160206125f4833981519152806102a08e015185098208905060e08a0151925060a08c01519350610eb483856101c6565b610ebd856101fb565b5f805160206125f4833981519152806102c08e015185098208905060a08a015192506101608c01519350610ef183856101c6565b610efa856101fb565b5f805160206125f4833981519152602060408b01510151840991505f805160206125f483398151915260e08b0151830992506101808c01519350610f3e83856101c6565b610f47856101fb565b6040805180820190915293506001845260026020850152610f77815f805160206125f483398151915203856101c6565b50610f81846101fb565b610fb08460200180517f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47039052565b505050610160880151805183526020908101519083015260e086015161018089015190610fdd81836101c6565b505050610fe9816101fb565b60205f6101808360085afa90508061100357611003610191565b50505f51979650505050505050565b805161101d90611da8565b61102a8160200151611da8565b6110378160400151611da8565b6110448160600151611da8565b6110518160800151611da8565b61105e8160a00151611da8565b61106b8160c00151611da8565b6110788160e00151611da8565b611086816101000151611da8565b611094816101200151611da8565b6110a2816101400151611da8565b6110b0816101600151611da8565b6110be816101800151611da8565b6110cc816101a0015161114d565b6110da816101c0015161114d565b6110e8816101e0015161114d565b6110f681610200015161114d565b61110481610220015161114d565b61111281610240015161114d565b61112081610260015161114d565b61112e81610280015161114d565b61113c816102a0015161114d565b61114a816102c0015161114d565b50565b5f805160206125f48339815191528110806102315760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c64000000000060448201526064015b60405180910390fd5b6111ef6040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b604051602081015f815260fe60e01b8152855160c01b6004820152602086015160c01b600c82015261028086015160208201526102a08601516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e086015180516101008301526020810151610120830152506101008601518051610140830152602081015161016083015250610120860151805161018083015260208101516101a08301525061014086015180516101c083015260208101516101e083015250610160860151805161020083015260208101516102208301525061018086015180516102408301526020810151610260830152506101e0860151805161028083015260208101516102a08301525061020086015180516102c083015260208101516102e083015250610220860151805161030083015260208101516103208301525061024086015180516103408301526020810151610360830152506101a0860151805161038083015260208101516103a0830152506101c086015180516103c083015260208101516103e0830152506102608601518051610400830152602081015161042083015250604086015180516104408301526020810151610460830152506060860151805161048083015260208101516104a083015250608086015180516104c083015260208101516104e08301525060a0860151805161050083015260208101516105208301525060c08601518051610540830152602081015161056083015250845161058082015260208501516105a082015260408501516105c082015260608501516105e0820152608085015161060082015260a085015161062082015260c085015161064082015260e085015161066082015283518051610660830152602081015161068083015250602084015180516106a083015260208101516106c083015250604084015180516106e083015260208101516107008301525060608401518051610720830152602081015161074083015250608084015180516107608301526020810151610780830152505f82526107c082206107c08201526107c0810191506020820190505f805160206125f483398151915282510660608401526020822081528091506020820190505f805160206125f4833981519152825106608084015260a0840151805182526020810151602083015250606082206040820192508083526020830191505f805160206125f4833981519152810684525f805160206125f48339815191528182095f805160206125f48339815191528282099150806020860152508060408501525060c084015180518252602081015160208301525060e084015180516040830152602081015160608301525061010084015180516080830152602081015160a083015250610120840151805160c0830152602081015160e08301525061014084015180516101008301526020810151610120830152506101608220610140820152610140810191506020820190505f805160206125f483398151915282510660a08401526101a084015181526101c084015160208201526101e084015160408201526102008401516060820152610220840151608082015261024084015160a082015261026084015160c082015261028084015160e08201526102a08401516101008201526102c08401516101208201526101608220610140820152610140810191506020820190505f805160206125f483398151915282510660c08401526101608401518051825260208101516020830152506101808401518051604083015260208101516060830152505060a081205f805160206125f4833981519152810660e084015250509392505050565b6117c0612104565b8162010000036118ff576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c100181526020016040518060e00160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd8152508152509050919050565b816210000003611a3f576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c10181526020016040518060e00160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c8152508152509050919050565b81602003611b7d576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e75080000181526020016040518060e00160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b0390438152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b611bb760405180606001604052805f81526020015f81526020015f81525090565b611bc18484611e51565b808252611bd19085908590611ea2565b60208201528051611be790859084908690611f13565b60408201529392505050565b60208101516040820151606085015160808601516101a08601516102408701515f959493600193909290915f805160206125f483398151915280808387095f805160206125f4833981519152868608088609945050506101c08801516102608901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f4833981519152868608088609945050506101e08801516102808901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f4833981519152868608088609945050506102008801516102a08901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f48339815191528686080886099450505061022088015191506102c08801515f805160206125f483398151915280825f805160206125f48339815191528587080985099350505050865160208801515f805160206125f4833981519152808683095f805160206125f48339815191520385089550505f805160206125f4833981519152808383095f805160206125f483398151915203860898975050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47911590151615611de157505050565b8251602084015182600384858586098509088382830914838210848410161693505050816101f65760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e7400000000000000000060448201526064016111a6565b81515f905f805160206125f483398151915290838015611e92578493505f5b82811015611e8657838586099450600101611e70565b50600184039350611e99565b6001830393505b50505092915050565b5f82600103611eb357506001611f0c565b815f03611ec157505f611f0c565b60208401515f805160206125f4833981519152905f90828186099050858015611eef57600187039250611ef6565b6001840392505b50611f0082612063565b91508282820993505050505b9392505050565b5f5f805160206125f4833981519152828203611f8c5760015f5b6007811015611f8157818603611f5e57868160078110611f4f57611f4f6125cb565b6020020151935050505061205b565b8280611f6c57611f6c6125df565b60408901516020015183099150600101611f2d565b505f9250505061205b565b611f94612128565b6040870151600160c0838101828152920190805b6007811015611fd55760208403935085868a85518903088309808552601f19909301929150600101611fa8565b505050505f805f90506001838960408c01515f5b6007811015612029578882518a85518c88518a0909098981880896505088898d84518c030886099450602093840193928301929190910190600101611fe9565b50505050809250505f61203b83612063565b905060208a01518581890996505084818709955084828709955050505050505b949350505050565b5f805f5f805160206125f4833981519152905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f519250816120fd5760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c65642100000060448201526064016111a6565b5050919050565b60405180606001604052805f81526020015f8152602001612123612128565b905290565b6040518060e001604052806007906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561217e5761217e612146565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561217e5761217e612146565b5f604082840312156121b8575f80fd5b6040516040810181811067ffffffffffffffff821117156121db576121db612146565b604052823581526020928301359281019290925250919050565b5f82601f830112612204575f80fd5b60405160e0810181811067ffffffffffffffff8211171561222757612227612146565b6040528060e084018581111561223b575f80fd5b845b8181101561225557803583526020928301920161223d565b509195945050505050565b5f6104808284031215612271575f80fd5b61227961215a565b905061228583836121a8565b815261229483604084016121a8565b60208201526122a683608084016121a8565b60408201526122b88360c084016121a8565b60608201526101006122cc848285016121a8565b60808301526101406122e0858286016121a8565b60a08401526101806122f4868287016121a8565b60c08501526101c0612308878288016121a8565b60e086015261020061231c888289016121a8565b858701526102409450612331888689016121a8565b61012087015261028061234689828a016121a8565b858801526102c0945061235b89868a016121a8565b61016088015261236f896103008a016121a8565b848801526103408801356101a0880152610360880135838801526103808801356101e08801526103a0880135828801526103c08801356102208801526103e08801358688015261040088013561026088015261042088013581880152505050506104408401356102a084015261046084013581840152505092915050565b5f805f838503610a60811215612401575f80fd5b61050080821215612410575f80fd5b612418612184565b9150853582526020860135602083015261243587604088016121a8565b604083015261244787608088016121a8565b60608301526124598760c088016121a8565b608083015261010061246d888289016121a8565b60a084015261014061248189828a016121a8565b60c08501526101806124958a828b016121a8565b60e08601526101c06124a98b828c016121a8565b8487015261020093506124be8b858c016121a8565b6101208701526102406124d38c828d016121a8565b8488015261028093506124e88c858d016121a8565b6101608801526124fc8c6102c08d016121a8565b8388015261250e8c6103008d016121a8565b6101a08801526125228c6103408d016121a8565b828801526125348c6103808d016121a8565b6101e08801526125488c6103c08d016121a8565b8588015261255a8c6104008d016121a8565b61022088015261256e8c6104408d016121a8565b81880152505050612583896104808a016121a8565b6102608501526104c08801358185015250506104e08601356102a08301528194506125b0878288016121f5565b935050506125c2856105e08601612260565b90509250925092565b634e487b7160e01b5f52603260045260245ffd5b634e487b7160e01b5f52601260045260245ffdfe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"a& a\x005`\x0B\x82\x82\x829\x80Q_\x1A`s\x14a\0)WcNH{q`\xE0\x1B_R_`\x04R`$_\xFD[0_R`s\x81S\x82\x81\xF3\xFEs\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x000\x14`\x80`@R`\x046\x10a\0JW_5`\xE0\x1C\x80c\x1Dq.'\x14a\0NW\x80c\xCESzw\x14a\0\x88W\x80c\xDFnl\xB4\x14a\0\xABW[_\x80\xFD[a\0u\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\x9Ba\0\x966`\x04a#\xEDV[a\0\xBFV[`@Q\x90\x15\x15\x81R` \x01a\0\x7FV[a\0u_\x80Q` a%\xF4\x839\x81Q\x91R\x81V[_a\0\xC9\x82a\x10\x12V[a\0\xD9\x83_[` \x02\x01Qa\x11MV[a\0\xE4\x83`\x01a\0\xCFV[a\0\xEF\x83`\x02a\0\xCFV[a\0\xFA\x83`\x03a\0\xCFV[a\x01\x05\x83`\x04a\0\xCFV[a\x01\x10\x83`\x05a\0\xCFV[a\x01\x1B\x83`\x06a\0\xCFV[_a\x01'\x85\x85\x85a\x11\xAFV[\x90P_a\x016\x86_\x01Qa\x17\xB8V[\x90P_a\x01H\x82\x84`\xA0\x01Q\x88a\x1B\x96V[\x90P_a\x01V\x84\x87\x84a\x1B\xF3V[\x90Pa\x025V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0C`$\x82\x01Rkerror verify`\xA0\x1B`D\x82\x01R`d\x81\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\r`$\x82\x01Rlerror pairing`\x98\x1B`D\x82\x01R`d\x81\xFD[`@Q\x81Q\x81R` \x82\x01Q` \x82\x01R\x82`@\x82\x01R`@_``\x83`\x07Z\xFA\x90P\x80a\x01\xF6Wa\x01\xF6a\x01]V[PPPV[`@\x80Q\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R_Q\x82\x84\x01RQ``\x82\x01R\x90\x82`\x80\x83`\x06Z\xFA\x90P\x80a\x021Wa\x021a\x01]V[PPV[`@Q`\xC0\x81\x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1`@\x83\x01R\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0``\x83\x01R\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4`\x80\x83\x01R\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U`\xA0\x83\x01R\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2a\x01\0\x83\x01R\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xEDa\x01 \x83\x01R\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[a\x01@\x83\x01R\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAAa\x01`\x83\x01Ra\x01\x80\x82\x01`@R_\x80_\x80`\x80\x8A\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R` \x8A\x01Q` \x8D\x01Q\t\x93P\x8AQ_\x80Q` a%\xF4\x839\x81Q\x91R`\xA0\x8D\x01Q``\x8E\x01Q\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xA0\x8F\x01Q\x85\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02 \x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x86\x08\x94PP`\xA0\x8D\x01Q\x94Pa\x05\xE0\x84\x86a\x01\xC6V[_Q\x86R` Q` \x87\x01R_\x80Q` a%\xF4\x839\x81Q\x91R``\x8C\x01Q\x8CQ\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\xC0\x8E\x01Q\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02@\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xA0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02`\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\x80\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\xA0\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92PP_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92P`\xC0\x8E\x01Q\x93Pa\x07\xA5\x83\x85a\x01\xC6V[a\x07\xAE\x85a\x01\xFBV[a\x01\xA0\x8C\x01Q\x92P`\xE0\x8E\x01Q\x93Pa\x07\xC7\x83\x85a\x01\xC6V[a\x07\xD0\x85a\x01\xFBV[a\x01\xC0\x8C\x01Q\x92Pa\x01\0\x8E\x01Q\x93Pa\x07\xEA\x83\x85a\x01\xC6V[a\x07\xF3\x85a\x01\xFBV[a\x01\xE0\x8C\x01Q\x92Pa\x01 \x8E\x01Q\x93Pa\x08\r\x83\x85a\x01\xC6V[a\x08\x16\x85a\x01\xFBV[a\x02\0\x8C\x01Q\x92Pa\x01@\x8E\x01Q\x93Pa\x080\x83\x85a\x01\xC6V[a\x089\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8D\x01Qa\x01\xA0\x8E\x01Q\t\x92Pa\x01`\x8E\x01Q\x93Pa\x08h\x83\x85a\x01\xC6V[a\x08q\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8D\x01Qa\x01\xE0\x8E\x01Q\t\x92Pa\x01\x80\x8E\x01Q\x93Pa\x08\xA0\x83\x85a\x01\xC6V[a\x08\xA9\x85a\x01\xFBV[a\x01\xA0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x01\xE0\x8E\x01Q\x93Pa\x08\xFC\x83\x85a\x01\xC6V[a\t\x05\x85a\x01\xFBV[a\x01\xC0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02\0\x8E\x01Q\x93Pa\tX\x83\x85a\x01\xC6V[a\ta\x85a\x01\xFBV[a\x01\xE0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02 \x8E\x01Q\x93Pa\t\xB4\x83\x85a\x01\xC6V[a\t\xBD\x85a\x01\xFBV[a\x02\0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02@\x8E\x01Q\x93Pa\n\x10\x83\x85a\x01\xC6V[a\n\x19\x85a\x01\xFBV[a\x02 \x8C\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92Pa\x01\xA0\x8E\x01Q\x93Pa\nB\x83\x85a\x01\xC6V[a\nK\x85a\x01\xFBV[`\x01\x92Pa\x01\xC0\x8E\x01Q\x93Pa\na\x83\x85a\x01\xC6V[a\nj\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8D\x01Qa\x01\xA0\x8E\x01Q\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8D\x01Q\x83\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8D\x01Q\x83\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02 \x8D\x01Q\x83\t\x92Pa\x02`\x8E\x01Q\x93Pa\n\xE1\x83\x85a\x01\xC6V[a\n\xEA\x85a\x01\xFBV[\x87Q_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92P`\xC0\x8C\x01Q\x93Pa\x0B\x0E\x83\x85a\x01\xC6V[a\x0B\x17\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R`\x01\x89Q\x08\x91P`\xA0\x8A\x01Q\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91PP_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91P`\xE0\x8B\x01Q\x92Pa\x0B~\x82\x84a\x01\xC6V[a\x0B\x87\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01\0\x8B\x01Q\x92Pa\x0B\xAC\x82\x84a\x01\xC6V[a\x0B\xB5\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01 \x8B\x01Q\x92Pa\x0B\xDA\x82\x84a\x01\xC6V[a\x0B\xE3\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01@\x8B\x01Q\x92Pa\x0C\x08\x82\x84a\x01\xC6V[a\x0C\x11\x84a\x01\xFBV[PPP`\xC0\x86\x01Q\x88Q\x90\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x90\x03a\x0C8\x82\x85a\x01\xC6V[a\x0CA\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xA0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P` \x8C\x01Q\x93Pa\x0C\x80\x83\x85a\x01\xC6V[a\x0C\x89\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xC0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`@\x8C\x01Q\x93Pa\x0C\xC8\x83\x85a\x01\xC6V[a\x0C\xD1\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xE0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P``\x8C\x01Q\x93Pa\r\x10\x83\x85a\x01\xC6V[a\r\x19\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\x80\x8C\x01Q\x93Pa\rX\x83\x85a\x01\xC6V[a\ra\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02 \x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`@\x8E\x01Q\x93Pa\r\xA0\x83\x85a\x01\xC6V[a\r\xA9\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02@\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P``\x8E\x01Q\x93Pa\r\xE8\x83\x85a\x01\xC6V[a\r\xF1\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02`\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\x80\x8E\x01Q\x93Pa\x0E0\x83\x85a\x01\xC6V[a\x0E9\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\x80\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\xA0\x8E\x01Q\x93Pa\x0Ex\x83\x85a\x01\xC6V[a\x0E\x81\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\xA0\x8E\x01Q\x85\t\x82\x08\x90P`\xE0\x8A\x01Q\x92P`\xA0\x8C\x01Q\x93Pa\x0E\xB4\x83\x85a\x01\xC6V[a\x0E\xBD\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\xC0\x8E\x01Q\x85\t\x82\x08\x90P`\xA0\x8A\x01Q\x92Pa\x01`\x8C\x01Q\x93Pa\x0E\xF1\x83\x85a\x01\xC6V[a\x0E\xFA\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R` `@\x8B\x01Q\x01Q\x84\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91R`\xE0\x8B\x01Q\x83\t\x92Pa\x01\x80\x8C\x01Q\x93Pa\x0F>\x83\x85a\x01\xC6V[a\x0FG\x85a\x01\xFBV[`@\x80Q\x80\x82\x01\x90\x91R\x93P`\x01\x84R`\x02` \x85\x01Ra\x0Fw\x81_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x85a\x01\xC6V[Pa\x0F\x81\x84a\x01\xFBV[a\x0F\xB0\x84` \x01\x80Q\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x03\x90RV[PPPa\x01`\x88\x01Q\x80Q\x83R` \x90\x81\x01Q\x90\x83\x01R`\xE0\x86\x01Qa\x01\x80\x89\x01Q\x90a\x0F\xDD\x81\x83a\x01\xC6V[PPPa\x0F\xE9\x81a\x01\xFBV[` _a\x01\x80\x83`\x08Z\xFA\x90P\x80a\x10\x03Wa\x10\x03a\x01\x91V[PP_Q\x97\x96PPPPPPPV[\x80Qa\x10\x1D\x90a\x1D\xA8V[a\x10*\x81` \x01Qa\x1D\xA8V[a\x107\x81`@\x01Qa\x1D\xA8V[a\x10D\x81``\x01Qa\x1D\xA8V[a\x10Q\x81`\x80\x01Qa\x1D\xA8V[a\x10^\x81`\xA0\x01Qa\x1D\xA8V[a\x10k\x81`\xC0\x01Qa\x1D\xA8V[a\x10x\x81`\xE0\x01Qa\x1D\xA8V[a\x10\x86\x81a\x01\0\x01Qa\x1D\xA8V[a\x10\x94\x81a\x01 \x01Qa\x1D\xA8V[a\x10\xA2\x81a\x01@\x01Qa\x1D\xA8V[a\x10\xB0\x81a\x01`\x01Qa\x1D\xA8V[a\x10\xBE\x81a\x01\x80\x01Qa\x1D\xA8V[a\x10\xCC\x81a\x01\xA0\x01Qa\x11MV[a\x10\xDA\x81a\x01\xC0\x01Qa\x11MV[a\x10\xE8\x81a\x01\xE0\x01Qa\x11MV[a\x10\xF6\x81a\x02\0\x01Qa\x11MV[a\x11\x04\x81a\x02 \x01Qa\x11MV[a\x11\x12\x81a\x02@\x01Qa\x11MV[a\x11 \x81a\x02`\x01Qa\x11MV[a\x11.\x81a\x02\x80\x01Qa\x11MV[a\x11<\x81a\x02\xA0\x01Qa\x11MV[a\x11J\x81a\x02\xC0\x01Qa\x11MV[PV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x10\x80a\x021W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[a\x11\xEF`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x85Q`\xC0\x1B`\x04\x82\x01R` \x86\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x86\x01Q` \x82\x01Ra\x02\xA0\x86\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x86\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x86\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x86\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x86\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x86\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x86\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x86\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x86\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x86\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x86\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x86\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x86\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x86\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x86\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x86\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x86\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x86\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x86\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x84Qa\x05\x80\x82\x01R` \x85\x01Qa\x05\xA0\x82\x01R`@\x85\x01Qa\x05\xC0\x82\x01R``\x85\x01Qa\x05\xE0\x82\x01R`\x80\x85\x01Qa\x06\0\x82\x01R`\xA0\x85\x01Qa\x06 \x82\x01R`\xC0\x85\x01Qa\x06@\x82\x01R`\xE0\x85\x01Qa\x06`\x82\x01R\x83Q\x80Qa\x06`\x83\x01R` \x81\x01Qa\x06\x80\x83\x01RP` \x84\x01Q\x80Qa\x06\xA0\x83\x01R` \x81\x01Qa\x06\xC0\x83\x01RP`@\x84\x01Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP``\x84\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`\x80\x84\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP_\x82Ra\x07\xC0\x82 a\x07\xC0\x82\x01Ra\x07\xC0\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06``\x84\x01R` \x82 \x81R\x80\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\x80\x84\x01R`\xA0\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 `@\x82\x01\x92P\x80\x83R` \x83\x01\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x06\x84R_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x82\t\x91P\x80` \x86\x01RP\x80`@\x85\x01RP`\xC0\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x84\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x84\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x84\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x84\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 a\x01@\x82\x01Ra\x01@\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\xA0\x84\x01Ra\x01\xA0\x84\x01Q\x81Ra\x01\xC0\x84\x01Q` \x82\x01Ra\x01\xE0\x84\x01Q`@\x82\x01Ra\x02\0\x84\x01Q``\x82\x01Ra\x02 \x84\x01Q`\x80\x82\x01Ra\x02@\x84\x01Q`\xA0\x82\x01Ra\x02`\x84\x01Q`\xC0\x82\x01Ra\x02\x80\x84\x01Q`\xE0\x82\x01Ra\x02\xA0\x84\x01Qa\x01\0\x82\x01Ra\x02\xC0\x84\x01Qa\x01 \x82\x01Ra\x01`\x82 a\x01@\x82\x01Ra\x01@\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\xC0\x84\x01Ra\x01`\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x84\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 _\x80Q` a%\xF4\x839\x81Q\x91R\x81\x06`\xE0\x84\x01RPP\x93\x92PPPV[a\x17\xC0a!\x04V[\x81b\x01\0\0\x03a\x18\xFFW`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x1A?W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\x1B}W`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1B\xB7`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\x1B\xC1\x84\x84a\x1EQV[\x80\x82Ra\x1B\xD1\x90\x85\x90\x85\x90a\x1E\xA2V[` \x82\x01R\x80Qa\x1B\xE7\x90\x85\x90\x84\x90\x86\x90a\x1F\x13V[`@\x82\x01R\x93\x92PPPV[` \x81\x01Q`@\x82\x01Q``\x85\x01Q`\x80\x86\x01Qa\x01\xA0\x86\x01Qa\x02@\x87\x01Q_\x95\x94\x93`\x01\x93\x90\x92\x90\x91_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x80\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x88\x01Qa\x02`\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x88\x01Qa\x02\x80\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x88\x01Qa\x02\xA0\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x88\x01Q\x91Pa\x02\xC0\x88\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x82_\x80Q` a%\xF4\x839\x81Q\x91R\x85\x87\x08\t\x85\t\x93PPPP\x86Q` \x88\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x86\x83\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x85\x08\x95PP_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x83\x83\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x86\x08\x98\x97PPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x1D\xE1WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x01\xF6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x11\xA6V[\x81Q_\x90_\x80Q` a%\xF4\x839\x81Q\x91R\x90\x83\x80\x15a\x1E\x92W\x84\x93P_[\x82\x81\x10\x15a\x1E\x86W\x83\x85\x86\t\x94P`\x01\x01a\x1EpV[P`\x01\x84\x03\x93Pa\x1E\x99V[`\x01\x83\x03\x93P[PPP\x92\x91PPV[_\x82`\x01\x03a\x1E\xB3WP`\x01a\x1F\x0CV[\x81_\x03a\x1E\xC1WP_a\x1F\x0CV[` \x84\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x1E\xEFW`\x01\x87\x03\x92Pa\x1E\xF6V[`\x01\x84\x03\x92P[Pa\x1F\0\x82a cV[\x91P\x82\x82\x82\t\x93PPPP[\x93\x92PPPV[__\x80Q` a%\xF4\x839\x81Q\x91R\x82\x82\x03a\x1F\x8CW`\x01_[`\x07\x81\x10\x15a\x1F\x81W\x81\x86\x03a\x1F^W\x86\x81`\x07\x81\x10a\x1FOWa\x1FOa%\xCBV[` \x02\x01Q\x93PPPPa [V[\x82\x80a\x1FlWa\x1Fla%\xDFV[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\x1F-V[P_\x92PPPa [V[a\x1F\x94a!(V[`@\x87\x01Q`\x01`\xC0\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x07\x81\x10\x15a\x1F\xD5W` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\x1F\xA8V[PPPP_\x80_\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x07\x81\x10\x15a )W\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x1F\xE9V[PPPP\x80\x92PP_a ;\x83a cV[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x95PPPPPP[\x94\x93PPPPV[_\x80__\x80Q` a%\xF4\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a \xFDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01a\x11\xA6V[PP\x91\x90PV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!#a!(V[\x90R\x90V[`@Q\x80`\xE0\x01`@R\x80`\x07\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!~Wa!~a!FV[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!~Wa!~a!FV[_`@\x82\x84\x03\x12\x15a!\xB8W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a!\xDBWa!\xDBa!FV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x82`\x1F\x83\x01\x12a\"\x04W_\x80\xFD[`@Q`\xE0\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"'Wa\"'a!FV[`@R\x80`\xE0\x84\x01\x85\x81\x11\x15a\";W_\x80\xFD[\x84[\x81\x81\x10\x15a\"UW\x805\x83R` \x92\x83\x01\x92\x01a\"=V[P\x91\x95\x94PPPPPV[_a\x04\x80\x82\x84\x03\x12\x15a\"qW_\x80\xFD[a\"ya!ZV[\x90Pa\"\x85\x83\x83a!\xA8V[\x81Ra\"\x94\x83`@\x84\x01a!\xA8V[` \x82\x01Ra\"\xA6\x83`\x80\x84\x01a!\xA8V[`@\x82\x01Ra\"\xB8\x83`\xC0\x84\x01a!\xA8V[``\x82\x01Ra\x01\0a\"\xCC\x84\x82\x85\x01a!\xA8V[`\x80\x83\x01Ra\x01@a\"\xE0\x85\x82\x86\x01a!\xA8V[`\xA0\x84\x01Ra\x01\x80a\"\xF4\x86\x82\x87\x01a!\xA8V[`\xC0\x85\x01Ra\x01\xC0a#\x08\x87\x82\x88\x01a!\xA8V[`\xE0\x86\x01Ra\x02\0a#\x1C\x88\x82\x89\x01a!\xA8V[\x85\x87\x01Ra\x02@\x94Pa#1\x88\x86\x89\x01a!\xA8V[a\x01 \x87\x01Ra\x02\x80a#F\x89\x82\x8A\x01a!\xA8V[\x85\x88\x01Ra\x02\xC0\x94Pa#[\x89\x86\x8A\x01a!\xA8V[a\x01`\x88\x01Ra#o\x89a\x03\0\x8A\x01a!\xA8V[\x84\x88\x01Ra\x03@\x88\x015a\x01\xA0\x88\x01Ra\x03`\x88\x015\x83\x88\x01Ra\x03\x80\x88\x015a\x01\xE0\x88\x01Ra\x03\xA0\x88\x015\x82\x88\x01Ra\x03\xC0\x88\x015a\x02 \x88\x01Ra\x03\xE0\x88\x015\x86\x88\x01Ra\x04\0\x88\x015a\x02`\x88\x01Ra\x04 \x88\x015\x81\x88\x01RPPPPa\x04@\x84\x015a\x02\xA0\x84\x01Ra\x04`\x84\x015\x81\x84\x01RPP\x92\x91PPV[_\x80_\x83\x85\x03a\n`\x81\x12\x15a$\x01W_\x80\xFD[a\x05\0\x80\x82\x12\x15a$\x10W_\x80\xFD[a$\x18a!\x84V[\x91P\x855\x82R` \x86\x015` \x83\x01Ra$5\x87`@\x88\x01a!\xA8V[`@\x83\x01Ra$G\x87`\x80\x88\x01a!\xA8V[``\x83\x01Ra$Y\x87`\xC0\x88\x01a!\xA8V[`\x80\x83\x01Ra\x01\0a$m\x88\x82\x89\x01a!\xA8V[`\xA0\x84\x01Ra\x01@a$\x81\x89\x82\x8A\x01a!\xA8V[`\xC0\x85\x01Ra\x01\x80a$\x95\x8A\x82\x8B\x01a!\xA8V[`\xE0\x86\x01Ra\x01\xC0a$\xA9\x8B\x82\x8C\x01a!\xA8V[\x84\x87\x01Ra\x02\0\x93Pa$\xBE\x8B\x85\x8C\x01a!\xA8V[a\x01 \x87\x01Ra\x02@a$\xD3\x8C\x82\x8D\x01a!\xA8V[\x84\x88\x01Ra\x02\x80\x93Pa$\xE8\x8C\x85\x8D\x01a!\xA8V[a\x01`\x88\x01Ra$\xFC\x8Ca\x02\xC0\x8D\x01a!\xA8V[\x83\x88\x01Ra%\x0E\x8Ca\x03\0\x8D\x01a!\xA8V[a\x01\xA0\x88\x01Ra%\"\x8Ca\x03@\x8D\x01a!\xA8V[\x82\x88\x01Ra%4\x8Ca\x03\x80\x8D\x01a!\xA8V[a\x01\xE0\x88\x01Ra%H\x8Ca\x03\xC0\x8D\x01a!\xA8V[\x85\x88\x01Ra%Z\x8Ca\x04\0\x8D\x01a!\xA8V[a\x02 \x88\x01Ra%n\x8Ca\x04@\x8D\x01a!\xA8V[\x81\x88\x01RPPPa%\x83\x89a\x04\x80\x8A\x01a!\xA8V[a\x02`\x85\x01Ra\x04\xC0\x88\x015\x81\x85\x01RPPa\x04\xE0\x86\x015a\x02\xA0\x83\x01R\x81\x94Pa%\xB0\x87\x82\x88\x01a!\xF5V[\x93PPPa%\xC2\x85a\x05\xE0\x86\x01a\"`V[\x90P\x92P\x92P\x92V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x730000000000000000000000000000000000000000301460806040526004361061004a575f3560e01c80631d712e271461004e578063ce537a7714610088578063df6e6cb4146100ab575b5f80fd5b6100757f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4781565b6040519081526020015b60405180910390f35b61009b6100963660046123ed565b6100bf565b604051901515815260200161007f565b6100755f805160206125f483398151915281565b5f6100c982611012565b6100d9835f5b602002015161114d565b6100e48360016100cf565b6100ef8360026100cf565b6100fa8360036100cf565b6101058360046100cf565b6101108360056100cf565b61011b8360066100cf565b5f6101278585856111af565b90505f610136865f01516117b8565b90505f610148828460a0015188611b96565b90505f610156848784611bf3565b9050610235565b60405162461bcd60e51b815260206004820152600c60248201526b6572726f722076657269667960a01b6044820152606481fd5b60405162461bcd60e51b815260206004820152600d60248201526c6572726f722070616972696e6760981b6044820152606481fd5b604051815181526020820151602082015282604082015260405f60608360075afa9050806101f6576101f661015d565b505050565b6040805182518152602080840151818301525f5182840152516060820152908260808360065afa9050806102315761023161015d565b5050565b60405160c081017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c160408301527f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b060608301527f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe460808301527f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5560a08301527f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c26101008301527f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed6101208301527f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b6101408301527f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa61016083015261018082016040525f805f8060808a01515f805160206125f483398151915260208a015160208d01510993508a515f805160206125f483398151915260a08d015160608e01510993505f805160206125f48339815191526101a08f0151850892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a850992505f805160206125f48339815191526101c08f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb025850992505f805160206125f48339815191526101e08f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992505f805160206125f48339815191526102008f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f48339815191527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e881850992505f805160206125f48339815191526102208f0151840892505f805160206125f483398151915282840892505f805160206125f483398151915281840990505f805160206125f483398151915281860894505060a08d015194506105e084866101c6565b5f51865260205160208701525f805160206125f483398151915260608c01518c510993505f805160206125f48339815191526102c08e0151850993505f805160206125f48339815191526102408e015160608d01510992505f805160206125f48339815191526101a08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102608e015160608d01510992505f805160206125f48339815191526101c08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102808e015160608d01510992505f805160206125f48339815191526101e08e0151840892505f805160206125f483398151915281840892505f805160206125f483398151915283850993505f805160206125f48339815191526102a08e015160608d01510992505f805160206125f48339815191526102008e0151840892505f805160206125f48339815191528184089250505f805160206125f48339815191528284095f805160206125f483398151915203925060c08e015193506107a583856101c6565b6107ae856101fb565b6101a08c0151925060e08e015193506107c783856101c6565b6107d0856101fb565b6101c08c015192506101008e015193506107ea83856101c6565b6107f3856101fb565b6101e08c015192506101208e0151935061080d83856101c6565b610816856101fb565b6102008c015192506101408e0151935061083083856101c6565b610839856101fb565b5f805160206125f48339815191526101c08d01516101a08e01510992506101608e0151935061086883856101c6565b610871856101fb565b5f805160206125f48339815191526102008d01516101e08e01510992506101808e015193506108a083856101c6565b6108a9856101fb565b6101a08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506101e08e015193506108fc83856101c6565b610905856101fb565b6101c08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102008e0151935061095883856101c6565b610961856101fb565b6101e08c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102208e015193506109b483856101c6565b6109bd856101fb565b6102008c015191505f805160206125f483398151915282830990505f805160206125f483398151915281820990505f805160206125f483398151915281830992506102408e01519350610a1083856101c6565b610a19856101fb565b6102208c01515f805160206125f48339815191520392506101a08e01519350610a4283856101c6565b610a4b856101fb565b600192506101c08e01519350610a6183856101c6565b610a6a856101fb565b5f805160206125f48339815191526101c08d01516101a08e01510991505f805160206125f48339815191526101e08d0151830991505f805160206125f48339815191526102008d0151830991505f805160206125f48339815191526102208d0151830992506102608e01519350610ae183856101c6565b610aea856101fb565b87515f805160206125f483398151915203925060c08c01519350610b0e83856101c6565b610b17856101fb565b5f805160206125f48339815191526001895108915060a08a015190505f805160206125f483398151915281820990505f805160206125f48339815191528183099150505f805160206125f4833981519152818309915060e08b01519250610b7e82846101c6565b610b87846101fb565b5f805160206125f483398151915281830991506101008b01519250610bac82846101c6565b610bb5846101fb565b5f805160206125f483398151915281830991506101208b01519250610bda82846101c6565b610be3846101fb565b5f805160206125f483398151915281830991506101408b01519250610c0882846101c6565b610c11846101fb565b50505060c0860151885190805f805160206125f4833981519152869003610c3882856101c6565b610c41856101fb565b5f805160206125f4833981519152806101a08e01518509820890505f805160206125f4833981519152828409925060208c01519350610c8083856101c6565b610c89856101fb565b5f805160206125f4833981519152806101c08e01518509820890505f805160206125f4833981519152828409925060408c01519350610cc883856101c6565b610cd1856101fb565b5f805160206125f4833981519152806101e08e01518509820890505f805160206125f4833981519152828409925060608c01519350610d1083856101c6565b610d19856101fb565b5f805160206125f4833981519152806102008e01518509820890505f805160206125f4833981519152828409925060808c01519350610d5883856101c6565b610d61856101fb565b5f805160206125f4833981519152806102208e01518509820890505f805160206125f4833981519152828409925060408e01519350610da083856101c6565b610da9856101fb565b5f805160206125f4833981519152806102408e01518509820890505f805160206125f4833981519152828409925060608e01519350610de883856101c6565b610df1856101fb565b5f805160206125f4833981519152806102608e01518509820890505f805160206125f4833981519152828409925060808e01519350610e3083856101c6565b610e39856101fb565b5f805160206125f4833981519152806102808e01518509820890505f805160206125f4833981519152828409925060a08e01519350610e7883856101c6565b610e81856101fb565b5f805160206125f4833981519152806102a08e015185098208905060e08a0151925060a08c01519350610eb483856101c6565b610ebd856101fb565b5f805160206125f4833981519152806102c08e015185098208905060a08a015192506101608c01519350610ef183856101c6565b610efa856101fb565b5f805160206125f4833981519152602060408b01510151840991505f805160206125f483398151915260e08b0151830992506101808c01519350610f3e83856101c6565b610f47856101fb565b6040805180820190915293506001845260026020850152610f77815f805160206125f483398151915203856101c6565b50610f81846101fb565b610fb08460200180517f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47039052565b505050610160880151805183526020908101519083015260e086015161018089015190610fdd81836101c6565b505050610fe9816101fb565b60205f6101808360085afa90508061100357611003610191565b50505f51979650505050505050565b805161101d90611da8565b61102a8160200151611da8565b6110378160400151611da8565b6110448160600151611da8565b6110518160800151611da8565b61105e8160a00151611da8565b61106b8160c00151611da8565b6110788160e00151611da8565b611086816101000151611da8565b611094816101200151611da8565b6110a2816101400151611da8565b6110b0816101600151611da8565b6110be816101800151611da8565b6110cc816101a0015161114d565b6110da816101c0015161114d565b6110e8816101e0015161114d565b6110f681610200015161114d565b61110481610220015161114d565b61111281610240015161114d565b61112081610260015161114d565b61112e81610280015161114d565b61113c816102a0015161114d565b61114a816102c0015161114d565b50565b5f805160206125f48339815191528110806102315760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c64000000000060448201526064015b60405180910390fd5b6111ef6040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b604051602081015f815260fe60e01b8152855160c01b6004820152602086015160c01b600c82015261028086015160208201526102a08601516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e086015180516101008301526020810151610120830152506101008601518051610140830152602081015161016083015250610120860151805161018083015260208101516101a08301525061014086015180516101c083015260208101516101e083015250610160860151805161020083015260208101516102208301525061018086015180516102408301526020810151610260830152506101e0860151805161028083015260208101516102a08301525061020086015180516102c083015260208101516102e083015250610220860151805161030083015260208101516103208301525061024086015180516103408301526020810151610360830152506101a0860151805161038083015260208101516103a0830152506101c086015180516103c083015260208101516103e0830152506102608601518051610400830152602081015161042083015250604086015180516104408301526020810151610460830152506060860151805161048083015260208101516104a083015250608086015180516104c083015260208101516104e08301525060a0860151805161050083015260208101516105208301525060c08601518051610540830152602081015161056083015250845161058082015260208501516105a082015260408501516105c082015260608501516105e0820152608085015161060082015260a085015161062082015260c085015161064082015260e085015161066082015283518051610660830152602081015161068083015250602084015180516106a083015260208101516106c083015250604084015180516106e083015260208101516107008301525060608401518051610720830152602081015161074083015250608084015180516107608301526020810151610780830152505f82526107c082206107c08201526107c0810191506020820190505f805160206125f483398151915282510660608401526020822081528091506020820190505f805160206125f4833981519152825106608084015260a0840151805182526020810151602083015250606082206040820192508083526020830191505f805160206125f4833981519152810684525f805160206125f48339815191528182095f805160206125f48339815191528282099150806020860152508060408501525060c084015180518252602081015160208301525060e084015180516040830152602081015160608301525061010084015180516080830152602081015160a083015250610120840151805160c0830152602081015160e08301525061014084015180516101008301526020810151610120830152506101608220610140820152610140810191506020820190505f805160206125f483398151915282510660a08401526101a084015181526101c084015160208201526101e084015160408201526102008401516060820152610220840151608082015261024084015160a082015261026084015160c082015261028084015160e08201526102a08401516101008201526102c08401516101208201526101608220610140820152610140810191506020820190505f805160206125f483398151915282510660c08401526101608401518051825260208101516020830152506101808401518051604083015260208101516060830152505060a081205f805160206125f4833981519152810660e084015250509392505050565b6117c0612104565b8162010000036118ff576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c100181526020016040518060e00160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd8152508152509050919050565b816210000003611a3f576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c10181526020016040518060e00160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c8152508152509050919050565b81602003611b7d576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e75080000181526020016040518060e00160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b0390438152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b611bb760405180606001604052805f81526020015f81526020015f81525090565b611bc18484611e51565b808252611bd19085908590611ea2565b60208201528051611be790859084908690611f13565b60408201529392505050565b60208101516040820151606085015160808601516101a08601516102408701515f959493600193909290915f805160206125f483398151915280808387095f805160206125f4833981519152868608088609945050506101c08801516102608901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f4833981519152868608088609945050506101e08801516102808901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f4833981519152868608088609945050506102008801516102a08901515f805160206125f4833981519152805f805160206125f48339815191528387095f805160206125f48339815191528686080886099450505061022088015191506102c08801515f805160206125f483398151915280825f805160206125f48339815191528587080985099350505050865160208801515f805160206125f4833981519152808683095f805160206125f48339815191520385089550505f805160206125f4833981519152808383095f805160206125f483398151915203860898975050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd47911590151615611de157505050565b8251602084015182600384858586098509088382830914838210848410161693505050816101f65760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e7400000000000000000060448201526064016111a6565b81515f905f805160206125f483398151915290838015611e92578493505f5b82811015611e8657838586099450600101611e70565b50600184039350611e99565b6001830393505b50505092915050565b5f82600103611eb357506001611f0c565b815f03611ec157505f611f0c565b60208401515f805160206125f4833981519152905f90828186099050858015611eef57600187039250611ef6565b6001840392505b50611f0082612063565b91508282820993505050505b9392505050565b5f5f805160206125f4833981519152828203611f8c5760015f5b6007811015611f8157818603611f5e57868160078110611f4f57611f4f6125cb565b6020020151935050505061205b565b8280611f6c57611f6c6125df565b60408901516020015183099150600101611f2d565b505f9250505061205b565b611f94612128565b6040870151600160c0838101828152920190805b6007811015611fd55760208403935085868a85518903088309808552601f19909301929150600101611fa8565b505050505f805f90506001838960408c01515f5b6007811015612029578882518a85518c88518a0909098981880896505088898d84518c030886099450602093840193928301929190910190600101611fe9565b50505050809250505f61203b83612063565b905060208a01518581890996505084818709955084828709955050505050505b949350505050565b5f805f5f805160206125f4833981519152905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f519250816120fd5760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c65642100000060448201526064016111a6565b5050919050565b60405180606001604052805f81526020015f8152602001612123612128565b905290565b6040518060e001604052806007906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561217e5761217e612146565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561217e5761217e612146565b5f604082840312156121b8575f80fd5b6040516040810181811067ffffffffffffffff821117156121db576121db612146565b604052823581526020928301359281019290925250919050565b5f82601f830112612204575f80fd5b60405160e0810181811067ffffffffffffffff8211171561222757612227612146565b6040528060e084018581111561223b575f80fd5b845b8181101561225557803583526020928301920161223d565b509195945050505050565b5f6104808284031215612271575f80fd5b61227961215a565b905061228583836121a8565b815261229483604084016121a8565b60208201526122a683608084016121a8565b60408201526122b88360c084016121a8565b60608201526101006122cc848285016121a8565b60808301526101406122e0858286016121a8565b60a08401526101806122f4868287016121a8565b60c08501526101c0612308878288016121a8565b60e086015261020061231c888289016121a8565b858701526102409450612331888689016121a8565b61012087015261028061234689828a016121a8565b858801526102c0945061235b89868a016121a8565b61016088015261236f896103008a016121a8565b848801526103408801356101a0880152610360880135838801526103808801356101e08801526103a0880135828801526103c08801356102208801526103e08801358688015261040088013561026088015261042088013581880152505050506104408401356102a084015261046084013581840152505092915050565b5f805f838503610a60811215612401575f80fd5b61050080821215612410575f80fd5b612418612184565b9150853582526020860135602083015261243587604088016121a8565b604083015261244787608088016121a8565b60608301526124598760c088016121a8565b608083015261010061246d888289016121a8565b60a084015261014061248189828a016121a8565b60c08501526101806124958a828b016121a8565b60e08601526101c06124a98b828c016121a8565b8487015261020093506124be8b858c016121a8565b6101208701526102406124d38c828d016121a8565b8488015261028093506124e88c858d016121a8565b6101608801526124fc8c6102c08d016121a8565b8388015261250e8c6103008d016121a8565b6101a08801526125228c6103408d016121a8565b828801526125348c6103808d016121a8565b6101e08801526125488c6103c08d016121a8565b8588015261255a8c6104008d016121a8565b61022088015261256e8c6104408d016121a8565b81880152505050612583896104808a016121a8565b6102608501526104c08801358185015250506104e08601356102a08301528194506125b0878288016121f5565b935050506125c2856105e08601612260565b90509250925092565b634e487b7160e01b5f52603260045260245ffd5b634e487b7160e01b5f52601260045260245ffdfe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"s\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\x000\x14`\x80`@R`\x046\x10a\0JW_5`\xE0\x1C\x80c\x1Dq.'\x14a\0NW\x80c\xCESzw\x14a\0\x88W\x80c\xDFnl\xB4\x14a\0\xABW[_\x80\xFD[a\0u\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\x9Ba\0\x966`\x04a#\xEDV[a\0\xBFV[`@Q\x90\x15\x15\x81R` \x01a\0\x7FV[a\0u_\x80Q` a%\xF4\x839\x81Q\x91R\x81V[_a\0\xC9\x82a\x10\x12V[a\0\xD9\x83_[` \x02\x01Qa\x11MV[a\0\xE4\x83`\x01a\0\xCFV[a\0\xEF\x83`\x02a\0\xCFV[a\0\xFA\x83`\x03a\0\xCFV[a\x01\x05\x83`\x04a\0\xCFV[a\x01\x10\x83`\x05a\0\xCFV[a\x01\x1B\x83`\x06a\0\xCFV[_a\x01'\x85\x85\x85a\x11\xAFV[\x90P_a\x016\x86_\x01Qa\x17\xB8V[\x90P_a\x01H\x82\x84`\xA0\x01Q\x88a\x1B\x96V[\x90P_a\x01V\x84\x87\x84a\x1B\xF3V[\x90Pa\x025V[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0C`$\x82\x01Rkerror verify`\xA0\x1B`D\x82\x01R`d\x81\xFD[`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\r`$\x82\x01Rlerror pairing`\x98\x1B`D\x82\x01R`d\x81\xFD[`@Q\x81Q\x81R` \x82\x01Q` \x82\x01R\x82`@\x82\x01R`@_``\x83`\x07Z\xFA\x90P\x80a\x01\xF6Wa\x01\xF6a\x01]V[PPPV[`@\x80Q\x82Q\x81R` \x80\x84\x01Q\x81\x83\x01R_Q\x82\x84\x01RQ``\x82\x01R\x90\x82`\x80\x83`\x06Z\xFA\x90P\x80a\x021Wa\x021a\x01]V[PPV[`@Q`\xC0\x81\x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1`@\x83\x01R\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0``\x83\x01R\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4`\x80\x83\x01R\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U`\xA0\x83\x01R\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2a\x01\0\x83\x01R\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xEDa\x01 \x83\x01R\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[a\x01@\x83\x01R\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAAa\x01`\x83\x01Ra\x01\x80\x82\x01`@R_\x80_\x80`\x80\x8A\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R` \x8A\x01Q` \x8D\x01Q\t\x93P\x8AQ_\x80Q` a%\xF4\x839\x81Q\x91R`\xA0\x8D\x01Q``\x8E\x01Q\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xA0\x8F\x01Q\x85\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02 \x8F\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x86\x08\x94PP`\xA0\x8D\x01Q\x94Pa\x05\xE0\x84\x86a\x01\xC6V[_Q\x86R` Q` \x87\x01R_\x80Q` a%\xF4\x839\x81Q\x91R``\x8C\x01Q\x8CQ\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\xC0\x8E\x01Q\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02@\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xA0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02`\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\x80\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x85\t\x93P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\xA0\x8E\x01Q``\x8D\x01Q\t\x92P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8E\x01Q\x84\x08\x92P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x84\x08\x92PP_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92P`\xC0\x8E\x01Q\x93Pa\x07\xA5\x83\x85a\x01\xC6V[a\x07\xAE\x85a\x01\xFBV[a\x01\xA0\x8C\x01Q\x92P`\xE0\x8E\x01Q\x93Pa\x07\xC7\x83\x85a\x01\xC6V[a\x07\xD0\x85a\x01\xFBV[a\x01\xC0\x8C\x01Q\x92Pa\x01\0\x8E\x01Q\x93Pa\x07\xEA\x83\x85a\x01\xC6V[a\x07\xF3\x85a\x01\xFBV[a\x01\xE0\x8C\x01Q\x92Pa\x01 \x8E\x01Q\x93Pa\x08\r\x83\x85a\x01\xC6V[a\x08\x16\x85a\x01\xFBV[a\x02\0\x8C\x01Q\x92Pa\x01@\x8E\x01Q\x93Pa\x080\x83\x85a\x01\xC6V[a\x089\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8D\x01Qa\x01\xA0\x8E\x01Q\t\x92Pa\x01`\x8E\x01Q\x93Pa\x08h\x83\x85a\x01\xC6V[a\x08q\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8D\x01Qa\x01\xE0\x8E\x01Q\t\x92Pa\x01\x80\x8E\x01Q\x93Pa\x08\xA0\x83\x85a\x01\xC6V[a\x08\xA9\x85a\x01\xFBV[a\x01\xA0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x01\xE0\x8E\x01Q\x93Pa\x08\xFC\x83\x85a\x01\xC6V[a\t\x05\x85a\x01\xFBV[a\x01\xC0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02\0\x8E\x01Q\x93Pa\tX\x83\x85a\x01\xC6V[a\ta\x85a\x01\xFBV[a\x01\xE0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02 \x8E\x01Q\x93Pa\t\xB4\x83\x85a\x01\xC6V[a\t\xBD\x85a\x01\xFBV[a\x02\0\x8C\x01Q\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x83\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x92Pa\x02@\x8E\x01Q\x93Pa\n\x10\x83\x85a\x01\xC6V[a\n\x19\x85a\x01\xFBV[a\x02 \x8C\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92Pa\x01\xA0\x8E\x01Q\x93Pa\nB\x83\x85a\x01\xC6V[a\nK\x85a\x01\xFBV[`\x01\x92Pa\x01\xC0\x8E\x01Q\x93Pa\na\x83\x85a\x01\xC6V[a\nj\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xC0\x8D\x01Qa\x01\xA0\x8E\x01Q\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x01\xE0\x8D\x01Q\x83\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02\0\x8D\x01Q\x83\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91Ra\x02 \x8D\x01Q\x83\t\x92Pa\x02`\x8E\x01Q\x93Pa\n\xE1\x83\x85a\x01\xC6V[a\n\xEA\x85a\x01\xFBV[\x87Q_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x92P`\xC0\x8C\x01Q\x93Pa\x0B\x0E\x83\x85a\x01\xC6V[a\x0B\x17\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R`\x01\x89Q\x08\x91P`\xA0\x8A\x01Q\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91PP_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91P`\xE0\x8B\x01Q\x92Pa\x0B~\x82\x84a\x01\xC6V[a\x0B\x87\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01\0\x8B\x01Q\x92Pa\x0B\xAC\x82\x84a\x01\xC6V[a\x0B\xB5\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01 \x8B\x01Q\x92Pa\x0B\xDA\x82\x84a\x01\xC6V[a\x0B\xE3\x84a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x83\t\x91Pa\x01@\x8B\x01Q\x92Pa\x0C\x08\x82\x84a\x01\xC6V[a\x0C\x11\x84a\x01\xFBV[PPP`\xC0\x86\x01Q\x88Q\x90\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x90\x03a\x0C8\x82\x85a\x01\xC6V[a\x0CA\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xA0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P` \x8C\x01Q\x93Pa\x0C\x80\x83\x85a\x01\xC6V[a\x0C\x89\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xC0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`@\x8C\x01Q\x93Pa\x0C\xC8\x83\x85a\x01\xC6V[a\x0C\xD1\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x01\xE0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P``\x8C\x01Q\x93Pa\r\x10\x83\x85a\x01\xC6V[a\r\x19\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\0\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\x80\x8C\x01Q\x93Pa\rX\x83\x85a\x01\xC6V[a\ra\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02 \x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`@\x8E\x01Q\x93Pa\r\xA0\x83\x85a\x01\xC6V[a\r\xA9\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02@\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P``\x8E\x01Q\x93Pa\r\xE8\x83\x85a\x01\xC6V[a\r\xF1\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02`\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\x80\x8E\x01Q\x93Pa\x0E0\x83\x85a\x01\xC6V[a\x0E9\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\x80\x8E\x01Q\x85\t\x82\x08\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x84\t\x92P`\xA0\x8E\x01Q\x93Pa\x0Ex\x83\x85a\x01\xC6V[a\x0E\x81\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\xA0\x8E\x01Q\x85\t\x82\x08\x90P`\xE0\x8A\x01Q\x92P`\xA0\x8C\x01Q\x93Pa\x0E\xB4\x83\x85a\x01\xC6V[a\x0E\xBD\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R\x80a\x02\xC0\x8E\x01Q\x85\t\x82\x08\x90P`\xA0\x8A\x01Q\x92Pa\x01`\x8C\x01Q\x93Pa\x0E\xF1\x83\x85a\x01\xC6V[a\x0E\xFA\x85a\x01\xFBV[_\x80Q` a%\xF4\x839\x81Q\x91R` `@\x8B\x01Q\x01Q\x84\t\x91P_\x80Q` a%\xF4\x839\x81Q\x91R`\xE0\x8B\x01Q\x83\t\x92Pa\x01\x80\x8C\x01Q\x93Pa\x0F>\x83\x85a\x01\xC6V[a\x0FG\x85a\x01\xFBV[`@\x80Q\x80\x82\x01\x90\x91R\x93P`\x01\x84R`\x02` \x85\x01Ra\x0Fw\x81_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x85a\x01\xC6V[Pa\x0F\x81\x84a\x01\xFBV[a\x0F\xB0\x84` \x01\x80Q\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x03\x90RV[PPPa\x01`\x88\x01Q\x80Q\x83R` \x90\x81\x01Q\x90\x83\x01R`\xE0\x86\x01Qa\x01\x80\x89\x01Q\x90a\x0F\xDD\x81\x83a\x01\xC6V[PPPa\x0F\xE9\x81a\x01\xFBV[` _a\x01\x80\x83`\x08Z\xFA\x90P\x80a\x10\x03Wa\x10\x03a\x01\x91V[PP_Q\x97\x96PPPPPPPV[\x80Qa\x10\x1D\x90a\x1D\xA8V[a\x10*\x81` \x01Qa\x1D\xA8V[a\x107\x81`@\x01Qa\x1D\xA8V[a\x10D\x81``\x01Qa\x1D\xA8V[a\x10Q\x81`\x80\x01Qa\x1D\xA8V[a\x10^\x81`\xA0\x01Qa\x1D\xA8V[a\x10k\x81`\xC0\x01Qa\x1D\xA8V[a\x10x\x81`\xE0\x01Qa\x1D\xA8V[a\x10\x86\x81a\x01\0\x01Qa\x1D\xA8V[a\x10\x94\x81a\x01 \x01Qa\x1D\xA8V[a\x10\xA2\x81a\x01@\x01Qa\x1D\xA8V[a\x10\xB0\x81a\x01`\x01Qa\x1D\xA8V[a\x10\xBE\x81a\x01\x80\x01Qa\x1D\xA8V[a\x10\xCC\x81a\x01\xA0\x01Qa\x11MV[a\x10\xDA\x81a\x01\xC0\x01Qa\x11MV[a\x10\xE8\x81a\x01\xE0\x01Qa\x11MV[a\x10\xF6\x81a\x02\0\x01Qa\x11MV[a\x11\x04\x81a\x02 \x01Qa\x11MV[a\x11\x12\x81a\x02@\x01Qa\x11MV[a\x11 \x81a\x02`\x01Qa\x11MV[a\x11.\x81a\x02\x80\x01Qa\x11MV[a\x11<\x81a\x02\xA0\x01Qa\x11MV[a\x11J\x81a\x02\xC0\x01Qa\x11MV[PV[_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x10\x80a\x021W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[a\x11\xEF`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x85Q`\xC0\x1B`\x04\x82\x01R` \x86\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x86\x01Q` \x82\x01Ra\x02\xA0\x86\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x86\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x86\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x86\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x86\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x86\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x86\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x86\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x86\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x86\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x86\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x86\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x86\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x86\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x86\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x86\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x86\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x86\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x86\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x84Qa\x05\x80\x82\x01R` \x85\x01Qa\x05\xA0\x82\x01R`@\x85\x01Qa\x05\xC0\x82\x01R``\x85\x01Qa\x05\xE0\x82\x01R`\x80\x85\x01Qa\x06\0\x82\x01R`\xA0\x85\x01Qa\x06 \x82\x01R`\xC0\x85\x01Qa\x06@\x82\x01R`\xE0\x85\x01Qa\x06`\x82\x01R\x83Q\x80Qa\x06`\x83\x01R` \x81\x01Qa\x06\x80\x83\x01RP` \x84\x01Q\x80Qa\x06\xA0\x83\x01R` \x81\x01Qa\x06\xC0\x83\x01RP`@\x84\x01Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP``\x84\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`\x80\x84\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP_\x82Ra\x07\xC0\x82 a\x07\xC0\x82\x01Ra\x07\xC0\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06``\x84\x01R` \x82 \x81R\x80\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\x80\x84\x01R`\xA0\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 `@\x82\x01\x92P\x80\x83R` \x83\x01\x91P_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x06\x84R_\x80Q` a%\xF4\x839\x81Q\x91R\x81\x82\t_\x80Q` a%\xF4\x839\x81Q\x91R\x82\x82\t\x91P\x80` \x86\x01RP\x80`@\x85\x01RP`\xC0\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x84\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x84\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x84\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x84\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 a\x01@\x82\x01Ra\x01@\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\xA0\x84\x01Ra\x01\xA0\x84\x01Q\x81Ra\x01\xC0\x84\x01Q` \x82\x01Ra\x01\xE0\x84\x01Q`@\x82\x01Ra\x02\0\x84\x01Q``\x82\x01Ra\x02 \x84\x01Q`\x80\x82\x01Ra\x02@\x84\x01Q`\xA0\x82\x01Ra\x02`\x84\x01Q`\xC0\x82\x01Ra\x02\x80\x84\x01Q`\xE0\x82\x01Ra\x02\xA0\x84\x01Qa\x01\0\x82\x01Ra\x02\xC0\x84\x01Qa\x01 \x82\x01Ra\x01`\x82 a\x01@\x82\x01Ra\x01@\x81\x01\x91P` \x82\x01\x90P_\x80Q` a%\xF4\x839\x81Q\x91R\x82Q\x06`\xC0\x84\x01Ra\x01`\x84\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x84\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 _\x80Q` a%\xF4\x839\x81Q\x91R\x81\x06`\xE0\x84\x01RPP\x93\x92PPPV[a\x17\xC0a!\x04V[\x81b\x01\0\0\x03a\x18\xFFW`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x1A?W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\x1B}W`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80`\xE0\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\x1B\xB7`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\x1B\xC1\x84\x84a\x1EQV[\x80\x82Ra\x1B\xD1\x90\x85\x90\x85\x90a\x1E\xA2V[` \x82\x01R\x80Qa\x1B\xE7\x90\x85\x90\x84\x90\x86\x90a\x1F\x13V[`@\x82\x01R\x93\x92PPPV[` \x81\x01Q`@\x82\x01Q``\x85\x01Q`\x80\x86\x01Qa\x01\xA0\x86\x01Qa\x02@\x87\x01Q_\x95\x94\x93`\x01\x93\x90\x92\x90\x91_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x80\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x88\x01Qa\x02`\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x88\x01Qa\x02\x80\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x88\x01Qa\x02\xA0\x89\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80_\x80Q` a%\xF4\x839\x81Q\x91R\x83\x87\t_\x80Q` a%\xF4\x839\x81Q\x91R\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x88\x01Q\x91Pa\x02\xC0\x88\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x82_\x80Q` a%\xF4\x839\x81Q\x91R\x85\x87\x08\t\x85\t\x93PPPP\x86Q` \x88\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x86\x83\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x85\x08\x95PP_\x80Q` a%\xF4\x839\x81Q\x91R\x80\x83\x83\t_\x80Q` a%\xF4\x839\x81Q\x91R\x03\x86\x08\x98\x97PPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x1D\xE1WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x01\xF6W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x11\xA6V[\x81Q_\x90_\x80Q` a%\xF4\x839\x81Q\x91R\x90\x83\x80\x15a\x1E\x92W\x84\x93P_[\x82\x81\x10\x15a\x1E\x86W\x83\x85\x86\t\x94P`\x01\x01a\x1EpV[P`\x01\x84\x03\x93Pa\x1E\x99V[`\x01\x83\x03\x93P[PPP\x92\x91PPV[_\x82`\x01\x03a\x1E\xB3WP`\x01a\x1F\x0CV[\x81_\x03a\x1E\xC1WP_a\x1F\x0CV[` \x84\x01Q_\x80Q` a%\xF4\x839\x81Q\x91R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x1E\xEFW`\x01\x87\x03\x92Pa\x1E\xF6V[`\x01\x84\x03\x92P[Pa\x1F\0\x82a cV[\x91P\x82\x82\x82\t\x93PPPP[\x93\x92PPPV[__\x80Q` a%\xF4\x839\x81Q\x91R\x82\x82\x03a\x1F\x8CW`\x01_[`\x07\x81\x10\x15a\x1F\x81W\x81\x86\x03a\x1F^W\x86\x81`\x07\x81\x10a\x1FOWa\x1FOa%\xCBV[` \x02\x01Q\x93PPPPa [V[\x82\x80a\x1FlWa\x1Fla%\xDFV[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\x1F-V[P_\x92PPPa [V[a\x1F\x94a!(V[`@\x87\x01Q`\x01`\xC0\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x07\x81\x10\x15a\x1F\xD5W` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\x1F\xA8V[PPPP_\x80_\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x07\x81\x10\x15a )W\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x1F\xE9V[PPPP\x80\x92PP_a ;\x83a cV[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x95PPPPPP[\x94\x93PPPPV[_\x80__\x80Q` a%\xF4\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a \xFDW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01a\x11\xA6V[PP\x91\x90PV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!#a!(V[\x90R\x90V[`@Q\x80`\xE0\x01`@R\x80`\x07\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!~Wa!~a!FV[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a!~Wa!~a!FV[_`@\x82\x84\x03\x12\x15a!\xB8W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a!\xDBWa!\xDBa!FV[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x82`\x1F\x83\x01\x12a\"\x04W_\x80\xFD[`@Q`\xE0\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"'Wa\"'a!FV[`@R\x80`\xE0\x84\x01\x85\x81\x11\x15a\";W_\x80\xFD[\x84[\x81\x81\x10\x15a\"UW\x805\x83R` \x92\x83\x01\x92\x01a\"=V[P\x91\x95\x94PPPPPV[_a\x04\x80\x82\x84\x03\x12\x15a\"qW_\x80\xFD[a\"ya!ZV[\x90Pa\"\x85\x83\x83a!\xA8V[\x81Ra\"\x94\x83`@\x84\x01a!\xA8V[` \x82\x01Ra\"\xA6\x83`\x80\x84\x01a!\xA8V[`@\x82\x01Ra\"\xB8\x83`\xC0\x84\x01a!\xA8V[``\x82\x01Ra\x01\0a\"\xCC\x84\x82\x85\x01a!\xA8V[`\x80\x83\x01Ra\x01@a\"\xE0\x85\x82\x86\x01a!\xA8V[`\xA0\x84\x01Ra\x01\x80a\"\xF4\x86\x82\x87\x01a!\xA8V[`\xC0\x85\x01Ra\x01\xC0a#\x08\x87\x82\x88\x01a!\xA8V[`\xE0\x86\x01Ra\x02\0a#\x1C\x88\x82\x89\x01a!\xA8V[\x85\x87\x01Ra\x02@\x94Pa#1\x88\x86\x89\x01a!\xA8V[a\x01 \x87\x01Ra\x02\x80a#F\x89\x82\x8A\x01a!\xA8V[\x85\x88\x01Ra\x02\xC0\x94Pa#[\x89\x86\x8A\x01a!\xA8V[a\x01`\x88\x01Ra#o\x89a\x03\0\x8A\x01a!\xA8V[\x84\x88\x01Ra\x03@\x88\x015a\x01\xA0\x88\x01Ra\x03`\x88\x015\x83\x88\x01Ra\x03\x80\x88\x015a\x01\xE0\x88\x01Ra\x03\xA0\x88\x015\x82\x88\x01Ra\x03\xC0\x88\x015a\x02 \x88\x01Ra\x03\xE0\x88\x015\x86\x88\x01Ra\x04\0\x88\x015a\x02`\x88\x01Ra\x04 \x88\x015\x81\x88\x01RPPPPa\x04@\x84\x015a\x02\xA0\x84\x01Ra\x04`\x84\x015\x81\x84\x01RPP\x92\x91PPV[_\x80_\x83\x85\x03a\n`\x81\x12\x15a$\x01W_\x80\xFD[a\x05\0\x80\x82\x12\x15a$\x10W_\x80\xFD[a$\x18a!\x84V[\x91P\x855\x82R` \x86\x015` \x83\x01Ra$5\x87`@\x88\x01a!\xA8V[`@\x83\x01Ra$G\x87`\x80\x88\x01a!\xA8V[``\x83\x01Ra$Y\x87`\xC0\x88\x01a!\xA8V[`\x80\x83\x01Ra\x01\0a$m\x88\x82\x89\x01a!\xA8V[`\xA0\x84\x01Ra\x01@a$\x81\x89\x82\x8A\x01a!\xA8V[`\xC0\x85\x01Ra\x01\x80a$\x95\x8A\x82\x8B\x01a!\xA8V[`\xE0\x86\x01Ra\x01\xC0a$\xA9\x8B\x82\x8C\x01a!\xA8V[\x84\x87\x01Ra\x02\0\x93Pa$\xBE\x8B\x85\x8C\x01a!\xA8V[a\x01 \x87\x01Ra\x02@a$\xD3\x8C\x82\x8D\x01a!\xA8V[\x84\x88\x01Ra\x02\x80\x93Pa$\xE8\x8C\x85\x8D\x01a!\xA8V[a\x01`\x88\x01Ra$\xFC\x8Ca\x02\xC0\x8D\x01a!\xA8V[\x83\x88\x01Ra%\x0E\x8Ca\x03\0\x8D\x01a!\xA8V[a\x01\xA0\x88\x01Ra%\"\x8Ca\x03@\x8D\x01a!\xA8V[\x82\x88\x01Ra%4\x8Ca\x03\x80\x8D\x01a!\xA8V[a\x01\xE0\x88\x01Ra%H\x8Ca\x03\xC0\x8D\x01a!\xA8V[\x85\x88\x01Ra%Z\x8Ca\x04\0\x8D\x01a!\xA8V[a\x02 \x88\x01Ra%n\x8Ca\x04@\x8D\x01a!\xA8V[\x81\x88\x01RPPPa%\x83\x89a\x04\x80\x8A\x01a!\xA8V[a\x02`\x85\x01Ra\x04\xC0\x88\x015\x81\x85\x01RPPa\x04\xE0\x86\x015a\x02\xA0\x83\x01R\x81\x94Pa%\xB0\x87\x82\x88\x01a!\xF5V[\x93PPPa%\xC2\x85a\x05\xE0\x86\x01a\"`V[\x90P\x92P\x92P\x92V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x17\0\n",
    );
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
    /**Function with signature `P_MOD()` and selector `0x1d712e27`.
    ```solidity
    function P_MOD() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct P_MODCall {}
    ///Container type for the return parameters of the [`P_MOD()`](P_MODCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct P_MODReturn {
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
            impl ::core::convert::From<P_MODCall> for UnderlyingRustTuple<'_> {
                fn from(value: P_MODCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for P_MODCall {
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
            impl ::core::convert::From<P_MODReturn> for UnderlyingRustTuple<'_> {
                fn from(value: P_MODReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for P_MODReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for P_MODCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = P_MODReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "P_MOD()";
            const SELECTOR: [u8; 4] = [29u8, 113u8, 46u8, 39u8];
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
    /**Function with signature `R_MOD()` and selector `0xdf6e6cb4`.
    ```solidity
    function R_MOD() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct R_MODCall {}
    ///Container type for the return parameters of the [`R_MOD()`](R_MODCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct R_MODReturn {
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
            impl ::core::convert::From<R_MODCall> for UnderlyingRustTuple<'_> {
                fn from(value: R_MODCall) -> Self {
                    ()
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for R_MODCall {
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
            impl ::core::convert::From<R_MODReturn> for UnderlyingRustTuple<'_> {
                fn from(value: R_MODReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for R_MODReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for R_MODCall {
            type Parameters<'a> = ();
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = R_MODReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "R_MOD()";
            const SELECTOR: [u8; 4] = [223u8, 110u8, 108u8, 180u8];
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
    /**Function with signature `verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[7],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))` and selector `0x77ae3f07`.
    ```solidity
    function verify(IPlonkVerifier.VerifyingKey memory vk, uint256[7] memory publicInput, IPlonkVerifier.PlonkProof memory proof) external view returns (bool success);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyCall {
        #[allow(missing_docs)]
        pub vk: <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub publicInput: [alloy::sol_types::private::primitives::aliases::U256; 7usize],
        #[allow(missing_docs)]
        pub proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[7],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))`](verifyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyReturn {
        #[allow(missing_docs)]
        pub success: bool,
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
                    7usize,
                >,
                IPlonkVerifier::PlonkProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
                [alloy::sol_types::private::primitives::aliases::U256; 7usize],
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
                    (value.vk, value.publicInput, value.proof)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for verifyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        vk: tuple.0,
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
                    (value.success,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for verifyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { success: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for verifyCall {
            type Parameters<'a> = (
                IPlonkVerifier::VerifyingKey,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    7usize,
                >,
                IPlonkVerifier::PlonkProof,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = verifyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "verify((uint256,uint256,(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),bytes32,bytes32),uint256[7],((uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),(uint256,uint256),uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256,uint256))";
            const SELECTOR: [u8; 4] = [119u8, 174u8, 63u8, 7u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <IPlonkVerifier::VerifyingKey as alloy_sol_types::SolType>::tokenize(&self.vk),
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        7usize,
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
    ///Container for all the [`PlonkVerifier2`](self) function calls.
    pub enum PlonkVerifier2Calls {
        #[allow(missing_docs)]
        P_MOD(P_MODCall),
        #[allow(missing_docs)]
        R_MOD(R_MODCall),
        #[allow(missing_docs)]
        verify(verifyCall),
    }
    #[automatically_derived]
    impl PlonkVerifier2Calls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[
            [29u8, 113u8, 46u8, 39u8],
            [119u8, 174u8, 63u8, 7u8],
            [223u8, 110u8, 108u8, 180u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PlonkVerifier2Calls {
        const NAME: &'static str = "PlonkVerifier2Calls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 3usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::P_MOD(_) => <P_MODCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::R_MOD(_) => <R_MODCall as alloy_sol_types::SolCall>::SELECTOR,
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
                -> alloy_sol_types::Result<PlonkVerifier2Calls>] = &[
                {
                    fn P_MOD(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifier2Calls> {
                        <P_MODCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifier2Calls::P_MOD)
                    }
                    P_MOD
                },
                {
                    fn verify(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifier2Calls> {
                        <verifyCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifier2Calls::verify)
                    }
                    verify
                },
                {
                    fn R_MOD(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifier2Calls> {
                        <R_MODCall as alloy_sol_types::SolCall>::abi_decode_raw(data, validate)
                            .map(PlonkVerifier2Calls::R_MOD)
                    }
                    R_MOD
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
                Self::P_MOD(inner) => {
                    <P_MODCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::R_MOD(inner) => {
                    <R_MODCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::verify(inner) => {
                    <verifyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::P_MOD(inner) => {
                    <P_MODCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::R_MOD(inner) => {
                    <R_MODCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::verify(inner) => {
                    <verifyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`PlonkVerifier2`](self) custom errors.
    pub enum PlonkVerifier2Errors {
        #[allow(missing_docs)]
        UnsupportedDegree(UnsupportedDegree),
    }
    #[automatically_derived]
    impl PlonkVerifier2Errors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[[226u8, 239u8, 9u8, 229u8]];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PlonkVerifier2Errors {
        const NAME: &'static str = "PlonkVerifier2Errors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 1usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::UnsupportedDegree(_) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::SELECTOR
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
                -> alloy_sol_types::Result<PlonkVerifier2Errors>] = &[{
                fn UnsupportedDegree(
                    data: &[u8],
                    validate: bool,
                ) -> alloy_sol_types::Result<PlonkVerifier2Errors> {
                    <UnsupportedDegree as alloy_sol_types::SolError>::abi_decode_raw(data, validate)
                        .map(PlonkVerifier2Errors::UnsupportedDegree)
                }
                UnsupportedDegree
            }];
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
                Self::UnsupportedDegree(inner) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::abi_encoded_size(inner)
                },
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::UnsupportedDegree(inner) => {
                    <UnsupportedDegree as alloy_sol_types::SolError>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`PlonkVerifier2`](self) contract instance.

    See the [wrapper's documentation](`PlonkVerifier2Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> PlonkVerifier2Instance<T, P, N> {
        PlonkVerifier2Instance::<T, P, N>::new(address, provider)
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
    ) -> impl ::core::future::Future<Output = alloy_contract::Result<PlonkVerifier2Instance<T, P, N>>>
    {
        PlonkVerifier2Instance::<T, P, N>::deploy(provider)
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
        PlonkVerifier2Instance::<T, P, N>::deploy_builder(provider)
    }
    /**A [`PlonkVerifier2`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`PlonkVerifier2`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct PlonkVerifier2Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for PlonkVerifier2Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("PlonkVerifier2Instance")
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
        > PlonkVerifier2Instance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`PlonkVerifier2`](self) contract instance.

        See the [wrapper's documentation](`PlonkVerifier2Instance`) for more details.*/
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
        ) -> alloy_contract::Result<PlonkVerifier2Instance<T, P, N>> {
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
    impl<T, P: ::core::clone::Clone, N> PlonkVerifier2Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> PlonkVerifier2Instance<T, P, N> {
            PlonkVerifier2Instance {
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
        > PlonkVerifier2Instance<T, P, N>
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
        ///Creates a new call builder for the [`P_MOD`] function.
        pub fn P_MOD(&self) -> alloy_contract::SolCallBuilder<T, &P, P_MODCall, N> {
            self.call_builder(&P_MODCall {})
        }
        ///Creates a new call builder for the [`R_MOD`] function.
        pub fn R_MOD(&self) -> alloy_contract::SolCallBuilder<T, &P, R_MODCall, N> {
            self.call_builder(&R_MODCall {})
        }
        ///Creates a new call builder for the [`verify`] function.
        pub fn verify(
            &self,
            vk: <IPlonkVerifier::VerifyingKey as alloy::sol_types::SolType>::RustType,
            publicInput: [alloy::sol_types::private::primitives::aliases::U256; 7usize],
            proof: <IPlonkVerifier::PlonkProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, verifyCall, N> {
            self.call_builder(&verifyCall {
                vk,
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
        > PlonkVerifier2Instance<T, P, N>
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
