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
///Module containing a contract's types and functions.
/**

```solidity
library PolynomialEvalV2 {
    struct EvalData { BN254.ScalarField vanishEval; BN254.ScalarField lagrangeOne; BN254.ScalarField piEval; }
    struct EvalDomain { uint256 logSize; uint256 sizeInv; uint256[11] elements; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod PolynomialEvalV2 {
    use alloy::sol_types as alloy_sol_types;

    use super::*;
    /**```solidity
    struct EvalData { BN254.ScalarField vanishEval; BN254.ScalarField lagrangeOne; BN254.ScalarField piEval; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EvalData {
        #[allow(missing_docs)]
        pub vanishEval: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub lagrangeOne: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub piEval: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
        type UnderlyingSolTuple<'a> = (BN254::ScalarField, BN254::ScalarField, BN254::ScalarField);
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
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
        impl ::core::convert::From<EvalData> for UnderlyingRustTuple<'_> {
            fn from(value: EvalData) -> Self {
                (value.vanishEval, value.lagrangeOne, value.piEval)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for EvalData {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    vanishEval: tuple.0,
                    lagrangeOne: tuple.1,
                    piEval: tuple.2,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for EvalData {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for EvalData {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.vanishEval),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.lagrangeOne),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.piEval),
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
        impl alloy_sol_types::SolType for EvalData {
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
        impl alloy_sol_types::SolStruct for EvalData {
            const NAME: &'static str = "EvalData";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "EvalData(uint256 vanishEval,uint256 lagrangeOne,uint256 piEval)",
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
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.vanishEval,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.lagrangeOne,
                    )
                    .0,
                    <BN254::ScalarField as alloy_sol_types::SolType>::eip712_data_word(
                        &self.piEval,
                    )
                    .0,
                ]
                .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for EvalData {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.vanishEval,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.lagrangeOne,
                    )
                    + <BN254::ScalarField as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.piEval,
                    )
            }
            #[inline]
            fn encode_topic_preimage(
                rust: &Self::RustType,
                out: &mut alloy_sol_types::private::Vec<u8>,
            ) {
                out.reserve(<Self as alloy_sol_types::EventTopic>::topic_preimage_length(rust));
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.vanishEval,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.lagrangeOne,
                    out,
                );
                <BN254::ScalarField as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.piEval,
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
    struct EvalDomain { uint256 logSize; uint256 sizeInv; uint256[11] elements; }
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct EvalDomain {
        #[allow(missing_docs)]
        pub logSize: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub sizeInv: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub elements: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
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
            alloy::sol_types::sol_data::FixedArray<alloy::sol_types::sol_data::Uint<256>, 11usize>,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::primitives::aliases::U256,
            alloy::sol_types::private::primitives::aliases::U256,
            [alloy::sol_types::private::primitives::aliases::U256; 11usize],
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
        impl ::core::convert::From<EvalDomain> for UnderlyingRustTuple<'_> {
            fn from(value: EvalDomain) -> Self {
                (value.logSize, value.sizeInv, value.elements)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for EvalDomain {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {
                    logSize: tuple.0,
                    sizeInv: tuple.1,
                    elements: tuple.2,
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for EvalDomain {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for EvalDomain {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.logSize,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.sizeInv,
                    ),
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::SolType>::tokenize(&self.elements),
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
        impl alloy_sol_types::SolType for EvalDomain {
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
        impl alloy_sol_types::SolStruct for EvalDomain {
            const NAME: &'static str = "EvalDomain";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "EvalDomain(uint256 logSize,uint256 sizeInv,uint256[11] elements)",
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
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.logSize)
                        .0,
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.sizeInv)
                        .0,
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::SolType>::eip712_data_word(&self.elements)
                        .0,
                ]
                    .concat()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for EvalDomain {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.logSize,
                    )
                    + <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.sizeInv,
                    )
                    + <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.elements,
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
                    &rust.logSize,
                    out,
                );
                <alloy::sol_types::sol_data::Uint<
                    256,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.sizeInv,
                    out,
                );
                <alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.elements, out
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
    /**Creates a new wrapper around an on-chain [`PolynomialEvalV2`](self) contract instance.

    See the [wrapper's documentation](`PolynomialEvalV2Instance`) for more details.*/
    #[inline]
    pub const fn new<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> PolynomialEvalV2Instance<T, P, N> {
        PolynomialEvalV2Instance::<T, P, N>::new(address, provider)
    }
    /**A [`PolynomialEvalV2`](self) instance.

    Contains type-safe methods for interacting with an on-chain instance of the
    [`PolynomialEvalV2`](self) contract located at a given `address`, using a given
    provider `P`.

    If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
    documentation on how to provide it), the `deploy` and `deploy_builder` methods can
    be used to deploy a new instance of the contract.

    See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct PolynomialEvalV2Instance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for PolynomialEvalV2Instance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("PolynomialEvalV2Instance")
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
        > PolynomialEvalV2Instance<T, P, N>
    {
        /**Creates a new wrapper around an on-chain [`PolynomialEvalV2`](self) contract instance.

        See the [wrapper's documentation](`PolynomialEvalV2Instance`) for more details.*/
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
    impl<T, P: ::core::clone::Clone, N> PolynomialEvalV2Instance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> PolynomialEvalV2Instance<T, P, N> {
            PolynomialEvalV2Instance {
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
        > PolynomialEvalV2Instance<T, P, N>
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
        > PolynomialEvalV2Instance<T, P, N>
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

library PolynomialEvalV2 {
    struct EvalData {
        BN254.ScalarField vanishEval;
        BN254.ScalarField lagrangeOne;
        BN254.ScalarField piEval;
    }
    struct EvalDomain {
        uint256 logSize;
        uint256 sizeInv;
        uint256[11] elements;
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
    function evalDataGen(PolynomialEvalV2.EvalDomain memory domain, uint256 zeta, uint256[11] memory publicInput) external view returns (PolynomialEvalV2.EvalData memory evalData);
    function evaluateLagrangeOne(PolynomialEvalV2.EvalDomain memory domain, BN254.ScalarField zeta, BN254.ScalarField vanishEval) external view returns (BN254.ScalarField res);
    function evaluatePiPoly(PolynomialEvalV2.EvalDomain memory domain, uint256[11] memory pi, uint256 zeta, uint256 vanishingPolyEval) external view returns (uint256 res);
    function evaluateVanishingPoly(PolynomialEvalV2.EvalDomain memory domain, uint256 zeta) external pure returns (uint256 res);
    function newEvalDomain(uint256 domainSize) external pure returns (PolynomialEvalV2.EvalDomain memory);
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
    "name": "evalDataGen",
    "inputs": [
      {
        "name": "domain",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalDomain",
        "components": [
          {
            "name": "logSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sizeInv",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "elements",
            "type": "uint256[11]",
            "internalType": "uint256[11]"
          }
        ]
      },
      {
        "name": "zeta",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "publicInput",
        "type": "uint256[11]",
        "internalType": "uint256[11]"
      }
    ],
    "outputs": [
      {
        "name": "evalData",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalData",
        "components": [
          {
            "name": "vanishEval",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "lagrangeOne",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          },
          {
            "name": "piEval",
            "type": "uint256",
            "internalType": "BN254.ScalarField"
          }
        ]
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "evaluateLagrangeOne",
    "inputs": [
      {
        "name": "domain",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalDomain",
        "components": [
          {
            "name": "logSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sizeInv",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "elements",
            "type": "uint256[11]",
            "internalType": "uint256[11]"
          }
        ]
      },
      {
        "name": "zeta",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      },
      {
        "name": "vanishEval",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "outputs": [
      {
        "name": "res",
        "type": "uint256",
        "internalType": "BN254.ScalarField"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "evaluatePiPoly",
    "inputs": [
      {
        "name": "domain",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalDomain",
        "components": [
          {
            "name": "logSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sizeInv",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "elements",
            "type": "uint256[11]",
            "internalType": "uint256[11]"
          }
        ]
      },
      {
        "name": "pi",
        "type": "uint256[11]",
        "internalType": "uint256[11]"
      },
      {
        "name": "zeta",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "vanishingPolyEval",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [
      {
        "name": "res",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "view"
  },
  {
    "type": "function",
    "name": "evaluateVanishingPoly",
    "inputs": [
      {
        "name": "domain",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalDomain",
        "components": [
          {
            "name": "logSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sizeInv",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "elements",
            "type": "uint256[11]",
            "internalType": "uint256[11]"
          }
        ]
      },
      {
        "name": "zeta",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [
      {
        "name": "res",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "stateMutability": "pure"
  },
  {
    "type": "function",
    "name": "newEvalDomain",
    "inputs": [
      {
        "name": "domainSize",
        "type": "uint256",
        "internalType": "uint256"
      }
    ],
    "outputs": [
      {
        "name": "",
        "type": "tuple",
        "internalType": "struct PolynomialEvalV2.EvalDomain",
        "components": [
          {
            "name": "logSize",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "sizeInv",
            "type": "uint256",
            "internalType": "uint256"
          },
          {
            "name": "elements",
            "type": "uint256[11]",
            "internalType": "uint256[11]"
          }
        ]
      }
    ],
    "stateMutability": "pure"
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
    ///0x608060405234801561000f575f80fd5b506128e78061001d5f395ff3fe608060405234801561000f575f80fd5b50600436106100e5575f3560e01c8063a197afc411610088578063bd00369a11610063578063bd00369a1461025d578063de24ac0f14610270578063e3512d5614610297578063f5144326146102be575f80fd5b8063a197afc4146101de578063ab959ee314610213578063af196ba214610236575f80fd5b80635a634f53116100c35780635a634f53146101715780637e6e47b41461018457806382d8a09914610197578063834c452a146101b7575f80fd5b80630c551f3f146100e95780634b4734e3146101235780635a14c0fe1461014a575b5f80fd5b6101107f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02581565b6040519081526020015b60405180910390f35b6101107f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581565b6101107f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a81565b61011061017f366004612366565b6102e5565b61011061019236600461239a565b610356565b6101aa6101a53660046123c5565b6103a7565b60405161011a91906123dc565b6101107f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181565b6101f16101ec366004612422565b61094e565b604080518251815260208084015190820152918101519082015260600161011a565b61022661022136600461263a565b6109ab565b604051901515815260200161011a565b6101107f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081565b61011061026b36600461280f565b610a46565b6101107f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88181565b6101107f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a81565b6101107f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe481565b5f826001036102f65750600161034f565b815f0361030457505f61034f565b60208401515f805160206128bb833981519152905f9082818609905085801561033257600187039250610339565b6001840392505b5061034382610b95565b91508282820993505050505b9392505050565b81515f905f805160206128bb83398151915290838015610397578493505f5b8281101561038b57838586099450600101610375565b5060018403935061039e565b6001830393505b50505092915050565b6103af6121b7565b816201000003610586576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c10018152602001604051806101600160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd81526020017f15ee2475bee517c4ee05e51fa1ee7312a8373a0b13db8c51baf04cb2e99bd2bd81526020017e6fab49b869ae62001deac878b2667bd31bf3e28e3a2d764aa49b8d9bbdd31081526020017f2e856bf6d037708ffa4c06d4d8820f45ccadce9c5a6d178cbd573f82e0f9701181526020017f1407eee35993f2b1ad5ec6d9b8950ca3af33135d06037f871c5e33bf566dd7b48152508152509050919050565b81621000000361075f576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c1018152602001604051806101600160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c81526020017f29e84143f5870d4776a92df8da8c6c9303d59088f37ba85f40cf6fd14265b4bc81526020017f1bf82deba7d74902c3708cc6e70e61f30512eca95655210e276e5858ce8f58e581526020017f22b94b2e2b0043d04e662d5ec018ea1c8a99a23a62c9eb46f0318f6a194985f081526020017f29969d8d5363bef1101a68e446a14e1da7ba9294e142a146a980fddb4d4d41a58152508152509050919050565b81602003610935576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e7508000018152602001604051806101600160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b03904381526020017ee14b6364a47e9c4284a9f80a5fc41cd212b0d4dbf8a5703770a40a9a34399081526020017f30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f70363681526020017f22399c34139bffada8de046aac50c9628e3517a3a452795364e777cd65bb9f4881526020017f2290ee31c482cf92b79b1944db1c0147635e9004db8c3b9d13644bef31ec3bd38152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b61096f60405180606001604052805f81526020015f81526020015f81525090565b6109798484610356565b80825261098990859085906102e5565b6020820152805161099f90859084908690610a46565b60408201529392505050565b5f6109b582610c3b565b6109c5835f5b6020020151610d76565b6109d08360016109bb565b6109db8360026109bb565b6109e68360036109bb565b6109f18360046109bb565b6109fc8360056109bb565b610a078360066109bb565b610a128360076109bb565b610a1d8360086109bb565b610a288360096109bb565b610a3383600a6109bb565b610a3e848484610dd7565b949350505050565b5f5f805160206128bb833981519152828203610abf5760015f5b600b811015610ab457818603610a91578681600b8110610a8257610a82612854565b60200201519350505050610a3e565b8280610a9f57610a9f612868565b60408901516020015183099150600101610a60565b505f92505050610a3e565b610ac76121db565b60408701516001610140838101828152920190805b600b811015610b095760208403935085868a85518903088309808552601f19909301929150600101610adc565b505050505f805f90506001838960408c01515f5b600b811015610b5d578882518a85518c88518a0909098981880896505088898d84518c030886099450602093840193928301929190910190600101610b1d565b50505050809250505f610b6f83610b95565b905060208a015185818909965050848187099550848287099a9950505050505050505050565b5f805f5f805160206128bb833981519152905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f51925081610c345760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c65642100000060448201526064015b60405180910390fd5b5050919050565b8051610c4690610fcb565b610c538160200151610fcb565b610c608160400151610fcb565b610c6d8160600151610fcb565b610c7a8160800151610fcb565b610c878160a00151610fcb565b610c948160c00151610fcb565b610ca18160e00151610fcb565b610caf816101000151610fcb565b610cbd816101200151610fcb565b610ccb816101400151610fcb565b610cd9816101600151610fcb565b610ce7816101800151610fcb565b610cf5816101a00151610d76565b610d03816101c00151610d76565b610d11816101e00151610d76565b610d1f816102000151610d76565b610d2d816102200151610d76565b610d3b816102400151610d76565b610d49816102600151610d76565b610d57816102800151610d76565b610d65816102a00151610d76565b610d73816102c00151610d76565b50565b5f805160206128bb833981519152811080610dd35760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401610c2b565b5050565b5f8360200151600b14610dfd576040516320fa9d8960e11b815260040160405180910390fd5b5f610e09858585611079565b90505f610e18865f01516103a7565b90505f610e2a828460a001518861094e565b9050610e4760405180604001604052805f81526020015f81525090565b604080518082019091525f8082526020820152610e7b876101600151610e768961018001518860e00151611608565b6116a9565b91505f80610e8b8b88878c61174d565b91509150610e9c81610e7684611985565b9250610eb583610e768b61016001518a60a00151611608565b60a08801516040880151602001519194505f805160206128bb833981519152918290820990508160e08a015182099050610ef885610e768d610180015184611608565b94505f60405180608001604052807f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081526020017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181526020017f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581526020017f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe48152509050610fb98782610fac89611985565b610fb4611a22565b611aef565b9e9d5050505050505050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4791159015161561100457505050565b8251602084015182600384858586098509088382830914838210848410161693505050816110745760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610c2b565b505050565b6110b96040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b5f5f805160206128bb8339815191529050604051602081015f815260fe60e01b8152865160c01b6004820152602087015160c01b600c82015261028087015160208201526102a08701516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e087015180516101008301526020810151610120830152506101008701518051610140830152602081015161016083015250610120870151805161018083015260208101516101a08301525061014087015180516101c083015260208101516101e083015250610160870151805161020083015260208101516102208301525061018087015180516102408301526020810151610260830152506101e0870151805161028083015260208101516102a08301525061020087015180516102c083015260208101516102e083015250610220870151805161030083015260208101516103208301525061024087015180516103408301526020810151610360830152506101a0870151805161038083015260208101516103a0830152506101c087015180516103c083015260208101516103e0830152506102608701518051610400830152602081015161042083015250604087015180516104408301526020810151610460830152506060870151805161048083015260208101516104a083015250608087015180516104c083015260208101516104e08301525060a0870151805161050083015260208101516105208301525060c08701518051610540830152602081015161056083015250855161058082015260208601516105a082015260408601516105c082015260608601516105e0820152608086015161060082015260a086015161062082015260c086015161064082015260e08601516106608201526101008601516106808201526101208601516106a08201526101408601516106c0820152845180516106e08301526020810151610700830152506020850151805161072083015260208101516107408301525060408501518051610760830152602081015161078083015250606085015180516107a083015260208101516107c083015250608085015180516107e08301526020810151610800830152505f82526108408220825282825106606085015260208220825282825106608085015260a085015180518252602081015160208301525060608220808352838106855283818209848282099150806020870152508060408601525060c085015180518252602081015160208301525060e085015180516040830152602081015160608301525061010085015180516080830152602081015160a083015250610120850151805160c0830152602081015160e0830152506101408501518051610100830152602081015161012083015250610160822082528282510660a08501526101a085015181526101c085015160208201526101e085015160408201526102008501516060820152610220850151608082015261024085015160a082015261026085015160c082015261028085015160e08201526102a08501516101008201526102c0850151610120820152610160822082528282510660c08501526101608501518051825260208101516020830152506101808501518051604083015260208101516060830152505060a0812082810660e08501525050509392505050565b604080518082019091525f80825260208201526116236121fa565b8351815260208085015190820152604081018390525f60608360808460076107d05a03fa90508080611653575f80fd5b50806116a15760405162461bcd60e51b815260206004820152601960248201527f426e3235343a207363616c6172206d756c206661696c656421000000000000006044820152606401610c2b565b505092915050565b604080518082019091525f80825260208201526116c4612218565b8351815260208085015181830152835160408301528301516060808301919091525f908360c08460066107d05a03fa905080806116ff575f80fd5b50806116a15760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a2067726f7570206164646974696f6e206661696c6564210000006044820152606401610c2b565b604080518082019091525f8082526020820152604080518082019091525f80825260208201525f61178087878787611bcd565b90505f805160206128bb8339815191525f61179c888789612097565b90506117a8818361287c565b60c08901516101a0880151919250908190849081908309840892506117d485610e768a5f015184611608565b955083828209905083846101c08a01518309840892506117fc86610e768a6020015184611608565b955083828209905083846101e08a015183098408925061182486610e768a6040015184611608565b955083828209905083846102008a015183098408925061184c86610e768a6060015184611608565b955083828209905083846102208a015183098408925061187486610e768a6080015184611608565b955083828209905083846102408a015183098408925061189c86610e768d6040015184611608565b955083828209905083846102608a01518309840892506118c486610e768d6060015184611608565b955083828209905083846102808a01518309840892506118ec86610e768d6080015184611608565b955083828209905083846102a08a015183098408925061191486610e768d60a0015184611608565b95505f8a60e00151905084856102c08b015183098508935061193e87610e768b60a0015184611608565b965061197461196e6040805180820182525f80825260209182015281518083019092526001825260029082015290565b85611608565b975050505050505094509492505050565b604080518082019091525f80825260208201528151602083015115901516156119ac575090565b6040518060400160405280835f015181526020017f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4784602001516119f0919061289b565b611a1a907f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4761287c565b905292915050565b611a4960405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f51915080611bc15760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610c2b565b50151595945050505050565b604080518082019091525f80825260208201525f805f805f5f805160206128bb833981519152905060808901518160208a015160208c0151099550895194508160a08b015160608c0151099350816101a089015185089250818184089250818584099450817f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a85099250816101c089015184089250818184089250818584099450817f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02585099250816101e089015184089250818184089250818584099450817f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992508161020089015184089250818184089250818584099450817f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88185099250816102208901518408925081818408925050808483099350808486089450611d3a8760a0015186611608565b9550885160608a015160808b0151838284099750836102c08b015189099750836102408b015183099550836101a08b015187089550838187089550838689099750836102608b015183099550836101c08b015187089550838187089550838689099750836102808b015183099550836101e08b015187089550838187089550838689099750836102a08b015183099550836102008b015187089550838187089550505050808386099450611e0186610e768c60c001518885611dfc919061287c565b611608565b9550611e1a86610e768c60e001518a6101a00151611608565b9550611e3486610e768c61010001518a6101c00151611608565b9550611e4e86610e768c61012001518a6101e00151611608565b9550611e6886610e768c61014001518a6102000151611608565b9550806101c08801516101a0890151099250611e8d86610e768c610160015186611608565b9550806102008801516101e0890151099250611eb286610e768c610180015186611608565b95506101a08701519250808384099150808283099150808284099250611ee186610e768c6101e0015186611608565b95506101c08701519250808384099150808283099150808284099250611f1086610e768c610200015186611608565b95506101e08701519250808384099150808283099150808284099250611f3f86610e768c610220015186611608565b95506102008701519250808384099150808283099150808284099250611f6e86610e768c610240015186611608565b9550611f8b86610e768c6101a00151611dfc8b6102200151612182565b9550611f9c868b6101c001516116a9565b9550806101c08801516101a0890151099250806101e08801518409925080610200880151840992508061022088015184099250611fe286610e768c610260015186611608565b9550611ff0885f0151612182565b945061200486610e768960c0015188611608565b955080600189510860a08a015190935081908009915080828409925080838609945061203886610e768960e0015188611608565b955080838609945061205386610e7689610100015188611608565b955080838609945061206e86610e7689610120015188611608565b955080838609945061208986610e7689610140015188611608565b9a9950505050505050505050565b5f805f805160206128bb83398151915290505f836020015190505f846040015190505f60019050606088015160808901516101a08901516102408a01518788898387098a868608088609945050506101c08901516102608a01518788898387098a868608088609945050506101e08901516102808a01518788898387098a868608088609945050506102008901516102a08a01518788898387098a8686080886099450505061022089015191506102c0890151868782898587080985099350505050875160208901518586868309870385089650508485838309860387089998505050505050505050565b5f61219a5f805160206128bb8339815191528361289b565b6121b1905f805160206128bb83398151915261287c565b92915050565b60405180606001604052805f81526020015f81526020016121d66121db565b905290565b604051806101600160405280600b906020820280368337509192915050565b60405180606001604052806003906020820280368337509192915050565b60405180608001604052806004906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561226e5761226e612236565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561226e5761226e612236565b5f82601f8301126122a7575f80fd5b60405161016080820182811067ffffffffffffffff821117156122cc576122cc612236565b604052830181858211156122de575f80fd5b845b828110156122f85780358252602091820191016122e0565b509195945050505050565b5f6101a08284031215612314575f80fd5b6040516060810181811067ffffffffffffffff8211171561233757612337612236565b8060405250809150823581526020830135602082015261235a8460408501612298565b60408201525092915050565b5f805f6101e08486031215612379575f80fd5b6123838585612303565b956101a085013595506101c0909401359392505050565b5f806101c083850312156123ac575f80fd5b6123b68484612303565b946101a0939093013593505050565b5f602082840312156123d5575f80fd5b5035919050565b81518152602080830151818301526040808401516101a08401929184015f5b600b811015612418578251825291830191908301906001016123fb565b5050505092915050565b5f805f6103208486031215612435575f80fd5b61243f8585612303565b92506101a08401359150612457856101c08601612298565b90509250925092565b5f60408284031215612470575f80fd5b6040516040810181811067ffffffffffffffff8211171561249357612493612236565b604052823581526020928301359281019290925250919050565b5f61048082840312156124be575f80fd5b6124c661224a565b90506124d28383612460565b81526124e18360408401612460565b60208201526124f38360808401612460565b60408201526125058360c08401612460565b606082015261010061251984828501612460565b608083015261014061252d85828601612460565b60a084015261018061254186828701612460565b60c08501526101c061255587828801612460565b60e086015261020061256988828901612460565b85870152610240945061257e88868901612460565b61012087015261028061259389828a01612460565b858801526102c094506125a889868a01612460565b6101608801526125bc896103008a01612460565b848801526103408801356101a0880152610360880135838801526103808801356101e08801526103a0880135828801526103c08801356102208801526103e08801358688015261040088013561026088015261042088013581880152505050506104408401356102a084015261046084013581840152505092915050565b5f805f838503610ae081121561264e575f80fd5b6105008082121561265d575f80fd5b612665612274565b915085358252602086013560208301526126828760408801612460565b60408301526126948760808801612460565b60608301526126a68760c08801612460565b60808301526101006126ba88828901612460565b60a08401526101406126ce89828a01612460565b60c08501526101806126e28a828b01612460565b60e08601526101c06126f68b828c01612460565b84870152610200935061270b8b858c01612460565b6101208701526102406127208c828d01612460565b8488015261028093506127358c858d01612460565b6101608801526127498c6102c08d01612460565b8388015261275b8c6103008d01612460565b6101a088015261276f8c6103408d01612460565b828801526127818c6103808d01612460565b6101e08801526127958c6103c08d01612460565b858801526127a78c6104008d01612460565b6102208801526127bb8c6104408d01612460565b818801525050506127d0896104808a01612460565b6102608501526104c08801358185015250506104e08601356102a08301528194506127fd87828801612298565b935050506124578561066086016124ad565b5f805f806103408587031215612823575f80fd5b61282d8686612303565b935061283d866101a08701612298565b939693955050505061030082013591610320013590565b634e487b7160e01b5f52603260045260245ffd5b634e487b7160e01b5f52601260045260245ffd5b818103818111156121b157634e487b7160e01b5f52601160045260245ffd5b5f826128b557634e487b7160e01b5f52601260045260245ffd5b50069056fe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[Pa(\xE7\x80a\0\x1D_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0\xE5W_5`\xE0\x1C\x80c\xA1\x97\xAF\xC4\x11a\0\x88W\x80c\xBD\x006\x9A\x11a\0cW\x80c\xBD\x006\x9A\x14a\x02]W\x80c\xDE$\xAC\x0F\x14a\x02pW\x80c\xE3Q-V\x14a\x02\x97W\x80c\xF5\x14C&\x14a\x02\xBEW_\x80\xFD[\x80c\xA1\x97\xAF\xC4\x14a\x01\xDEW\x80c\xAB\x95\x9E\xE3\x14a\x02\x13W\x80c\xAF\x19k\xA2\x14a\x026W_\x80\xFD[\x80cZcOS\x11a\0\xC3W\x80cZcOS\x14a\x01qW\x80c~nG\xB4\x14a\x01\x84W\x80c\x82\xD8\xA0\x99\x14a\x01\x97W\x80c\x83LE*\x14a\x01\xB7W_\x80\xFD[\x80c\x0CU\x1F?\x14a\0\xE9W\x80cKG4\xE3\x14a\x01#W\x80cZ\x14\xC0\xFE\x14a\x01JW[_\x80\xFD[a\x01\x10\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x01\x10\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81V[a\x01\x10\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x81V[a\x01\x10a\x01\x7F6`\x04a#fV[a\x02\xE5V[a\x01\x10a\x01\x926`\x04a#\x9AV[a\x03VV[a\x01\xAAa\x01\xA56`\x04a#\xC5V[a\x03\xA7V[`@Qa\x01\x1A\x91\x90a#\xDCV[a\x01\x10\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81V[a\x01\xF1a\x01\xEC6`\x04a$\"V[a\tNV[`@\x80Q\x82Q\x81R` \x80\x84\x01Q\x90\x82\x01R\x91\x81\x01Q\x90\x82\x01R``\x01a\x01\x1AV[a\x02&a\x02!6`\x04a&:V[a\t\xABV[`@Q\x90\x15\x15\x81R` \x01a\x01\x1AV[a\x01\x10\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81V[a\x01\x10a\x02k6`\x04a(\x0FV[a\nFV[a\x01\x10\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x81V[a\x01\x10\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x81V[a\x01\x10\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81V[_\x82`\x01\x03a\x02\xF6WP`\x01a\x03OV[\x81_\x03a\x03\x04WP_a\x03OV[` \x84\x01Q_\x80Q` a(\xBB\x839\x81Q\x91R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x032W`\x01\x87\x03\x92Pa\x039V[`\x01\x84\x03\x92P[Pa\x03C\x82a\x0B\x95V[\x91P\x82\x82\x82\t\x93PPPP[\x93\x92PPPV[\x81Q_\x90_\x80Q` a(\xBB\x839\x81Q\x91R\x90\x83\x80\x15a\x03\x97W\x84\x93P_[\x82\x81\x10\x15a\x03\x8BW\x83\x85\x86\t\x94P`\x01\x01a\x03uV[P`\x01\x84\x03\x93Pa\x03\x9EV[`\x01\x83\x03\x93P[PPP\x92\x91PPV[a\x03\xAFa!\xB7V[\x81b\x01\0\0\x03a\x05\x86W`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81R` \x01\x7F\x15\xEE$u\xBE\xE5\x17\xC4\xEE\x05\xE5\x1F\xA1\xEEs\x12\xA87:\x0B\x13\xDB\x8CQ\xBA\xF0L\xB2\xE9\x9B\xD2\xBD\x81R` \x01~o\xABI\xB8i\xAEb\0\x1D\xEA\xC8x\xB2f{\xD3\x1B\xF3\xE2\x8E:-vJ\xA4\x9B\x8D\x9B\xBD\xD3\x10\x81R` \x01\x7F.\x85k\xF6\xD07p\x8F\xFAL\x06\xD4\xD8\x82\x0FE\xCC\xAD\xCE\x9CZm\x17\x8C\xBDW?\x82\xE0\xF9p\x11\x81R` \x01\x7F\x14\x07\xEE\xE3Y\x93\xF2\xB1\xAD^\xC6\xD9\xB8\x95\x0C\xA3\xAF3\x13]\x06\x03\x7F\x87\x1C^3\xBFVm\xD7\xB4\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x07_W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81R` \x01\x7F)\xE8AC\xF5\x87\rGv\xA9-\xF8\xDA\x8Cl\x93\x03\xD5\x90\x88\xF3{\xA8_@\xCFo\xD1Be\xB4\xBC\x81R` \x01\x7F\x1B\xF8-\xEB\xA7\xD7I\x02\xC3p\x8C\xC6\xE7\x0Ea\xF3\x05\x12\xEC\xA9VU!\x0E'nXX\xCE\x8FX\xE5\x81R` \x01\x7F\"\xB9K.+\0C\xD0Nf-^\xC0\x18\xEA\x1C\x8A\x99\xA2:b\xC9\xEBF\xF01\x8Fj\x19I\x85\xF0\x81R` \x01\x7F)\x96\x9D\x8DSc\xBE\xF1\x10\x1Ah\xE4F\xA1N\x1D\xA7\xBA\x92\x94\xE1B\xA1F\xA9\x80\xFD\xDBMMA\xA5\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\t5W`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81R` \x01~\xE1Kcd\xA4~\x9CB\x84\xA9\xF8\n_\xC4\x1C\xD2\x12\xB0\xD4\xDB\xF8\xA5p7p\xA4\n\x9A49\x90\x81R` \x01\x7F0dNr\xE11\xA0)\x04\x8Bn\x19?\xD8A\x04\\\xEA$\xF6\xFDsk\xEC#\x12\x04p\x8Fp66\x81R` \x01\x7F\"9\x9C4\x13\x9B\xFF\xAD\xA8\xDE\x04j\xACP\xC9b\x8E5\x17\xA3\xA4RySd\xE7w\xCDe\xBB\x9FH\x81R` \x01\x7F\"\x90\xEE1\xC4\x82\xCF\x92\xB7\x9B\x19D\xDB\x1C\x01Gc^\x90\x04\xDB\x8C;\x9D\x13dK\xEF1\xEC;\xD3\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\to`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\ty\x84\x84a\x03VV[\x80\x82Ra\t\x89\x90\x85\x90\x85\x90a\x02\xE5V[` \x82\x01R\x80Qa\t\x9F\x90\x85\x90\x84\x90\x86\x90a\nFV[`@\x82\x01R\x93\x92PPPV[_a\t\xB5\x82a\x0C;V[a\t\xC5\x83_[` \x02\x01Qa\rvV[a\t\xD0\x83`\x01a\t\xBBV[a\t\xDB\x83`\x02a\t\xBBV[a\t\xE6\x83`\x03a\t\xBBV[a\t\xF1\x83`\x04a\t\xBBV[a\t\xFC\x83`\x05a\t\xBBV[a\n\x07\x83`\x06a\t\xBBV[a\n\x12\x83`\x07a\t\xBBV[a\n\x1D\x83`\x08a\t\xBBV[a\n(\x83`\ta\t\xBBV[a\n3\x83`\na\t\xBBV[a\n>\x84\x84\x84a\r\xD7V[\x94\x93PPPPV[__\x80Q` a(\xBB\x839\x81Q\x91R\x82\x82\x03a\n\xBFW`\x01_[`\x0B\x81\x10\x15a\n\xB4W\x81\x86\x03a\n\x91W\x86\x81`\x0B\x81\x10a\n\x82Wa\n\x82a(TV[` \x02\x01Q\x93PPPPa\n>V[\x82\x80a\n\x9FWa\n\x9Fa(hV[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\n`V[P_\x92PPPa\n>V[a\n\xC7a!\xDBV[`@\x87\x01Q`\x01a\x01@\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x0B\x81\x10\x15a\x0B\tW` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\n\xDCV[PPPP_\x80_\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x0B\x81\x10\x15a\x0B]W\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x0B\x1DV[PPPP\x80\x92PP_a\x0Bo\x83a\x0B\x95V[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x9A\x99PPPPPPPPPPV[_\x80__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a\x0C4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[PP\x91\x90PV[\x80Qa\x0CF\x90a\x0F\xCBV[a\x0CS\x81` \x01Qa\x0F\xCBV[a\x0C`\x81`@\x01Qa\x0F\xCBV[a\x0Cm\x81``\x01Qa\x0F\xCBV[a\x0Cz\x81`\x80\x01Qa\x0F\xCBV[a\x0C\x87\x81`\xA0\x01Qa\x0F\xCBV[a\x0C\x94\x81`\xC0\x01Qa\x0F\xCBV[a\x0C\xA1\x81`\xE0\x01Qa\x0F\xCBV[a\x0C\xAF\x81a\x01\0\x01Qa\x0F\xCBV[a\x0C\xBD\x81a\x01 \x01Qa\x0F\xCBV[a\x0C\xCB\x81a\x01@\x01Qa\x0F\xCBV[a\x0C\xD9\x81a\x01`\x01Qa\x0F\xCBV[a\x0C\xE7\x81a\x01\x80\x01Qa\x0F\xCBV[a\x0C\xF5\x81a\x01\xA0\x01Qa\rvV[a\r\x03\x81a\x01\xC0\x01Qa\rvV[a\r\x11\x81a\x01\xE0\x01Qa\rvV[a\r\x1F\x81a\x02\0\x01Qa\rvV[a\r-\x81a\x02 \x01Qa\rvV[a\r;\x81a\x02@\x01Qa\rvV[a\rI\x81a\x02`\x01Qa\rvV[a\rW\x81a\x02\x80\x01Qa\rvV[a\re\x81a\x02\xA0\x01Qa\rvV[a\rs\x81a\x02\xC0\x01Qa\rvV[PV[_\x80Q` a(\xBB\x839\x81Q\x91R\x81\x10\x80a\r\xD3W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PPV[_\x83` \x01Q`\x0B\x14a\r\xFDW`@Qc \xFA\x9D\x89`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x0E\t\x85\x85\x85a\x10yV[\x90P_a\x0E\x18\x86_\x01Qa\x03\xA7V[\x90P_a\x0E*\x82\x84`\xA0\x01Q\x88a\tNV[\x90Pa\x0EG`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0E{\x87a\x01`\x01Qa\x0Ev\x89a\x01\x80\x01Q\x88`\xE0\x01Qa\x16\x08V[a\x16\xA9V[\x91P_\x80a\x0E\x8B\x8B\x88\x87\x8Ca\x17MV[\x91P\x91Pa\x0E\x9C\x81a\x0Ev\x84a\x19\x85V[\x92Pa\x0E\xB5\x83a\x0Ev\x8Ba\x01`\x01Q\x8A`\xA0\x01Qa\x16\x08V[`\xA0\x88\x01Q`@\x88\x01Q` \x01Q\x91\x94P_\x80Q` a(\xBB\x839\x81Q\x91R\x91\x82\x90\x82\t\x90P\x81`\xE0\x8A\x01Q\x82\t\x90Pa\x0E\xF8\x85a\x0Ev\x8Da\x01\x80\x01Q\x84a\x16\x08V[\x94P_`@Q\x80`\x80\x01`@R\x80\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81R` \x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81R` \x01\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81R` \x01\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81RP\x90Pa\x0F\xB9\x87\x82a\x0F\xAC\x89a\x19\x85V[a\x0F\xB4a\x1A\"V[a\x1A\xEFV[\x9E\x9DPPPPPPPPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x10\x04WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x10tW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PPPV[a\x10\xB9`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x86Q`\xC0\x1B`\x04\x82\x01R` \x87\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x87\x01Q` \x82\x01Ra\x02\xA0\x87\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x87\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x87\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x87\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x87\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x87\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x87\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x87\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x87\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x87\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x87\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x87\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x87\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x87\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x87\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x87\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x87\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x87\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x87\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x85Qa\x05\x80\x82\x01R` \x86\x01Qa\x05\xA0\x82\x01R`@\x86\x01Qa\x05\xC0\x82\x01R``\x86\x01Qa\x05\xE0\x82\x01R`\x80\x86\x01Qa\x06\0\x82\x01R`\xA0\x86\x01Qa\x06 \x82\x01R`\xC0\x86\x01Qa\x06@\x82\x01R`\xE0\x86\x01Qa\x06`\x82\x01Ra\x01\0\x86\x01Qa\x06\x80\x82\x01Ra\x01 \x86\x01Qa\x06\xA0\x82\x01Ra\x01@\x86\x01Qa\x06\xC0\x82\x01R\x84Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP` \x85\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`@\x85\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP``\x85\x01Q\x80Qa\x07\xA0\x83\x01R` \x81\x01Qa\x07\xC0\x83\x01RP`\x80\x85\x01Q\x80Qa\x07\xE0\x83\x01R` \x81\x01Qa\x08\0\x83\x01RP_\x82Ra\x08@\x82 \x82R\x82\x82Q\x06``\x85\x01R` \x82 \x82R\x82\x82Q\x06`\x80\x85\x01R`\xA0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 \x80\x83R\x83\x81\x06\x85R\x83\x81\x82\t\x84\x82\x82\t\x91P\x80` \x87\x01RP\x80`@\x86\x01RP`\xC0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x85\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x85\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x85\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 \x82R\x82\x82Q\x06`\xA0\x85\x01Ra\x01\xA0\x85\x01Q\x81Ra\x01\xC0\x85\x01Q` \x82\x01Ra\x01\xE0\x85\x01Q`@\x82\x01Ra\x02\0\x85\x01Q``\x82\x01Ra\x02 \x85\x01Q`\x80\x82\x01Ra\x02@\x85\x01Q`\xA0\x82\x01Ra\x02`\x85\x01Q`\xC0\x82\x01Ra\x02\x80\x85\x01Q`\xE0\x82\x01Ra\x02\xA0\x85\x01Qa\x01\0\x82\x01Ra\x02\xC0\x85\x01Qa\x01 \x82\x01Ra\x01`\x82 \x82R\x82\x82Q\x06`\xC0\x85\x01Ra\x01`\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 \x82\x81\x06`\xE0\x85\x01RPPP\x93\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16#a!\xFAV[\x83Q\x81R` \x80\x85\x01Q\x90\x82\x01R`@\x81\x01\x83\x90R_``\x83`\x80\x84`\x07a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x16SW_\x80\xFD[P\x80a\x16\xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x19`$\x82\x01R\x7FBn254: scalar mul failed!\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PP\x92\x91PPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16\xC4a\"\x18V[\x83Q\x81R` \x80\x85\x01Q\x81\x83\x01R\x83Q`@\x83\x01R\x83\x01Q``\x80\x83\x01\x91\x90\x91R_\x90\x83`\xC0\x84`\x06a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x16\xFFW_\x80\xFD[P\x80a\x16\xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: group addition failed!\0\0\0`D\x82\x01R`d\x01a\x0C+V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x17\x80\x87\x87\x87\x87a\x1B\xCDV[\x90P_\x80Q` a(\xBB\x839\x81Q\x91R_a\x17\x9C\x88\x87\x89a \x97V[\x90Pa\x17\xA8\x81\x83a(|V[`\xC0\x89\x01Qa\x01\xA0\x88\x01Q\x91\x92P\x90\x81\x90\x84\x90\x81\x90\x83\t\x84\x08\x92Pa\x17\xD4\x85a\x0Ev\x8A_\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xC0\x8A\x01Q\x83\t\x84\x08\x92Pa\x17\xFC\x86a\x0Ev\x8A` \x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xE0\x8A\x01Q\x83\t\x84\x08\x92Pa\x18$\x86a\x0Ev\x8A`@\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\0\x8A\x01Q\x83\t\x84\x08\x92Pa\x18L\x86a\x0Ev\x8A``\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02 \x8A\x01Q\x83\t\x84\x08\x92Pa\x18t\x86a\x0Ev\x8A`\x80\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02@\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\x9C\x86a\x0Ev\x8D`@\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02`\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\xC4\x86a\x0Ev\x8D``\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\x80\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\xEC\x86a\x0Ev\x8D`\x80\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\xA0\x8A\x01Q\x83\t\x84\x08\x92Pa\x19\x14\x86a\x0Ev\x8D`\xA0\x01Q\x84a\x16\x08V[\x95P_\x8A`\xE0\x01Q\x90P\x84\x85a\x02\xC0\x8B\x01Q\x83\t\x85\x08\x93Pa\x19>\x87a\x0Ev\x8B`\xA0\x01Q\x84a\x16\x08V[\x96Pa\x19ta\x19n`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01R\x81Q\x80\x83\x01\x90\x92R`\x01\x82R`\x02\x90\x82\x01R\x90V[\x85a\x16\x08V[\x97PPPPPPP\x94P\x94\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x19\xACWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x84` \x01Qa\x19\xF0\x91\x90a(\x9BV[a\x1A\x1A\x90\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDGa(|V[\x90R\x92\x91PPV[a\x1AI`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x1B\xC1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[P\x15\x15\x95\x94PPPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_\x80_\x80__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`\x80\x89\x01Q\x81` \x8A\x01Q` \x8C\x01Q\t\x95P\x89Q\x94P\x81`\xA0\x8B\x01Q``\x8C\x01Q\t\x93P\x81a\x01\xA0\x89\x01Q\x85\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P\x81a\x01\xC0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P\x81a\x01\xE0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P\x81a\x02\0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P\x81a\x02 \x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92PP\x80\x84\x83\t\x93P\x80\x84\x86\x08\x94Pa\x1D:\x87`\xA0\x01Q\x86a\x16\x08V[\x95P\x88Q``\x8A\x01Q`\x80\x8B\x01Q\x83\x82\x84\t\x97P\x83a\x02\xC0\x8B\x01Q\x89\t\x97P\x83a\x02@\x8B\x01Q\x83\t\x95P\x83a\x01\xA0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02`\x8B\x01Q\x83\t\x95P\x83a\x01\xC0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\x80\x8B\x01Q\x83\t\x95P\x83a\x01\xE0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\xA0\x8B\x01Q\x83\t\x95P\x83a\x02\0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95PPPP\x80\x83\x86\t\x94Pa\x1E\x01\x86a\x0Ev\x8C`\xC0\x01Q\x88\x85a\x1D\xFC\x91\x90a(|V[a\x16\x08V[\x95Pa\x1E\x1A\x86a\x0Ev\x8C`\xE0\x01Q\x8Aa\x01\xA0\x01Qa\x16\x08V[\x95Pa\x1E4\x86a\x0Ev\x8Ca\x01\0\x01Q\x8Aa\x01\xC0\x01Qa\x16\x08V[\x95Pa\x1EN\x86a\x0Ev\x8Ca\x01 \x01Q\x8Aa\x01\xE0\x01Qa\x16\x08V[\x95Pa\x1Eh\x86a\x0Ev\x8Ca\x01@\x01Q\x8Aa\x02\0\x01Qa\x16\x08V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92Pa\x1E\x8D\x86a\x0Ev\x8Ca\x01`\x01Q\x86a\x16\x08V[\x95P\x80a\x02\0\x88\x01Qa\x01\xE0\x89\x01Q\t\x92Pa\x1E\xB2\x86a\x0Ev\x8Ca\x01\x80\x01Q\x86a\x16\x08V[\x95Pa\x01\xA0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1E\xE1\x86a\x0Ev\x8Ca\x01\xE0\x01Q\x86a\x16\x08V[\x95Pa\x01\xC0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1F\x10\x86a\x0Ev\x8Ca\x02\0\x01Q\x86a\x16\x08V[\x95Pa\x01\xE0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1F?\x86a\x0Ev\x8Ca\x02 \x01Q\x86a\x16\x08V[\x95Pa\x02\0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1Fn\x86a\x0Ev\x8Ca\x02@\x01Q\x86a\x16\x08V[\x95Pa\x1F\x8B\x86a\x0Ev\x8Ca\x01\xA0\x01Qa\x1D\xFC\x8Ba\x02 \x01Qa!\x82V[\x95Pa\x1F\x9C\x86\x8Ba\x01\xC0\x01Qa\x16\xA9V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92P\x80a\x01\xE0\x88\x01Q\x84\t\x92P\x80a\x02\0\x88\x01Q\x84\t\x92P\x80a\x02 \x88\x01Q\x84\t\x92Pa\x1F\xE2\x86a\x0Ev\x8Ca\x02`\x01Q\x86a\x16\x08V[\x95Pa\x1F\xF0\x88_\x01Qa!\x82V[\x94Pa \x04\x86a\x0Ev\x89`\xC0\x01Q\x88a\x16\x08V[\x95P\x80`\x01\x89Q\x08`\xA0\x8A\x01Q\x90\x93P\x81\x90\x80\t\x91P\x80\x82\x84\t\x92P\x80\x83\x86\t\x94Pa 8\x86a\x0Ev\x89`\xE0\x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa S\x86a\x0Ev\x89a\x01\0\x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa n\x86a\x0Ev\x89a\x01 \x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa \x89\x86a\x0Ev\x89a\x01@\x01Q\x88a\x16\x08V[\x9A\x99PPPPPPPPPPV[_\x80_\x80Q` a(\xBB\x839\x81Q\x91R\x90P_\x83` \x01Q\x90P_\x84`@\x01Q\x90P_`\x01\x90P``\x88\x01Q`\x80\x89\x01Qa\x01\xA0\x89\x01Qa\x02@\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x89\x01Qa\x02`\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x89\x01Qa\x02\x80\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x89\x01Qa\x02\xA0\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x89\x01Q\x91Pa\x02\xC0\x89\x01Q\x86\x87\x82\x89\x85\x87\x08\t\x85\t\x93PPPP\x87Q` \x89\x01Q\x85\x86\x86\x83\t\x87\x03\x85\x08\x96PP\x84\x85\x83\x83\t\x86\x03\x87\x08\x99\x98PPPPPPPPPV[_a!\x9A_\x80Q` a(\xBB\x839\x81Q\x91R\x83a(\x9BV[a!\xB1\x90_\x80Q` a(\xBB\x839\x81Q\x91Ra(|V[\x92\x91PPV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!\xD6a!\xDBV[\x90R\x90V[`@Q\x80a\x01`\x01`@R\x80`\x0B\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80``\x01`@R\x80`\x03\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80`\x80\x01`@R\x80`\x04\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"nWa\"na\"6V[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"nWa\"na\"6V[_\x82`\x1F\x83\x01\x12a\"\xA7W_\x80\xFD[`@Qa\x01`\x80\x82\x01\x82\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\xCCWa\"\xCCa\"6V[`@R\x83\x01\x81\x85\x82\x11\x15a\"\xDEW_\x80\xFD[\x84[\x82\x81\x10\x15a\"\xF8W\x805\x82R` \x91\x82\x01\x91\x01a\"\xE0V[P\x91\x95\x94PPPPPV[_a\x01\xA0\x82\x84\x03\x12\x15a#\x14W_\x80\xFD[`@Q``\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a#7Wa#7a\"6V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01Ra#Z\x84`@\x85\x01a\"\x98V[`@\x82\x01RP\x92\x91PPV[_\x80_a\x01\xE0\x84\x86\x03\x12\x15a#yW_\x80\xFD[a#\x83\x85\x85a#\x03V[\x95a\x01\xA0\x85\x015\x95Pa\x01\xC0\x90\x94\x015\x93\x92PPPV[_\x80a\x01\xC0\x83\x85\x03\x12\x15a#\xACW_\x80\xFD[a#\xB6\x84\x84a#\x03V[\x94a\x01\xA0\x93\x90\x93\x015\x93PPPV[_` \x82\x84\x03\x12\x15a#\xD5W_\x80\xFD[P5\x91\x90PV[\x81Q\x81R` \x80\x83\x01Q\x81\x83\x01R`@\x80\x84\x01Qa\x01\xA0\x84\x01\x92\x91\x84\x01_[`\x0B\x81\x10\x15a$\x18W\x82Q\x82R\x91\x83\x01\x91\x90\x83\x01\x90`\x01\x01a#\xFBV[PPPP\x92\x91PPV[_\x80_a\x03 \x84\x86\x03\x12\x15a$5W_\x80\xFD[a$?\x85\x85a#\x03V[\x92Pa\x01\xA0\x84\x015\x91Pa$W\x85a\x01\xC0\x86\x01a\"\x98V[\x90P\x92P\x92P\x92V[_`@\x82\x84\x03\x12\x15a$pW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a$\x93Wa$\x93a\"6V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a$\xBEW_\x80\xFD[a$\xC6a\"JV[\x90Pa$\xD2\x83\x83a$`V[\x81Ra$\xE1\x83`@\x84\x01a$`V[` \x82\x01Ra$\xF3\x83`\x80\x84\x01a$`V[`@\x82\x01Ra%\x05\x83`\xC0\x84\x01a$`V[``\x82\x01Ra\x01\0a%\x19\x84\x82\x85\x01a$`V[`\x80\x83\x01Ra\x01@a%-\x85\x82\x86\x01a$`V[`\xA0\x84\x01Ra\x01\x80a%A\x86\x82\x87\x01a$`V[`\xC0\x85\x01Ra\x01\xC0a%U\x87\x82\x88\x01a$`V[`\xE0\x86\x01Ra\x02\0a%i\x88\x82\x89\x01a$`V[\x85\x87\x01Ra\x02@\x94Pa%~\x88\x86\x89\x01a$`V[a\x01 \x87\x01Ra\x02\x80a%\x93\x89\x82\x8A\x01a$`V[\x85\x88\x01Ra\x02\xC0\x94Pa%\xA8\x89\x86\x8A\x01a$`V[a\x01`\x88\x01Ra%\xBC\x89a\x03\0\x8A\x01a$`V[\x84\x88\x01Ra\x03@\x88\x015a\x01\xA0\x88\x01Ra\x03`\x88\x015\x83\x88\x01Ra\x03\x80\x88\x015a\x01\xE0\x88\x01Ra\x03\xA0\x88\x015\x82\x88\x01Ra\x03\xC0\x88\x015a\x02 \x88\x01Ra\x03\xE0\x88\x015\x86\x88\x01Ra\x04\0\x88\x015a\x02`\x88\x01Ra\x04 \x88\x015\x81\x88\x01RPPPPa\x04@\x84\x015a\x02\xA0\x84\x01Ra\x04`\x84\x015\x81\x84\x01RPP\x92\x91PPV[_\x80_\x83\x85\x03a\n\xE0\x81\x12\x15a&NW_\x80\xFD[a\x05\0\x80\x82\x12\x15a&]W_\x80\xFD[a&ea\"tV[\x91P\x855\x82R` \x86\x015` \x83\x01Ra&\x82\x87`@\x88\x01a$`V[`@\x83\x01Ra&\x94\x87`\x80\x88\x01a$`V[``\x83\x01Ra&\xA6\x87`\xC0\x88\x01a$`V[`\x80\x83\x01Ra\x01\0a&\xBA\x88\x82\x89\x01a$`V[`\xA0\x84\x01Ra\x01@a&\xCE\x89\x82\x8A\x01a$`V[`\xC0\x85\x01Ra\x01\x80a&\xE2\x8A\x82\x8B\x01a$`V[`\xE0\x86\x01Ra\x01\xC0a&\xF6\x8B\x82\x8C\x01a$`V[\x84\x87\x01Ra\x02\0\x93Pa'\x0B\x8B\x85\x8C\x01a$`V[a\x01 \x87\x01Ra\x02@a' \x8C\x82\x8D\x01a$`V[\x84\x88\x01Ra\x02\x80\x93Pa'5\x8C\x85\x8D\x01a$`V[a\x01`\x88\x01Ra'I\x8Ca\x02\xC0\x8D\x01a$`V[\x83\x88\x01Ra'[\x8Ca\x03\0\x8D\x01a$`V[a\x01\xA0\x88\x01Ra'o\x8Ca\x03@\x8D\x01a$`V[\x82\x88\x01Ra'\x81\x8Ca\x03\x80\x8D\x01a$`V[a\x01\xE0\x88\x01Ra'\x95\x8Ca\x03\xC0\x8D\x01a$`V[\x85\x88\x01Ra'\xA7\x8Ca\x04\0\x8D\x01a$`V[a\x02 \x88\x01Ra'\xBB\x8Ca\x04@\x8D\x01a$`V[\x81\x88\x01RPPPa'\xD0\x89a\x04\x80\x8A\x01a$`V[a\x02`\x85\x01Ra\x04\xC0\x88\x015\x81\x85\x01RPPa\x04\xE0\x86\x015a\x02\xA0\x83\x01R\x81\x94Pa'\xFD\x87\x82\x88\x01a\"\x98V[\x93PPPa$W\x85a\x06`\x86\x01a$\xADV[_\x80_\x80a\x03@\x85\x87\x03\x12\x15a(#W_\x80\xFD[a(-\x86\x86a#\x03V[\x93Pa(=\x86a\x01\xA0\x87\x01a\"\x98V[\x93\x96\x93\x95PPPPa\x03\0\x82\x015\x91a\x03 \x015\x90V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a!\xB1WcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[_\x82a(\xB5WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x17\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405234801561000f575f80fd5b50600436106100e5575f3560e01c8063a197afc411610088578063bd00369a11610063578063bd00369a1461025d578063de24ac0f14610270578063e3512d5614610297578063f5144326146102be575f80fd5b8063a197afc4146101de578063ab959ee314610213578063af196ba214610236575f80fd5b80635a634f53116100c35780635a634f53146101715780637e6e47b41461018457806382d8a09914610197578063834c452a146101b7575f80fd5b80630c551f3f146100e95780634b4734e3146101235780635a14c0fe1461014a575b5f80fd5b6101107f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02581565b6040519081526020015b60405180910390f35b6101107f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581565b6101107f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a81565b61011061017f366004612366565b6102e5565b61011061019236600461239a565b610356565b6101aa6101a53660046123c5565b6103a7565b60405161011a91906123dc565b6101107f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181565b6101f16101ec366004612422565b61094e565b604080518251815260208084015190820152918101519082015260600161011a565b61022661022136600461263a565b6109ab565b604051901515815260200161011a565b6101107f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081565b61011061026b36600461280f565b610a46565b6101107f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88181565b6101107f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a81565b6101107f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe481565b5f826001036102f65750600161034f565b815f0361030457505f61034f565b60208401515f805160206128bb833981519152905f9082818609905085801561033257600187039250610339565b6001840392505b5061034382610b95565b91508282820993505050505b9392505050565b81515f905f805160206128bb83398151915290838015610397578493505f5b8281101561038b57838586099450600101610375565b5060018403935061039e565b6001830393505b50505092915050565b6103af6121b7565b816201000003610586576040518060600160405280601081526020017f30641e0e92bebef818268d663bcad6dbcfd6c0149170f6d7d350b1b1fa6c10018152602001604051806101600160405280600181526020017eeeb2cb5981ed45649abebde081dcff16c8601de4347e7dd1628ba2daac43b781526020017f2d1ba66f5941dc91017171fa69ec2bd0022a2a2d4115a009a93458fd4e26ecfb81526020017f086812a00ac43ea801669c640171203c41a496671bfbc065ac8db24d52cf31e581526020017f2d965651cdd9e4811f4e51b80ddca8a8b4a93ee17420aae6adaa01c2617c6e8581526020017f12597a56c2e438620b9041b98992ae0d4e705b780057bf7766a2767cece16e1d81526020017f02d94117cd17bcf1290fd67c01155dd40807857dff4a5a0b4dc67befa8aa34fd81526020017f15ee2475bee517c4ee05e51fa1ee7312a8373a0b13db8c51baf04cb2e99bd2bd81526020017e6fab49b869ae62001deac878b2667bd31bf3e28e3a2d764aa49b8d9bbdd31081526020017f2e856bf6d037708ffa4c06d4d8820f45ccadce9c5a6d178cbd573f82e0f9701181526020017f1407eee35993f2b1ad5ec6d9b8950ca3af33135d06037f871c5e33bf566dd7b48152508152509050919050565b81621000000361075f576040518060600160405280601481526020017f30644b6c9c4a72169e4daa317d25f04512ae15c53b34e8f5acd8e155d0a6c1018152602001604051806101600160405280600181526020017f26125da10a0ed06327508aba06d1e303ac616632dbed349f53422da95333785781526020017f2260e724844bca5251829353968e4915305258418357473a5c1d597f613f6cbd81526020017f2087ea2cd664278608fb0ebdb820907f598502c81b6690c185e2bf15cb935f4281526020017f19ddbcaf3a8d46c15c0176fbb5b95e4dc57088ff13f4d1bd84c6bfa57dcdc0e081526020017f05a2c85cfc591789605cae818e37dd4161eef9aa666bec6fe4288d09e6d2341881526020017f11f70e5363258ff4f0d716a653e1dc41f1c64484d7f4b6e219d6377614a3905c81526020017f29e84143f5870d4776a92df8da8c6c9303d59088f37ba85f40cf6fd14265b4bc81526020017f1bf82deba7d74902c3708cc6e70e61f30512eca95655210e276e5858ce8f58e581526020017f22b94b2e2b0043d04e662d5ec018ea1c8a99a23a62c9eb46f0318f6a194985f081526020017f29969d8d5363bef1101a68e446a14e1da7ba9294e142a146a980fddb4d4d41a58152508152509050919050565b81602003610935576040518060600160405280600581526020017f2ee12bff4a2813286a8dc388cd754d9a3ef2490635eba50cb9c2e5e7508000018152602001604051806101600160405280600181526020017f09c532c6306b93d29678200d47c0b2a99c18d51b838eeb1d3eed4c533bb512d081526020017f21082ca216cbbf4e1c6e4f4594dd508c996dfbe1174efb98b11509c6e306460b81526020017f1277ae6415f0ef18f2ba5fb162c39eb7311f386e2d26d64401f4a25da77c253b81526020017f2b337de1c8c14f22ec9b9e2f96afef3652627366f8170a0a948dad4ac1bd5e8081526020017f2fbd4dd2976be55d1a163aa9820fb88dfac5ddce77e1872e90632027327a5ebe81526020017f107aab49e65a67f9da9cd2abf78be38bd9dc1d5db39f81de36bcfa5b4b03904381526020017ee14b6364a47e9c4284a9f80a5fc41cd212b0d4dbf8a5703770a40a9a34399081526020017f30644e72e131a029048b6e193fd841045cea24f6fd736bec231204708f70363681526020017f22399c34139bffada8de046aac50c9628e3517a3a452795364e777cd65bb9f4881526020017f2290ee31c482cf92b79b1944db1c0147635e9004db8c3b9d13644bef31ec3bd38152508152509050919050565b60405163e2ef09e560e01b815260040160405180910390fd5b61096f60405180606001604052805f81526020015f81526020015f81525090565b6109798484610356565b80825261098990859085906102e5565b6020820152805161099f90859084908690610a46565b60408201529392505050565b5f6109b582610c3b565b6109c5835f5b6020020151610d76565b6109d08360016109bb565b6109db8360026109bb565b6109e68360036109bb565b6109f18360046109bb565b6109fc8360056109bb565b610a078360066109bb565b610a128360076109bb565b610a1d8360086109bb565b610a288360096109bb565b610a3383600a6109bb565b610a3e848484610dd7565b949350505050565b5f5f805160206128bb833981519152828203610abf5760015f5b600b811015610ab457818603610a91578681600b8110610a8257610a82612854565b60200201519350505050610a3e565b8280610a9f57610a9f612868565b60408901516020015183099150600101610a60565b505f92505050610a3e565b610ac76121db565b60408701516001610140838101828152920190805b600b811015610b095760208403935085868a85518903088309808552601f19909301929150600101610adc565b505050505f805f90506001838960408c01515f5b600b811015610b5d578882518a85518c88518a0909098981880896505088898d84518c030886099450602093840193928301929190910190600101610b1d565b50505050809250505f610b6f83610b95565b905060208a015185818909965050848187099550848287099a9950505050505050505050565b5f805f5f805160206128bb833981519152905060405160208152602080820152602060408201528460608201526002820360808201528160a082015260205f60c08360055afa9250505f51925081610c345760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a20706f7720707265636f6d70696c65206661696c65642100000060448201526064015b60405180910390fd5b5050919050565b8051610c4690610fcb565b610c538160200151610fcb565b610c608160400151610fcb565b610c6d8160600151610fcb565b610c7a8160800151610fcb565b610c878160a00151610fcb565b610c948160c00151610fcb565b610ca18160e00151610fcb565b610caf816101000151610fcb565b610cbd816101200151610fcb565b610ccb816101400151610fcb565b610cd9816101600151610fcb565b610ce7816101800151610fcb565b610cf5816101a00151610d76565b610d03816101c00151610d76565b610d11816101e00151610d76565b610d1f816102000151610d76565b610d2d816102200151610d76565b610d3b816102400151610d76565b610d49816102600151610d76565b610d57816102800151610d76565b610d65816102a00151610d76565b610d73816102c00151610d76565b50565b5f805160206128bb833981519152811080610dd35760405162461bcd60e51b815260206004820152601b60248201527f426e3235343a20696e76616c6964207363616c6172206669656c6400000000006044820152606401610c2b565b5050565b5f8360200151600b14610dfd576040516320fa9d8960e11b815260040160405180910390fd5b5f610e09858585611079565b90505f610e18865f01516103a7565b90505f610e2a828460a001518861094e565b9050610e4760405180604001604052805f81526020015f81525090565b604080518082019091525f8082526020820152610e7b876101600151610e768961018001518860e00151611608565b6116a9565b91505f80610e8b8b88878c61174d565b91509150610e9c81610e7684611985565b9250610eb583610e768b61016001518a60a00151611608565b60a08801516040880151602001519194505f805160206128bb833981519152918290820990508160e08a015182099050610ef885610e768d610180015184611608565b94505f60405180608001604052807f0118c4d5b837bcc2bc89b5b398b5974e9f5944073b32078b7e231fec938883b081526020017f260e01b251f6f1c7e7ff4e580791dee8ea51d87a358e038b4efe30fac09383c181526020017f22febda3c0c0632a56475b4214e5615e11e6dd3f96e6cea2854a87d4dacc5e5581526020017f04fc6369f7110fe3d25156c1bb9a72859cf2a04641f99ba4ee413c80da6a5fe48152509050610fb98782610fac89611985565b610fb4611a22565b611aef565b9e9d5050505050505050505050505050565b805160208201515f917f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4791159015161561100457505050565b8251602084015182600384858586098509088382830914838210848410161693505050816110745760405162461bcd60e51b815260206004820152601760248201527f426e3235343a20696e76616c696420473120706f696e740000000000000000006044820152606401610c2b565b505050565b6110b96040518061010001604052805f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81526020015f81525090565b5f5f805160206128bb8339815191529050604051602081015f815260fe60e01b8152865160c01b6004820152602087015160c01b600c82015261028087015160208201526102a08701516040820152600160608201527f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a60808201527f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02560a08201527f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a60c08201527f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88160e082015260e087015180516101008301526020810151610120830152506101008701518051610140830152602081015161016083015250610120870151805161018083015260208101516101a08301525061014087015180516101c083015260208101516101e083015250610160870151805161020083015260208101516102208301525061018087015180516102408301526020810151610260830152506101e0870151805161028083015260208101516102a08301525061020087015180516102c083015260208101516102e083015250610220870151805161030083015260208101516103208301525061024087015180516103408301526020810151610360830152506101a0870151805161038083015260208101516103a0830152506101c087015180516103c083015260208101516103e0830152506102608701518051610400830152602081015161042083015250604087015180516104408301526020810151610460830152506060870151805161048083015260208101516104a083015250608087015180516104c083015260208101516104e08301525060a0870151805161050083015260208101516105208301525060c08701518051610540830152602081015161056083015250855161058082015260208601516105a082015260408601516105c082015260608601516105e0820152608086015161060082015260a086015161062082015260c086015161064082015260e08601516106608201526101008601516106808201526101208601516106a08201526101408601516106c0820152845180516106e08301526020810151610700830152506020850151805161072083015260208101516107408301525060408501518051610760830152602081015161078083015250606085015180516107a083015260208101516107c083015250608085015180516107e08301526020810151610800830152505f82526108408220825282825106606085015260208220825282825106608085015260a085015180518252602081015160208301525060608220808352838106855283818209848282099150806020870152508060408601525060c085015180518252602081015160208301525060e085015180516040830152602081015160608301525061010085015180516080830152602081015160a083015250610120850151805160c0830152602081015160e0830152506101408501518051610100830152602081015161012083015250610160822082528282510660a08501526101a085015181526101c085015160208201526101e085015160408201526102008501516060820152610220850151608082015261024085015160a082015261026085015160c082015261028085015160e08201526102a08501516101008201526102c0850151610120820152610160822082528282510660c08501526101608501518051825260208101516020830152506101808501518051604083015260208101516060830152505060a0812082810660e08501525050509392505050565b604080518082019091525f80825260208201526116236121fa565b8351815260208085015190820152604081018390525f60608360808460076107d05a03fa90508080611653575f80fd5b50806116a15760405162461bcd60e51b815260206004820152601960248201527f426e3235343a207363616c6172206d756c206661696c656421000000000000006044820152606401610c2b565b505092915050565b604080518082019091525f80825260208201526116c4612218565b8351815260208085015181830152835160408301528301516060808301919091525f908360c08460066107d05a03fa905080806116ff575f80fd5b50806116a15760405162461bcd60e51b815260206004820152601d60248201527f426e3235343a2067726f7570206164646974696f6e206661696c6564210000006044820152606401610c2b565b604080518082019091525f8082526020820152604080518082019091525f80825260208201525f61178087878787611bcd565b90505f805160206128bb8339815191525f61179c888789612097565b90506117a8818361287c565b60c08901516101a0880151919250908190849081908309840892506117d485610e768a5f015184611608565b955083828209905083846101c08a01518309840892506117fc86610e768a6020015184611608565b955083828209905083846101e08a015183098408925061182486610e768a6040015184611608565b955083828209905083846102008a015183098408925061184c86610e768a6060015184611608565b955083828209905083846102208a015183098408925061187486610e768a6080015184611608565b955083828209905083846102408a015183098408925061189c86610e768d6040015184611608565b955083828209905083846102608a01518309840892506118c486610e768d6060015184611608565b955083828209905083846102808a01518309840892506118ec86610e768d6080015184611608565b955083828209905083846102a08a015183098408925061191486610e768d60a0015184611608565b95505f8a60e00151905084856102c08b015183098508935061193e87610e768b60a0015184611608565b965061197461196e6040805180820182525f80825260209182015281518083019092526001825260029082015290565b85611608565b975050505050505094509492505050565b604080518082019091525f80825260208201528151602083015115901516156119ac575090565b6040518060400160405280835f015181526020017f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4784602001516119f0919061289b565b611a1a907f30644e72e131a029b85045b68181585d97816a916871ca8d3c208c16d87cfd4761287c565b905292915050565b611a4960405180608001604052805f81526020015f81526020015f81526020015f81525090565b60405180608001604052807f1800deef121f1e76426a00665e5c4479674322d4f75edadd46debd5cd992f6ed81526020017f198e9393920d483a7260bfb731fb5d25f1aa493335a9e71297e485b7aef312c281526020017f12c85ea5db8c6deb4aab71808dcb408fe3d1e7690c43d37b4ce6cc0166fa7daa81526020017f090689d0585ff075ec9e99ad690c3395bc4b313370b38ef355acdadcd122975b815250905090565b5f805f6040518751815260208801516020820152602087015160408201528651606082015260608701516080820152604087015160a0820152855160c0820152602086015160e0820152602085015161010082015284516101208201526060850151610140820152604085015161016082015260205f6101808360085afa9150505f51915080611bc15760405162461bcd60e51b815260206004820152601c60248201527f426e3235343a2050616972696e6720636865636b206661696c656421000000006044820152606401610c2b565b50151595945050505050565b604080518082019091525f80825260208201525f805f805f5f805160206128bb833981519152905060808901518160208a015160208c0151099550895194508160a08b015160608c0151099350816101a089015185089250818184089250818584099450817f2f8dd1f1a7583c42c4e12a44e110404c73ca6c94813f85835da4fb7bb1301d4a85099250816101c089015184089250818184089250818584099450817f1ee678a0470a75a6eaa8fe837060498ba828a3703b311d0f77f010424afeb02585099250816101e089015184089250818184089250818584099450817f2042a587a90c187b0a087c03e29c968b950b1db26d5c82d666905a6895790c0a850992508161020089015184089250818184089250818584099450817f2e2b91456103698adf57b799969dea1c8f739da5d8d40dd3eb9222db7c81e88185099250816102208901518408925081818408925050808483099350808486089450611d3a8760a0015186611608565b9550885160608a015160808b0151838284099750836102c08b015189099750836102408b015183099550836101a08b015187089550838187089550838689099750836102608b015183099550836101c08b015187089550838187089550838689099750836102808b015183099550836101e08b015187089550838187089550838689099750836102a08b015183099550836102008b015187089550838187089550505050808386099450611e0186610e768c60c001518885611dfc919061287c565b611608565b9550611e1a86610e768c60e001518a6101a00151611608565b9550611e3486610e768c61010001518a6101c00151611608565b9550611e4e86610e768c61012001518a6101e00151611608565b9550611e6886610e768c61014001518a6102000151611608565b9550806101c08801516101a0890151099250611e8d86610e768c610160015186611608565b9550806102008801516101e0890151099250611eb286610e768c610180015186611608565b95506101a08701519250808384099150808283099150808284099250611ee186610e768c6101e0015186611608565b95506101c08701519250808384099150808283099150808284099250611f1086610e768c610200015186611608565b95506101e08701519250808384099150808283099150808284099250611f3f86610e768c610220015186611608565b95506102008701519250808384099150808283099150808284099250611f6e86610e768c610240015186611608565b9550611f8b86610e768c6101a00151611dfc8b6102200151612182565b9550611f9c868b6101c001516116a9565b9550806101c08801516101a0890151099250806101e08801518409925080610200880151840992508061022088015184099250611fe286610e768c610260015186611608565b9550611ff0885f0151612182565b945061200486610e768960c0015188611608565b955080600189510860a08a015190935081908009915080828409925080838609945061203886610e768960e0015188611608565b955080838609945061205386610e7689610100015188611608565b955080838609945061206e86610e7689610120015188611608565b955080838609945061208986610e7689610140015188611608565b9a9950505050505050505050565b5f805f805160206128bb83398151915290505f836020015190505f846040015190505f60019050606088015160808901516101a08901516102408a01518788898387098a868608088609945050506101c08901516102608a01518788898387098a868608088609945050506101e08901516102808a01518788898387098a868608088609945050506102008901516102a08a01518788898387098a8686080886099450505061022089015191506102c0890151868782898587080985099350505050875160208901518586868309870385089650508485838309860387089998505050505050505050565b5f61219a5f805160206128bb8339815191528361289b565b6121b1905f805160206128bb83398151915261287c565b92915050565b60405180606001604052805f81526020015f81526020016121d66121db565b905290565b604051806101600160405280600b906020820280368337509192915050565b60405180606001604052806003906020820280368337509192915050565b60405180608001604052806004906020820280368337509192915050565b634e487b7160e01b5f52604160045260245ffd5b6040516102e0810167ffffffffffffffff8111828210171561226e5761226e612236565b60405290565b6040516102c0810167ffffffffffffffff8111828210171561226e5761226e612236565b5f82601f8301126122a7575f80fd5b60405161016080820182811067ffffffffffffffff821117156122cc576122cc612236565b604052830181858211156122de575f80fd5b845b828110156122f85780358252602091820191016122e0565b509195945050505050565b5f6101a08284031215612314575f80fd5b6040516060810181811067ffffffffffffffff8211171561233757612337612236565b8060405250809150823581526020830135602082015261235a8460408501612298565b60408201525092915050565b5f805f6101e08486031215612379575f80fd5b6123838585612303565b956101a085013595506101c0909401359392505050565b5f806101c083850312156123ac575f80fd5b6123b68484612303565b946101a0939093013593505050565b5f602082840312156123d5575f80fd5b5035919050565b81518152602080830151818301526040808401516101a08401929184015f5b600b811015612418578251825291830191908301906001016123fb565b5050505092915050565b5f805f6103208486031215612435575f80fd5b61243f8585612303565b92506101a08401359150612457856101c08601612298565b90509250925092565b5f60408284031215612470575f80fd5b6040516040810181811067ffffffffffffffff8211171561249357612493612236565b604052823581526020928301359281019290925250919050565b5f61048082840312156124be575f80fd5b6124c661224a565b90506124d28383612460565b81526124e18360408401612460565b60208201526124f38360808401612460565b60408201526125058360c08401612460565b606082015261010061251984828501612460565b608083015261014061252d85828601612460565b60a084015261018061254186828701612460565b60c08501526101c061255587828801612460565b60e086015261020061256988828901612460565b85870152610240945061257e88868901612460565b61012087015261028061259389828a01612460565b858801526102c094506125a889868a01612460565b6101608801526125bc896103008a01612460565b848801526103408801356101a0880152610360880135838801526103808801356101e08801526103a0880135828801526103c08801356102208801526103e08801358688015261040088013561026088015261042088013581880152505050506104408401356102a084015261046084013581840152505092915050565b5f805f838503610ae081121561264e575f80fd5b6105008082121561265d575f80fd5b612665612274565b915085358252602086013560208301526126828760408801612460565b60408301526126948760808801612460565b60608301526126a68760c08801612460565b60808301526101006126ba88828901612460565b60a08401526101406126ce89828a01612460565b60c08501526101806126e28a828b01612460565b60e08601526101c06126f68b828c01612460565b84870152610200935061270b8b858c01612460565b6101208701526102406127208c828d01612460565b8488015261028093506127358c858d01612460565b6101608801526127498c6102c08d01612460565b8388015261275b8c6103008d01612460565b6101a088015261276f8c6103408d01612460565b828801526127818c6103808d01612460565b6101e08801526127958c6103c08d01612460565b858801526127a78c6104008d01612460565b6102208801526127bb8c6104408d01612460565b818801525050506127d0896104808a01612460565b6102608501526104c08801358185015250506104e08601356102a08301528194506127fd87828801612298565b935050506124578561066086016124ad565b5f805f806103408587031215612823575f80fd5b61282d8686612303565b935061283d866101a08701612298565b939693955050505061030082013591610320013590565b634e487b7160e01b5f52603260045260245ffd5b634e487b7160e01b5f52601260045260245ffd5b818103818111156121b157634e487b7160e01b5f52601160045260245ffd5b5f826128b557634e487b7160e01b5f52601260045260245ffd5b50069056fe30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001a164736f6c6343000817000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0\xE5W_5`\xE0\x1C\x80c\xA1\x97\xAF\xC4\x11a\0\x88W\x80c\xBD\x006\x9A\x11a\0cW\x80c\xBD\x006\x9A\x14a\x02]W\x80c\xDE$\xAC\x0F\x14a\x02pW\x80c\xE3Q-V\x14a\x02\x97W\x80c\xF5\x14C&\x14a\x02\xBEW_\x80\xFD[\x80c\xA1\x97\xAF\xC4\x14a\x01\xDEW\x80c\xAB\x95\x9E\xE3\x14a\x02\x13W\x80c\xAF\x19k\xA2\x14a\x026W_\x80\xFD[\x80cZcOS\x11a\0\xC3W\x80cZcOS\x14a\x01qW\x80c~nG\xB4\x14a\x01\x84W\x80c\x82\xD8\xA0\x99\x14a\x01\x97W\x80c\x83LE*\x14a\x01\xB7W_\x80\xFD[\x80c\x0CU\x1F?\x14a\0\xE9W\x80cKG4\xE3\x14a\x01#W\x80cZ\x14\xC0\xFE\x14a\x01JW[_\x80\xFD[a\x01\x10\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x01\x10\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81V[a\x01\x10\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x81V[a\x01\x10a\x01\x7F6`\x04a#fV[a\x02\xE5V[a\x01\x10a\x01\x926`\x04a#\x9AV[a\x03VV[a\x01\xAAa\x01\xA56`\x04a#\xC5V[a\x03\xA7V[`@Qa\x01\x1A\x91\x90a#\xDCV[a\x01\x10\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81V[a\x01\xF1a\x01\xEC6`\x04a$\"V[a\tNV[`@\x80Q\x82Q\x81R` \x80\x84\x01Q\x90\x82\x01R\x91\x81\x01Q\x90\x82\x01R``\x01a\x01\x1AV[a\x02&a\x02!6`\x04a&:V[a\t\xABV[`@Q\x90\x15\x15\x81R` \x01a\x01\x1AV[a\x01\x10\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81V[a\x01\x10a\x02k6`\x04a(\x0FV[a\nFV[a\x01\x10\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x81V[a\x01\x10\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x81V[a\x01\x10\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81V[_\x82`\x01\x03a\x02\xF6WP`\x01a\x03OV[\x81_\x03a\x03\x04WP_a\x03OV[` \x84\x01Q_\x80Q` a(\xBB\x839\x81Q\x91R\x90_\x90\x82\x81\x86\t\x90P\x85\x80\x15a\x032W`\x01\x87\x03\x92Pa\x039V[`\x01\x84\x03\x92P[Pa\x03C\x82a\x0B\x95V[\x91P\x82\x82\x82\t\x93PPPP[\x93\x92PPPV[\x81Q_\x90_\x80Q` a(\xBB\x839\x81Q\x91R\x90\x83\x80\x15a\x03\x97W\x84\x93P_[\x82\x81\x10\x15a\x03\x8BW\x83\x85\x86\t\x94P`\x01\x01a\x03uV[P`\x01\x84\x03\x93Pa\x03\x9EV[`\x01\x83\x03\x93P[PPP\x92\x91PPV[a\x03\xAFa!\xB7V[\x81b\x01\0\0\x03a\x05\x86W`@Q\x80``\x01`@R\x80`\x10\x81R` \x01\x7F0d\x1E\x0E\x92\xBE\xBE\xF8\x18&\x8Df;\xCA\xD6\xDB\xCF\xD6\xC0\x14\x91p\xF6\xD7\xD3P\xB1\xB1\xFAl\x10\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01~\xEE\xB2\xCBY\x81\xEDEd\x9A\xBE\xBD\xE0\x81\xDC\xFF\x16\xC8`\x1D\xE44~}\xD1b\x8B\xA2\xDA\xACC\xB7\x81R` \x01\x7F-\x1B\xA6oYA\xDC\x91\x01qq\xFAi\xEC+\xD0\x02**-A\x15\xA0\t\xA94X\xFDN&\xEC\xFB\x81R` \x01\x7F\x08h\x12\xA0\n\xC4>\xA8\x01f\x9Cd\x01q <A\xA4\x96g\x1B\xFB\xC0e\xAC\x8D\xB2MR\xCF1\xE5\x81R` \x01\x7F-\x96VQ\xCD\xD9\xE4\x81\x1FNQ\xB8\r\xDC\xA8\xA8\xB4\xA9>\xE1t \xAA\xE6\xAD\xAA\x01\xC2a|n\x85\x81R` \x01\x7F\x12YzV\xC2\xE48b\x0B\x90A\xB9\x89\x92\xAE\rNp[x\0W\xBFwf\xA2v|\xEC\xE1n\x1D\x81R` \x01\x7F\x02\xD9A\x17\xCD\x17\xBC\xF1)\x0F\xD6|\x01\x15]\xD4\x08\x07\x85}\xFFJZ\x0BM\xC6{\xEF\xA8\xAA4\xFD\x81R` \x01\x7F\x15\xEE$u\xBE\xE5\x17\xC4\xEE\x05\xE5\x1F\xA1\xEEs\x12\xA87:\x0B\x13\xDB\x8CQ\xBA\xF0L\xB2\xE9\x9B\xD2\xBD\x81R` \x01~o\xABI\xB8i\xAEb\0\x1D\xEA\xC8x\xB2f{\xD3\x1B\xF3\xE2\x8E:-vJ\xA4\x9B\x8D\x9B\xBD\xD3\x10\x81R` \x01\x7F.\x85k\xF6\xD07p\x8F\xFAL\x06\xD4\xD8\x82\x0FE\xCC\xAD\xCE\x9CZm\x17\x8C\xBDW?\x82\xE0\xF9p\x11\x81R` \x01\x7F\x14\x07\xEE\xE3Y\x93\xF2\xB1\xAD^\xC6\xD9\xB8\x95\x0C\xA3\xAF3\x13]\x06\x03\x7F\x87\x1C^3\xBFVm\xD7\xB4\x81RP\x81RP\x90P\x91\x90PV[\x81b\x10\0\0\x03a\x07_W`@Q\x80``\x01`@R\x80`\x14\x81R` \x01\x7F0dKl\x9CJr\x16\x9EM\xAA1}%\xF0E\x12\xAE\x15\xC5;4\xE8\xF5\xAC\xD8\xE1U\xD0\xA6\xC1\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F&\x12]\xA1\n\x0E\xD0c'P\x8A\xBA\x06\xD1\xE3\x03\xACaf2\xDB\xED4\x9FSB-\xA9S3xW\x81R` \x01\x7F\"`\xE7$\x84K\xCARQ\x82\x93S\x96\x8EI\x150RXA\x83WG:\\\x1DY\x7Fa?l\xBD\x81R` \x01\x7F \x87\xEA,\xD6d'\x86\x08\xFB\x0E\xBD\xB8 \x90\x7FY\x85\x02\xC8\x1Bf\x90\xC1\x85\xE2\xBF\x15\xCB\x93_B\x81R` \x01\x7F\x19\xDD\xBC\xAF:\x8DF\xC1\\\x01v\xFB\xB5\xB9^M\xC5p\x88\xFF\x13\xF4\xD1\xBD\x84\xC6\xBF\xA5}\xCD\xC0\xE0\x81R` \x01\x7F\x05\xA2\xC8\\\xFCY\x17\x89`\\\xAE\x81\x8E7\xDDAa\xEE\xF9\xAAfk\xECo\xE4(\x8D\t\xE6\xD24\x18\x81R` \x01\x7F\x11\xF7\x0ESc%\x8F\xF4\xF0\xD7\x16\xA6S\xE1\xDCA\xF1\xC6D\x84\xD7\xF4\xB6\xE2\x19\xD67v\x14\xA3\x90\\\x81R` \x01\x7F)\xE8AC\xF5\x87\rGv\xA9-\xF8\xDA\x8Cl\x93\x03\xD5\x90\x88\xF3{\xA8_@\xCFo\xD1Be\xB4\xBC\x81R` \x01\x7F\x1B\xF8-\xEB\xA7\xD7I\x02\xC3p\x8C\xC6\xE7\x0Ea\xF3\x05\x12\xEC\xA9VU!\x0E'nXX\xCE\x8FX\xE5\x81R` \x01\x7F\"\xB9K.+\0C\xD0Nf-^\xC0\x18\xEA\x1C\x8A\x99\xA2:b\xC9\xEBF\xF01\x8Fj\x19I\x85\xF0\x81R` \x01\x7F)\x96\x9D\x8DSc\xBE\xF1\x10\x1Ah\xE4F\xA1N\x1D\xA7\xBA\x92\x94\xE1B\xA1F\xA9\x80\xFD\xDBMMA\xA5\x81RP\x81RP\x90P\x91\x90PV[\x81` \x03a\t5W`@Q\x80``\x01`@R\x80`\x05\x81R` \x01\x7F.\xE1+\xFFJ(\x13(j\x8D\xC3\x88\xCDuM\x9A>\xF2I\x065\xEB\xA5\x0C\xB9\xC2\xE5\xE7P\x80\0\x01\x81R` \x01`@Q\x80a\x01`\x01`@R\x80`\x01\x81R` \x01\x7F\t\xC52\xC60k\x93\xD2\x96x \rG\xC0\xB2\xA9\x9C\x18\xD5\x1B\x83\x8E\xEB\x1D>\xEDLS;\xB5\x12\xD0\x81R` \x01\x7F!\x08,\xA2\x16\xCB\xBFN\x1CnOE\x94\xDDP\x8C\x99m\xFB\xE1\x17N\xFB\x98\xB1\x15\t\xC6\xE3\x06F\x0B\x81R` \x01\x7F\x12w\xAEd\x15\xF0\xEF\x18\xF2\xBA_\xB1b\xC3\x9E\xB71\x1F8n-&\xD6D\x01\xF4\xA2]\xA7|%;\x81R` \x01\x7F+3}\xE1\xC8\xC1O\"\xEC\x9B\x9E/\x96\xAF\xEF6Rbsf\xF8\x17\n\n\x94\x8D\xADJ\xC1\xBD^\x80\x81R` \x01\x7F/\xBDM\xD2\x97k\xE5]\x1A\x16:\xA9\x82\x0F\xB8\x8D\xFA\xC5\xDD\xCEw\xE1\x87.\x90c '2z^\xBE\x81R` \x01\x7F\x10z\xABI\xE6Zg\xF9\xDA\x9C\xD2\xAB\xF7\x8B\xE3\x8B\xD9\xDC\x1D]\xB3\x9F\x81\xDE6\xBC\xFA[K\x03\x90C\x81R` \x01~\xE1Kcd\xA4~\x9CB\x84\xA9\xF8\n_\xC4\x1C\xD2\x12\xB0\xD4\xDB\xF8\xA5p7p\xA4\n\x9A49\x90\x81R` \x01\x7F0dNr\xE11\xA0)\x04\x8Bn\x19?\xD8A\x04\\\xEA$\xF6\xFDsk\xEC#\x12\x04p\x8Fp66\x81R` \x01\x7F\"9\x9C4\x13\x9B\xFF\xAD\xA8\xDE\x04j\xACP\xC9b\x8E5\x17\xA3\xA4RySd\xE7w\xCDe\xBB\x9FH\x81R` \x01\x7F\"\x90\xEE1\xC4\x82\xCF\x92\xB7\x9B\x19D\xDB\x1C\x01Gc^\x90\x04\xDB\x8C;\x9D\x13dK\xEF1\xEC;\xD3\x81RP\x81RP\x90P\x91\x90PV[`@Qc\xE2\xEF\t\xE5`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[a\to`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81RP\x90V[a\ty\x84\x84a\x03VV[\x80\x82Ra\t\x89\x90\x85\x90\x85\x90a\x02\xE5V[` \x82\x01R\x80Qa\t\x9F\x90\x85\x90\x84\x90\x86\x90a\nFV[`@\x82\x01R\x93\x92PPPV[_a\t\xB5\x82a\x0C;V[a\t\xC5\x83_[` \x02\x01Qa\rvV[a\t\xD0\x83`\x01a\t\xBBV[a\t\xDB\x83`\x02a\t\xBBV[a\t\xE6\x83`\x03a\t\xBBV[a\t\xF1\x83`\x04a\t\xBBV[a\t\xFC\x83`\x05a\t\xBBV[a\n\x07\x83`\x06a\t\xBBV[a\n\x12\x83`\x07a\t\xBBV[a\n\x1D\x83`\x08a\t\xBBV[a\n(\x83`\ta\t\xBBV[a\n3\x83`\na\t\xBBV[a\n>\x84\x84\x84a\r\xD7V[\x94\x93PPPPV[__\x80Q` a(\xBB\x839\x81Q\x91R\x82\x82\x03a\n\xBFW`\x01_[`\x0B\x81\x10\x15a\n\xB4W\x81\x86\x03a\n\x91W\x86\x81`\x0B\x81\x10a\n\x82Wa\n\x82a(TV[` \x02\x01Q\x93PPPPa\n>V[\x82\x80a\n\x9FWa\n\x9Fa(hV[`@\x89\x01Q` \x01Q\x83\t\x91P`\x01\x01a\n`V[P_\x92PPPa\n>V[a\n\xC7a!\xDBV[`@\x87\x01Q`\x01a\x01@\x83\x81\x01\x82\x81R\x92\x01\x90\x80[`\x0B\x81\x10\x15a\x0B\tW` \x84\x03\x93P\x85\x86\x8A\x85Q\x89\x03\x08\x83\t\x80\x85R`\x1F\x19\x90\x93\x01\x92\x91P`\x01\x01a\n\xDCV[PPPP_\x80_\x90P`\x01\x83\x89`@\x8C\x01Q_[`\x0B\x81\x10\x15a\x0B]W\x88\x82Q\x8A\x85Q\x8C\x88Q\x8A\t\t\t\x89\x81\x88\x08\x96PP\x88\x89\x8D\x84Q\x8C\x03\x08\x86\t\x94P` \x93\x84\x01\x93\x92\x83\x01\x92\x91\x90\x91\x01\x90`\x01\x01a\x0B\x1DV[PPPP\x80\x92PP_a\x0Bo\x83a\x0B\x95V[\x90P` \x8A\x01Q\x85\x81\x89\t\x96PP\x84\x81\x87\t\x95P\x84\x82\x87\t\x9A\x99PPPPPPPPPPV[_\x80__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x84``\x82\x01R`\x02\x82\x03`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x92PP_Q\x92P\x81a\x0C4W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: pow precompile failed!\0\0\0`D\x82\x01R`d\x01[`@Q\x80\x91\x03\x90\xFD[PP\x91\x90PV[\x80Qa\x0CF\x90a\x0F\xCBV[a\x0CS\x81` \x01Qa\x0F\xCBV[a\x0C`\x81`@\x01Qa\x0F\xCBV[a\x0Cm\x81``\x01Qa\x0F\xCBV[a\x0Cz\x81`\x80\x01Qa\x0F\xCBV[a\x0C\x87\x81`\xA0\x01Qa\x0F\xCBV[a\x0C\x94\x81`\xC0\x01Qa\x0F\xCBV[a\x0C\xA1\x81`\xE0\x01Qa\x0F\xCBV[a\x0C\xAF\x81a\x01\0\x01Qa\x0F\xCBV[a\x0C\xBD\x81a\x01 \x01Qa\x0F\xCBV[a\x0C\xCB\x81a\x01@\x01Qa\x0F\xCBV[a\x0C\xD9\x81a\x01`\x01Qa\x0F\xCBV[a\x0C\xE7\x81a\x01\x80\x01Qa\x0F\xCBV[a\x0C\xF5\x81a\x01\xA0\x01Qa\rvV[a\r\x03\x81a\x01\xC0\x01Qa\rvV[a\r\x11\x81a\x01\xE0\x01Qa\rvV[a\r\x1F\x81a\x02\0\x01Qa\rvV[a\r-\x81a\x02 \x01Qa\rvV[a\r;\x81a\x02@\x01Qa\rvV[a\rI\x81a\x02`\x01Qa\rvV[a\rW\x81a\x02\x80\x01Qa\rvV[a\re\x81a\x02\xA0\x01Qa\rvV[a\rs\x81a\x02\xC0\x01Qa\rvV[PV[_\x80Q` a(\xBB\x839\x81Q\x91R\x81\x10\x80a\r\xD3W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7FBn254: invalid scalar field\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PPV[_\x83` \x01Q`\x0B\x14a\r\xFDW`@Qc \xFA\x9D\x89`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\x0E\t\x85\x85\x85a\x10yV[\x90P_a\x0E\x18\x86_\x01Qa\x03\xA7V[\x90P_a\x0E*\x82\x84`\xA0\x01Q\x88a\tNV[\x90Pa\x0EG`@Q\x80`@\x01`@R\x80_\x81R` \x01_\x81RP\x90V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0E{\x87a\x01`\x01Qa\x0Ev\x89a\x01\x80\x01Q\x88`\xE0\x01Qa\x16\x08V[a\x16\xA9V[\x91P_\x80a\x0E\x8B\x8B\x88\x87\x8Ca\x17MV[\x91P\x91Pa\x0E\x9C\x81a\x0Ev\x84a\x19\x85V[\x92Pa\x0E\xB5\x83a\x0Ev\x8Ba\x01`\x01Q\x8A`\xA0\x01Qa\x16\x08V[`\xA0\x88\x01Q`@\x88\x01Q` \x01Q\x91\x94P_\x80Q` a(\xBB\x839\x81Q\x91R\x91\x82\x90\x82\t\x90P\x81`\xE0\x8A\x01Q\x82\t\x90Pa\x0E\xF8\x85a\x0Ev\x8Da\x01\x80\x01Q\x84a\x16\x08V[\x94P_`@Q\x80`\x80\x01`@R\x80\x7F\x01\x18\xC4\xD5\xB87\xBC\xC2\xBC\x89\xB5\xB3\x98\xB5\x97N\x9FYD\x07;2\x07\x8B~#\x1F\xEC\x93\x88\x83\xB0\x81R` \x01\x7F&\x0E\x01\xB2Q\xF6\xF1\xC7\xE7\xFFNX\x07\x91\xDE\xE8\xEAQ\xD8z5\x8E\x03\x8BN\xFE0\xFA\xC0\x93\x83\xC1\x81R` \x01\x7F\"\xFE\xBD\xA3\xC0\xC0c*VG[B\x14\xE5a^\x11\xE6\xDD?\x96\xE6\xCE\xA2\x85J\x87\xD4\xDA\xCC^U\x81R` \x01\x7F\x04\xFCci\xF7\x11\x0F\xE3\xD2QV\xC1\xBB\x9Ar\x85\x9C\xF2\xA0FA\xF9\x9B\xA4\xEEA<\x80\xDAj_\xE4\x81RP\x90Pa\x0F\xB9\x87\x82a\x0F\xAC\x89a\x19\x85V[a\x0F\xB4a\x1A\"V[a\x1A\xEFV[\x9E\x9DPPPPPPPPPPPPPPV[\x80Q` \x82\x01Q_\x91\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x91\x15\x90\x15\x16\x15a\x10\x04WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x10tW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PPPV[a\x10\xB9`@Q\x80a\x01\0\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`@Q` \x81\x01_\x81R`\xFE`\xE0\x1B\x81R\x86Q`\xC0\x1B`\x04\x82\x01R` \x87\x01Q`\xC0\x1B`\x0C\x82\x01Ra\x02\x80\x87\x01Q` \x82\x01Ra\x02\xA0\x87\x01Q`@\x82\x01R`\x01``\x82\x01R\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ`\x80\x82\x01R\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%`\xA0\x82\x01R\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n`\xC0\x82\x01R\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81`\xE0\x82\x01R`\xE0\x87\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01\0\x87\x01Q\x80Qa\x01@\x83\x01R` \x81\x01Qa\x01`\x83\x01RPa\x01 \x87\x01Q\x80Qa\x01\x80\x83\x01R` \x81\x01Qa\x01\xA0\x83\x01RPa\x01@\x87\x01Q\x80Qa\x01\xC0\x83\x01R` \x81\x01Qa\x01\xE0\x83\x01RPa\x01`\x87\x01Q\x80Qa\x02\0\x83\x01R` \x81\x01Qa\x02 \x83\x01RPa\x01\x80\x87\x01Q\x80Qa\x02@\x83\x01R` \x81\x01Qa\x02`\x83\x01RPa\x01\xE0\x87\x01Q\x80Qa\x02\x80\x83\x01R` \x81\x01Qa\x02\xA0\x83\x01RPa\x02\0\x87\x01Q\x80Qa\x02\xC0\x83\x01R` \x81\x01Qa\x02\xE0\x83\x01RPa\x02 \x87\x01Q\x80Qa\x03\0\x83\x01R` \x81\x01Qa\x03 \x83\x01RPa\x02@\x87\x01Q\x80Qa\x03@\x83\x01R` \x81\x01Qa\x03`\x83\x01RPa\x01\xA0\x87\x01Q\x80Qa\x03\x80\x83\x01R` \x81\x01Qa\x03\xA0\x83\x01RPa\x01\xC0\x87\x01Q\x80Qa\x03\xC0\x83\x01R` \x81\x01Qa\x03\xE0\x83\x01RPa\x02`\x87\x01Q\x80Qa\x04\0\x83\x01R` \x81\x01Qa\x04 \x83\x01RP`@\x87\x01Q\x80Qa\x04@\x83\x01R` \x81\x01Qa\x04`\x83\x01RP``\x87\x01Q\x80Qa\x04\x80\x83\x01R` \x81\x01Qa\x04\xA0\x83\x01RP`\x80\x87\x01Q\x80Qa\x04\xC0\x83\x01R` \x81\x01Qa\x04\xE0\x83\x01RP`\xA0\x87\x01Q\x80Qa\x05\0\x83\x01R` \x81\x01Qa\x05 \x83\x01RP`\xC0\x87\x01Q\x80Qa\x05@\x83\x01R` \x81\x01Qa\x05`\x83\x01RP\x85Qa\x05\x80\x82\x01R` \x86\x01Qa\x05\xA0\x82\x01R`@\x86\x01Qa\x05\xC0\x82\x01R``\x86\x01Qa\x05\xE0\x82\x01R`\x80\x86\x01Qa\x06\0\x82\x01R`\xA0\x86\x01Qa\x06 \x82\x01R`\xC0\x86\x01Qa\x06@\x82\x01R`\xE0\x86\x01Qa\x06`\x82\x01Ra\x01\0\x86\x01Qa\x06\x80\x82\x01Ra\x01 \x86\x01Qa\x06\xA0\x82\x01Ra\x01@\x86\x01Qa\x06\xC0\x82\x01R\x84Q\x80Qa\x06\xE0\x83\x01R` \x81\x01Qa\x07\0\x83\x01RP` \x85\x01Q\x80Qa\x07 \x83\x01R` \x81\x01Qa\x07@\x83\x01RP`@\x85\x01Q\x80Qa\x07`\x83\x01R` \x81\x01Qa\x07\x80\x83\x01RP``\x85\x01Q\x80Qa\x07\xA0\x83\x01R` \x81\x01Qa\x07\xC0\x83\x01RP`\x80\x85\x01Q\x80Qa\x07\xE0\x83\x01R` \x81\x01Qa\x08\0\x83\x01RP_\x82Ra\x08@\x82 \x82R\x82\x82Q\x06``\x85\x01R` \x82 \x82R\x82\x82Q\x06`\x80\x85\x01R`\xA0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP``\x82 \x80\x83R\x83\x81\x06\x85R\x83\x81\x82\t\x84\x82\x82\t\x91P\x80` \x87\x01RP\x80`@\x86\x01RP`\xC0\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RP`\xE0\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPa\x01\0\x85\x01Q\x80Q`\x80\x83\x01R` \x81\x01Q`\xA0\x83\x01RPa\x01 \x85\x01Q\x80Q`\xC0\x83\x01R` \x81\x01Q`\xE0\x83\x01RPa\x01@\x85\x01Q\x80Qa\x01\0\x83\x01R` \x81\x01Qa\x01 \x83\x01RPa\x01`\x82 \x82R\x82\x82Q\x06`\xA0\x85\x01Ra\x01\xA0\x85\x01Q\x81Ra\x01\xC0\x85\x01Q` \x82\x01Ra\x01\xE0\x85\x01Q`@\x82\x01Ra\x02\0\x85\x01Q``\x82\x01Ra\x02 \x85\x01Q`\x80\x82\x01Ra\x02@\x85\x01Q`\xA0\x82\x01Ra\x02`\x85\x01Q`\xC0\x82\x01Ra\x02\x80\x85\x01Q`\xE0\x82\x01Ra\x02\xA0\x85\x01Qa\x01\0\x82\x01Ra\x02\xC0\x85\x01Qa\x01 \x82\x01Ra\x01`\x82 \x82R\x82\x82Q\x06`\xC0\x85\x01Ra\x01`\x85\x01Q\x80Q\x82R` \x81\x01Q` \x83\x01RPa\x01\x80\x85\x01Q\x80Q`@\x83\x01R` \x81\x01Q``\x83\x01RPP`\xA0\x81 \x82\x81\x06`\xE0\x85\x01RPPP\x93\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16#a!\xFAV[\x83Q\x81R` \x80\x85\x01Q\x90\x82\x01R`@\x81\x01\x83\x90R_``\x83`\x80\x84`\x07a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x16SW_\x80\xFD[P\x80a\x16\xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x19`$\x82\x01R\x7FBn254: scalar mul failed!\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[PP\x92\x91PPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x16\xC4a\"\x18V[\x83Q\x81R` \x80\x85\x01Q\x81\x83\x01R\x83Q`@\x83\x01R\x83\x01Q``\x80\x83\x01\x91\x90\x91R_\x90\x83`\xC0\x84`\x06a\x07\xD0Z\x03\xFA\x90P\x80\x80a\x16\xFFW_\x80\xFD[P\x80a\x16\xA1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1D`$\x82\x01R\x7FBn254: group addition failed!\0\0\0`D\x82\x01R`d\x01a\x0C+V[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x17\x80\x87\x87\x87\x87a\x1B\xCDV[\x90P_\x80Q` a(\xBB\x839\x81Q\x91R_a\x17\x9C\x88\x87\x89a \x97V[\x90Pa\x17\xA8\x81\x83a(|V[`\xC0\x89\x01Qa\x01\xA0\x88\x01Q\x91\x92P\x90\x81\x90\x84\x90\x81\x90\x83\t\x84\x08\x92Pa\x17\xD4\x85a\x0Ev\x8A_\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xC0\x8A\x01Q\x83\t\x84\x08\x92Pa\x17\xFC\x86a\x0Ev\x8A` \x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x01\xE0\x8A\x01Q\x83\t\x84\x08\x92Pa\x18$\x86a\x0Ev\x8A`@\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\0\x8A\x01Q\x83\t\x84\x08\x92Pa\x18L\x86a\x0Ev\x8A``\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02 \x8A\x01Q\x83\t\x84\x08\x92Pa\x18t\x86a\x0Ev\x8A`\x80\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02@\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\x9C\x86a\x0Ev\x8D`@\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02`\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\xC4\x86a\x0Ev\x8D``\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\x80\x8A\x01Q\x83\t\x84\x08\x92Pa\x18\xEC\x86a\x0Ev\x8D`\x80\x01Q\x84a\x16\x08V[\x95P\x83\x82\x82\t\x90P\x83\x84a\x02\xA0\x8A\x01Q\x83\t\x84\x08\x92Pa\x19\x14\x86a\x0Ev\x8D`\xA0\x01Q\x84a\x16\x08V[\x95P_\x8A`\xE0\x01Q\x90P\x84\x85a\x02\xC0\x8B\x01Q\x83\t\x85\x08\x93Pa\x19>\x87a\x0Ev\x8B`\xA0\x01Q\x84a\x16\x08V[\x96Pa\x19ta\x19n`@\x80Q\x80\x82\x01\x82R_\x80\x82R` \x91\x82\x01R\x81Q\x80\x83\x01\x90\x92R`\x01\x82R`\x02\x90\x82\x01R\x90V[\x85a\x16\x08V[\x97PPPPPPP\x94P\x94\x92PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x19\xACWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\x84` \x01Qa\x19\xF0\x91\x90a(\x9BV[a\x1A\x1A\x90\x7F0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDGa(|V[\x90R\x92\x91PPV[a\x1AI`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x1B\xC1W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x0C+V[P\x15\x15\x95\x94PPPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_\x80_\x80__\x80Q` a(\xBB\x839\x81Q\x91R\x90P`\x80\x89\x01Q\x81` \x8A\x01Q` \x8C\x01Q\t\x95P\x89Q\x94P\x81`\xA0\x8B\x01Q``\x8C\x01Q\t\x93P\x81a\x01\xA0\x89\x01Q\x85\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F/\x8D\xD1\xF1\xA7X<B\xC4\xE1*D\xE1\x10@Ls\xCAl\x94\x81?\x85\x83]\xA4\xFB{\xB10\x1DJ\x85\t\x92P\x81a\x01\xC0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F\x1E\xE6x\xA0G\nu\xA6\xEA\xA8\xFE\x83p`I\x8B\xA8(\xA3p;1\x1D\x0Fw\xF0\x10BJ\xFE\xB0%\x85\t\x92P\x81a\x01\xE0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F B\xA5\x87\xA9\x0C\x18{\n\x08|\x03\xE2\x9C\x96\x8B\x95\x0B\x1D\xB2m\\\x82\xD6f\x90Zh\x95y\x0C\n\x85\t\x92P\x81a\x02\0\x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92P\x81\x85\x84\t\x94P\x81\x7F.+\x91Ea\x03i\x8A\xDFW\xB7\x99\x96\x9D\xEA\x1C\x8Fs\x9D\xA5\xD8\xD4\r\xD3\xEB\x92\"\xDB|\x81\xE8\x81\x85\t\x92P\x81a\x02 \x89\x01Q\x84\x08\x92P\x81\x81\x84\x08\x92PP\x80\x84\x83\t\x93P\x80\x84\x86\x08\x94Pa\x1D:\x87`\xA0\x01Q\x86a\x16\x08V[\x95P\x88Q``\x8A\x01Q`\x80\x8B\x01Q\x83\x82\x84\t\x97P\x83a\x02\xC0\x8B\x01Q\x89\t\x97P\x83a\x02@\x8B\x01Q\x83\t\x95P\x83a\x01\xA0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02`\x8B\x01Q\x83\t\x95P\x83a\x01\xC0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\x80\x8B\x01Q\x83\t\x95P\x83a\x01\xE0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95P\x83\x86\x89\t\x97P\x83a\x02\xA0\x8B\x01Q\x83\t\x95P\x83a\x02\0\x8B\x01Q\x87\x08\x95P\x83\x81\x87\x08\x95PPPP\x80\x83\x86\t\x94Pa\x1E\x01\x86a\x0Ev\x8C`\xC0\x01Q\x88\x85a\x1D\xFC\x91\x90a(|V[a\x16\x08V[\x95Pa\x1E\x1A\x86a\x0Ev\x8C`\xE0\x01Q\x8Aa\x01\xA0\x01Qa\x16\x08V[\x95Pa\x1E4\x86a\x0Ev\x8Ca\x01\0\x01Q\x8Aa\x01\xC0\x01Qa\x16\x08V[\x95Pa\x1EN\x86a\x0Ev\x8Ca\x01 \x01Q\x8Aa\x01\xE0\x01Qa\x16\x08V[\x95Pa\x1Eh\x86a\x0Ev\x8Ca\x01@\x01Q\x8Aa\x02\0\x01Qa\x16\x08V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92Pa\x1E\x8D\x86a\x0Ev\x8Ca\x01`\x01Q\x86a\x16\x08V[\x95P\x80a\x02\0\x88\x01Qa\x01\xE0\x89\x01Q\t\x92Pa\x1E\xB2\x86a\x0Ev\x8Ca\x01\x80\x01Q\x86a\x16\x08V[\x95Pa\x01\xA0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1E\xE1\x86a\x0Ev\x8Ca\x01\xE0\x01Q\x86a\x16\x08V[\x95Pa\x01\xC0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1F\x10\x86a\x0Ev\x8Ca\x02\0\x01Q\x86a\x16\x08V[\x95Pa\x01\xE0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1F?\x86a\x0Ev\x8Ca\x02 \x01Q\x86a\x16\x08V[\x95Pa\x02\0\x87\x01Q\x92P\x80\x83\x84\t\x91P\x80\x82\x83\t\x91P\x80\x82\x84\t\x92Pa\x1Fn\x86a\x0Ev\x8Ca\x02@\x01Q\x86a\x16\x08V[\x95Pa\x1F\x8B\x86a\x0Ev\x8Ca\x01\xA0\x01Qa\x1D\xFC\x8Ba\x02 \x01Qa!\x82V[\x95Pa\x1F\x9C\x86\x8Ba\x01\xC0\x01Qa\x16\xA9V[\x95P\x80a\x01\xC0\x88\x01Qa\x01\xA0\x89\x01Q\t\x92P\x80a\x01\xE0\x88\x01Q\x84\t\x92P\x80a\x02\0\x88\x01Q\x84\t\x92P\x80a\x02 \x88\x01Q\x84\t\x92Pa\x1F\xE2\x86a\x0Ev\x8Ca\x02`\x01Q\x86a\x16\x08V[\x95Pa\x1F\xF0\x88_\x01Qa!\x82V[\x94Pa \x04\x86a\x0Ev\x89`\xC0\x01Q\x88a\x16\x08V[\x95P\x80`\x01\x89Q\x08`\xA0\x8A\x01Q\x90\x93P\x81\x90\x80\t\x91P\x80\x82\x84\t\x92P\x80\x83\x86\t\x94Pa 8\x86a\x0Ev\x89`\xE0\x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa S\x86a\x0Ev\x89a\x01\0\x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa n\x86a\x0Ev\x89a\x01 \x01Q\x88a\x16\x08V[\x95P\x80\x83\x86\t\x94Pa \x89\x86a\x0Ev\x89a\x01@\x01Q\x88a\x16\x08V[\x9A\x99PPPPPPPPPPV[_\x80_\x80Q` a(\xBB\x839\x81Q\x91R\x90P_\x83` \x01Q\x90P_\x84`@\x01Q\x90P_`\x01\x90P``\x88\x01Q`\x80\x89\x01Qa\x01\xA0\x89\x01Qa\x02@\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xC0\x89\x01Qa\x02`\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x01\xE0\x89\x01Qa\x02\x80\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02\0\x89\x01Qa\x02\xA0\x8A\x01Q\x87\x88\x89\x83\x87\t\x8A\x86\x86\x08\x08\x86\t\x94PPPa\x02 \x89\x01Q\x91Pa\x02\xC0\x89\x01Q\x86\x87\x82\x89\x85\x87\x08\t\x85\t\x93PPPP\x87Q` \x89\x01Q\x85\x86\x86\x83\t\x87\x03\x85\x08\x96PP\x84\x85\x83\x83\t\x86\x03\x87\x08\x99\x98PPPPPPPPPV[_a!\x9A_\x80Q` a(\xBB\x839\x81Q\x91R\x83a(\x9BV[a!\xB1\x90_\x80Q` a(\xBB\x839\x81Q\x91Ra(|V[\x92\x91PPV[`@Q\x80``\x01`@R\x80_\x81R` \x01_\x81R` \x01a!\xD6a!\xDBV[\x90R\x90V[`@Q\x80a\x01`\x01`@R\x80`\x0B\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80``\x01`@R\x80`\x03\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[`@Q\x80`\x80\x01`@R\x80`\x04\x90` \x82\x02\x806\x837P\x91\x92\x91PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x02\xE0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"nWa\"na\"6V[`@R\x90V[`@Qa\x02\xC0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\"nWa\"na\"6V[_\x82`\x1F\x83\x01\x12a\"\xA7W_\x80\xFD[`@Qa\x01`\x80\x82\x01\x82\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\"\xCCWa\"\xCCa\"6V[`@R\x83\x01\x81\x85\x82\x11\x15a\"\xDEW_\x80\xFD[\x84[\x82\x81\x10\x15a\"\xF8W\x805\x82R` \x91\x82\x01\x91\x01a\"\xE0V[P\x91\x95\x94PPPPPV[_a\x01\xA0\x82\x84\x03\x12\x15a#\x14W_\x80\xFD[`@Q``\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a#7Wa#7a\"6V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01Ra#Z\x84`@\x85\x01a\"\x98V[`@\x82\x01RP\x92\x91PPV[_\x80_a\x01\xE0\x84\x86\x03\x12\x15a#yW_\x80\xFD[a#\x83\x85\x85a#\x03V[\x95a\x01\xA0\x85\x015\x95Pa\x01\xC0\x90\x94\x015\x93\x92PPPV[_\x80a\x01\xC0\x83\x85\x03\x12\x15a#\xACW_\x80\xFD[a#\xB6\x84\x84a#\x03V[\x94a\x01\xA0\x93\x90\x93\x015\x93PPPV[_` \x82\x84\x03\x12\x15a#\xD5W_\x80\xFD[P5\x91\x90PV[\x81Q\x81R` \x80\x83\x01Q\x81\x83\x01R`@\x80\x84\x01Qa\x01\xA0\x84\x01\x92\x91\x84\x01_[`\x0B\x81\x10\x15a$\x18W\x82Q\x82R\x91\x83\x01\x91\x90\x83\x01\x90`\x01\x01a#\xFBV[PPPP\x92\x91PPV[_\x80_a\x03 \x84\x86\x03\x12\x15a$5W_\x80\xFD[a$?\x85\x85a#\x03V[\x92Pa\x01\xA0\x84\x015\x91Pa$W\x85a\x01\xC0\x86\x01a\"\x98V[\x90P\x92P\x92P\x92V[_`@\x82\x84\x03\x12\x15a$pW_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a$\x93Wa$\x93a\"6V[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_a\x04\x80\x82\x84\x03\x12\x15a$\xBEW_\x80\xFD[a$\xC6a\"JV[\x90Pa$\xD2\x83\x83a$`V[\x81Ra$\xE1\x83`@\x84\x01a$`V[` \x82\x01Ra$\xF3\x83`\x80\x84\x01a$`V[`@\x82\x01Ra%\x05\x83`\xC0\x84\x01a$`V[``\x82\x01Ra\x01\0a%\x19\x84\x82\x85\x01a$`V[`\x80\x83\x01Ra\x01@a%-\x85\x82\x86\x01a$`V[`\xA0\x84\x01Ra\x01\x80a%A\x86\x82\x87\x01a$`V[`\xC0\x85\x01Ra\x01\xC0a%U\x87\x82\x88\x01a$`V[`\xE0\x86\x01Ra\x02\0a%i\x88\x82\x89\x01a$`V[\x85\x87\x01Ra\x02@\x94Pa%~\x88\x86\x89\x01a$`V[a\x01 \x87\x01Ra\x02\x80a%\x93\x89\x82\x8A\x01a$`V[\x85\x88\x01Ra\x02\xC0\x94Pa%\xA8\x89\x86\x8A\x01a$`V[a\x01`\x88\x01Ra%\xBC\x89a\x03\0\x8A\x01a$`V[\x84\x88\x01Ra\x03@\x88\x015a\x01\xA0\x88\x01Ra\x03`\x88\x015\x83\x88\x01Ra\x03\x80\x88\x015a\x01\xE0\x88\x01Ra\x03\xA0\x88\x015\x82\x88\x01Ra\x03\xC0\x88\x015a\x02 \x88\x01Ra\x03\xE0\x88\x015\x86\x88\x01Ra\x04\0\x88\x015a\x02`\x88\x01Ra\x04 \x88\x015\x81\x88\x01RPPPPa\x04@\x84\x015a\x02\xA0\x84\x01Ra\x04`\x84\x015\x81\x84\x01RPP\x92\x91PPV[_\x80_\x83\x85\x03a\n\xE0\x81\x12\x15a&NW_\x80\xFD[a\x05\0\x80\x82\x12\x15a&]W_\x80\xFD[a&ea\"tV[\x91P\x855\x82R` \x86\x015` \x83\x01Ra&\x82\x87`@\x88\x01a$`V[`@\x83\x01Ra&\x94\x87`\x80\x88\x01a$`V[``\x83\x01Ra&\xA6\x87`\xC0\x88\x01a$`V[`\x80\x83\x01Ra\x01\0a&\xBA\x88\x82\x89\x01a$`V[`\xA0\x84\x01Ra\x01@a&\xCE\x89\x82\x8A\x01a$`V[`\xC0\x85\x01Ra\x01\x80a&\xE2\x8A\x82\x8B\x01a$`V[`\xE0\x86\x01Ra\x01\xC0a&\xF6\x8B\x82\x8C\x01a$`V[\x84\x87\x01Ra\x02\0\x93Pa'\x0B\x8B\x85\x8C\x01a$`V[a\x01 \x87\x01Ra\x02@a' \x8C\x82\x8D\x01a$`V[\x84\x88\x01Ra\x02\x80\x93Pa'5\x8C\x85\x8D\x01a$`V[a\x01`\x88\x01Ra'I\x8Ca\x02\xC0\x8D\x01a$`V[\x83\x88\x01Ra'[\x8Ca\x03\0\x8D\x01a$`V[a\x01\xA0\x88\x01Ra'o\x8Ca\x03@\x8D\x01a$`V[\x82\x88\x01Ra'\x81\x8Ca\x03\x80\x8D\x01a$`V[a\x01\xE0\x88\x01Ra'\x95\x8Ca\x03\xC0\x8D\x01a$`V[\x85\x88\x01Ra'\xA7\x8Ca\x04\0\x8D\x01a$`V[a\x02 \x88\x01Ra'\xBB\x8Ca\x04@\x8D\x01a$`V[\x81\x88\x01RPPPa'\xD0\x89a\x04\x80\x8A\x01a$`V[a\x02`\x85\x01Ra\x04\xC0\x88\x015\x81\x85\x01RPPa\x04\xE0\x86\x015a\x02\xA0\x83\x01R\x81\x94Pa'\xFD\x87\x82\x88\x01a\"\x98V[\x93PPPa$W\x85a\x06`\x86\x01a$\xADV[_\x80_\x80a\x03@\x85\x87\x03\x12\x15a(#W_\x80\xFD[a(-\x86\x86a#\x03V[\x93Pa(=\x86a\x01\xA0\x87\x01a\"\x98V[\x93\x96\x93\x95PPPPa\x03\0\x82\x015\x91a\x03 \x015\x90V[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[\x81\x81\x03\x81\x81\x11\x15a!\xB1WcNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[_\x82a(\xB5WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V\xFE0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X](3\xE8Hy\xB9p\x91C\xE1\xF5\x93\xF0\0\0\x01\xA1dsolcC\0\x08\x17\0\n",
    );
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
    /**Function with signature `BETA_H_X0()` and selector `0x834c452a`.
    ```solidity
    function BETA_H_X0() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X0Call {}
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
    /**Function with signature `BETA_H_X1()` and selector `0xaf196ba2`.
    ```solidity
    function BETA_H_X1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_X1Call {}
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
    /**Function with signature `BETA_H_Y0()` and selector `0xf5144326`.
    ```solidity
    function BETA_H_Y0() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y0Call {}
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
    /**Function with signature `BETA_H_Y1()` and selector `0x4b4734e3`.
    ```solidity
    function BETA_H_Y1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct BETA_H_Y1Call {}
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
    /**Function with signature `COSET_K1()` and selector `0xe3512d56`.
    ```solidity
    function COSET_K1() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K1Call {}
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
    /**Function with signature `COSET_K2()` and selector `0x0c551f3f`.
    ```solidity
    function COSET_K2() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K2Call {}
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
    /**Function with signature `COSET_K3()` and selector `0x5a14c0fe`.
    ```solidity
    function COSET_K3() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K3Call {}
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
    /**Function with signature `COSET_K4()` and selector `0xde24ac0f`.
    ```solidity
    function COSET_K4() external view returns (uint256);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct COSET_K4Call {}
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
    /**Function with signature `evalDataGen((uint256,uint256,uint256[11]),uint256,uint256[11])` and selector `0xa197afc4`.
    ```solidity
    function evalDataGen(PolynomialEvalV2.EvalDomain memory domain, uint256 zeta, uint256[11] memory publicInput) external view returns (PolynomialEvalV2.EvalData memory evalData);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evalDataGenCall {
        #[allow(missing_docs)]
        pub domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub zeta: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub publicInput: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
    }
    ///Container type for the return parameters of the [`evalDataGen((uint256,uint256,uint256[11]),uint256,uint256[11])`](evalDataGenCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evalDataGenReturn {
        #[allow(missing_docs)]
        pub evalData: <PolynomialEvalV2::EvalData as alloy::sol_types::SolType>::RustType,
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
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
                alloy::sol_types::private::primitives::aliases::U256,
                [alloy::sol_types::private::primitives::aliases::U256; 11usize],
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
            impl ::core::convert::From<evalDataGenCall> for UnderlyingRustTuple<'_> {
                fn from(value: evalDataGenCall) -> Self {
                    (value.domain, value.zeta, value.publicInput)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evalDataGenCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        domain: tuple.0,
                        zeta: tuple.1,
                        publicInput: tuple.2,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (PolynomialEvalV2::EvalData,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<PolynomialEvalV2::EvalData as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<evalDataGenReturn> for UnderlyingRustTuple<'_> {
                fn from(value: evalDataGenReturn) -> Self {
                    (value.evalData,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evalDataGenReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { evalData: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for evalDataGenCall {
            type Parameters<'a> = (
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = evalDataGenReturn;
            type ReturnTuple<'a> = (PolynomialEvalV2::EvalData,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "evalDataGen((uint256,uint256,uint256[11]),uint256,uint256[11])";
            const SELECTOR: [u8; 4] = [161u8, 151u8, 175u8, 196u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <PolynomialEvalV2::EvalDomain as alloy_sol_types::SolType>::tokenize(
                        &self.domain,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.zeta,
                    ),
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::SolType>::tokenize(&self.publicInput),
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
    /**Function with signature `evaluateLagrangeOne((uint256,uint256,uint256[11]),uint256,uint256)` and selector `0x5a634f53`.
    ```solidity
    function evaluateLagrangeOne(PolynomialEvalV2.EvalDomain memory domain, BN254.ScalarField zeta, BN254.ScalarField vanishEval) external view returns (BN254.ScalarField res);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluateLagrangeOneCall {
        #[allow(missing_docs)]
        pub domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub zeta: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub vanishEval: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
    }
    ///Container type for the return parameters of the [`evaluateLagrangeOne((uint256,uint256,uint256[11]),uint256,uint256)`](evaluateLagrangeOneCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluateLagrangeOneReturn {
        #[allow(missing_docs)]
        pub res: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
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
                PolynomialEvalV2::EvalDomain,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<evaluateLagrangeOneCall> for UnderlyingRustTuple<'_> {
                fn from(value: evaluateLagrangeOneCall) -> Self {
                    (value.domain, value.zeta, value.vanishEval)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluateLagrangeOneCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        domain: tuple.0,
                        zeta: tuple.1,
                        vanishEval: tuple.2,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (BN254::ScalarField,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<BN254::ScalarField as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<evaluateLagrangeOneReturn> for UnderlyingRustTuple<'_> {
                fn from(value: evaluateLagrangeOneReturn) -> Self {
                    (value.res,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluateLagrangeOneReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { res: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for evaluateLagrangeOneCall {
            type Parameters<'a> = (
                PolynomialEvalV2::EvalDomain,
                BN254::ScalarField,
                BN254::ScalarField,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = evaluateLagrangeOneReturn;
            type ReturnTuple<'a> = (BN254::ScalarField,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "evaluateLagrangeOne((uint256,uint256,uint256[11]),uint256,uint256)";
            const SELECTOR: [u8; 4] = [90u8, 99u8, 79u8, 83u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <PolynomialEvalV2::EvalDomain as alloy_sol_types::SolType>::tokenize(
                        &self.domain,
                    ),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.zeta),
                    <BN254::ScalarField as alloy_sol_types::SolType>::tokenize(&self.vanishEval),
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
    /**Function with signature `evaluatePiPoly((uint256,uint256,uint256[11]),uint256[11],uint256,uint256)` and selector `0xbd00369a`.
    ```solidity
    function evaluatePiPoly(PolynomialEvalV2.EvalDomain memory domain, uint256[11] memory pi, uint256 zeta, uint256 vanishingPolyEval) external view returns (uint256 res);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluatePiPolyCall {
        #[allow(missing_docs)]
        pub domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub pi: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
        #[allow(missing_docs)]
        pub zeta: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub vanishingPolyEval: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`evaluatePiPoly((uint256,uint256,uint256[11]),uint256[11],uint256,uint256)`](evaluatePiPolyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluatePiPolyReturn {
        #[allow(missing_docs)]
        pub res: alloy::sol_types::private::primitives::aliases::U256,
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
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
                [alloy::sol_types::private::primitives::aliases::U256; 11usize],
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
            impl ::core::convert::From<evaluatePiPolyCall> for UnderlyingRustTuple<'_> {
                fn from(value: evaluatePiPolyCall) -> Self {
                    (value.domain, value.pi, value.zeta, value.vanishingPolyEval)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluatePiPolyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        domain: tuple.0,
                        pi: tuple.1,
                        zeta: tuple.2,
                        vanishingPolyEval: tuple.3,
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
            impl ::core::convert::From<evaluatePiPolyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: evaluatePiPolyReturn) -> Self {
                    (value.res,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluatePiPolyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { res: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for evaluatePiPolyCall {
            type Parameters<'a> = (
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::Uint<256>,
                    11usize,
                >,
                alloy::sol_types::sol_data::Uint<256>,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = evaluatePiPolyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "evaluatePiPoly((uint256,uint256,uint256[11]),uint256[11],uint256,uint256)";
            const SELECTOR: [u8; 4] = [189u8, 0u8, 54u8, 154u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <PolynomialEvalV2::EvalDomain as alloy_sol_types::SolType>::tokenize(
                        &self.domain,
                    ),
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::Uint<256>,
                        11usize,
                    > as alloy_sol_types::SolType>::tokenize(&self.pi),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.zeta,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.vanishingPolyEval,
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
    /**Function with signature `evaluateVanishingPoly((uint256,uint256,uint256[11]),uint256)` and selector `0x7e6e47b4`.
    ```solidity
    function evaluateVanishingPoly(PolynomialEvalV2.EvalDomain memory domain, uint256 zeta) external pure returns (uint256 res);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluateVanishingPolyCall {
        #[allow(missing_docs)]
        pub domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
        #[allow(missing_docs)]
        pub zeta: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`evaluateVanishingPoly((uint256,uint256,uint256[11]),uint256)`](evaluateVanishingPolyCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct evaluateVanishingPolyReturn {
        #[allow(missing_docs)]
        pub res: alloy::sol_types::private::primitives::aliases::U256,
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
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::Uint<256>,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<evaluateVanishingPolyCall> for UnderlyingRustTuple<'_> {
                fn from(value: evaluateVanishingPolyCall) -> Self {
                    (value.domain, value.zeta)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluateVanishingPolyCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        domain: tuple.0,
                        zeta: tuple.1,
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
            impl ::core::convert::From<evaluateVanishingPolyReturn> for UnderlyingRustTuple<'_> {
                fn from(value: evaluateVanishingPolyReturn) -> Self {
                    (value.res,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for evaluateVanishingPolyReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { res: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for evaluateVanishingPolyCall {
            type Parameters<'a> = (
                PolynomialEvalV2::EvalDomain,
                alloy::sol_types::sol_data::Uint<256>,
            );
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = evaluateVanishingPolyReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str =
                "evaluateVanishingPoly((uint256,uint256,uint256[11]),uint256)";
            const SELECTOR: [u8; 4] = [126u8, 110u8, 71u8, 180u8];
            #[inline]
            fn new<'a>(
                tuple: <Self::Parameters<'a> as alloy_sol_types::SolType>::RustType,
            ) -> Self {
                tuple.into()
            }
            #[inline]
            fn tokenize(&self) -> Self::Token<'_> {
                (
                    <PolynomialEvalV2::EvalDomain as alloy_sol_types::SolType>::tokenize(
                        &self.domain,
                    ),
                    <alloy::sol_types::sol_data::Uint<256> as alloy_sol_types::SolType>::tokenize(
                        &self.zeta,
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
    /**Function with signature `newEvalDomain(uint256)` and selector `0x82d8a099`.
    ```solidity
    function newEvalDomain(uint256 domainSize) external pure returns (PolynomialEvalV2.EvalDomain memory);
    ```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newEvalDomainCall {
        #[allow(missing_docs)]
        pub domainSize: alloy::sol_types::private::primitives::aliases::U256,
    }
    ///Container type for the return parameters of the [`newEvalDomain(uint256)`](newEvalDomainCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct newEvalDomainReturn {
        #[allow(missing_docs)]
        pub _0: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<newEvalDomainCall> for UnderlyingRustTuple<'_> {
                fn from(value: newEvalDomainCall) -> Self {
                    (value.domainSize,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newEvalDomainCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        domainSize: tuple.0,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            type UnderlyingSolTuple<'a> = (PolynomialEvalV2::EvalDomain,);
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> =
                (<PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,);
            #[cfg(test)]
            #[allow(dead_code, unreachable_patterns)]
            fn _type_assertion(_t: alloy_sol_types::private::AssertTypeEq<UnderlyingRustTuple>) {
                match _t {
                    alloy_sol_types::private::AssertTypeEq::<
                        <UnderlyingSolTuple as alloy_sol_types::SolType>::RustType,
                    >(_) => {},
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<newEvalDomainReturn> for UnderlyingRustTuple<'_> {
                fn from(value: newEvalDomainReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>> for newEvalDomainReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for newEvalDomainCall {
            type Parameters<'a> = (alloy::sol_types::sol_data::Uint<256>,);
            type Token<'a> = <Self::Parameters<'a> as alloy_sol_types::SolType>::Token<'a>;
            type Return = newEvalDomainReturn;
            type ReturnTuple<'a> = (PolynomialEvalV2::EvalDomain,);
            type ReturnToken<'a> = <Self::ReturnTuple<'a> as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "newEvalDomain(uint256)";
            const SELECTOR: [u8; 4] = [130u8, 216u8, 160u8, 153u8];
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
                        &self.domainSize,
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
        evalDataGen(evalDataGenCall),
        #[allow(missing_docs)]
        evaluateLagrangeOne(evaluateLagrangeOneCall),
        #[allow(missing_docs)]
        evaluatePiPoly(evaluatePiPolyCall),
        #[allow(missing_docs)]
        evaluateVanishingPoly(evaluateVanishingPolyCall),
        #[allow(missing_docs)]
        newEvalDomain(newEvalDomainCall),
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
            [90u8, 99u8, 79u8, 83u8],
            [126u8, 110u8, 71u8, 180u8],
            [130u8, 216u8, 160u8, 153u8],
            [131u8, 76u8, 69u8, 42u8],
            [161u8, 151u8, 175u8, 196u8],
            [171u8, 149u8, 158u8, 227u8],
            [175u8, 25u8, 107u8, 162u8],
            [189u8, 0u8, 54u8, 154u8],
            [222u8, 36u8, 172u8, 15u8],
            [227u8, 81u8, 45u8, 86u8],
            [245u8, 20u8, 67u8, 38u8],
        ];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for PlonkVerifierV2Calls {
        const NAME: &'static str = "PlonkVerifierV2Calls";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 14usize;
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
                Self::evalDataGen(_) => <evalDataGenCall as alloy_sol_types::SolCall>::SELECTOR,
                Self::evaluateLagrangeOne(_) => {
                    <evaluateLagrangeOneCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::evaluatePiPoly(_) => {
                    <evaluatePiPolyCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::evaluateVanishingPoly(_) => {
                    <evaluateVanishingPolyCall as alloy_sol_types::SolCall>::SELECTOR
                },
                Self::newEvalDomain(_) => <newEvalDomainCall as alloy_sol_types::SolCall>::SELECTOR,
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
                    fn evaluateLagrangeOne(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <evaluateLagrangeOneCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Calls::evaluateLagrangeOne)
                    }
                    evaluateLagrangeOne
                },
                {
                    fn evaluateVanishingPoly(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <evaluateVanishingPolyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Calls::evaluateVanishingPoly)
                    }
                    evaluateVanishingPoly
                },
                {
                    fn newEvalDomain(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <newEvalDomainCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Calls::newEvalDomain)
                    }
                    newEvalDomain
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
                    fn evalDataGen(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <evalDataGenCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Calls::evalDataGen)
                    }
                    evalDataGen
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
                    fn evaluatePiPoly(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<PlonkVerifierV2Calls> {
                        <evaluatePiPolyCall as alloy_sol_types::SolCall>::abi_decode_raw(
                            data, validate,
                        )
                        .map(PlonkVerifierV2Calls::evaluatePiPoly)
                    }
                    evaluatePiPoly
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
                Self::evalDataGen(inner) => {
                    <evalDataGenCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::evaluateLagrangeOne(inner) => {
                    <evaluateLagrangeOneCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::evaluatePiPoly(inner) => {
                    <evaluatePiPolyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::evaluateVanishingPoly(inner) => {
                    <evaluateVanishingPolyCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
                },
                Self::newEvalDomain(inner) => {
                    <newEvalDomainCall as alloy_sol_types::SolCall>::abi_encoded_size(inner)
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
                Self::evalDataGen(inner) => {
                    <evalDataGenCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::evaluateLagrangeOne(inner) => {
                    <evaluateLagrangeOneCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::evaluatePiPoly(inner) => {
                    <evaluatePiPolyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::evaluateVanishingPoly(inner) => {
                    <evaluateVanishingPolyCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner, out,
                    )
                },
                Self::newEvalDomain(inner) => {
                    <newEvalDomainCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
                Self::verify(inner) => {
                    <verifyCall as alloy_sol_types::SolCall>::abi_encode_raw(inner, out)
                },
            }
        }
    }
    ///Container for all the [`PlonkVerifierV2`](self) custom errors.
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
        ///Creates a new call builder for the [`evalDataGen`] function.
        pub fn evalDataGen(
            &self,
            domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
            zeta: alloy::sol_types::private::primitives::aliases::U256,
            publicInput: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
        ) -> alloy_contract::SolCallBuilder<T, &P, evalDataGenCall, N> {
            self.call_builder(&evalDataGenCall {
                domain,
                zeta,
                publicInput,
            })
        }
        ///Creates a new call builder for the [`evaluateLagrangeOne`] function.
        pub fn evaluateLagrangeOne(
            &self,
            domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
            zeta: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
            vanishEval: <BN254::ScalarField as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, evaluateLagrangeOneCall, N> {
            self.call_builder(&evaluateLagrangeOneCall {
                domain,
                zeta,
                vanishEval,
            })
        }
        ///Creates a new call builder for the [`evaluatePiPoly`] function.
        pub fn evaluatePiPoly(
            &self,
            domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
            pi: [alloy::sol_types::private::primitives::aliases::U256; 11usize],
            zeta: alloy::sol_types::private::primitives::aliases::U256,
            vanishingPolyEval: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, evaluatePiPolyCall, N> {
            self.call_builder(&evaluatePiPolyCall {
                domain,
                pi,
                zeta,
                vanishingPolyEval,
            })
        }
        ///Creates a new call builder for the [`evaluateVanishingPoly`] function.
        pub fn evaluateVanishingPoly(
            &self,
            domain: <PolynomialEvalV2::EvalDomain as alloy::sol_types::SolType>::RustType,
            zeta: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, evaluateVanishingPolyCall, N> {
            self.call_builder(&evaluateVanishingPolyCall { domain, zeta })
        }
        ///Creates a new call builder for the [`newEvalDomain`] function.
        pub fn newEvalDomain(
            &self,
            domainSize: alloy::sol_types::private::primitives::aliases::U256,
        ) -> alloy_contract::SolCallBuilder<T, &P, newEvalDomainCall, N> {
            self.call_builder(&newEvalDomainCall { domainSize })
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
