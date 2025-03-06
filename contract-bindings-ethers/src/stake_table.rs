pub use stake_table::*;
/// This module was auto-generated with ethers-rs Abigen.
/// More information at: <https://github.com/gakonst/ethers-rs>
#[allow(
    clippy::enum_variant_names,
    clippy::too_many_arguments,
    clippy::upper_case_acronyms,
    clippy::type_complexity,
    dead_code,
    non_camel_case_types
)]
pub mod stake_table {
    pub use super::super::shared_types::*;
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_tokenAddress"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_exitEscrowPeriod"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("uint256"),
                        ),
                    },
                    ::ethers::core::abi::ethabi::Param {
                        name: ::std::borrow::ToOwned::to_owned("_initialOwner"),
                        kind: ::ethers::core::abi::ethabi::ParamType::Address,
                        internal_type: ::core::option::Option::Some(
                            ::std::borrow::ToOwned::to_owned("address"),
                        ),
                    },
                ],
            }),
            functions: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("_hashBlsKey"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("_hashBlsKey"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("blsVK"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                            ],),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("struct BN254.G2Point"),
                            ),
                        },],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::FixedBytes(32usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bytes32"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("_isEqualBlsKey"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("_isEqualBlsKey"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("a"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G2Point"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("b"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G2Point"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bool"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::Pure,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("admin"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("admin"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("claimValidatorExit"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("claimValidatorExit"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("validator"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("claimWithdrawal"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("claimWithdrawal"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("validator"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("delegate"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("delegate"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("validator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("address"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("deregisterValidator"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("deregisterValidator",),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("exitEscrowPeriod"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("exitEscrowPeriod"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("initialize"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("initialize"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("initializedAtBlock"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("initializedAtBlock"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("owner"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("owner"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("registerValidator"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("registerValidator"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("blsVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G2Point"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("schnorrVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned(
                                        "struct EdOnBN254.EdOnBN254Point",
                                    ),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("blsSig"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G1Point"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("commission"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(16usize),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint16"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("renounceOwnership"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("renounceOwnership"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("tokenAddress"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("tokenAddress"),
                        inputs: ::std::vec![],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("transferOwnership"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("transferOwnership"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("newOwner"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("undelegate"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("undelegate"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("validator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("address"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("updateConsensusKeys"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("updateConsensusKeys",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("newBlsVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G2Point"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("newSchnorrVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned(
                                        "struct EdOnBN254.EdOnBN254Point",
                                    ),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("newBlsSig"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G1Point"),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validatorExits"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("validatorExits"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("validator"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("unlocksAt"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("validators"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("validators"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("validator"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                        outputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("isRegistered"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("bool"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("status"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(8usize),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned(
                                        "enum StakeTable.ValidatorStatus",
                                    ),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("delegatedAmount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::View,
                    },],
                ),
            ]),
            events: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("ConsensusKeysUpdated"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("ConsensusKeysUpdated",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("account"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("blsVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("schnorrVK"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("Delegated"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Delegated"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("delegator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("validator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("Initialized"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Initialized"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
                            name: ::std::borrow::ToOwned::to_owned("version"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(64usize),
                            indexed: false,
                        },],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("OwnershipTransferred"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("OwnershipTransferred",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("previousOwner"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: true,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("newOwner"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: true,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("Undelegated"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Undelegated"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("delegator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("validator"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorExit"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("ValidatorExit"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::EventParam {
                            name: ::std::borrow::ToOwned::to_owned("validator"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            indexed: false,
                        },],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorRegistered"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("ValidatorRegistered",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("account"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("blsVk"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("schnorrVk"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                    ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ],),
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("commission"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(16usize),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("Withdrawal"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("Withdrawal"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("account"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Address,
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("amount"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("BLSSigVerificationFailed"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("BLSSigVerificationFailed",),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InsufficientAllowance"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InsufficientAllowance",),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::string::String::new(),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::string::String::new(),
                                kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("uint256"),
                                ),
                            },
                        ],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InsufficientBalance"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InsufficientBalance",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
                            kind: ::ethers::core::abi::ethabi::ParamType::Uint(256usize,),
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("uint256"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidCommission"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InvalidCommission"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidInitialization"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InvalidInitialization",),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("InvalidSchnorrVK"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InvalidSchnorrVK"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NotInitializing"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("NotInitializing"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("NothingToWithdraw"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("NothingToWithdraw"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("OwnableInvalidOwner"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("OwnableInvalidOwner",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("owner"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("OwnableUnauthorizedAccount"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("OwnableUnauthorizedAccount",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("account"),
                            kind: ::ethers::core::abi::ethabi::ParamType::Address,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("address"),
                            ),
                        },],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("PrematureWithdrawal"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("PrematureWithdrawal",),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("UnknownValidator"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("UnknownValidator"),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorAlreadyExited"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ValidatorAlreadyExited",),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorAlreadyRegistered"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ValidatorAlreadyRegistered",),
                        inputs: ::std::vec![],
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("ValidatorNotExited"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("ValidatorNotExited"),
                        inputs: ::std::vec![],
                    },],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static STAKETABLE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x10W_\x80\xFD[P`@Qb\0!\xB98\x03\x80b\0!\xB9\x839\x81\x01`@\x81\x90Rb\0\x003\x91b\0\x02+V[\x80`\x01`\x01`\xA0\x1B\x03\x81\x16b\0\0bW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01`@Q\x80\x91\x03\x90\xFD[b\0\0m\x81b\0\0\xABV[Pb\0\0xb\0\0\xFAV[P`\x06\x80T`\x01`\x01`\xA0\x1B\x03\x90\x93\x16`\x01`\x01`\xA0\x1B\x03\x19\x93\x84\x16\x17\x90U`\x07U`\x08\x80T\x90\x91\x163\x17\x90Ub\0\x02iV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15b\0\x01DWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15b\0\x01`WP0;\x15[\x90P\x81\x15\x80\x15b\0\x01oWP\x80\x15[\x15b\0\x01\x8EW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84T`\x01`\x01`@\x1B\x03\x19\x16`\x01\x17\x85U\x83\x15b\0\x01\xBDW\x84T`\xFF`@\x1B\x19\x16h\x01\0\0\0\0\0\0\0\0\x17\x85U[C`\x01U\x83\x15b\0\x02\x08W\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[\x80Q`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14b\0\x02&W_\x80\xFD[\x91\x90PV[_\x80_``\x84\x86\x03\x12\x15b\0\x02>W_\x80\xFD[b\0\x02I\x84b\0\x02\x0FV[\x92P` \x84\x01Q\x91Pb\0\x02``@\x85\x01b\0\x02\x0FV[\x90P\x92P\x92P\x92V[a\x1FB\x80b\0\x02w_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\x01\x1CW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xA9W\x80c\xB5\xEC\xB3D\x11a\0nW\x80c\xB5\xEC\xB3D\x14a\x02\x1BW\x80c\xF2\xF8\n\x18\x14a\x02:W\x80c\xF2\xFD\xE3\x8B\x14a\x02]W\x80c\xF8Q\xA4@\x14a\x02pW\x80c\xFAR\xC7\xD8\x14a\x02\x83W_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x01\xB5W\x80c\x9B0\xA5\xE6\x14a\x01\xD9W\x80c\x9Dv\xEAX\x14a\x01\xECW\x80c\x9E\x9A\x8F1\x14a\x01\xFFW\x80c\xA3\x06j\xAB\x14a\x02\x08W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\0\xEFW\x80cM\x99\xDD\x16\x14a\x01wW\x80cUD\xC2\xF1\x14a\x01\x8AW\x80cj\x91\x1C\xCF\x14a\x01\x9DW\x80cqP\x18\xA6\x14a\x01\xA5W\x80c\x81)\xFC\x1C\x14a\x01\xADW_\x80\xFD[\x80c\x02n@+\x14a\x01 W\x80c\x13\xB9\x05z\x14a\x015W\x80c!@\xFE\xCD\x14a\x01HW\x80c>\x9D\xF9\xB5\x14a\x01[W[_\x80\xFD[a\x013a\x01.6`\x04a\x196V[a\x02\xC5V[\0[a\x013a\x01C6`\x04a\x1A;V[a\x04\x0FV[a\x013a\x01V6`\x04a\x1A\x99V[a\x05+V[a\x01d`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x013a\x01\x856`\x04a\x196V[a\x06,V[a\x013a\x01\x986`\x04a\x1A\xB2V[a\x07\x98V[a\x013a\x08\x1DV[a\x013a\x08\x9EV[a\x013a\x08\xB1V[_T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01nV[a\x01da\x01\xE76`\x04a\x1A\xF6V[a\t\xB5V[`\x06Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x01d`\x07T\x81V[a\x013a\x02\x166`\x04a\x1A\x99V[a\n\x0FV[a\x01da\x02)6`\x04a\x1A\x99V[`\x03` R_\x90\x81R`@\x90 T\x81V[a\x02Ma\x02H6`\x04a\x1B\x10V[a\x0B\x12V[`@Q\x90\x15\x15\x81R` \x01a\x01nV[a\x013a\x02k6`\x04a\x1A\x99V[a\x0B[V[`\x08Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x02\xB6a\x02\x916`\x04a\x1A\x99V[`\x02` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x80\x83\x16\x92a\x01\0\x90\x04\x16\x90\x83V[`@Qa\x01n\x93\x92\x91\x90a\x1BXV[a\x02\xCE\x82a\x0B\x98V[a\x02\xD7\x82a\x0B\xD0V[`\x06T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03#W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x03G\x91\x90a\x1B\x8EV[\x90P\x81\x81\x10\x15a\x03yW`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x02` R`@\x81 `\x01\x01\x80T\x84\x92\x90a\x03\xA3\x90\x84\x90a\x1B\xB9V[\x90\x91UPP`\x06Ta\x03\xC0\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0C\x06V[`@\x80Q3\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16` \x82\x01R\x90\x81\x01\x83\x90R\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90``\x01[`@Q\x80\x91\x03\x90\xA1PPPV[a\x04\x183a\x0C\x97V[a\x04!\x83a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x04J\x81\x84\x87a\r\x0FV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x04qW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q``\x81\x01\x90\x91R`\x01\x81R` \x81\x01_\x81R_` \x91\x82\x01\x81\x90R3\x81R`\x02\x82R`@\x90 \x82Q\x81T\x90\x15\x15`\xFF\x19\x82\x16\x81\x17\x83U\x92\x84\x01Q\x91\x92\x83\x91a\xFF\0\x19\x90\x91\x16a\xFF\xFF\x19\x90\x91\x16\x17a\x01\0\x83`\x01\x81\x11\x15a\x04\xD7Wa\x04\xD7a\x1BDV[\x02\x17\x90UP`@\x91\x82\x01Q`\x01\x90\x91\x01UQ\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90a\x05\x1C\x903\x90\x88\x90\x88\x90\x87\x90a\x1B\xCCV[`@Q\x80\x91\x03\x90\xA1PPPPPV[3_\x90\x81R`\x03` R`@\x81 T\x90\x81\x90\x03a\x05[W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x05|W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x05\xC0W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x06Ta\x05\xF6\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01a\x04\x02V[a\x065\x82a\x0B\x98V[a\x06>\x82a\x0B\xD0V[`\x013_\x90\x81R`\x02` R`@\x90 Ta\x01\0\x90\x04`\xFF\x16`\x01\x81\x11\x15a\x06hWa\x06ha\x1BDV[\x03a\x06\x86W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\x06\xCDW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x06\xFF\x90\x84\x90a\x1C/V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x07TBa\x07%\x91\x90a\x1B\xB9V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x83Q\x91\x82R\x81\x01\x91\x90\x91R\x90\x81\x01\x83\x90R\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x90``\x01a\x04\x02V[a\x07\xA13a\x0B\x98V[a\x07\xAA3a\x0B\xD0V[a\x07\xB3\x82a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x07\xDC\x81\x83\x86a\r\x0FV[\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D3\x85\x85`@Qa\x08\x0F\x93\x92\x91\x90a\x1CBV[`@Q\x80\x91\x03\x90\xA1PPPPV[a\x08&3a\x0B\x98V[a\x08/3a\x0B\xD0V[3_\x90\x81R`\x02` R`@\x90 \x80Ta\xFF\0\x19\x16a\x01\0\x17\x90U`\x07Ta\x08W\x90Ba\x1B\xB9V[3_\x81\x81R`\x03` \x90\x81R`@\x91\x82\x90 \x93\x90\x93UQ\x90\x81R\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\x01`@Q\x80\x91\x03\x90\xA1V[a\x08\xA6a\x0E'V[a\x08\xAF_a\x0ESV[V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x08\xF6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\t\x12WP0;\x15[\x90P\x81\x15\x80\x15a\t WP\x80\x15[\x15a\t>W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\thW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\t\xAEW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01a\x05\x1CV[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xF2\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\nUW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\n\x99W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x06Ta\n\xD5\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0B,WP\x81` \x01Q\x83` \x01Q\x14[\x80\x15a\x0B?WP\x81`@\x01Q\x83`@\x01Q\x14[\x80\x15a\x0BRWP\x81``\x01Q\x83``\x01Q\x14[\x90P[\x92\x91PPV[a\x0Bca\x0E'V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0B\x8CW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x03pV[a\x0B\x95\x81a\x0ESV[PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16a\x0B\x95W`@QcW\xFD\xF4\x0B`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 T\x15a\x0B\x95W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\t\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16\x15a\x0B\x95W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0C\xED\x82\x82a\x0E\xA2V[\x15a\r\x0BW`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPV[a\r\x18\x82a\x0E\xBEV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a\x1E\xF2`$\x919\x90P_\x84\x82`@Q` \x01a\rH\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\rc\x82a\x0FYV[\x90Pa\r\x80\x81\x85a\rs\x88a\x10FV[a\r{a\x10\xBDV[a\x11\x8AV[a\r\x9CW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\x0E!W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x03pV[PPPPV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xAFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03pV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0BRWPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` a\x1F\x16\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x0E\xE4WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0FTW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x0Fv\x83a\x12jV[\x90P_\x80Q` a\x1F\x16\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x0F\x9DWa\x0F\x9Da\x1C\xD7V[\x84\x82\t\x90P\x82\x80a\x0F\xB0Wa\x0F\xB0a\x1C\xD7V[\x82\x82\x08\x90P_\x80a\x0F\xC0\x83a\x14sV[\x92P\x90P[\x80a\x10)W\x84\x80a\x0F\xD8Wa\x0F\xD8a\x1C\xD7V[`\x01\x87\x08\x95P\x84\x80a\x0F\xECWa\x0F\xECa\x1C\xD7V[\x86\x87\t\x92P\x84\x80a\x0F\xFFWa\x0F\xFFa\x1C\xD7V[\x86\x84\t\x92P\x84\x80a\x10\x12Wa\x10\x12a\x1C\xD7V[\x84\x84\x08\x92Pa\x10 \x83a\x14sV[\x92P\x90Pa\x0F\xC5V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x10mWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a\x1F\x16\x839\x81Q\x91R\x84` \x01Qa\x10\x9E\x91\x90a\x1C\xEBV[a\x10\xB5\x90_\x80Q` a\x1F\x16\x839\x81Q\x91Ra\x1C/V[\x90R\x92\x91PPV[a\x10\xE4`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x12\\W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x03pV[P\x15\x15\x90P[\x94\x93PPPPV[_\x80a\x12u\x83a\x15jV[\x80Q\x90\x91P`0\x81\x14a\x12\x8AWa\x12\x8Aa\x1D\nV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x12\xA4Wa\x12\xA4a\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x12\xCEW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x13=W\x83`\x01a\x12\xE8\x83\x86a\x1C/V[a\x12\xF2\x91\x90a\x1C/V[\x81Q\x81\x10a\x13\x02Wa\x13\x02a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x13\x1FWa\x13\x1Fa\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x12\xD3V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x13\xCDW\x83\x81a\x13y\x85\x88a\x1C/V[a\x13\x83\x91\x90a\x1B\xB9V[\x81Q\x81\x10a\x13\x93Wa\x13\x93a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x13\xB3Wa\x13\xB3a\x1D\x1EV[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x13eV[P_a\x13\xD8\x82a\x18\xB4V[\x90Pa\x01\0_\x80Q` a\x1F\x16\x839\x81Q\x91R_a\x13\xF6\x86\x89a\x1C/V[\x90P_[\x81\x81\x10\x15a\x14cW_\x88`\x01a\x14\x10\x84\x86a\x1C/V[a\x14\x1A\x91\x90a\x1C/V[\x81Q\x81\x10a\x14*Wa\x14*a\x1D\x1EV[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x14BWa\x14Ba\x1C\xD7V[\x85\x87\t\x95P\x83\x80a\x14UWa\x14Ua\x1C\xD7V[\x81\x87\x08\x95PP`\x01\x01a\x13\xFAV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a\x1F\x16\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x150W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[\x80`\x01\x84\x90\x1B\x11\x15a\x15IWa\x15F\x83\x82a\x1C/V[\x92P[\x80\x80a\x15WWa\x15Wa\x1C\xD7V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x15\xAA\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x15\xD1\x92\x91\x90a\x1D2V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x15\xF3\x91\x90a\x1D\\V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x16\x1D\x90\x83\x90\x83\x90` \x01a\x1DtV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x16\x8DWa\x16\x8Da\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x16\xB7W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x16\xCE\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x177W\x81\x81\x81Q\x81\x10a\x16\xFCWa\x16\xFCa\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x17\x19Wa\x17\x19a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x16\xE1V[P_\x84`@Q` \x01a\x17L\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a\x17\xDEW_\x83\x82\x81Q\x81\x10a\x17\x85Wa\x17\x85a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x17\xA2Wa\x17\xA2a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a\x17\xC3\x92\x91\x90a\x1D\x98V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x17jV[P\x86\x88\x87`@Q` \x01a\x17\xF4\x93\x92\x91\x90a\x1D\xBCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a\x18\"\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a\x18B\x8A`\xFF\x8D\x16a\x1C/V[\x81\x10\x15a\x18\xA3W\x82\x81\x81Q\x81\x10a\x18[Wa\x18[a\x1D\x1EV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a\x18u\x83\x8Da\x1B\xB9V[\x81Q\x81\x10a\x18\x85Wa\x18\x85a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x185V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x19\x14W\x83\x81\x81Q\x81\x10a\x18\xD3Wa\x18\xD3a\x1D\x1EV[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a\x18\xEB\x91\x90a\x1D\xEFV[a\x18\xF6\x90`\x02a\x1E\xE6V[a\x19\0\x91\x90a\x1D\xEFV[a\x19\n\x90\x83a\x1B\xB9V[\x91P`\x01\x01a\x18\xB8V[P\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x191W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\x19GW_\x80\xFD[a\x19P\x83a\x19\x1BV[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[_`\x80\x82\x84\x03\x12\x15a\x19\x82W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x19\xB1WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\x19\xF2W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x1A!WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a\x1AOW_\x80\xFD[a\x1AY\x86\x86a\x19rV[\x93Pa\x1Ah\x86`\x80\x87\x01a\x19\xE2V[\x92Pa\x1Aw\x86`\xC0\x87\x01a\x19\xE2V[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a\x1A\x8EW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a\x1A\xA9W_\x80\xFD[a\x0BR\x82a\x19\x1BV[_\x80_a\x01\0\x84\x86\x03\x12\x15a\x1A\xC5W_\x80\xFD[a\x1A\xCF\x85\x85a\x19rV[\x92Pa\x1A\xDE\x85`\x80\x86\x01a\x19\xE2V[\x91Pa\x1A\xED\x85`\xC0\x86\x01a\x19\xE2V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a\x1B\x06W_\x80\xFD[a\x0BR\x83\x83a\x19rV[_\x80a\x01\0\x83\x85\x03\x12\x15a\x1B\"W_\x80\xFD[a\x1B,\x84\x84a\x19rV[\x91Pa\x1B;\x84`\x80\x85\x01a\x19rV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x83\x15\x15\x81R``\x81\x01`\x02\x84\x10a\x1B}WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x82\x01\x93\x90\x93R`@\x01R\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1B\x9EW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x85\x16\x81Ra\x01\0\x81\x01a\x1C\x0B` \x83\x01\x86\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x83Q`\xA0\x83\x01R` \x84\x01Q`\xC0\x83\x01Ra\xFF\xFF\x83\x16`\xE0\x83\x01R\x95\x94PPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x84\x16\x81R`\xE0\x81\x01a\x1C\x80` \x83\x01\x85\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x82Q`\xA0\x83\x01R` \x83\x01Q`\xC0\x83\x01Ra\x12bV[_\x81Q_[\x81\x81\x10\x15a\x1C\xB5W` \x81\x85\x01\x81\x01Q\x86\x83\x01R\x01a\x1C\x9BV[P_\x93\x01\x92\x83RP\x90\x91\x90PV[_a\x12ba\x1C\xD1\x83\x86a\x1C\x96V[\x84a\x1C\x96V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a\x1D\x05WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a\x1D=\x82\x85a\x1C\x96V[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a\x1Dg\x82\x84a\x1C\x96V[_\x81R`\x01\x01\x93\x92PPPV[_a\x1D\x7F\x82\x85a\x1C\x96V[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a\x1D\xA3\x82\x85a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a\x1D\xC7\x82\x86a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0BUWa\x0BUa\x1B\xA5V[`\x01\x81\x81[\x80\x85\x11\x15a\x1E@W\x81_\x19\x04\x82\x11\x15a\x1E&Wa\x1E&a\x1B\xA5V[\x80\x85\x16\x15a\x1E3W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x1E\x0BV[P\x92P\x92\x90PV[_\x82a\x1EVWP`\x01a\x0BUV[\x81a\x1EbWP_a\x0BUV[\x81`\x01\x81\x14a\x1ExW`\x02\x81\x14a\x1E\x82Wa\x1E\x9EV[`\x01\x91PPa\x0BUV[`\xFF\x84\x11\x15a\x1E\x93Wa\x1E\x93a\x1B\xA5V[PP`\x01\x82\x1Ba\x0BUV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x1E\xC1WP\x81\x81\na\x0BUV[a\x1E\xCB\x83\x83a\x1E\x06V[\x80_\x19\x04\x82\x11\x15a\x1E\xDEWa\x1E\xDEa\x1B\xA5V[\x02\x93\x92PPPV[_a\x0BR\x83\x83a\x1EHV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1dsolcC\0\x08\x17\0\n";
    /// The bytecode of the contract.
    pub static STAKETABLE_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\x01\x1CW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0\xA9W\x80c\xB5\xEC\xB3D\x11a\0nW\x80c\xB5\xEC\xB3D\x14a\x02\x1BW\x80c\xF2\xF8\n\x18\x14a\x02:W\x80c\xF2\xFD\xE3\x8B\x14a\x02]W\x80c\xF8Q\xA4@\x14a\x02pW\x80c\xFAR\xC7\xD8\x14a\x02\x83W_\x80\xFD[\x80c\x8D\xA5\xCB[\x14a\x01\xB5W\x80c\x9B0\xA5\xE6\x14a\x01\xD9W\x80c\x9Dv\xEAX\x14a\x01\xECW\x80c\x9E\x9A\x8F1\x14a\x01\xFFW\x80c\xA3\x06j\xAB\x14a\x02\x08W_\x80\xFD[\x80cM\x99\xDD\x16\x11a\0\xEFW\x80cM\x99\xDD\x16\x14a\x01wW\x80cUD\xC2\xF1\x14a\x01\x8AW\x80cj\x91\x1C\xCF\x14a\x01\x9DW\x80cqP\x18\xA6\x14a\x01\xA5W\x80c\x81)\xFC\x1C\x14a\x01\xADW_\x80\xFD[\x80c\x02n@+\x14a\x01 W\x80c\x13\xB9\x05z\x14a\x015W\x80c!@\xFE\xCD\x14a\x01HW\x80c>\x9D\xF9\xB5\x14a\x01[W[_\x80\xFD[a\x013a\x01.6`\x04a\x196V[a\x02\xC5V[\0[a\x013a\x01C6`\x04a\x1A;V[a\x04\x0FV[a\x013a\x01V6`\x04a\x1A\x99V[a\x05+V[a\x01d`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\x013a\x01\x856`\x04a\x196V[a\x06,V[a\x013a\x01\x986`\x04a\x1A\xB2V[a\x07\x98V[a\x013a\x08\x1DV[a\x013a\x08\x9EV[a\x013a\x08\xB1V[_T`\x01`\x01`\xA0\x1B\x03\x16[`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\x01nV[a\x01da\x01\xE76`\x04a\x1A\xF6V[a\t\xB5V[`\x06Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x01d`\x07T\x81V[a\x013a\x02\x166`\x04a\x1A\x99V[a\n\x0FV[a\x01da\x02)6`\x04a\x1A\x99V[`\x03` R_\x90\x81R`@\x90 T\x81V[a\x02Ma\x02H6`\x04a\x1B\x10V[a\x0B\x12V[`@Q\x90\x15\x15\x81R` \x01a\x01nV[a\x013a\x02k6`\x04a\x1A\x99V[a\x0B[V[`\x08Ta\x01\xC1\x90`\x01`\x01`\xA0\x1B\x03\x16\x81V[a\x02\xB6a\x02\x916`\x04a\x1A\x99V[`\x02` R_\x90\x81R`@\x90 \x80T`\x01\x90\x91\x01T`\xFF\x80\x83\x16\x92a\x01\0\x90\x04\x16\x90\x83V[`@Qa\x01n\x93\x92\x91\x90a\x1BXV[a\x02\xCE\x82a\x0B\x98V[a\x02\xD7\x82a\x0B\xD0V[`\x06T`@Qcn\xB1v\x9F`\xE1\x1B\x81R3`\x04\x82\x01R0`$\x82\x01R_\x91`\x01`\x01`\xA0\x1B\x03\x16\x90c\xDDb\xED>\x90`D\x01` `@Q\x80\x83\x03\x81\x86Z\xFA\x15\x80\x15a\x03#W=_\x80>=_\xFD[PPPP`@Q=`\x1F\x19`\x1F\x82\x01\x16\x82\x01\x80`@RP\x81\x01\x90a\x03G\x91\x90a\x1B\x8EV[\x90P\x81\x81\x10\x15a\x03yW`@Qc\x05Ce\xBB`\xE3\x1B\x81R`\x04\x81\x01\x82\x90R`$\x81\x01\x83\x90R`D\x01[`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x02` R`@\x81 `\x01\x01\x80T\x84\x92\x90a\x03\xA3\x90\x84\x90a\x1B\xB9V[\x90\x91UPP`\x06Ta\x03\xC0\x90`\x01`\x01`\xA0\x1B\x03\x1630\x85a\x0C\x06V[`@\x80Q3\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16` \x82\x01R\x90\x81\x01\x83\x90R\x7F\xE5T\x1Aka\x03\xD4\xFA~\x02\x1E\xD5O\xAD9\xC6o'\xA7k\xD1=7L\xF6$\n\xE6\xBD\x0B\xB7+\x90``\x01[`@Q\x80\x91\x03\x90\xA1PPPV[a\x04\x183a\x0C\x97V[a\x04!\x83a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x04J\x81\x84\x87a\r\x0FV[a'\x10\x82a\xFF\xFF\x16\x11\x15a\x04qW`@Qc\xDC\x81\xDB\x85`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q``\x81\x01\x90\x91R`\x01\x81R` \x81\x01_\x81R_` \x91\x82\x01\x81\x90R3\x81R`\x02\x82R`@\x90 \x82Q\x81T\x90\x15\x15`\xFF\x19\x82\x16\x81\x17\x83U\x92\x84\x01Q\x91\x92\x83\x91a\xFF\0\x19\x90\x91\x16a\xFF\xFF\x19\x90\x91\x16\x17a\x01\0\x83`\x01\x81\x11\x15a\x04\xD7Wa\x04\xD7a\x1BDV[\x02\x17\x90UP`@\x91\x82\x01Q`\x01\x90\x91\x01UQ\x7F\xF6\xE85\x9CWR\x0BF\x964sk\xFC;\xB7\xEC\\\xBD\x1A\x0B\xD2\x8B\x10\xA8'W\x93\xBBs\x0By\x7F\x90a\x05\x1C\x903\x90\x88\x90\x88\x90\x87\x90a\x1B\xCCV[`@Q\x80\x91\x03\x90\xA1PPPPPV[3_\x90\x81R`\x03` R`@\x81 T\x90\x81\x90\x03a\x05[W`@Qcy)\x8AS`\xE1\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80B\x10\x15a\x05|W`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\x05\xC0W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x84\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x91\x90\x91U`\x06Ta\x05\xF6\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01a\x04\x02V[a\x065\x82a\x0B\x98V[a\x06>\x82a\x0B\xD0V[`\x013_\x90\x81R`\x02` R`@\x90 Ta\x01\0\x90\x04`\xFF\x16`\x01\x81\x11\x15a\x06hWa\x06ha\x1BDV[\x03a\x06\x86W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x82\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 T\x81\x81\x10\x15a\x06\xCDW`@Qc\x92fSQ`\xE0\x1B\x81R`\x04\x81\x01\x83\x90R`$\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x83\x16_\x90\x81R`\x04` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 \x80T\x84\x92\x90a\x06\xFF\x90\x84\x90a\x1C/V[\x92PP\x81\x90UP`@Q\x80`@\x01`@R\x80\x83\x81R` \x01`\x07TBa\x07%\x91\x90a\x1B\xB9V[\x90R`\x01`\x01`\xA0\x1B\x03\x84\x16_\x81\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x90\x83R\x92\x81\x90 \x85Q\x81U\x94\x82\x01Q`\x01\x90\x95\x01\x94\x90\x94U\x83Q\x91\x82R\x81\x01\x91\x90\x91R\x90\x81\x01\x83\x90R\x7FM\x10\xBD\x04\x97u\xC7{\xD7\xF2U\x19Z\xFB\xA5\x08\x80(\xEC\xB3\xC7\xC2w\xD3\x93\xCC\xFFy4\xF2\xF9,\x90``\x01a\x04\x02V[a\x07\xA13a\x0B\x98V[a\x07\xAA3a\x0B\xD0V[a\x07\xB3\x82a\x0C\xD0V[`@\x80Q3` \x82\x01R_\x91\x01`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90Pa\x07\xDC\x81\x83\x86a\r\x0FV[\x7F\x80\xD8\xA4\xA1f3(\xA9\x98\xD4U[\xA2\x1D\x8B\xBAn\xF1Wj\x8C^\x9D'\xF9\xC5E\xF1\xA3\xD5+\x1D3\x85\x85`@Qa\x08\x0F\x93\x92\x91\x90a\x1CBV[`@Q\x80\x91\x03\x90\xA1PPPPV[a\x08&3a\x0B\x98V[a\x08/3a\x0B\xD0V[3_\x90\x81R`\x02` R`@\x90 \x80Ta\xFF\0\x19\x16a\x01\0\x17\x90U`\x07Ta\x08W\x90Ba\x1B\xB9V[3_\x81\x81R`\x03` \x90\x81R`@\x91\x82\x90 \x93\x90\x93UQ\x90\x81R\x7F\xFB$0ST\xC8wb\xD5WHz\xE4\xA5d\xE8\xD0>\xCB\xB9\xA9}\xD8\xAF\xFF\x8E\x1Fo\xCA\xF0\xDD\x16\x91\x01`@Q\x80\x91\x03\x90\xA1V[a\x08\xA6a\x0E'V[a\x08\xAF_a\x0ESV[V[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x08\xF6WP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\t\x12WP0;\x15[\x90P\x81\x15\x80\x15a\t WP\x80\x15[\x15a\t>W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\thW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\t\xAEW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01a\x05\x1CV[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\t\xF2\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x90 `\x01\x01TB\x10\x15a\nUW`@QcZwCW`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x84R\x90\x91R\x81 T\x90\x81\x90\x03a\n\x99W`@Qc\x06\x86\x82{`\xE5\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x80\x83\x16_\x90\x81R`\x05` \x90\x81R`@\x80\x83 3\x80\x85R\x92R\x82 \x82\x81U`\x01\x01\x91\x90\x91U`\x06Ta\n\xD5\x92\x16\x90\x83a\r\xA4V[`@\x80Q3\x81R` \x81\x01\x83\x90R\x7F\x7F\xCFS,\x15\xF0\xA6\xDB\x0B\xD6\xD0\xE08\xBE\xA7\x1D0\xD8\x08\xC7\xD9\x8C\xB3\xBFrh\xA9[\xF5\x08\x1Be\x91\x01`@Q\x80\x91\x03\x90\xA1PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0B,WP\x81` \x01Q\x83` \x01Q\x14[\x80\x15a\x0B?WP\x81`@\x01Q\x83`@\x01Q\x14[\x80\x15a\x0BRWP\x81``\x01Q\x83``\x01Q\x14[\x90P[\x92\x91PPV[a\x0Bca\x0E'V[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x0B\x8CW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01a\x03pV[a\x0B\x95\x81a\x0ESV[PV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16a\x0B\x95W`@QcW\xFD\xF4\x0B`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x03` R`@\x90 T\x15a\x0B\x95W`@Qc\xEA\xB4\xA9c`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[_`@Qc#\xB8r\xDD`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x85\x16`\x04\x82\x01R`\x01`\x01`\xA0\x1B\x03\x84\x16`$\x82\x01R\x82`D\x82\x01R` _`d\x83_\x8AZ\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\t\xAEW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x14`$\x82\x01Rs\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x94\x93\xD3W\xD1\x90RS\x11Q`b\x1B`D\x82\x01R`d\x01a\x03pV[`\x01`\x01`\xA0\x1B\x03\x81\x16_\x90\x81R`\x02` R`@\x90 T`\xFF\x16\x15a\x0B\x95W`@Qc\x13.~\xFB`\xE3\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01Ra\x0C\xED\x82\x82a\x0E\xA2V[\x15a\r\x0BW`@Qc\x06\xCFC\x8F`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPV[a\r\x18\x82a\x0E\xBEV[_`@Q\x80``\x01`@R\x80`$\x81R` \x01a\x1E\xF2`$\x919\x90P_\x84\x82`@Q` \x01a\rH\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_a\rc\x82a\x0FYV[\x90Pa\r\x80\x81\x85a\rs\x88a\x10FV[a\r{a\x10\xBDV[a\x11\x8AV[a\r\x9CW`@Qb\xCE\xD3\xE5`\xE4\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[PPPPPPV[_`@Qc\xA9\x05\x9C\xBB`\xE0\x1B\x81R`\x01`\x01`\xA0\x1B\x03\x84\x16`\x04\x82\x01R\x82`$\x82\x01R` _`D\x83_\x89Z\xF1=\x15`\x1F=\x11`\x01_Q\x14\x16\x17\x16\x91PP\x80a\x0E!W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x0F`$\x82\x01Rn\x15\x14\x90S\x94\xD1\x91T\x97\xD1\x90RS\x11Q`\x8A\x1B`D\x82\x01R`d\x01a\x03pV[PPPPV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x08\xAFW`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03pV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x80Q\x82Q_\x91\x14\x80\x15a\x0BRWPP` \x90\x81\x01Q\x91\x01Q\x14\x90V[\x80Q` \x82\x01Q_\x91_\x80Q` a\x1F\x16\x839\x81Q\x91R\x91\x15\x90\x15\x16\x15a\x0E\xE4WPPPV[\x82Q` \x84\x01Q\x82`\x03\x84\x85\x85\x86\t\x85\t\x08\x83\x82\x83\t\x14\x83\x82\x10\x84\x84\x10\x16\x16\x93PPP\x81a\x0FTW`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x17`$\x82\x01R\x7FBn254: invalid G1 point\0\0\0\0\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[PPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R_a\x0Fv\x83a\x12jV[\x90P_\x80Q` a\x1F\x16\x839\x81Q\x91R`\x03_\x82\x84\x85\t\x90P\x82\x80a\x0F\x9DWa\x0F\x9Da\x1C\xD7V[\x84\x82\t\x90P\x82\x80a\x0F\xB0Wa\x0F\xB0a\x1C\xD7V[\x82\x82\x08\x90P_\x80a\x0F\xC0\x83a\x14sV[\x92P\x90P[\x80a\x10)W\x84\x80a\x0F\xD8Wa\x0F\xD8a\x1C\xD7V[`\x01\x87\x08\x95P\x84\x80a\x0F\xECWa\x0F\xECa\x1C\xD7V[\x86\x87\t\x92P\x84\x80a\x0F\xFFWa\x0F\xFFa\x1C\xD7V[\x86\x84\t\x92P\x84\x80a\x10\x12Wa\x10\x12a\x1C\xD7V[\x84\x84\x08\x92Pa\x10 \x83a\x14sV[\x92P\x90Pa\x0F\xC5V[P`@\x80Q\x80\x82\x01\x90\x91R\x94\x85R` \x85\x01RP\x91\x94\x93PPPPV[`@\x80Q\x80\x82\x01\x90\x91R_\x80\x82R` \x82\x01R\x81Q` \x83\x01Q\x15\x90\x15\x16\x15a\x10mWP\x90V[`@Q\x80`@\x01`@R\x80\x83_\x01Q\x81R` \x01_\x80Q` a\x1F\x16\x839\x81Q\x91R\x84` \x01Qa\x10\x9E\x91\x90a\x1C\xEBV[a\x10\xB5\x90_\x80Q` a\x1F\x16\x839\x81Q\x91Ra\x1C/V[\x90R\x92\x91PPV[a\x10\xE4`@Q\x80`\x80\x01`@R\x80_\x81R` \x01_\x81R` \x01_\x81R` \x01_\x81RP\x90V[`@Q\x80`\x80\x01`@R\x80\x7F\x18\0\xDE\xEF\x12\x1F\x1EvBj\0f^\\DygC\"\xD4\xF7^\xDA\xDDF\xDE\xBD\\\xD9\x92\xF6\xED\x81R` \x01\x7F\x19\x8E\x93\x93\x92\rH:r`\xBF\xB71\xFB]%\xF1\xAAI35\xA9\xE7\x12\x97\xE4\x85\xB7\xAE\xF3\x12\xC2\x81R` \x01\x7F\x12\xC8^\xA5\xDB\x8Cm\xEBJ\xABq\x80\x8D\xCB@\x8F\xE3\xD1\xE7i\x0CC\xD3{L\xE6\xCC\x01f\xFA}\xAA\x81R` \x01\x7F\t\x06\x89\xD0X_\xF0u\xEC\x9E\x99\xADi\x0C3\x95\xBCK13p\xB3\x8E\xF3U\xAC\xDA\xDC\xD1\"\x97[\x81RP\x90P\x90V[_\x80_`@Q\x87Q\x81R` \x88\x01Q` \x82\x01R` \x87\x01Q`@\x82\x01R\x86Q``\x82\x01R``\x87\x01Q`\x80\x82\x01R`@\x87\x01Q`\xA0\x82\x01R\x85Q`\xC0\x82\x01R` \x86\x01Q`\xE0\x82\x01R` \x85\x01Qa\x01\0\x82\x01R\x84Qa\x01 \x82\x01R``\x85\x01Qa\x01@\x82\x01R`@\x85\x01Qa\x01`\x82\x01R` _a\x01\x80\x83`\x08Z\xFA\x91PP_Q\x91P\x80a\x12\\W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1C`$\x82\x01R\x7FBn254: Pairing check failed!\0\0\0\0`D\x82\x01R`d\x01a\x03pV[P\x15\x15\x90P[\x94\x93PPPPV[_\x80a\x12u\x83a\x15jV[\x80Q\x90\x91P`0\x81\x14a\x12\x8AWa\x12\x8Aa\x1D\nV[_\x81g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x12\xA4Wa\x12\xA4a\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x12\xCEW` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_[\x82\x81\x10\x15a\x13=W\x83`\x01a\x12\xE8\x83\x86a\x1C/V[a\x12\xF2\x91\x90a\x1C/V[\x81Q\x81\x10a\x13\x02Wa\x13\x02a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x82\x82\x81Q\x81\x10a\x13\x1FWa\x13\x1Fa\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x12\xD3V[P`@\x80Q`\x1F\x80\x82Ra\x04\0\x82\x01\x90\x92R_\x90\x82` \x82\x01a\x03\xE0\x806\x837\x01\x90PP\x90P_[\x82\x81\x10\x15a\x13\xCDW\x83\x81a\x13y\x85\x88a\x1C/V[a\x13\x83\x91\x90a\x1B\xB9V[\x81Q\x81\x10a\x13\x93Wa\x13\x93a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B`\xF8\x1C\x82\x82\x81Q\x81\x10a\x13\xB3Wa\x13\xB3a\x1D\x1EV[`\xFF\x90\x92\x16` \x92\x83\x02\x91\x90\x91\x01\x90\x91\x01R`\x01\x01a\x13eV[P_a\x13\xD8\x82a\x18\xB4V[\x90Pa\x01\0_\x80Q` a\x1F\x16\x839\x81Q\x91R_a\x13\xF6\x86\x89a\x1C/V[\x90P_[\x81\x81\x10\x15a\x14cW_\x88`\x01a\x14\x10\x84\x86a\x1C/V[a\x14\x1A\x91\x90a\x1C/V[\x81Q\x81\x10a\x14*Wa\x14*a\x1D\x1EV[\x01` \x01Q`\xF8\x1C\x90P\x83\x80a\x14BWa\x14Ba\x1C\xD7V[\x85\x87\t\x95P\x83\x80a\x14UWa\x14Ua\x1C\xD7V[\x81\x87\x08\x95PP`\x01\x01a\x13\xFAV[P\x92\x9A\x99PPPPPPPPPPV[_\x80_\x80_\x7F\x0C\x19\x13\x9C\xB8Lh\nn\x14\x11m\xA0`V\x17e\xE0Z\xA4Z\x1Cr\xA3O\x08#\x05\xB6\x1F?R\x90P__\x80Q` a\x1F\x16\x839\x81Q\x91R\x90P`@Q` \x81R` \x80\x82\x01R` `@\x82\x01R\x87``\x82\x01R\x82`\x80\x82\x01R\x81`\xA0\x82\x01R` _`\xC0\x83`\x05Z\xFA\x94PP_Q\x92P\x83a\x150W`@QbF\x1B\xCD`\xE5\x1B\x81R` `\x04\x82\x01R`\x1B`$\x82\x01R\x7Fpow precompile call failed!\0\0\0\0\0`D\x82\x01R`d\x01a\x03pV[\x80`\x01\x84\x90\x1B\x11\x15a\x15IWa\x15F\x83\x82a\x1C/V[\x92P[\x80\x80a\x15WWa\x15Wa\x1C\xD7V[\x83\x84\t\x96\x90\x96\x14\x96\x91\x95P\x90\x93PPPPV[`@\x80Q`0\x80\x82R``\x82\x81\x01\x90\x93R\x90` \x90`\x01`\xF9\x1B\x90_\x90\x84` \x82\x01\x81\x806\x837\x01\x90PP\x90P\x80\x86`@Q` \x01a\x15\xAA\x92\x91\x90a\x1C\xC3V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80\x84`\xF8\x1B`@Q` \x01a\x15\xD1\x92\x91\x90a\x1D2V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P\x80`@Q` \x01a\x15\xF3\x91\x90a\x1D\\V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x90\x82\x90R\x91Pa\x01\x01`\xF0\x1B\x90a\x16\x1D\x90\x83\x90\x83\x90` \x01a\x1DtV[`@\x80Q\x80\x83\x03`\x1F\x19\x01\x81R\x82\x82R\x80Q` \x91\x82\x01 \x81\x84\x01\x81\x90R`\x01`\xF8\x1B\x84\x84\x01R`\x01`\x01`\xF0\x1B\x03\x19\x85\x16`A\x85\x01R\x82Q`#\x81\x86\x03\x01\x81R`C\x90\x94\x01\x90\x92R\x82Q\x90\x83\x01 \x91\x93P\x90_`\xFF\x88\x16g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x15a\x16\x8DWa\x16\x8Da\x19^V[`@Q\x90\x80\x82R\x80`\x1F\x01`\x1F\x19\x16` \x01\x82\x01`@R\x80\x15a\x16\xB7W` \x82\x01\x81\x806\x837\x01\x90P[P\x90P_\x82`@Q` \x01a\x16\xCE\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x90P_[\x81Q\x81\x10\x15a\x177W\x81\x81\x81Q\x81\x10a\x16\xFCWa\x16\xFCa\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x82\x81Q\x81\x10a\x17\x19Wa\x17\x19a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x16\xE1V[P_\x84`@Q` \x01a\x17L\x91\x81R` \x01\x90V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R` \x83\x01\x90\x91R_\x80\x83R\x91\x98P\x91P[\x89\x81\x10\x15a\x17\xDEW_\x83\x82\x81Q\x81\x10a\x17\x85Wa\x17\x85a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x83\x83\x81Q\x81\x10a\x17\xA2Wa\x17\xA2a\x1D\x1EV[` \x01\x01Q`\xF8\x1C`\xF8\x1B\x18\x90P\x88\x81`@Q` \x01a\x17\xC3\x92\x91\x90a\x1D\x98V[`@\x80Q`\x1F\x19\x81\x84\x03\x01\x81R\x91\x90R\x98PP`\x01\x01a\x17jV[P\x86\x88\x87`@Q` \x01a\x17\xF4\x93\x92\x91\x90a\x1D\xBCV[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x96P\x86\x80Q\x90` \x01 \x93P\x83`@Q` \x01a\x18\"\x91\x81R` \x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x91P_[a\x18B\x8A`\xFF\x8D\x16a\x1C/V[\x81\x10\x15a\x18\xA3W\x82\x81\x81Q\x81\x10a\x18[Wa\x18[a\x1D\x1EV[\x01` \x01Q`\x01`\x01`\xF8\x1B\x03\x19\x16\x84a\x18u\x83\x8Da\x1B\xB9V[\x81Q\x81\x10a\x18\x85Wa\x18\x85a\x1D\x1EV[` \x01\x01\x90`\x01`\x01`\xF8\x1B\x03\x19\x16\x90\x81_\x1A\x90SP`\x01\x01a\x185V[P\x91\x9B\x9APPPPPPPPPPPV[_\x80\x80[\x83Q\x81\x10\x15a\x19\x14W\x83\x81\x81Q\x81\x10a\x18\xD3Wa\x18\xD3a\x1D\x1EV[` \x02` \x01\x01Q`\xFF\x16\x81`\x08a\x18\xEB\x91\x90a\x1D\xEFV[a\x18\xF6\x90`\x02a\x1E\xE6V[a\x19\0\x91\x90a\x1D\xEFV[a\x19\n\x90\x83a\x1B\xB9V[\x91P`\x01\x01a\x18\xB8V[P\x92\x91PPV[\x805`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x191W_\x80\xFD[\x91\x90PV[_\x80`@\x83\x85\x03\x12\x15a\x19GW_\x80\xFD[a\x19P\x83a\x19\x1BV[\x94` \x93\x90\x93\x015\x93PPPV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[_`\x80\x82\x84\x03\x12\x15a\x19\x82W_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x19\xB1WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`@\x82\x84\x03\x12\x15a\x19\xF2W_\x80\xFD[`@Q`@\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x1A!WcNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@R\x825\x81R` \x92\x83\x015\x92\x81\x01\x92\x90\x92RP\x91\x90PV[_\x80_\x80a\x01 \x85\x87\x03\x12\x15a\x1AOW_\x80\xFD[a\x1AY\x86\x86a\x19rV[\x93Pa\x1Ah\x86`\x80\x87\x01a\x19\xE2V[\x92Pa\x1Aw\x86`\xC0\x87\x01a\x19\xE2V[\x91Pa\x01\0\x85\x015a\xFF\xFF\x81\x16\x81\x14a\x1A\x8EW_\x80\xFD[\x93\x96\x92\x95P\x90\x93PPV[_` \x82\x84\x03\x12\x15a\x1A\xA9W_\x80\xFD[a\x0BR\x82a\x19\x1BV[_\x80_a\x01\0\x84\x86\x03\x12\x15a\x1A\xC5W_\x80\xFD[a\x1A\xCF\x85\x85a\x19rV[\x92Pa\x1A\xDE\x85`\x80\x86\x01a\x19\xE2V[\x91Pa\x1A\xED\x85`\xC0\x86\x01a\x19\xE2V[\x90P\x92P\x92P\x92V[_`\x80\x82\x84\x03\x12\x15a\x1B\x06W_\x80\xFD[a\x0BR\x83\x83a\x19rV[_\x80a\x01\0\x83\x85\x03\x12\x15a\x1B\"W_\x80\xFD[a\x1B,\x84\x84a\x19rV[\x91Pa\x1B;\x84`\x80\x85\x01a\x19rV[\x90P\x92P\x92\x90PV[cNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[\x83\x15\x15\x81R``\x81\x01`\x02\x84\x10a\x1B}WcNH{q`\xE0\x1B_R`!`\x04R`$_\xFD[` \x82\x01\x93\x90\x93R`@\x01R\x91\x90PV[_` \x82\x84\x03\x12\x15a\x1B\x9EW_\x80\xFD[PQ\x91\x90PV[cNH{q`\xE0\x1B_R`\x11`\x04R`$_\xFD[\x80\x82\x01\x80\x82\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x85\x16\x81Ra\x01\0\x81\x01a\x1C\x0B` \x83\x01\x86\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x83Q`\xA0\x83\x01R` \x84\x01Q`\xC0\x83\x01Ra\xFF\xFF\x83\x16`\xE0\x83\x01R\x95\x94PPPPPV[\x81\x81\x03\x81\x81\x11\x15a\x0BUWa\x0BUa\x1B\xA5V[`\x01`\x01`\xA0\x1B\x03\x84\x16\x81R`\xE0\x81\x01a\x1C\x80` \x83\x01\x85\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x82Q`\xA0\x83\x01R` \x83\x01Q`\xC0\x83\x01Ra\x12bV[_\x81Q_[\x81\x81\x10\x15a\x1C\xB5W` \x81\x85\x01\x81\x01Q\x86\x83\x01R\x01a\x1C\x9BV[P_\x93\x01\x92\x83RP\x90\x91\x90PV[_a\x12ba\x1C\xD1\x83\x86a\x1C\x96V[\x84a\x1C\x96V[cNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[_\x82a\x1D\x05WcNH{q`\xE0\x1B_R`\x12`\x04R`$_\xFD[P\x06\x90V[cNH{q`\xE0\x1B_R`\x01`\x04R`$_\xFD[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[_a\x1D=\x82\x85a\x1C\x96V[_\x81R`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16`\x01\x84\x01RPP`\x02\x01\x91\x90PV[_a\x1Dg\x82\x84a\x1C\x96V[_\x81R`\x01\x01\x93\x92PPPV[_a\x1D\x7F\x82\x85a\x1C\x96V[`\x01`\x01`\xF0\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x02\x01\x91\x90PV[_a\x1D\xA3\x82\x85a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x93\x90\x93\x16\x83RPP`\x01\x01\x91\x90PV[_a\x1D\xC7\x82\x86a\x1C\x96V[`\x01`\x01`\xF8\x1B\x03\x19\x94\x90\x94\x16\x84RPP`\x01`\x01`\xF0\x1B\x03\x19\x16`\x01\x82\x01R`\x03\x01\x91\x90PV[\x80\x82\x02\x81\x15\x82\x82\x04\x84\x14\x17a\x0BUWa\x0BUa\x1B\xA5V[`\x01\x81\x81[\x80\x85\x11\x15a\x1E@W\x81_\x19\x04\x82\x11\x15a\x1E&Wa\x1E&a\x1B\xA5V[\x80\x85\x16\x15a\x1E3W\x91\x81\x02\x91[\x93\x84\x1C\x93\x90\x80\x02\x90a\x1E\x0BV[P\x92P\x92\x90PV[_\x82a\x1EVWP`\x01a\x0BUV[\x81a\x1EbWP_a\x0BUV[\x81`\x01\x81\x14a\x1ExW`\x02\x81\x14a\x1E\x82Wa\x1E\x9EV[`\x01\x91PPa\x0BUV[`\xFF\x84\x11\x15a\x1E\x93Wa\x1E\x93a\x1B\xA5V[PP`\x01\x82\x1Ba\x0BUV[P` \x83\x10a\x013\x83\x10\x16`N\x84\x10`\x0B\x84\x10\x16\x17\x15a\x1E\xC1WP\x81\x81\na\x0BUV[a\x1E\xCB\x83\x83a\x1E\x06V[\x80_\x19\x04\x82\x11\x15a\x1E\xDEWa\x1E\xDEa\x1B\xA5V[\x02\x93\x92PPPV[_a\x0BR\x83\x83a\x1EHV\xFEBLS_SIG_BN254G1_XMD:KECCAK_NCTH_NUL_0dNr\xE11\xA0)\xB8PE\xB6\x81\x81X]\x97\x81j\x91hq\xCA\x8D< \x8C\x16\xD8|\xFDG\xA1dsolcC\0\x08\x17\0\n";
    /// The deployed bytecode of the contract.
    pub static STAKETABLE_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct StakeTable<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for StakeTable<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for StakeTable<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for StakeTable<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for StakeTable<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(StakeTable))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> StakeTable<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(
                address.into(),
                STAKETABLE_ABI.clone(),
                client,
            ))
        }
        /// Constructs the general purpose `Deployer` instance based on the provided constructor arguments and sends it.
        /// Returns a new instance of a deployer that returns an instance of this contract after sending the transaction
        ///
        /// Notes:
        /// - If there are no constructor arguments, you should pass `()` as the argument.
        /// - The default poll duration is 7 seconds.
        /// - The default number of confirmations is 1 block.
        ///
        ///
        /// # Example
        ///
        /// Generate contract bindings with `abigen!` and deploy a new contract instance.
        ///
        /// *Note*: this requires a `bytecode` and `abi` object in the `greeter.json` artifact.
        ///
        /// ```ignore
        /// # async fn deploy<M: ethers::providers::Middleware>(client: ::std::sync::Arc<M>) {
        ///     abigen!(Greeter, "../greeter.json");
        ///
        ///    let greeter_contract = Greeter::deploy(client, "Hello world!".to_string()).unwrap().send().await.unwrap();
        ///    let msg = greeter_contract.greet().call().await.unwrap();
        /// # }
        /// ```
        pub fn deploy<T: ::ethers::core::abi::Tokenize>(
            client: ::std::sync::Arc<M>,
            constructor_args: T,
        ) -> ::core::result::Result<
            ::ethers::contract::builders::ContractDeployer<M, Self>,
            ::ethers::contract::ContractError<M>,
        > {
            let factory = ::ethers::contract::ContractFactory::new(
                STAKETABLE_ABI.clone(),
                STAKETABLE_BYTECODE.clone().into(),
                client,
            );
            let deployer = factory.deploy(constructor_args)?;
            let deployer = ::ethers::contract::ContractDeployer::new(deployer);
            Ok(deployer)
        }
        ///Calls the contract's `_hashBlsKey` (0x9b30a5e6) function
        pub fn hash_bls_key(
            &self,
            bls_vk: G2Point,
        ) -> ::ethers::contract::builders::ContractCall<M, [u8; 32]> {
            self.0
                .method_hash([155, 48, 165, 230], (bls_vk,))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `_isEqualBlsKey` (0xf2f80a18) function
        pub fn is_equal_bls_key(
            &self,
            a: G2Point,
            b: G2Point,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([242, 248, 10, 24], (a, b))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `admin` (0xf851a440) function
        pub fn admin(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
            self.0
                .method_hash([248, 81, 164, 64], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `claimValidatorExit` (0x2140fecd) function
        pub fn claim_validator_exit(
            &self,
            validator: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([33, 64, 254, 205], validator)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `claimWithdrawal` (0xa3066aab) function
        pub fn claim_withdrawal(
            &self,
            validator: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([163, 6, 106, 171], validator)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `delegate` (0x026e402b) function
        pub fn delegate(
            &self,
            validator: ::ethers::core::types::Address,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([2, 110, 64, 43], (validator, amount))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `deregisterValidator` (0x6a911ccf) function
        pub fn deregister_validator(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([106, 145, 28, 207], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `exitEscrowPeriod` (0x9e9a8f31) function
        pub fn exit_escrow_period(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([158, 154, 143, 49], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `initialize` (0x8129fc1c) function
        pub fn initialize(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([129, 41, 252, 28], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `initializedAtBlock` (0x3e9df9b5) function
        pub fn initialized_at_block(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([62, 157, 249, 181], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `owner` (0x8da5cb5b) function
        pub fn owner(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
            self.0
                .method_hash([141, 165, 203, 91], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `registerValidator` (0x13b9057a) function
        pub fn register_validator(
            &self,
            bls_vk: G2Point,
            schnorr_vk: EdOnBN254Point,
            bls_sig: G1Point,
            commission: u16,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([19, 185, 5, 122], (bls_vk, schnorr_vk, bls_sig, commission))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `renounceOwnership` (0x715018a6) function
        pub fn renounce_ownership(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([113, 80, 24, 166], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `tokenAddress` (0x9d76ea58) function
        pub fn token_address(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::Address> {
            self.0
                .method_hash([157, 118, 234, 88], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `transferOwnership` (0xf2fde38b) function
        pub fn transfer_ownership(
            &self,
            new_owner: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([242, 253, 227, 139], new_owner)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `undelegate` (0x4d99dd16) function
        pub fn undelegate(
            &self,
            validator: ::ethers::core::types::Address,
            amount: ::ethers::core::types::U256,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([77, 153, 221, 22], (validator, amount))
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `updateConsensusKeys` (0x5544c2f1) function
        pub fn update_consensus_keys(
            &self,
            new_bls_vk: G2Point,
            new_schnorr_vk: EdOnBN254Point,
            new_bls_sig: G1Point,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash(
                    [85, 68, 194, 241],
                    (new_bls_vk, new_schnorr_vk, new_bls_sig),
                )
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `validatorExits` (0xb5ecb344) function
        pub fn validator_exits(
            &self,
            validator: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([181, 236, 179, 68], validator)
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `validators` (0xfa52c7d8) function
        pub fn validators(
            &self,
            validator: ::ethers::core::types::Address,
        ) -> ::ethers::contract::builders::ContractCall<M, (bool, u8, ::ethers::core::types::U256)>
        {
            self.0
                .method_hash([250, 82, 199, 216], validator)
                .expect("method not found (this should never happen)")
        }
        ///Gets the contract's `ConsensusKeysUpdated` event
        pub fn consensus_keys_updated_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ConsensusKeysUpdatedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `Delegated` event
        pub fn delegated_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, DelegatedFilter> {
            self.0.event()
        }
        ///Gets the contract's `Initialized` event
        pub fn initialized_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, InitializedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `OwnershipTransferred` event
        pub fn ownership_transferred_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, OwnershipTransferredFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `Undelegated` event
        pub fn undelegated_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, UndelegatedFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `ValidatorExit` event
        pub fn validator_exit_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ValidatorExitFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `ValidatorRegistered` event
        pub fn validator_registered_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, ValidatorRegisteredFilter>
        {
            self.0.event()
        }
        ///Gets the contract's `Withdrawal` event
        pub fn withdrawal_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, WithdrawalFilter> {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StakeTableEvents> {
            self.0
                .event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>> for StakeTable<M> {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
    ///Custom Error type `BLSSigVerificationFailed` with signature `BLSSigVerificationFailed()` and selector `0x0ced3e50`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "BLSSigVerificationFailed", abi = "BLSSigVerificationFailed()")]
    pub struct BLSSigVerificationFailed;
    ///Custom Error type `InsufficientAllowance` with signature `InsufficientAllowance(uint256,uint256)` and selector `0x2a1b2dd8`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(
        name = "InsufficientAllowance",
        abi = "InsufficientAllowance(uint256,uint256)"
    )]
    pub struct InsufficientAllowance(
        pub ::ethers::core::types::U256,
        pub ::ethers::core::types::U256,
    );
    ///Custom Error type `InsufficientBalance` with signature `InsufficientBalance(uint256)` and selector `0x92665351`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "InsufficientBalance", abi = "InsufficientBalance(uint256)")]
    pub struct InsufficientBalance(pub ::ethers::core::types::U256);
    ///Custom Error type `InvalidCommission` with signature `InvalidCommission()` and selector `0xdc81db85`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "InvalidCommission", abi = "InvalidCommission()")]
    pub struct InvalidCommission;
    ///Custom Error type `InvalidInitialization` with signature `InvalidInitialization()` and selector `0xf92ee8a9`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "InvalidInitialization", abi = "InvalidInitialization()")]
    pub struct InvalidInitialization;
    ///Custom Error type `InvalidSchnorrVK` with signature `InvalidSchnorrVK()` and selector `0x06cf438f`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "InvalidSchnorrVK", abi = "InvalidSchnorrVK()")]
    pub struct InvalidSchnorrVK;
    ///Custom Error type `NotInitializing` with signature `NotInitializing()` and selector `0xd7e6bcf8`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "NotInitializing", abi = "NotInitializing()")]
    pub struct NotInitializing;
    ///Custom Error type `NothingToWithdraw` with signature `NothingToWithdraw()` and selector `0xd0d04f60`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "NothingToWithdraw", abi = "NothingToWithdraw()")]
    pub struct NothingToWithdraw;
    ///Custom Error type `OwnableInvalidOwner` with signature `OwnableInvalidOwner(address)` and selector `0x1e4fbdf7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "OwnableInvalidOwner", abi = "OwnableInvalidOwner(address)")]
    pub struct OwnableInvalidOwner {
        pub owner: ::ethers::core::types::Address,
    }
    ///Custom Error type `OwnableUnauthorizedAccount` with signature `OwnableUnauthorizedAccount(address)` and selector `0x118cdaa7`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(
        name = "OwnableUnauthorizedAccount",
        abi = "OwnableUnauthorizedAccount(address)"
    )]
    pub struct OwnableUnauthorizedAccount {
        pub account: ::ethers::core::types::Address,
    }
    ///Custom Error type `PrematureWithdrawal` with signature `PrematureWithdrawal()` and selector `0x5a774357`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "PrematureWithdrawal", abi = "PrematureWithdrawal()")]
    pub struct PrematureWithdrawal;
    ///Custom Error type `UnknownValidator` with signature `UnknownValidator()` and selector `0x57fdf40b`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "UnknownValidator", abi = "UnknownValidator()")]
    pub struct UnknownValidator;
    ///Custom Error type `ValidatorAlreadyExited` with signature `ValidatorAlreadyExited()` and selector `0xeab4a963`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ValidatorAlreadyExited", abi = "ValidatorAlreadyExited()")]
    pub struct ValidatorAlreadyExited;
    ///Custom Error type `ValidatorAlreadyRegistered` with signature `ValidatorAlreadyRegistered()` and selector `0x9973f7d8`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(
        name = "ValidatorAlreadyRegistered",
        abi = "ValidatorAlreadyRegistered()"
    )]
    pub struct ValidatorAlreadyRegistered;
    ///Custom Error type `ValidatorNotExited` with signature `ValidatorNotExited()` and selector `0xf25314a6`
    #[derive(
        Clone,
        ::ethers::contract::EthError,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[etherror(name = "ValidatorNotExited", abi = "ValidatorNotExited()")]
    pub struct ValidatorNotExited;
    ///Container type for all of the contract's custom errors
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        serde::Serialize,
        serde::Deserialize,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
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
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for StakeTableErrors {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) =
                <::std::string::String as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RevertString(decoded));
            }
            if let Ok(decoded) =
                <BLSSigVerificationFailed as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::BLSSigVerificationFailed(decoded));
            }
            if let Ok(decoded) =
                <InsufficientAllowance as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InsufficientAllowance(decoded));
            }
            if let Ok(decoded) =
                <InsufficientBalance as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InsufficientBalance(decoded));
            }
            if let Ok(decoded) = <InvalidCommission as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InvalidCommission(decoded));
            }
            if let Ok(decoded) =
                <InvalidInitialization as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InvalidInitialization(decoded));
            }
            if let Ok(decoded) = <InvalidSchnorrVK as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InvalidSchnorrVK(decoded));
            }
            if let Ok(decoded) = <NotInitializing as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotInitializing(decoded));
            }
            if let Ok(decoded) = <NothingToWithdraw as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::NothingToWithdraw(decoded));
            }
            if let Ok(decoded) =
                <OwnableInvalidOwner as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OwnableInvalidOwner(decoded));
            }
            if let Ok(decoded) =
                <OwnableUnauthorizedAccount as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::OwnableUnauthorizedAccount(decoded));
            }
            if let Ok(decoded) =
                <PrematureWithdrawal as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::PrematureWithdrawal(decoded));
            }
            if let Ok(decoded) = <UnknownValidator as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::UnknownValidator(decoded));
            }
            if let Ok(decoded) =
                <ValidatorAlreadyExited as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ValidatorAlreadyExited(decoded));
            }
            if let Ok(decoded) =
                <ValidatorAlreadyRegistered as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ValidatorAlreadyRegistered(decoded));
            }
            if let Ok(decoded) =
                <ValidatorNotExited as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ValidatorNotExited(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for StakeTableErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::BLSSigVerificationFailed(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InsufficientAllowance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InsufficientBalance(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidCommission(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::InvalidInitialization(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::InvalidSchnorrVK(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::NotInitializing(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::NothingToWithdraw(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OwnableInvalidOwner(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::OwnableUnauthorizedAccount(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::PrematureWithdrawal(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::UnknownValidator(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::ValidatorAlreadyExited(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorAlreadyRegistered(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorNotExited(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for StakeTableErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <BLSSigVerificationFailed as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <InsufficientAllowance as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <InsufficientBalance as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <InvalidCommission as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <InvalidInitialization as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector == <InvalidSchnorrVK as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector == <NotInitializing as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <NothingToWithdraw as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <OwnableInvalidOwner as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <OwnableUnauthorizedAccount as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <PrematureWithdrawal as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector == <UnknownValidator as ::ethers::contract::EthError>::selector() => {
                    true
                }
                _ if selector
                    == <ValidatorAlreadyExited as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ValidatorAlreadyRegistered as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ if selector
                    == <ValidatorNotExited as ::ethers::contract::EthError>::selector() =>
                {
                    true
                }
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for StakeTableErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::BLSSigVerificationFailed(element) => ::core::fmt::Display::fmt(element, f),
                Self::InsufficientAllowance(element) => ::core::fmt::Display::fmt(element, f),
                Self::InsufficientBalance(element) => ::core::fmt::Display::fmt(element, f),
                Self::InvalidCommission(element) => ::core::fmt::Display::fmt(element, f),
                Self::InvalidInitialization(element) => ::core::fmt::Display::fmt(element, f),
                Self::InvalidSchnorrVK(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotInitializing(element) => ::core::fmt::Display::fmt(element, f),
                Self::NothingToWithdraw(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnableInvalidOwner(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnableUnauthorizedAccount(element) => ::core::fmt::Display::fmt(element, f),
                Self::PrematureWithdrawal(element) => ::core::fmt::Display::fmt(element, f),
                Self::UnknownValidator(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorAlreadyExited(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorAlreadyRegistered(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorNotExited(element) => ::core::fmt::Display::fmt(element, f),
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for StakeTableErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<BLSSigVerificationFailed> for StakeTableErrors {
        fn from(value: BLSSigVerificationFailed) -> Self {
            Self::BLSSigVerificationFailed(value)
        }
    }
    impl ::core::convert::From<InsufficientAllowance> for StakeTableErrors {
        fn from(value: InsufficientAllowance) -> Self {
            Self::InsufficientAllowance(value)
        }
    }
    impl ::core::convert::From<InsufficientBalance> for StakeTableErrors {
        fn from(value: InsufficientBalance) -> Self {
            Self::InsufficientBalance(value)
        }
    }
    impl ::core::convert::From<InvalidCommission> for StakeTableErrors {
        fn from(value: InvalidCommission) -> Self {
            Self::InvalidCommission(value)
        }
    }
    impl ::core::convert::From<InvalidInitialization> for StakeTableErrors {
        fn from(value: InvalidInitialization) -> Self {
            Self::InvalidInitialization(value)
        }
    }
    impl ::core::convert::From<InvalidSchnorrVK> for StakeTableErrors {
        fn from(value: InvalidSchnorrVK) -> Self {
            Self::InvalidSchnorrVK(value)
        }
    }
    impl ::core::convert::From<NotInitializing> for StakeTableErrors {
        fn from(value: NotInitializing) -> Self {
            Self::NotInitializing(value)
        }
    }
    impl ::core::convert::From<NothingToWithdraw> for StakeTableErrors {
        fn from(value: NothingToWithdraw) -> Self {
            Self::NothingToWithdraw(value)
        }
    }
    impl ::core::convert::From<OwnableInvalidOwner> for StakeTableErrors {
        fn from(value: OwnableInvalidOwner) -> Self {
            Self::OwnableInvalidOwner(value)
        }
    }
    impl ::core::convert::From<OwnableUnauthorizedAccount> for StakeTableErrors {
        fn from(value: OwnableUnauthorizedAccount) -> Self {
            Self::OwnableUnauthorizedAccount(value)
        }
    }
    impl ::core::convert::From<PrematureWithdrawal> for StakeTableErrors {
        fn from(value: PrematureWithdrawal) -> Self {
            Self::PrematureWithdrawal(value)
        }
    }
    impl ::core::convert::From<UnknownValidator> for StakeTableErrors {
        fn from(value: UnknownValidator) -> Self {
            Self::UnknownValidator(value)
        }
    }
    impl ::core::convert::From<ValidatorAlreadyExited> for StakeTableErrors {
        fn from(value: ValidatorAlreadyExited) -> Self {
            Self::ValidatorAlreadyExited(value)
        }
    }
    impl ::core::convert::From<ValidatorAlreadyRegistered> for StakeTableErrors {
        fn from(value: ValidatorAlreadyRegistered) -> Self {
            Self::ValidatorAlreadyRegistered(value)
        }
    }
    impl ::core::convert::From<ValidatorNotExited> for StakeTableErrors {
        fn from(value: ValidatorNotExited) -> Self {
            Self::ValidatorNotExited(value)
        }
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(
        name = "ConsensusKeysUpdated",
        abi = "ConsensusKeysUpdated(address,(uint256,uint256,uint256,uint256),(uint256,uint256))"
    )]
    pub struct ConsensusKeysUpdatedFilter {
        pub account: ::ethers::core::types::Address,
        pub bls_vk: G2Point,
        pub schnorr_vk: EdOnBN254Point,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(name = "Delegated", abi = "Delegated(address,address,uint256)")]
    pub struct DelegatedFilter {
        pub delegator: ::ethers::core::types::Address,
        pub validator: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(name = "Initialized", abi = "Initialized(uint64)")]
    pub struct InitializedFilter {
        pub version: u64,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(
        name = "OwnershipTransferred",
        abi = "OwnershipTransferred(address,address)"
    )]
    pub struct OwnershipTransferredFilter {
        #[ethevent(indexed)]
        pub previous_owner: ::ethers::core::types::Address,
        #[ethevent(indexed)]
        pub new_owner: ::ethers::core::types::Address,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(name = "Undelegated", abi = "Undelegated(address,address,uint256)")]
    pub struct UndelegatedFilter {
        pub delegator: ::ethers::core::types::Address,
        pub validator: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(name = "ValidatorExit", abi = "ValidatorExit(address)")]
    pub struct ValidatorExitFilter {
        pub validator: ::ethers::core::types::Address,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(
        name = "ValidatorRegistered",
        abi = "ValidatorRegistered(address,(uint256,uint256,uint256,uint256),(uint256,uint256),uint16)"
    )]
    pub struct ValidatorRegisteredFilter {
        pub account: ::ethers::core::types::Address,
        pub bls_vk: G2Point,
        pub schnorr_vk: EdOnBN254Point,
        pub commission: u16,
    }
    #[derive(
        Clone,
        ::ethers::contract::EthEvent,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethevent(name = "Withdrawal", abi = "Withdrawal(address,uint256)")]
    pub struct WithdrawalFilter {
        pub account: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all of the contract's events
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        serde::Serialize,
        serde::Deserialize,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub enum StakeTableEvents {
        ConsensusKeysUpdatedFilter(ConsensusKeysUpdatedFilter),
        DelegatedFilter(DelegatedFilter),
        InitializedFilter(InitializedFilter),
        OwnershipTransferredFilter(OwnershipTransferredFilter),
        UndelegatedFilter(UndelegatedFilter),
        ValidatorExitFilter(ValidatorExitFilter),
        ValidatorRegisteredFilter(ValidatorRegisteredFilter),
        WithdrawalFilter(WithdrawalFilter),
    }
    impl ::ethers::contract::EthLogDecode for StakeTableEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = ConsensusKeysUpdatedFilter::decode_log(log) {
                return Ok(StakeTableEvents::ConsensusKeysUpdatedFilter(decoded));
            }
            if let Ok(decoded) = DelegatedFilter::decode_log(log) {
                return Ok(StakeTableEvents::DelegatedFilter(decoded));
            }
            if let Ok(decoded) = InitializedFilter::decode_log(log) {
                return Ok(StakeTableEvents::InitializedFilter(decoded));
            }
            if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
                return Ok(StakeTableEvents::OwnershipTransferredFilter(decoded));
            }
            if let Ok(decoded) = UndelegatedFilter::decode_log(log) {
                return Ok(StakeTableEvents::UndelegatedFilter(decoded));
            }
            if let Ok(decoded) = ValidatorExitFilter::decode_log(log) {
                return Ok(StakeTableEvents::ValidatorExitFilter(decoded));
            }
            if let Ok(decoded) = ValidatorRegisteredFilter::decode_log(log) {
                return Ok(StakeTableEvents::ValidatorRegisteredFilter(decoded));
            }
            if let Ok(decoded) = WithdrawalFilter::decode_log(log) {
                return Ok(StakeTableEvents::WithdrawalFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for StakeTableEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::ConsensusKeysUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::DelegatedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::InitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnershipTransferredFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::UndelegatedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorExitFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorRegisteredFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::WithdrawalFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<ConsensusKeysUpdatedFilter> for StakeTableEvents {
        fn from(value: ConsensusKeysUpdatedFilter) -> Self {
            Self::ConsensusKeysUpdatedFilter(value)
        }
    }
    impl ::core::convert::From<DelegatedFilter> for StakeTableEvents {
        fn from(value: DelegatedFilter) -> Self {
            Self::DelegatedFilter(value)
        }
    }
    impl ::core::convert::From<InitializedFilter> for StakeTableEvents {
        fn from(value: InitializedFilter) -> Self {
            Self::InitializedFilter(value)
        }
    }
    impl ::core::convert::From<OwnershipTransferredFilter> for StakeTableEvents {
        fn from(value: OwnershipTransferredFilter) -> Self {
            Self::OwnershipTransferredFilter(value)
        }
    }
    impl ::core::convert::From<UndelegatedFilter> for StakeTableEvents {
        fn from(value: UndelegatedFilter) -> Self {
            Self::UndelegatedFilter(value)
        }
    }
    impl ::core::convert::From<ValidatorExitFilter> for StakeTableEvents {
        fn from(value: ValidatorExitFilter) -> Self {
            Self::ValidatorExitFilter(value)
        }
    }
    impl ::core::convert::From<ValidatorRegisteredFilter> for StakeTableEvents {
        fn from(value: ValidatorRegisteredFilter) -> Self {
            Self::ValidatorRegisteredFilter(value)
        }
    }
    impl ::core::convert::From<WithdrawalFilter> for StakeTableEvents {
        fn from(value: WithdrawalFilter) -> Self {
            Self::WithdrawalFilter(value)
        }
    }
    ///Container type for all input parameters for the `_hashBlsKey` function with signature `_hashBlsKey((uint256,uint256,uint256,uint256))` and selector `0x9b30a5e6`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(
        name = "_hashBlsKey",
        abi = "_hashBlsKey((uint256,uint256,uint256,uint256))"
    )]
    pub struct HashBlsKeyCall {
        pub bls_vk: G2Point,
    }
    ///Container type for all input parameters for the `_isEqualBlsKey` function with signature `_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))` and selector `0xf2f80a18`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(
        name = "_isEqualBlsKey",
        abi = "_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))"
    )]
    pub struct IsEqualBlsKeyCall {
        pub a: G2Point,
        pub b: G2Point,
    }
    ///Container type for all input parameters for the `admin` function with signature `admin()` and selector `0xf851a440`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "admin", abi = "admin()")]
    pub struct AdminCall;
    ///Container type for all input parameters for the `claimValidatorExit` function with signature `claimValidatorExit(address)` and selector `0x2140fecd`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "claimValidatorExit", abi = "claimValidatorExit(address)")]
    pub struct ClaimValidatorExitCall {
        pub validator: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `claimWithdrawal` function with signature `claimWithdrawal(address)` and selector `0xa3066aab`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "claimWithdrawal", abi = "claimWithdrawal(address)")]
    pub struct ClaimWithdrawalCall {
        pub validator: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `delegate` function with signature `delegate(address,uint256)` and selector `0x026e402b`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "delegate", abi = "delegate(address,uint256)")]
    pub struct DelegateCall {
        pub validator: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `deregisterValidator` function with signature `deregisterValidator()` and selector `0x6a911ccf`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "deregisterValidator", abi = "deregisterValidator()")]
    pub struct DeregisterValidatorCall;
    ///Container type for all input parameters for the `exitEscrowPeriod` function with signature `exitEscrowPeriod()` and selector `0x9e9a8f31`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "exitEscrowPeriod", abi = "exitEscrowPeriod()")]
    pub struct ExitEscrowPeriodCall;
    ///Container type for all input parameters for the `initialize` function with signature `initialize()` and selector `0x8129fc1c`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "initialize", abi = "initialize()")]
    pub struct InitializeCall;
    ///Container type for all input parameters for the `initializedAtBlock` function with signature `initializedAtBlock()` and selector `0x3e9df9b5`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "initializedAtBlock", abi = "initializedAtBlock()")]
    pub struct InitializedAtBlockCall;
    ///Container type for all input parameters for the `owner` function with signature `owner()` and selector `0x8da5cb5b`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "owner", abi = "owner()")]
    pub struct OwnerCall;
    ///Container type for all input parameters for the `registerValidator` function with signature `registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)` and selector `0x13b9057a`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(
        name = "registerValidator",
        abi = "registerValidator((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256),uint16)"
    )]
    pub struct RegisterValidatorCall {
        pub bls_vk: G2Point,
        pub schnorr_vk: EdOnBN254Point,
        pub bls_sig: G1Point,
        pub commission: u16,
    }
    ///Container type for all input parameters for the `renounceOwnership` function with signature `renounceOwnership()` and selector `0x715018a6`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "renounceOwnership", abi = "renounceOwnership()")]
    pub struct RenounceOwnershipCall;
    ///Container type for all input parameters for the `tokenAddress` function with signature `tokenAddress()` and selector `0x9d76ea58`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "tokenAddress", abi = "tokenAddress()")]
    pub struct TokenAddressCall;
    ///Container type for all input parameters for the `transferOwnership` function with signature `transferOwnership(address)` and selector `0xf2fde38b`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "transferOwnership", abi = "transferOwnership(address)")]
    pub struct TransferOwnershipCall {
        pub new_owner: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `undelegate` function with signature `undelegate(address,uint256)` and selector `0x4d99dd16`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "undelegate", abi = "undelegate(address,uint256)")]
    pub struct UndelegateCall {
        pub validator: ::ethers::core::types::Address,
        pub amount: ::ethers::core::types::U256,
    }
    ///Container type for all input parameters for the `updateConsensusKeys` function with signature `updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))` and selector `0x5544c2f1`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(
        name = "updateConsensusKeys",
        abi = "updateConsensusKeys((uint256,uint256,uint256,uint256),(uint256,uint256),(uint256,uint256))"
    )]
    pub struct UpdateConsensusKeysCall {
        pub new_bls_vk: G2Point,
        pub new_schnorr_vk: EdOnBN254Point,
        pub new_bls_sig: G1Point,
    }
    ///Container type for all input parameters for the `validatorExits` function with signature `validatorExits(address)` and selector `0xb5ecb344`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "validatorExits", abi = "validatorExits(address)")]
    pub struct ValidatorExitsCall {
        pub validator: ::ethers::core::types::Address,
    }
    ///Container type for all input parameters for the `validators` function with signature `validators(address)` and selector `0xfa52c7d8`
    #[derive(
        Clone,
        ::ethers::contract::EthCall,
        ::ethers::contract::EthDisplay,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    #[ethcall(name = "validators", abi = "validators(address)")]
    pub struct ValidatorsCall {
        pub validator: ::ethers::core::types::Address,
    }
    ///Container type for all of the contract's call
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        serde::Serialize,
        serde::Deserialize,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub enum StakeTableCalls {
        HashBlsKey(HashBlsKeyCall),
        IsEqualBlsKey(IsEqualBlsKeyCall),
        Admin(AdminCall),
        ClaimValidatorExit(ClaimValidatorExitCall),
        ClaimWithdrawal(ClaimWithdrawalCall),
        Delegate(DelegateCall),
        DeregisterValidator(DeregisterValidatorCall),
        ExitEscrowPeriod(ExitEscrowPeriodCall),
        Initialize(InitializeCall),
        InitializedAtBlock(InitializedAtBlockCall),
        Owner(OwnerCall),
        RegisterValidator(RegisterValidatorCall),
        RenounceOwnership(RenounceOwnershipCall),
        TokenAddress(TokenAddressCall),
        TransferOwnership(TransferOwnershipCall),
        Undelegate(UndelegateCall),
        UpdateConsensusKeys(UpdateConsensusKeysCall),
        ValidatorExits(ValidatorExitsCall),
        Validators(ValidatorsCall),
    }
    impl ::ethers::core::abi::AbiDecode for StakeTableCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <HashBlsKeyCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::HashBlsKey(decoded));
            }
            if let Ok(decoded) = <IsEqualBlsKeyCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::IsEqualBlsKey(decoded));
            }
            if let Ok(decoded) = <AdminCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Admin(decoded));
            }
            if let Ok(decoded) =
                <ClaimValidatorExitCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ClaimValidatorExit(decoded));
            }
            if let Ok(decoded) =
                <ClaimWithdrawalCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ClaimWithdrawal(decoded));
            }
            if let Ok(decoded) = <DelegateCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Delegate(decoded));
            }
            if let Ok(decoded) =
                <DeregisterValidatorCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::DeregisterValidator(decoded));
            }
            if let Ok(decoded) =
                <ExitEscrowPeriodCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ExitEscrowPeriod(decoded));
            }
            if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Initialize(decoded));
            }
            if let Ok(decoded) =
                <InitializedAtBlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InitializedAtBlock(decoded));
            }
            if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Owner(decoded));
            }
            if let Ok(decoded) =
                <RegisterValidatorCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RegisterValidator(decoded));
            }
            if let Ok(decoded) =
                <RenounceOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RenounceOwnership(decoded));
            }
            if let Ok(decoded) = <TokenAddressCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::TokenAddress(decoded));
            }
            if let Ok(decoded) =
                <TransferOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::TransferOwnership(decoded));
            }
            if let Ok(decoded) = <UndelegateCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Undelegate(decoded));
            }
            if let Ok(decoded) =
                <UpdateConsensusKeysCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::UpdateConsensusKeys(decoded));
            }
            if let Ok(decoded) =
                <ValidatorExitsCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::ValidatorExits(decoded));
            }
            if let Ok(decoded) = <ValidatorsCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Validators(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for StakeTableCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::HashBlsKey(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::IsEqualBlsKey(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Admin(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::ClaimValidatorExit(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ClaimWithdrawal(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Delegate(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::DeregisterValidator(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ExitEscrowPeriod(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::InitializedAtBlock(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RegisterValidator(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RenounceOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TokenAddress(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TransferOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Undelegate(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::UpdateConsensusKeys(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                }
                Self::ValidatorExits(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Validators(element) => ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for StakeTableCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::HashBlsKey(element) => ::core::fmt::Display::fmt(element, f),
                Self::IsEqualBlsKey(element) => ::core::fmt::Display::fmt(element, f),
                Self::Admin(element) => ::core::fmt::Display::fmt(element, f),
                Self::ClaimValidatorExit(element) => ::core::fmt::Display::fmt(element, f),
                Self::ClaimWithdrawal(element) => ::core::fmt::Display::fmt(element, f),
                Self::Delegate(element) => ::core::fmt::Display::fmt(element, f),
                Self::DeregisterValidator(element) => ::core::fmt::Display::fmt(element, f),
                Self::ExitEscrowPeriod(element) => ::core::fmt::Display::fmt(element, f),
                Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
                Self::InitializedAtBlock(element) => ::core::fmt::Display::fmt(element, f),
                Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
                Self::RegisterValidator(element) => ::core::fmt::Display::fmt(element, f),
                Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::TokenAddress(element) => ::core::fmt::Display::fmt(element, f),
                Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::Undelegate(element) => ::core::fmt::Display::fmt(element, f),
                Self::UpdateConsensusKeys(element) => ::core::fmt::Display::fmt(element, f),
                Self::ValidatorExits(element) => ::core::fmt::Display::fmt(element, f),
                Self::Validators(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<HashBlsKeyCall> for StakeTableCalls {
        fn from(value: HashBlsKeyCall) -> Self {
            Self::HashBlsKey(value)
        }
    }
    impl ::core::convert::From<IsEqualBlsKeyCall> for StakeTableCalls {
        fn from(value: IsEqualBlsKeyCall) -> Self {
            Self::IsEqualBlsKey(value)
        }
    }
    impl ::core::convert::From<AdminCall> for StakeTableCalls {
        fn from(value: AdminCall) -> Self {
            Self::Admin(value)
        }
    }
    impl ::core::convert::From<ClaimValidatorExitCall> for StakeTableCalls {
        fn from(value: ClaimValidatorExitCall) -> Self {
            Self::ClaimValidatorExit(value)
        }
    }
    impl ::core::convert::From<ClaimWithdrawalCall> for StakeTableCalls {
        fn from(value: ClaimWithdrawalCall) -> Self {
            Self::ClaimWithdrawal(value)
        }
    }
    impl ::core::convert::From<DelegateCall> for StakeTableCalls {
        fn from(value: DelegateCall) -> Self {
            Self::Delegate(value)
        }
    }
    impl ::core::convert::From<DeregisterValidatorCall> for StakeTableCalls {
        fn from(value: DeregisterValidatorCall) -> Self {
            Self::DeregisterValidator(value)
        }
    }
    impl ::core::convert::From<ExitEscrowPeriodCall> for StakeTableCalls {
        fn from(value: ExitEscrowPeriodCall) -> Self {
            Self::ExitEscrowPeriod(value)
        }
    }
    impl ::core::convert::From<InitializeCall> for StakeTableCalls {
        fn from(value: InitializeCall) -> Self {
            Self::Initialize(value)
        }
    }
    impl ::core::convert::From<InitializedAtBlockCall> for StakeTableCalls {
        fn from(value: InitializedAtBlockCall) -> Self {
            Self::InitializedAtBlock(value)
        }
    }
    impl ::core::convert::From<OwnerCall> for StakeTableCalls {
        fn from(value: OwnerCall) -> Self {
            Self::Owner(value)
        }
    }
    impl ::core::convert::From<RegisterValidatorCall> for StakeTableCalls {
        fn from(value: RegisterValidatorCall) -> Self {
            Self::RegisterValidator(value)
        }
    }
    impl ::core::convert::From<RenounceOwnershipCall> for StakeTableCalls {
        fn from(value: RenounceOwnershipCall) -> Self {
            Self::RenounceOwnership(value)
        }
    }
    impl ::core::convert::From<TokenAddressCall> for StakeTableCalls {
        fn from(value: TokenAddressCall) -> Self {
            Self::TokenAddress(value)
        }
    }
    impl ::core::convert::From<TransferOwnershipCall> for StakeTableCalls {
        fn from(value: TransferOwnershipCall) -> Self {
            Self::TransferOwnership(value)
        }
    }
    impl ::core::convert::From<UndelegateCall> for StakeTableCalls {
        fn from(value: UndelegateCall) -> Self {
            Self::Undelegate(value)
        }
    }
    impl ::core::convert::From<UpdateConsensusKeysCall> for StakeTableCalls {
        fn from(value: UpdateConsensusKeysCall) -> Self {
            Self::UpdateConsensusKeys(value)
        }
    }
    impl ::core::convert::From<ValidatorExitsCall> for StakeTableCalls {
        fn from(value: ValidatorExitsCall) -> Self {
            Self::ValidatorExits(value)
        }
    }
    impl ::core::convert::From<ValidatorsCall> for StakeTableCalls {
        fn from(value: ValidatorsCall) -> Self {
            Self::Validators(value)
        }
    }
    ///Container type for all return fields from the `_hashBlsKey` function with signature `_hashBlsKey((uint256,uint256,uint256,uint256))` and selector `0x9b30a5e6`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct HashBlsKeyReturn(pub [u8; 32]);
    ///Container type for all return fields from the `_isEqualBlsKey` function with signature `_isEqualBlsKey((uint256,uint256,uint256,uint256),(uint256,uint256,uint256,uint256))` and selector `0xf2f80a18`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct IsEqualBlsKeyReturn(pub bool);
    ///Container type for all return fields from the `admin` function with signature `admin()` and selector `0xf851a440`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct AdminReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `exitEscrowPeriod` function with signature `exitEscrowPeriod()` and selector `0x9e9a8f31`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct ExitEscrowPeriodReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `initializedAtBlock` function with signature `initializedAtBlock()` and selector `0x3e9df9b5`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct InitializedAtBlockReturn(pub ::ethers::core::types::U256);
    ///Container type for all return fields from the `owner` function with signature `owner()` and selector `0x8da5cb5b`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct OwnerReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `tokenAddress` function with signature `tokenAddress()` and selector `0x9d76ea58`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct TokenAddressReturn(pub ::ethers::core::types::Address);
    ///Container type for all return fields from the `validatorExits` function with signature `validatorExits(address)` and selector `0xb5ecb344`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct ValidatorExitsReturn {
        pub unlocks_at: ::ethers::core::types::U256,
    }
    ///Container type for all return fields from the `validators` function with signature `validators(address)` and selector `0xfa52c7d8`
    #[derive(
        Clone,
        ::ethers::contract::EthAbiType,
        ::ethers::contract::EthAbiCodec,
        serde::Serialize,
        serde::Deserialize,
        Default,
        Debug,
        PartialEq,
        Eq,
        Hash,
    )]
    pub struct ValidatorsReturn {
        pub is_registered: bool,
        pub status: u8,
        pub delegated_amount: ::ethers::core::types::U256,
    }
}
