///Module containing a contract's types and functions.
/**

```solidity
library IRewardClaim {
    struct LifetimeRewardsProof { bytes32[] siblings; }
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**```solidity
struct LifetimeRewardsProof { bytes32[] siblings; }
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct LifetimeRewardsProof {
        #[allow(missing_docs)]
        pub siblings: alloy::sol_types::private::Vec<
            alloy::sol_types::private::FixedBytes<32>,
        >,
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
            alloy::sol_types::sol_data::Array<
                alloy::sol_types::sol_data::FixedBytes<32>,
            >,
        );
        #[doc(hidden)]
        type UnderlyingRustTuple<'a> = (
            alloy::sol_types::private::Vec<alloy::sol_types::private::FixedBytes<32>>,
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
                    <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::FixedBytes<32>,
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
                    "LifetimeRewardsProof(bytes32[] siblings)",
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
                <alloy::sol_types::sol_data::Array<
                    alloy::sol_types::sol_data::FixedBytes<32>,
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
                    + <alloy::sol_types::sol_data::Array<
                        alloy::sol_types::sol_data::FixedBytes<32>,
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
                <alloy::sol_types::sol_data::Array<
                    alloy::sol_types::sol_data::FixedBytes<32>,
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
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> IRewardClaimInstance<T, P, N> {
        IRewardClaimInstance::<T, P, N>::new(address, provider)
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
    pub struct IRewardClaimInstance<T, P, N = alloy_contract::private::Ethereum> {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for IRewardClaimInstance<T, P, N> {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple("IRewardClaimInstance").field(&self.address).finish()
        }
    }
    /// Instantiation and getters/setters.
    #[automatically_derived]
    impl<
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > IRewardClaimInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`IRewardClaim`](self) contract instance.

See the [wrapper's documentation](`IRewardClaimInstance`) for more details.*/
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
    impl<T, P: ::core::clone::Clone, N> IRewardClaimInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> IRewardClaimInstance<T, P, N> {
            IRewardClaimInstance {
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
    > IRewardClaimInstance<T, P, N> {
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
    > IRewardClaimInstance<T, P, N> {
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
library IRewardClaim {
    struct LifetimeRewardsProof {
        bytes32[] siblings;
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
            "type": "bytes32[]",
            "internalType": "bytes32[]"
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
    ///0x6080604052348015600e575f5ffd5b506103188061001c5f395ff3fe608060405234801561000f575f5ffd5b5060043610610029575f3560e01c8063178d28261461002d575b5f5ffd5b61004061003b366004610165565b610054565b604051901515815260200160405180910390f35b5f84610069858561006486610241565b610073565b1495945050505050565b805180515f919060a01461009a576040516313717da960e21b815260040160405180910390fd5b5f6100a485610119565b90505f5b60a081101561010f575f8382815181106100c4576100c46102f7565b60209081029190910101519050600188831c1680156100f3576040805183815260208101869052209350610105565b60408051858152602081018490522093505b50506001016100a8565b5095945050505050565b5f5f8260405160200161012e91815260200190565b60408051808303601f1901815282825280516020918201208184015281518084038201815292820190915281519101209392505050565b5f5f5f5f60808587031215610178575f5ffd5b8435935060208501356001600160a01b0381168114610195575f5ffd5b925060408501359150606085013567ffffffffffffffff8111156101b7575f5ffd5b8501602081880312156101c8575f5ffd5b939692955090935050565b634e487b7160e01b5f52604160045260245ffd5b6040516020810167ffffffffffffffff8111828210171561020a5761020a6101d3565b60405290565b604051601f8201601f1916810167ffffffffffffffff81118282101715610239576102396101d3565b604052919050565b5f60208236031215610251575f5ffd5b6102596101e7565b823567ffffffffffffffff81111561026f575f5ffd5b830136601f82011261027f575f5ffd5b803567ffffffffffffffff811115610299576102996101d3565b8060051b6102a960208201610210565b918252602081840181019290810190368411156102c4575f5ffd5b6020850194505b838510156102ea578435808352602095860195909350909101906102cb565b8552509295945050505050565b634e487b7160e01b5f52603260045260245ffdfea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15`\x0EW__\xFD[Pa\x03\x18\x80a\0\x1C_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0)W_5`\xE0\x1C\x80c\x17\x8D(&\x14a\0-W[__\xFD[a\0@a\0;6`\x04a\x01eV[a\0TV[`@Q\x90\x15\x15\x81R` \x01`@Q\x80\x91\x03\x90\xF3[_\x84a\0i\x85\x85a\0d\x86a\x02AV[a\0sV[\x14\x95\x94PPPPPV[\x80Q\x80Q_\x91\x90`\xA0\x14a\0\x9AW`@Qc\x13q}\xA9`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\0\xA4\x85a\x01\x19V[\x90P_[`\xA0\x81\x10\x15a\x01\x0FW_\x83\x82\x81Q\x81\x10a\0\xC4Wa\0\xC4a\x02\xF7V[` \x90\x81\x02\x91\x90\x91\x01\x01Q\x90P`\x01\x88\x83\x1C\x16\x80\x15a\0\xF3W`@\x80Q\x83\x81R` \x81\x01\x86\x90R \x93Pa\x01\x05V[`@\x80Q\x85\x81R` \x81\x01\x84\x90R \x93P[PP`\x01\x01a\0\xA8V[P\x95\x94PPPPPV[__\x82`@Q` \x01a\x01.\x91\x81R` \x01\x90V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01R\x81Q\x80\x84\x03\x82\x01\x81R\x92\x82\x01\x90\x91R\x81Q\x91\x01 \x93\x92PPPV[____`\x80\x85\x87\x03\x12\x15a\x01xW__\xFD[\x845\x93P` \x85\x015`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x01\x95W__\xFD[\x92P`@\x85\x015\x91P``\x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x01\xB7W__\xFD[\x85\x01` \x81\x88\x03\x12\x15a\x01\xC8W__\xFD[\x93\x96\x92\x95P\x90\x93PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q` \x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x02\nWa\x02\na\x01\xD3V[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x029Wa\x029a\x01\xD3V[`@R\x91\x90PV[_` \x826\x03\x12\x15a\x02QW__\xFD[a\x02Ya\x01\xE7V[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02oW__\xFD[\x83\x016`\x1F\x82\x01\x12a\x02\x7FW__\xFD[\x805g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02\x99Wa\x02\x99a\x01\xD3V[\x80`\x05\x1Ba\x02\xA9` \x82\x01a\x02\x10V[\x91\x82R` \x81\x84\x01\x81\x01\x92\x90\x81\x01\x906\x84\x11\x15a\x02\xC4W__\xFD[` \x85\x01\x94P[\x83\x85\x10\x15a\x02\xEAW\x845\x80\x83R` \x95\x86\x01\x95\x90\x93P\x90\x91\x01\x90a\x02\xCBV[\x85RP\x92\x95\x94PPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    /// The runtime bytecode of the contract, as deployed on the network.
    ///
    /// ```text
    ///0x608060405234801561000f575f5ffd5b5060043610610029575f3560e01c8063178d28261461002d575b5f5ffd5b61004061003b366004610165565b610054565b604051901515815260200160405180910390f35b5f84610069858561006486610241565b610073565b1495945050505050565b805180515f919060a01461009a576040516313717da960e21b815260040160405180910390fd5b5f6100a485610119565b90505f5b60a081101561010f575f8382815181106100c4576100c46102f7565b60209081029190910101519050600188831c1680156100f3576040805183815260208101869052209350610105565b60408051858152602081018490522093505b50506001016100a8565b5095945050505050565b5f5f8260405160200161012e91815260200190565b60408051808303601f1901815282825280516020918201208184015281518084038201815292820190915281519101209392505050565b5f5f5f5f60808587031215610178575f5ffd5b8435935060208501356001600160a01b0381168114610195575f5ffd5b925060408501359150606085013567ffffffffffffffff8111156101b7575f5ffd5b8501602081880312156101c8575f5ffd5b939692955090935050565b634e487b7160e01b5f52604160045260245ffd5b6040516020810167ffffffffffffffff8111828210171561020a5761020a6101d3565b60405290565b604051601f8201601f1916810167ffffffffffffffff81118282101715610239576102396101d3565b604052919050565b5f60208236031215610251575f5ffd5b6102596101e7565b823567ffffffffffffffff81111561026f575f5ffd5b830136601f82011261027f575f5ffd5b803567ffffffffffffffff811115610299576102996101d3565b8060051b6102a960208201610210565b918252602081840181019290810190368411156102c4575f5ffd5b6020850194505b838510156102ea578435808352602095860195909350909101906102cb565b8552509295945050505050565b634e487b7160e01b5f52603260045260245ffdfea164736f6c634300081c000a
    /// ```
    #[rustfmt::skip]
    #[allow(clippy::all)]
    pub static DEPLOYED_BYTECODE: alloy_sol_types::private::Bytes = alloy_sol_types::private::Bytes::from_static(
        b"`\x80`@R4\x80\x15a\0\x0FW__\xFD[P`\x046\x10a\0)W_5`\xE0\x1C\x80c\x17\x8D(&\x14a\0-W[__\xFD[a\0@a\0;6`\x04a\x01eV[a\0TV[`@Q\x90\x15\x15\x81R` \x01`@Q\x80\x91\x03\x90\xF3[_\x84a\0i\x85\x85a\0d\x86a\x02AV[a\0sV[\x14\x95\x94PPPPPV[\x80Q\x80Q_\x91\x90`\xA0\x14a\0\x9AW`@Qc\x13q}\xA9`\xE2\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_a\0\xA4\x85a\x01\x19V[\x90P_[`\xA0\x81\x10\x15a\x01\x0FW_\x83\x82\x81Q\x81\x10a\0\xC4Wa\0\xC4a\x02\xF7V[` \x90\x81\x02\x91\x90\x91\x01\x01Q\x90P`\x01\x88\x83\x1C\x16\x80\x15a\0\xF3W`@\x80Q\x83\x81R` \x81\x01\x86\x90R \x93Pa\x01\x05V[`@\x80Q\x85\x81R` \x81\x01\x84\x90R \x93P[PP`\x01\x01a\0\xA8V[P\x95\x94PPPPPV[__\x82`@Q` \x01a\x01.\x91\x81R` \x01\x90V[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01R\x81Q\x80\x84\x03\x82\x01\x81R\x92\x82\x01\x90\x91R\x81Q\x91\x01 \x93\x92PPPV[____`\x80\x85\x87\x03\x12\x15a\x01xW__\xFD[\x845\x93P` \x85\x015`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x01\x95W__\xFD[\x92P`@\x85\x015\x91P``\x85\x015g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x01\xB7W__\xFD[\x85\x01` \x81\x88\x03\x12\x15a\x01\xC8W__\xFD[\x93\x96\x92\x95P\x90\x93PPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q` \x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x02\nWa\x02\na\x01\xD3V[`@R\x90V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x029Wa\x029a\x01\xD3V[`@R\x91\x90PV[_` \x826\x03\x12\x15a\x02QW__\xFD[a\x02Ya\x01\xE7V[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02oW__\xFD[\x83\x016`\x1F\x82\x01\x12a\x02\x7FW__\xFD[\x805g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x02\x99Wa\x02\x99a\x01\xD3V[\x80`\x05\x1Ba\x02\xA9` \x82\x01a\x02\x10V[\x91\x82R` \x81\x84\x01\x81\x01\x92\x90\x81\x01\x906\x84\x11\x15a\x02\xC4W__\xFD[` \x85\x01\x94P[\x83\x85\x10\x15a\x02\xEAW\x845\x80\x83R` \x95\x86\x01\x95\x90\x93P\x90\x91\x01\x90a\x02\xCBV[\x85RP\x92\x95\x94PPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x1C\0\n",
    );
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Custom error with signature `InvalidProofLength()` and selector `0x4dc5f6a4`.
```solidity
error InvalidProofLength();
```*/
    #[allow(non_camel_case_types, non_snake_case, clippy::pub_underscore_fields)]
    #[derive(Clone)]
    pub struct InvalidProofLength {}
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
        impl ::core::convert::From<InvalidProofLength> for UnderlyingRustTuple<'_> {
            fn from(value: InvalidProofLength) -> Self {
                ()
            }
        }
        #[automatically_derived]
        #[doc(hidden)]
        impl ::core::convert::From<UnderlyingRustTuple<'_>> for InvalidProofLength {
            fn from(tuple: UnderlyingRustTuple<'_>) -> Self {
                Self {}
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
        }
    };
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    /**Function with signature `verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[]))` and selector `0x178d2826`.
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
    #[derive(Default, Debug, PartialEq, Eq, Hash)]
    ///Container type for the return parameters of the [`verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[]))`](verifyAuthRootCommitmentCall) function.
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
            type Return = verifyAuthRootCommitmentReturn;
            type ReturnTuple<'a> = (alloy::sol_types::sol_data::Bool,);
            type ReturnToken<'a> = <Self::ReturnTuple<
                'a,
            > as alloy_sol_types::SolType>::Token<'a>;
            const SIGNATURE: &'static str = "verifyAuthRootCommitment(bytes32,address,uint256,(bytes32[]))";
            const SELECTOR: [u8; 4] = [23u8, 141u8, 40u8, 38u8];
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
    ///Container for all the [`RewardClaimPrototypeMock`](self) function calls.
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
        pub const SELECTORS: &'static [[u8; 4usize]] = &[[23u8, 141u8, 40u8, 38u8]];
    }
    #[automatically_derived]
    impl alloy_sol_types::SolInterface for RewardClaimPrototypeMockCalls {
        const NAME: &'static str = "RewardClaimPrototypeMockCalls";
        const MIN_DATA_LENGTH: usize = 160usize;
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
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
                bool,
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls>] = &[
                {
                    fn verifyAuthRootCommitment(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockCalls> {
                        <verifyAuthRootCommitmentCall as alloy_sol_types::SolCall>::abi_decode_raw(
                                data,
                                validate,
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
            DECODE_SHIMS[idx](data, validate)
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
            validate: bool,
        ) -> alloy_sol_types::Result<Self> {
            static DECODE_SHIMS: &[fn(
                &[u8],
                bool,
            ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors>] = &[
                {
                    fn InvalidProofLength(
                        data: &[u8],
                        validate: bool,
                    ) -> alloy_sol_types::Result<RewardClaimPrototypeMockErrors> {
                        <InvalidProofLength as alloy_sol_types::SolError>::abi_decode_raw(
                                data,
                                validate,
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
            DECODE_SHIMS[idx](data, validate)
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
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    >(
        address: alloy_sol_types::private::Address,
        provider: P,
    ) -> RewardClaimPrototypeMockInstance<T, P, N> {
        RewardClaimPrototypeMockInstance::<T, P, N>::new(address, provider)
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
        Output = alloy_contract::Result<RewardClaimPrototypeMockInstance<T, P, N>>,
    > {
        RewardClaimPrototypeMockInstance::<T, P, N>::deploy(provider)
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
        RewardClaimPrototypeMockInstance::<T, P, N>::deploy_builder(provider)
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
        T,
        P,
        N = alloy_contract::private::Ethereum,
    > {
        address: alloy_sol_types::private::Address,
        provider: P,
        _network_transport: ::core::marker::PhantomData<(N, T)>,
    }
    #[automatically_derived]
    impl<T, P, N> ::core::fmt::Debug for RewardClaimPrototypeMockInstance<T, P, N> {
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
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > RewardClaimPrototypeMockInstance<T, P, N> {
        /**Creates a new wrapper around an on-chain [`RewardClaimPrototypeMock`](self) contract instance.

See the [wrapper's documentation](`RewardClaimPrototypeMockInstance`) for more details.*/
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
        ) -> alloy_contract::Result<RewardClaimPrototypeMockInstance<T, P, N>> {
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
    impl<T, P: ::core::clone::Clone, N> RewardClaimPrototypeMockInstance<T, &P, N> {
        /// Clones the provider and returns a new instance with the cloned provider.
        #[inline]
        pub fn with_cloned_provider(self) -> RewardClaimPrototypeMockInstance<T, P, N> {
            RewardClaimPrototypeMockInstance {
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
    > RewardClaimPrototypeMockInstance<T, P, N> {
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
        ///Creates a new call builder for the [`verifyAuthRootCommitment`] function.
        pub fn verifyAuthRootCommitment(
            &self,
            commitment: alloy::sol_types::private::FixedBytes<32>,
            account: alloy::sol_types::private::Address,
            amount: alloy::sol_types::private::primitives::aliases::U256,
            proof: <IRewardClaim::LifetimeRewardsProof as alloy::sol_types::SolType>::RustType,
        ) -> alloy_contract::SolCallBuilder<T, &P, verifyAuthRootCommitmentCall, N> {
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
        T: alloy_contract::private::Transport + ::core::clone::Clone,
        P: alloy_contract::private::Provider<T, N>,
        N: alloy_contract::private::Network,
    > RewardClaimPrototypeMockInstance<T, P, N> {
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
