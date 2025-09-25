///Module containing a contract's types and functions.
/**

```solidity
library IRewardClaim {
    struct LifetimeRewardsProof { bytes32[160] siblings; }
}
```*/
#[allow(
    non_camel_case_types,
    non_snake_case,
    clippy::pub_underscore_fields,
    clippy::style,
    clippy::empty_structs_with_brackets
)]
pub mod IRewardClaim {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Debug, PartialEq, Eq, Hash)]
    /**```solidity
struct LifetimeRewardsProof { bytes32[160] siblings; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct LifetimeRewardsProof {
        #[allow(missing_docs)]
        pub siblings: [alloy::sol_types::private::FixedBytes<32>; 160usize],
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
        #[allow(dead_code)]
        type UnderlyingSolTuple<'a> = (
            alloy::sol_types::sol_data::FixedArray<
                alloy::sol_types::sol_data::FixedBytes<32>,
                160usize,
            >,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            [alloy::sol_types::private::FixedBytes<32>; 160usize],
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
        impl ::core::convert::From<LifetimeRewardsProof> for UnderlyingRustTuple<'_> {
            fn from(value: LifetimeRewardsProof) -> Self {
                (value.siblings,)
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for LifetimeRewardsProof {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self { siblings: tuple.0 }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolValue for LifetimeRewardsProof {
            type SolType = Self;
        }
        #[automatically_derived]
        impl alloy_sol_types::private::SolTypeValue<Self> for LifetimeRewardsProof {
            #[inline]
            fn stv_to_tokens(&self) -> <Self as alloy_sol_types::SolType>::Token<'_> {
                (
                    <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::FixedBytes<32>,
                        160usize,
                    > as alloy_sol_types::SolType>::tokenize(&self.siblings),
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
        impl alloy_sol_types::SolType for LifetimeRewardsProof {
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
        impl alloy_sol_types::SolStruct for LifetimeRewardsProof {
            const NAME: &'static str = "LifetimeRewardsProof";
            #[inline]
            fn eip712_root_type() -> alloy_sol_types::private::Cow<'static, str> {
                alloy_sol_types::private::Cow::Borrowed(
                    "LifetimeRewardsProof(bytes32[160] siblings)",
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
                <alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::FixedBytes<32>,
                    160usize,
                > as alloy_sol_types::SolType>::eip712_data_word(&self.siblings)
                    .0
                    .to_vec()
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::EventTopic for LifetimeRewardsProof {
            #[inline]
            fn topic_preimage_length(rust: &Self::RustType) -> usize {
                0usize
                    + <alloy::sol_types::sol_data::FixedArray<
                        alloy::sol_types::sol_data::FixedBytes<32>,
                        160usize,
                    > as alloy_sol_types::EventTopic>::topic_preimage_length(
                        &rust.siblings,
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
                <alloy::sol_types::sol_data::FixedArray<
                    alloy::sol_types::sol_data::FixedBytes<32>,
                    160usize,
                > as alloy_sol_types::EventTopic>::encode_topic_preimage(
                    &rust.siblings,
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
    /**Creates a new wrapper around an on-chain [`IRewardClaim`](self) contract instance.

See the [wrapper's documentation](`IRewardClaimInstance`) for more details.*/
    #[inline]
    pub const fn new<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        __provider: P,
    ) -> IRewardClaimInstance<P, N> {
        IRewardClaimInstance::<P, N>::new(address, __provider)
    }
    /**A [`IRewardClaim`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`IRewardClaim`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct IRewardClaimInstance<P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for IRewardClaimInstance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("IRewardClaimInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > IRewardClaimInstance<P, N> {
        /**Creates a new wrapper around an on-chain [`IRewardClaim`](self) contract instance.

See the [wrapper's documentation](`IRewardClaimInstance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            __provider: P,
        ) -> Self {
            Self {
                address,
                provider: __provider,
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
    impl<P: ::core::clone::Clone, N> IRewardClaimInstance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> IRewardClaimInstance<P, N> {
            IRewardClaimInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > IRewardClaimInstance<P, N> {
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
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > IRewardClaimInstance<P, N> {
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
library IRewardClaim {
    struct LifetimeRewardsProof {
        bytes32[160] siblings;
    }
}

interface RewardClaimPrototypeMock {
    error InvalidProofLength();

    function verifyAuthRootCommitment(bytes32 commitment, address account, uint256 amount, IRewardClaim.LifetimeRewardsProof memory proof) external pure returns (bool);
}
```

...which was generated by the following JSON ABI:
```json
[
  {
    "type": "function",
    "name": "verifyAuthRootCommitment",
    "inputs": [
      {
        "name": "commitment",
        "type": "bytes32",
        "internalType": "bytes32"
      },
      {
        "name": "account",
        "type": "address",
        "internalType": "address"
      },
      {
        "name": "amount",
        "type": "uint256",
        "internalType": "uint256"
      },
      {
        "name": "proof",
        "type": "tuple",
        "internalType": "struct IRewardClaim.LifetimeRewardsProof",
        "components": [
          {
            "name": "siblings",
            "type": "bytes32[160]",
            "internalType": "bytes32[160]"
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
    "type": "error",
    "name": "InvalidProofLength",
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
pub mod RewardClaimPrototypeMock {
    use super::*;
    use alloy::sol_types as alloy_sol_types;
    /// The creation / init bytecode of the contract.
    ///
    /// ```text
    ///0x6080604052348015600e575f5ffd5b506102838061001c5f395ff3fe608060405234801561000f575f5ffd5b5060043610610029575f3560e01c806353e09bb61461002d575b5f5ffd5b61004061003b366004610142565b610054565b604051901515815260200160405180910390f35b5f8461006f858561006a368790038701876101db565b610079565b1495945050505050565b80515f905f610087856100f6565b90505f5b60a08110156100ec575f838260a081106100a7576100a7610262565b60200201519050600188831c1680156100d05760408051838152602081018690522093506100e2565b60408051858152602081018490522093505b505060010161008b565b5095945050505050565b5f5f8260405160200161010b91815260200190565b60408051808303601f1901815282825280516020918201208184015281518084038201815292820190915281519101209392505050565b5f5f5f5f848603611460811215610157575f5ffd5b8535945060208601356001600160a01b0381168114610174575f5ffd5b935060408601359250611400605f198201121561018f575f5ffd5b509295919450926060019150565b634e487b7160e01b5f52604160045260245ffd5b604051611400810167ffffffffffffffff811182821017156101d5576101d561019d565b60405290565b5f6114008284031280156101ed575f5ffd5b506040516020810167ffffffffffffffff811182821017156102115761021161019d565b604052601f83018413610222575f5ffd5b61022a6101b1565b8061140085018681111561023c575f5ffd5b855b8181101561025657803584526020938401930161023e565b50508252509392505050565b634e487b7160e01b5f52603260045260245ffdfea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15`\x0EW__\xFD[Pa\x02\x83\x80a\0\x1C_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0)W_5`\xE0\x1C\x80cS\xE0\x9B\xB6\x14a\0-W[__\xFD[a\0@a\0;6`\x04a\x01BV[a\0TV[`@Q\x90\x15\x15\x81R` \x01`@Q\x80\x91\x03\x90\xF3[_\x84a\0o\x85\x85a\0j6\x87\x90\x03\x87\x01\x87a\x01\xDBV[a\0yV[\x14\x95\x94PPPPPV[\x80Q_\x90_a\0\x87\x85a\0\xF6V[\x90P_[`\xA0\x81\x10\x15a\0\xECW_\x83\x82`\xA0\x81\x10a\0\xA7Wa\0\xA7a\x02bV[` \x02\x01Q\x90P`\x01\x88\x83\x1C\x16\x80\x15a\0\xD0W`@\x80Q\x83\x81R` \x81\x01\x86\x90R \x93Pa\0\xE2V[`@\x80Q\x85\x81R` \x81\x01\x84\x90R \x93P[PP`\x01\x01a\0\x8BV[P\x95\x94PPPPPV[__\x82`@Q` \x01a\x01\x0B\x91\x81R` \x01\x90V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01R\x81Q\x80\x84\x03\x82\x01\x81R\x92\x82\x01\x90\x91R\x81Q\x91\x01 \x93\x92PPPV[____\x84\x86\x03a\x14`\x81\x12\x15a\x01WW__\xFD[\x855\x94P` \x86\x015`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x01tW__\xFD[\x93P`@\x86\x015\x92Pa\x14\0`_\x19\x82\x01\x12\x15a\x01\x8FW__\xFD[P\x92\x95\x91\x94P\x92``\x01\x91PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x14\0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x01\xD5Wa\x01\xD5a\x01\x9DV[`@R\x90V[_a\x14\0\x82\x84\x03\x12\x80\x15a\x01\xEDW__\xFD[P`@Q` \x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x02\x11Wa\x02\x11a\x01\x9DV[`@R`\x1F\x83\x01\x84\x13a\x02\"W__\xFD[a\x02*a\x01\xB1V[\x80a\x14\0\x85\x01\x86\x81\x11\x15a\x02<W__\xFD[\x85[\x81\x81\x10\x15a\x02VW\x805\x84R` \x93\x84\x01\x93\x01a\x02>V[PP\x82RP\x93\x92PPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405234801561000f575f5ffd5b5060043610610029575f3560e01c806353e09bb61461002d575b5f5ffd5b61004061003b366004610142565b610054565b604051901515815260200160405180910390f35b5f8461006f858561006a368790038701876101db565b610079565b1495945050505050565b80515f905f610087856100f6565b90505f5b60a08110156100ec575f838260a081106100a7576100a7610262565b60200201519050600188831c1680156100d05760408051838152602081018690522093506100e2565b60408051858152602081018490522093505b505060010161008b565b5095945050505050565b5f5f8260405160200161010b91815260200190565b60408051808303601f1901815282825280516020918201208184015281518084038201815292820190915281519101209392505050565b5f5f5f5f848603611460811215610157575f5ffd5b8535945060208601356001600160a01b0381168114610174575f5ffd5b935060408601359250611400605f198201121561018f575f5ffd5b509295919450926060019150565b634e487b7160e01b5f52604160045260245ffd5b604051611400810167ffffffffffffffff811182821017156101d5576101d561019d565b60405290565b5f6114008284031280156101ed575f5ffd5b506040516020810167ffffffffffffffff811182821017156102115761021161019d565b604052601f83018413610222575f5ffd5b61022a6101b1565b8061140085018681111561023c575f5ffd5b855b8181101561025657803584526020938401930161023e565b50508252509392505050565b634e487b7160e01b5f52603260045260245ffdfea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0)W_5`\xE0\x1C\x80cS\xE0\x9B\xB6\x14a\0-W[__\xFD[a\0@a\0;6`\x04a\x01BV[a\0TV[`@Q\x90\x15\x15\x81R` \x01`@Q\x80\x91\x03\x90\xF3[_\x84a\0o\x85\x85a\0j6\x87\x90\x03\x87\x01\x87a\x01\xDBV[a\0yV[\x14\x95\x94PPPPPV[\x80Q_\x90_a\0\x87\x85a\0\xF6V[\x90P_[`\xA0\x81\x10\x15a\0\xECW_\x83\x82`\xA0\x81\x10a\0\xA7Wa\0\xA7a\x02bV[` \x02\x01Q\x90P`\x01\x88\x83\x1C\x16\x80\x15a\0\xD0W`@\x80Q\x83\x81R` \x81\x01\x86\x90R \x93Pa\0\xE2V[`@\x80Q\x85\x81R` \x81\x01\x84\x90R \x93P[PP`\x01\x01a\0\x8BV[P\x95\x94PPPPPV[__\x82`@Q` \x01a\x01\x0B\x91\x81R` \x01\x90V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01R\x81Q\x80\x84\x03\x82\x01\x81R\x92\x82\x01\x90\x91R\x81Q\x91\x01 \x93\x92PPPV[____\x84\x86\x03a\x14`\x81\x12\x15a\x01WW__\xFD[\x855\x94P` \x86\x015`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x01tW__\xFD[\x93P`@\x86\x015\x92Pa\x14\0`_\x19\x82\x01\x12\x15a\x01\x8FW__\xFD[P\x92\x95\x91\x94P\x92``\x01\x91PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Qa\x14\0\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x01\xD5Wa\x01\xD5a\x01\x9DV[`@R\x90V[_a\x14\0\x82\x84\x03\x12\x80\x15a\x01\xEDW__\xFD[P`@Q` \x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x02\x11Wa\x02\x11a\x01\x9DV[`@R`\x1F\x83\x01\x84\x13a\x02\"W__\xFD[a\x02*a\x01\xB1V[\x80a\x14\0\x85\x01\x86\x81\x11\x15a\x02<W__\xFD[\x85[\x81\x81\x10\x15a\x02VW\x805\x84R` \x93\x84\x01\x93\x01a\x02>V[PP\x82RP\x93\x92PPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidProofLength()` and selector `0x4dc5f6a4`.
```solidity
error InvalidProofLength();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidProofLength;
    #[allow(
        non_camel_case_types,
        non_snake_case,
        clippy::pub_underscore_fields,
        clippy::style
    )]
    const _: () = {
        use alloy::sol_types as alloy_sol_types;
        #[doc(hidden)]
        #[allow(dead_code)]
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
        impl ::core::convert::From<InvalidProofLength> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidProofLength) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidProofLength {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolError for InvalidProofLength {
            type Parameters<'a> = UnderlyingSolTuple<'a>;
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "InvalidProofLength()";
            const SELECTOR: [u8; 4] = [77u8, 197u8, 246u8, 164u8];
            #[inline]
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
                <Self::Parameters<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence_validate(data)
                    .map(Self::new)
            }
        }
    };
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Debug, PartialEq, Eq, Hash)]
    /**Function with signature `verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[160]))` and selector `0x53e09bb6`.
```solidity
function verifyAuthRootCommitment(bytes32 commitment, address account, uint256 amount, IRewardClaim.LifetimeRewardsProof memory proof) external pure returns (bool);
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyAuthRootCommitmentCall {
        #[allow(missing_docs)]
        pub commitment: alloy::sol_types::private::FixedBytes<32>,
        #[allow(missing_docs)]
        pub account: alloy::sol_types::private::Address,
        #[allow(missing_docs)]
        pub amount: alloy::sol_types::private::primitives::aliases::U256,
        #[allow(missing_docs)]
        pub proof: <IRewardClaim::LifetimeRewardsProof as alloy::sol_types::SolType>::RustType,
    }
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[160]))`](verifyAuthRootCommitmentCall) function.
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct verifyAuthRootCommitmentReturn {
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
            #[allow(dead_code)]
            type UnderlyingSolTuple<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                IRewardClaim::LifetimeRewardsProof,
            );
            #[doc(hidden)]
            type UnderlyingRustTuple<'a> = (
                alloy::sol_types::private::FixedBytes<32>,
                alloy::sol_types::private::Address,
                alloy::sol_types::private::primitives::aliases::U256,
                <IRewardClaim::LifetimeRewardsProof as alloy::sol_types::SolType>::RustType,
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
            impl ::core::convert::From<verifyAuthRootCommitmentCall>
            for UnderlyingRustTuple<'_> {
                fn from(value: verifyAuthRootCommitmentCall) -> Self {
                    (value.commitment, value.account, value.amount, value.proof)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for verifyAuthRootCommitmentCall {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self {
                        commitment: tuple.0,
                        account: tuple.1,
                        amount: tuple.2,
                        proof: tuple.3,
                    }
                }
            }
        }
        {
            #[doc(hidden)]
            #[allow(dead_code)]
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
            impl ::core::convert::From<verifyAuthRootCommitmentReturn>
            for UnderlyingRustTuple<'_> {
                fn from(value: verifyAuthRootCommitmentReturn) -> Self {
                    (value._0,)
                }
            }
            #[automatically_derived]
            #[doc(hidden)]
            impl ::core::convert::From<UnderlyingRustTuple<'_>>
            for verifyAuthRootCommitmentReturn {
                fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                    Self { _0: tuple.0 }
                }
            }
        }
        #[automatically_derived]
        impl alloy_sol_types::SolCall for verifyAuthRootCommitmentCall {
            type Parameters<'a> = (
                alloy::sol_types::sol_data::FixedBytes<32>,
                alloy::sol_types::sol_data::Address,
                alloy::sol_types::sol_data::Uint<256>,
                IRewardClaim::LifetimeRewardsProof,
            );
            type Token<'a> = <Self::Parameters<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            type Return = bool;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[160]))";
            const SELECTOR: [u8; 4] = [83u8, 224u8, 155u8, 182u8];
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
                    > as alloy_sol_types::SolType>::tokenize(&self.commitment),
                    <alloy::sol_types::sol_data::Address as alloy_sol_types::SolType>::tokenize(
                        &self.account,
                    ),
                    <alloy::sol_types::sol_data::Uint<
                        256,
                    > as alloy_sol_types::SolType>::tokenize(&self.amount),
                    <IRewardClaim::LifetimeRewardsProof as alloy_sol_types::SolType>::tokenize(
                        &self.proof,
                    ),
                )
            }
            #[inline]
            fn tokenize_returns(ret: &Self::Return) -> Self::ReturnToken<'_> {
                (
                    <alloy::sol_types::sol_data::Bool as alloy_sol_types::SolType>::tokenize(
                        ret,
                    ),
                )
            }
            #[inline]
            fn abi_decode_returns(data: &[u8]) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence(data)
                    .map(|r| {
                        let r: verifyAuthRootCommitmentReturn = r.into();
                        r._0
                    })
            }
            #[inline]
            fn abi_decode_returns_validate(
                data: &[u8],
            ) -> alloy_sol_types::Result<Self::Return> {
                <Self::ReturnTuple<
                    '_,
                > as alloy_sol_types::SolType>::abi_decode_sequence_validate(data)
                    .map(|r| {
                        let r: verifyAuthRootCommitmentReturn = r.into();
                        r._0
                    })
            }
        }
    };
    ///Container for all the [`RewardClaimPrototypeMock`](self) function calls.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive()]
    pub enum RewardClaimPrototypeMockCalls {
        #[allow(missing_docs)]
        verifyAuthRootCommitment(verifyAuthRootCommitmentCall),
    }
    #[automatically_derived]
    impl RewardClaimPrototypeMockCalls {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[[83u8, 224u8, 155u8, 182u8]];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for RewardClaimPrototypeMockCalls {
        const NAME: &'static str = "RewardClaimPrototypeMockCalls";
        const MIN_DATA_LENGTH: usize = 5216usize;
        const COUNT: usize = 1usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::verifyAuthRootCommitment(_) => {
                    <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::SELECTOR
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
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls>] = &[
                {
                    fn verifyAuthRootCommitment(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls> {
                        <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                            )
                            .map(RewardClaimPrototypeMockCalls::verifyAuthRootCommitment)
                    }
                    verifyAuthRootCommitment
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
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls>] = &[
                {
                    fn verifyAuthRootCommitment(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls> {
                        <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw_validate(
                                data,
                            )
                            .map(RewardClaimPrototypeMockCalls::verifyAuthRootCommitment)
                    }
                    verifyAuthRootCommitment
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
            DECODE_VALIDATE_SHIMS[idx](data)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::verifyAuthRootCommitment(inner) => {
                    <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::abi_encoded_size(
                        inner,
                    )
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::verifyAuthRootCommitment(inner) => {
                    <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
            }
        }
    }
    ///Container for all the [`RewardClaimPrototypeMock`](self) custom errors.
    #[derive(serde::Serialize, serde::Deserialize)]
    #[derive(Debug, PartialEq, Eq, Hash)]
    pub enum RewardClaimPrototypeMockErrors {
        #[allow(missing_docs)]
        InvalidProofLength(InvalidProofLength),
    }
    #[automatically_derived]
    impl RewardClaimPrototypeMockErrors {
        /// All the selectors of this enum.
        ///
        /// Note that the selectors might not be in the same order as the variants.
        /// No guarantees are made about the order of the selectors.
        ///
        /// Prefer using `SolInterface` methods instead.
        pub const SELECTORS: &'static [[u8; 4usize]] = &[[77u8, 197u8, 246u8, 164u8]];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for RewardClaimPrototypeMockErrors {
        const NAME: &'static str = "RewardClaimPrototypeMockErrors";
        const MIN_DATA_LENGTH: usize = 0usize;
        const COUNT: usize = 1usize;
        #[inline]
        fn selector(&self) -> [u8; 4] {
            match self {
                Self::InvalidProofLength(_) => {
                    <InvalidProofLength as alloy_sol_types::SolError>::SELECTOR
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
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors>] = &[
                {
                    fn InvalidProofLength(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors> {
                        <InvalidProofLength as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                            )
                            .map(RewardClaimPrototypeMockErrors::InvalidProofLength)
                    }
                    InvalidProofLength
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
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors>] = &[
                {
                    fn InvalidProofLength(
                        data: &[u8],
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors> {
                        <InvalidProofLength as alloy_sol_types::SolError>::abi_decode_raw_validate(
                                data,
                            )
                            .map(RewardClaimPrototypeMockErrors::InvalidProofLength)
                    }
                    InvalidProofLength
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
            DECODE_VALIDATE_SHIMS[idx](data)
        }
        #[inline]
        fn abi_encoded_size(&self) -> usize {
            match self {
                Self::InvalidProofLength(inner) => {
                    <InvalidProofLength as alloy_sol_types::SolError>::abi_encoded_size(
                        inner,
                    )
                }
            }
        }
        #[inline]
        fn abi_encode_raw(&self, out: &mut alloy_sol_types::private::Vec<u8>) {
            match self {
                Self::InvalidProofLength(inner) => {
                    <InvalidProofLength as alloy_sol_types::SolError>::abi_encode_raw(
                        inner,
                        out,
                    )
                }
            }
        }
    }
    use alloy::contract as alloy_contract;
    /**Creates a new wrapper around an on-chain [`RewardClaimPrototypeMock`](self) contract instance.

See the [wrapper's documentation](`RewardClaimPrototypeMockInstance`) for more details.*/
    #[inline]
    pub const fn new<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        __provider: P,
    ) -> RewardClaimPrototypeMockInstance<P, N> {
        RewardClaimPrototypeMockInstance::<P, N>::new(address, __provider)
    }
    /**Deploys this contract using the given `provider` and constructor arguments, if any.

Returns a new instance of the contract, if the deployment was successful.

For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
    #[inline]
    pub fn deploy<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(
        __provider: P,
    ) -> impl ::core::future::Future<
        Output = alloy_contract::Result<RewardClaimPrototypeMockInstance<P, N>>,
    > {
        RewardClaimPrototypeMockInstance::<P, N>::deploy(__provider)
    }
    /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
and constructor arguments, if any.

This is a simple wrapper around creating a `RawCallBuilder` with the data set to
the bytecode concatenated with the constructor's ABI-encoded arguments.*/
    #[inline]
    pub fn deploy_builder<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    >(__provider: P) -> alloy_contract::RawCallBuilder<P, N> {
        RewardClaimPrototypeMockInstance::<P, N>::deploy_builder(__provider)
    }
    /**A [`RewardClaimPrototypeMock`](self) instance.

Contains type-safe methods for interacting with an on-chain instance of the
[`RewardClaimPrototypeMock`](self) contract located at a given `address`, using a given
provider `P`.

If the contract bytecode is available (see the [`sol!`](alloy_sol_types::sol!)
documentation on how to provide it), the `deploy` and `deploy_builder` methods can
be used to deploy a new instance of the contract.

See the [module-level documentation](self) for all the available methods.*/
    #[derive(Clone)]
    pub struct RewardClaimPrototypeMockInstance<
        P,
        N = alloy_contract::private::Ethereum,
    > {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network: ::core::marker::PhantomData<N>,
    }
    #[automatically_derived]
    impl<P, N> ::core::fmt::Debug for RewardClaimPrototypeMockInstance<P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("RewardClaimPrototypeMockInstance")
                .field(&self.address)
                .finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > RewardClaimPrototypeMockInstance<P, N> {
        /**Creates a new wrapper around an on-chain [`RewardClaimPrototypeMock`](self) contract instance.

See the [wrapper's documentation](`RewardClaimPrototypeMockInstance`) for more details.*/
        #[inline]
        pub const fn new(
            address: alloy_sol_types::private::Address,
            __provider: P,
        ) -> Self {
            Self {
                address,
                provider: __provider,
                _network: ::core::marker::PhantomData,
            }
        }
        /**Deploys this contract using the given `provider` and constructor arguments, if any.

Returns a new instance of the contract, if the deployment was successful.

For more fine-grained control over the deployment process, use [`deploy_builder`] instead.*/
        #[inline]
        pub async fn deploy(
            __provider: P,
        ) -> alloy_contract::Result<RewardClaimPrototypeMockInstance<P, N>> {
            let call_builder = Self::deploy_builder(__provider);
            let contract_address = call_builder.deploy().await?;
            Ok(Self::new(contract_address, call_builder.provider))
        }
        /**Creates a `RawCallBuilder` for deploying this contract using the given `provider`
and constructor arguments, if any.

This is a simple wrapper around creating a `RawCallBuilder` with the data set to
the bytecode concatenated with the constructor's ABI-encoded arguments.*/
        #[inline]
        pub fn deploy_builder(__provider: P) -> alloy_contract::RawCallBuilder<P, N> {
            alloy_contract::RawCallBuilder::new_raw_deploy(
                __provider,
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
    impl<P: ::core::clone::Clone, N> RewardClaimPrototypeMockInstance<&P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> RewardClaimPrototypeMockInstance<P, N> {
            RewardClaimPrototypeMockInstance {
                address: self.address,
                provider: ::core::clone::Clone::clone(&self.provider),
                _network: ::core::marker::PhantomData,
            }
        }
    }
    /// Function calls.
    #[automatically_derived]
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > RewardClaimPrototypeMockInstance<P, N> {
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
        ///Creates a new call builder for the [`verifyAuthRootCommitment`] function.
        pub fn verifyAuthRootCommitment(
            &self,
            commitment: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
            proof: <IRewardClaim::LifetimeRewardsProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<&P, verifyAuthRootCommitmentCall, N> {
            self.call_builder(
                &verifyAuthRootCommitmentCall {
                    commitment,
                    account,
                    amount,
                    proof,
                },
            )
        }
    }
    /// Event filters.
    #[automatically_derived]
    impl<
        P: alloy_contract::private::Provider<N>,
        N: alloy_contract::private::Network,
    > RewardClaimPrototypeMockInstance<P, N> {
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
