use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// ethers-compatible/equivalent serialization
pub mod ethers_serde {
    use super::*;

    /// for alloy's U256
    pub mod u256 {
        use alloy::primitives::U256;
        use ethers_core::types::U256 as EthersU256;

        use super::*;

        pub fn serialize<S: Serializer>(v: &U256, serializer: S) -> Result<S::Ok, S::Error> {
            let v_ethers = EthersU256(v.into_limbs());
            v_ethers.serialize(serializer)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<U256, D::Error> {
            let v_ethers = EthersU256::deserialize(deserializer)?;
            Ok(U256::from_limbs(v_ethers.0))
        }

        #[test]
        fn test_u256_serde() {
            use rand::Rng;

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "u256")] pub U256);

            let mut bytes = [0u8; 32];
            rand::thread_rng().fill(&mut bytes);
            let v = Wrapper(U256::from_le_bytes(bytes));
            let v_ethers = EthersU256::from_little_endian(&bytes);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );

            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    /// for alloy's U256
    pub mod u512 {
        use alloy::primitives::U512;
        use ethers_core::types::U512 as EthersU512;

        use super::*;

        pub fn serialize<S: Serializer>(v: &U512, serializer: S) -> Result<S::Ok, S::Error> {
            let v_ethers = EthersU512(v.into_limbs());
            v_ethers.serialize(serializer)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<U512, D::Error> {
            let v_ethers = EthersU512::deserialize(deserializer)?;
            Ok(U512::from_limbs(v_ethers.0))
        }

        #[test]
        fn test_u512_serde() {
            use rand::Rng;

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "u512")] pub U512);

            let mut bytes = [0u8; 64];
            rand::thread_rng().fill(&mut bytes);
            let v = Wrapper(U512::from_le_bytes(bytes));
            let v_ethers = EthersU512::from_little_endian(&bytes);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    /// for alloy's Address
    pub mod address {
        use alloy::primitives::Address;
        use ethers_core::types::{Address as EthersAddress, H160};

        use super::*;

        pub fn serialize<S: Serializer>(v: &Address, serializer: S) -> Result<S::Ok, S::Error> {
            let v_ethers = H160(v.0 .0);
            v_ethers.serialize(serializer)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Address, D::Error> {
            let v_ethers = EthersAddress::deserialize(deserializer)?;
            Ok(Address::new(v_ethers.0))
        }

        #[test]
        fn test_address_serde() {
            use rand::Rng;

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "address")] pub Address);

            let mut bytes = [0u8; 20];
            rand::thread_rng().fill(&mut bytes);
            let v = Wrapper(Address::from_slice(&bytes));
            let v_ethers = EthersAddress::from(&bytes);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    /// for Option<Address>
    pub mod option_address {
        use alloy::primitives::Address;
        use ethers_core::types::{Address as EthersAddress, H160};

        use super::*;

        pub fn serialize<S: Serializer>(
            v: &Option<Address>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            if let Some(v) = v {
                let v_ethers = H160(v.0 .0);
                Some(v_ethers).serialize(serializer)
            } else {
                v.serialize(serializer)
            }
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<Address>, D::Error> {
            let v_ethers = Option::<EthersAddress>::deserialize(deserializer)?;
            Ok(v_ethers.map(|v| Address::new(v.0)))
        }

        #[test]
        fn test_option_address_serde() {
            use rand::Rng;

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "option_address")] pub Option<Address>);

            let mut bytes = [0u8; 20];
            rand::thread_rng().fill(&mut bytes);
            let v = Wrapper(Some(Address::from_slice(&bytes)));
            let v_ethers = Some(EthersAddress::from(&bytes));
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);

            let v = Wrapper(None);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize::<Option<EthersAddress>>(&None).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    use alloy::primitives::{PrimitiveSignature, U256};
    /// the compatible representation of the old ethers' signature
    /// especially the `v` parity field.
    #[derive(Serialize, Deserialize)]
    #[serde(rename = "Signature")]
    struct CompatRepr {
        #[serde(with = "u256")]
        r: U256,
        #[serde(with = "u256")]
        s: U256,
        v: u64,
    }

    impl From<CompatRepr> for PrimitiveSignature {
        fn from(repr: CompatRepr) -> Self {
            PrimitiveSignature::new(repr.r, repr.s, repr.v == 28)
        }
    }
    impl From<&PrimitiveSignature> for CompatRepr {
        fn from(sig: &PrimitiveSignature) -> Self {
            CompatRepr {
                r: sig.r(),
                s: sig.s(),
                v: if sig.v() { 28u64 } else { 27u64 },
            }
        }
    }

    /// PrimitiveSignature
    pub mod signature {
        use super::*;

        pub fn serialize<S: Serializer>(
            sig: &PrimitiveSignature,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            let repr: CompatRepr = sig.into();
            repr.serialize(serializer)
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<PrimitiveSignature, D::Error> {
            let repr = CompatRepr::deserialize(deserializer)?;
            if repr.v != 27 && repr.v != 28 {
                return Err(serde::de::Error::custom("wrong v, only 27 or 28"));
            }
            Ok(repr.into())
        }

        #[test]
        fn test_signature_serde() {
            use alloy::primitives::b256;
            use ethers_core::types::{Signature as EthersSignature, U256 as EthersU256};

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "signature")] pub PrimitiveSignature);

            let v = Wrapper(PrimitiveSignature::from_scalars_and_parity(
                b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
                b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
                false,
            ));
            let v_ethers = EthersSignature {
                r: EthersU256::from(
                    "840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565",
                ),
                s: EthersU256::from(
                    "25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1",
                ),
                v: 27,
            };
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    /// Option<PrimitiveSignature>
    pub mod option_signature {
        use super::*;

        pub fn serialize<S: Serializer, T: Into<PrimitiveSignature> + Serialize + Clone>(
            sig: &Option<T>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            if let Some(sig) = sig {
                let sig: PrimitiveSignature = sig.clone().into();
                let repr: CompatRepr = (&sig).into();
                Some(repr).serialize(serializer)
            } else {
                sig.serialize(serializer)
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>, T: From<PrimitiveSignature>>(
            deserializer: D,
        ) -> Result<Option<T>, D::Error> {
            let repr = <Option<CompatRepr>>::deserialize(deserializer)?;

            Ok(repr.map(|v| {
                let sig: PrimitiveSignature = v.into();
                sig.into()
            }))
        }

        #[test]
        fn test_option_signature_serde() {
            use alloy::primitives::b256;
            use ethers_core::types::{Signature as EthersSignature, U256 as EthersU256};

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "option_signature")] pub Option<PrimitiveSignature>);

            let v = Wrapper(Some(PrimitiveSignature::from_scalars_and_parity(
                b256!("840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565"),
                b256!("25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1"),
                false,
            )));
            let v_ethers = Some(EthersSignature {
                r: EthersU256::from(
                    "840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565",
                ),
                s: EthersU256::from(
                    "25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1",
                ),
                v: 27,
            });
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);

            let v = Wrapper(None);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize::<Option<EthersSignature>>(&None).unwrap()
            );
            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }

    /// for B256
    pub mod b256 {
        use alloy::primitives::B256;
        use ethers_core::types::H256;

        use super::*;

        pub fn serialize<S: Serializer>(v: &B256, serializer: S) -> Result<S::Ok, S::Error> {
            H256(v.0).serialize(serializer)
        }
        pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<B256, D::Error> {
            let v_ethers = H256::deserialize(deserializer)?;
            Ok(B256::new(v_ethers.0))
        }

        #[test]
        fn test_b256_serde() {
            use rand::Rng;

            #[derive(Serialize, Deserialize, Debug, PartialEq)]
            #[serde(transparent)]
            struct Wrapper(#[serde(with = "b256")] pub B256);

            let mut bytes = [0u8; 32];
            rand::thread_rng().fill(&mut bytes);
            let v = Wrapper(B256::new(bytes));
            let v_ethers = H256(bytes);
            assert_eq!(
                bincode::serialize(&v).unwrap(),
                bincode::serialize(&v_ethers).unwrap()
            );

            let de: Wrapper = bincode::deserialize(&bincode::serialize(&v).unwrap()).unwrap();
            assert_eq!(de, v);
        }
    }
}
