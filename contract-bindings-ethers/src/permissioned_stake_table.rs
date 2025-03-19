pub use permissioned_stake_table::*;
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
pub mod permissioned_stake_table {
    #[allow(deprecated)]
    fn __abi() -> ::ethers::core::abi::Abi {
        ::ethers::core::abi::ethabi::Contract {
            constructor: ::core::option::Option::Some(::ethers::core::abi::ethabi::Constructor {
                inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                    name: ::std::borrow::ToOwned::to_owned("initialStakers"),
                    kind: ::ethers::core::abi::ethabi::ParamType::Array(::std::boxed::Box::new(
                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                            ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                            ],),
                            ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                            ],),
                            ::ethers::core::abi::ethabi::ParamType::Bool,
                        ],),
                    ),),
                    internal_type: ::core::option::Option::Some(::std::borrow::ToOwned::to_owned(
                        "struct PermissionedStakeTable.NodeInfo[]",
                    ),),
                },],
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
                    ::std::borrow::ToOwned::to_owned("isStaker"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("isStaker"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::borrow::ToOwned::to_owned("staker"),
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
                            kind: ::ethers::core::abi::ethabi::ParamType::Bool,
                            internal_type: ::core::option::Option::Some(
                                ::std::borrow::ToOwned::to_owned("bool"),
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
                    ::std::borrow::ToOwned::to_owned("update"),
                    ::std::vec![::ethers::core::abi::ethabi::Function {
                        name: ::std::borrow::ToOwned::to_owned("update"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("stakersToRemove"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                    ::std::boxed::Box::new(
                                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],),
                                    ),
                                ),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned("struct BN254.G2Point[]"),
                                ),
                            },
                            ::ethers::core::abi::ethabi::Param {
                                name: ::std::borrow::ToOwned::to_owned("newStakers"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                    ::std::boxed::Box::new(
                                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
                                        ],),
                                    ),
                                ),
                                internal_type: ::core::option::Option::Some(
                                    ::std::borrow::ToOwned::to_owned(
                                        "struct PermissionedStakeTable.NodeInfo[]",
                                    ),
                                ),
                            },
                        ],
                        outputs: ::std::vec![],
                        constant: ::core::option::Option::None,
                        state_mutability: ::ethers::core::abi::ethabi::StateMutability::NonPayable,
                    },],
                ),
            ]),
            events: ::core::convert::From::from([
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
                    ::std::borrow::ToOwned::to_owned("StakersUpdated"),
                    ::std::vec![::ethers::core::abi::ethabi::Event {
                        name: ::std::borrow::ToOwned::to_owned("StakersUpdated"),
                        inputs: ::std::vec![
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("removed"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                    ::std::boxed::Box::new(
                                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                            ::ethers::core::abi::ethabi::ParamType::Uint(256usize),
                                        ],),
                                    ),
                                ),
                                indexed: false,
                            },
                            ::ethers::core::abi::ethabi::EventParam {
                                name: ::std::borrow::ToOwned::to_owned("added"),
                                kind: ::ethers::core::abi::ethabi::ParamType::Array(
                                    ::std::boxed::Box::new(
                                        ::ethers::core::abi::ethabi::ParamType::Tuple(::std::vec![
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Tuple(
                                                ::std::vec![
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                    ::ethers::core::abi::ethabi::ParamType::Uint(
                                                        256usize
                                                    ),
                                                ],
                                            ),
                                            ::ethers::core::abi::ethabi::ParamType::Bool,
                                        ],),
                                    ),
                                ),
                                indexed: false,
                            },
                        ],
                        anonymous: false,
                    },],
                ),
            ]),
            errors: ::core::convert::From::from([
                (
                    ::std::borrow::ToOwned::to_owned("InvalidInitialization"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("InvalidInitialization",),
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
                    ::std::borrow::ToOwned::to_owned("StakerAlreadyExists"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("StakerAlreadyExists",),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
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
                    },],
                ),
                (
                    ::std::borrow::ToOwned::to_owned("StakerNotFound"),
                    ::std::vec![::ethers::core::abi::ethabi::AbiError {
                        name: ::std::borrow::ToOwned::to_owned("StakerNotFound"),
                        inputs: ::std::vec![::ethers::core::abi::ethabi::Param {
                            name: ::std::string::String::new(),
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
                    },],
                ),
            ]),
            receive: false,
            fallback: false,
        }
    }
    ///The parsed JSON ABI of the contract.
    pub static PERMISSIONEDSTAKETABLE_ABI: ::ethers::contract::Lazy<::ethers::core::abi::Abi> =
        ::ethers::contract::Lazy::new(__abi);
    #[rustfmt::skip]
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x10W_\x80\xFD[P`@Qb\0\rI8\x03\x80b\0\rI\x839\x81\x01`@\x81\x90Rb\0\x003\x91b\0\x03\xB7V[3\x80b\0\0ZW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[b\0\0e\x81b\0\0\x86V[Pb\0\0pb\0\0\xD5V[C`\x01Ub\0\0\x7F\x81b\0\x01\x89V[Pb\0\x04\xF5V[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x90\x04`\xFF\x16\x15b\0\x01&W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x80T`\x01`\x01`@\x1B\x03\x90\x81\x16\x14b\0\x01\x86W\x80T`\x01`\x01`@\x1B\x03\x19\x16`\x01`\x01`@\x1B\x03\x90\x81\x17\x82U`@Q\x90\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PV[_[\x81Q\x81\x10\x15b\0\x02\\W_b\0\x01\xC6\x83\x83\x81Q\x81\x10b\0\x01\xAFWb\0\x01\xAFb\0\x04\xE1V[` \x02` \x01\x01Q_\x01Qb\0\x02`` \x1B` \x1CV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15b\0\x02:W\x82\x82\x81Q\x81\x10b\0\x01\xF5Wb\0\x01\xF5b\0\x04\xE1V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01b\0\0QV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01b\0\x01\x8BV[PPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01b\0\x02\x9E\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x02\xF4Wb\0\x02\xF4b\0\x02\xBBV[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x02\xF4Wb\0\x02\xF4b\0\x02\xBBV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03JWb\0\x03Jb\0\x02\xBBV[`@R\x91\x90PV[_`@\x82\x84\x03\x12\x15b\0\x03cW_\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03\x88Wb\0\x03\x88b\0\x02\xBBV[`@R\x82Q\x81R` \x92\x83\x01Q\x92\x81\x01\x92\x90\x92RP\x91\x90PV[\x80Q\x80\x15\x15\x81\x14b\0\x03\xB2W_\x80\xFD[\x91\x90PV[_` \x80\x83\x85\x03\x12\x15b\0\x03\xC9W_\x80\xFD[\x82Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x03\xE0W_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12b\0\x03\xF4W_\x80\xFD[\x81Q\x81\x81\x11\x15b\0\x04\tWb\0\x04\tb\0\x02\xBBV[b\0\x04\x19\x84\x82`\x05\x1B\x01b\0\x03\x1FV[\x81\x81R\x84\x81\x01\x92P`\xE0\x91\x82\x02\x84\x01\x85\x01\x91\x88\x83\x11\x15b\0\x048W_\x80\xFD[\x93\x85\x01\x93[\x82\x85\x10\x15b\0\x04\xD5W\x84\x89\x03\x81\x81\x12\x15b\0\x04VW_\x80\xFD[b\0\x04`b\0\x02\xCFV[`\x80\x80\x83\x12\x15b\0\x04oW_\x80\xFD[b\0\x04yb\0\x02\xFAV[\x92P\x87Q\x83R\x88\x88\x01Q\x89\x84\x01R`@\x80\x89\x01Q\x81\x85\x01R``\x80\x8A\x01Q\x81\x86\x01RP\x83\x83Rb\0\x04\xAD\x8D\x83\x8B\x01b\0\x03RV[\x8A\x84\x01Rb\0\x04\xBF`\xC0\x8A\x01b\0\x03\xA2V[\x90\x83\x01RP\x85RP\x93\x84\x01\x93\x92\x85\x01\x92b\0\x04=V[P\x97\x96PPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[a\x08F\x80b\0\x05\x03_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0zW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0XW\x80c\x8D\xA5\xCB[\x14a\0\xC7W\x80c\x9B0\xA5\xE6\x14a\0\xE1W\x80c\xCB\xBA|x\x14a\0\xF4W\x80c\xF2\xFD\xE3\x8B\x14a\x01\x07W_\x80\xFD[\x80c>\x9D\xF9\xB5\x14a\0~W\x80cqP\x18\xA6\x14a\0\x9AW\x80cu\xD7\x05\xE9\x14a\0\xA4W[_\x80\xFD[a\0\x87`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xA2a\x01\x1AV[\0[a\0\xB7a\0\xB26`\x04a\x053V[a\x01-V[`@Q\x90\x15\x15\x81R` \x01a\0\x91V[_T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\x91V[a\0\x87a\0\xEF6`\x04a\x053V[a\x01SV[a\0\xA2a\x01\x026`\x04a\x06JV[a\x01\xADV[a\0\xA2a\x01\x156`\x04a\x07\x10V[a\x02\x04V[a\x01\"a\x02FV[a\x01+_a\x02rV[V[_`\x02_a\x01:\x84a\x01SV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x92\x91PPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x01\x90\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\x01\xB5a\x02FV[a\x01\xBE\x82a\x02\xC1V[a\x01\xC7\x81a\x03}V[\x7FK\xB3\x1C\xD9\xAE\x87\xA3\xF8(\x95X\xC7\x9B\xE5#%\x06\xCE6\xDEBi\xC8\xBAE4\xA6\xFB\xE9\xE2\x96]\x82\x82`@Qa\x01\xF8\x92\x91\x90a\x076V[`@Q\x80\x91\x03\x90\xA1PPV[a\x02\x0Ca\x02FV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x02:W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x02C\x81a\x02rV[PV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01+W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x021V[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[_[\x81Q\x81\x10\x15a\x03yW_a\x02\xEF\x83\x83\x81Q\x81\x10a\x02\xE2Wa\x02\xE2a\x08%V[` \x02` \x01\x01Qa\x01SV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16a\x03\\W\x82\x82\x81Q\x81\x10a\x03\x19Wa\x03\x19a\x08%V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q`@\x80Qc4\xA7V\x1F`\xE0\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x021V[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16\x90U`\x01\x01a\x02\xC3V[PPV[_[\x81Q\x81\x10\x15a\x03yW_a\x03\xAE\x83\x83\x81Q\x81\x10a\x03\x9EWa\x03\x9Ea\x08%V[` \x02` \x01\x01Q_\x01Qa\x01SV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15a\x04\x1DW\x82\x82\x81Q\x81\x10a\x03\xD9Wa\x03\xD9a\x08%V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x021V[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01a\x03\x7FV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04uWa\x04ua\x04>V[`@R\x90V[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04uWa\x04ua\x04>V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04\xC7Wa\x04\xC7a\x04>V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\x04\xDFW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x05\x02Wa\x05\x02a\x04>V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`\x80\x82\x84\x03\x12\x15a\x05CW_\x80\xFD[a\x05M\x83\x83a\x04\xCFV[\x93\x92PPPV[_g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x05mWa\x05ma\x04>V[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\x05\x86W_\x80\xFD[\x815` a\x05\x9Ba\x05\x96\x83a\x05TV[a\x04\x9EV[\x82\x81R`\xE0\x92\x83\x02\x85\x01\x82\x01\x92\x82\x82\x01\x91\x90\x87\x85\x11\x15a\x05\xB9W_\x80\xFD[\x83\x87\x01[\x85\x81\x10\x15a\x06=W\x80\x89\x03\x82\x81\x12\x15a\x05\xD4W_\x80\xFD[a\x05\xDCa\x04RV[a\x05\xE6\x8B\x84a\x04\xCFV[\x81R`@\x80`\x7F\x19\x84\x01\x12\x15a\x05\xFAW_\x80\xFD[a\x06\x02a\x04{V[`\x80\x85\x015\x81R`\xA0\x85\x015\x89\x82\x01R\x82\x89\x01R`\xC0\x84\x015\x92P\x82\x15\x15\x83\x14a\x06*W_\x80\xFD[\x81\x01\x91\x90\x91R\x84R\x92\x84\x01\x92\x81\x01a\x05\xBDV[P\x90\x97\x96PPPPPPPV[_\x80`@\x83\x85\x03\x12\x15a\x06[W_\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x06rW_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12a\x06\x85W_\x80\xFD[\x815` a\x06\x95a\x05\x96\x83a\x05TV[\x80\x83\x82R` \x82\x01\x91P` \x84`\x07\x1B\x87\x01\x01\x93P\x89\x84\x11\x15a\x06\xB6W_\x80\xFD[` \x86\x01\x95P[\x83\x86\x10\x15a\x06\xE1Wa\x06\xCF\x8A\x87a\x04\xCFV[\x82R\x82\x82\x01\x91P`\x80\x86\x01\x95Pa\x06\xBDV[\x96PPPP` \x85\x015\x91P\x80\x82\x11\x15a\x06\xF9W_\x80\xFD[Pa\x07\x06\x85\x82\x86\x01a\x05wV[\x91PP\x92P\x92\x90PV[_` \x82\x84\x03\x12\x15a\x07 W_\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05MW_\x80\xFD[`@\x80\x82R\x83Q\x82\x82\x01\x81\x90R_\x91\x90` \x90``\x85\x01\x90\x82\x88\x01\x85[\x82\x81\x10\x15a\x07\x9AWa\x07\x87\x84\x83Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[`\x80\x93\x90\x93\x01\x92\x90\x84\x01\x90`\x01\x01a\x07SV[PPP\x84\x81\x03\x82\x86\x01R\x85Q\x80\x82R\x86\x83\x01\x91\x83\x01\x90_[\x81\x81\x10\x15a\x08\x17W\x83Qa\x07\xE8\x84\x82Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x80\x86\x01Q\x80Q`\x80\x86\x01R\x86\x01Q`\xA0\x85\x01R\x86\x01Q\x15\x15`\xC0\x84\x01R\x92\x84\x01\x92`\xE0\x90\x92\x01\x91`\x01\x01a\x07\xB2V[P\x90\x98\x97PPPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x17\0\n";
    /// The bytecode of the contract.
    pub static PERMISSIONEDSTAKETABLE_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0zW_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0XW\x80c\x8D\xA5\xCB[\x14a\0\xC7W\x80c\x9B0\xA5\xE6\x14a\0\xE1W\x80c\xCB\xBA|x\x14a\0\xF4W\x80c\xF2\xFD\xE3\x8B\x14a\x01\x07W_\x80\xFD[\x80c>\x9D\xF9\xB5\x14a\0~W\x80cqP\x18\xA6\x14a\0\x9AW\x80cu\xD7\x05\xE9\x14a\0\xA4W[_\x80\xFD[a\0\x87`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xA2a\x01\x1AV[\0[a\0\xB7a\0\xB26`\x04a\x053V[a\x01-V[`@Q\x90\x15\x15\x81R` \x01a\0\x91V[_T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\x91V[a\0\x87a\0\xEF6`\x04a\x053V[a\x01SV[a\0\xA2a\x01\x026`\x04a\x06JV[a\x01\xADV[a\0\xA2a\x01\x156`\x04a\x07\x10V[a\x02\x04V[a\x01\"a\x02FV[a\x01+_a\x02rV[V[_`\x02_a\x01:\x84a\x01SV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x92\x91PPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x01\x90\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\x01\xB5a\x02FV[a\x01\xBE\x82a\x02\xC1V[a\x01\xC7\x81a\x03}V[\x7FK\xB3\x1C\xD9\xAE\x87\xA3\xF8(\x95X\xC7\x9B\xE5#%\x06\xCE6\xDEBi\xC8\xBAE4\xA6\xFB\xE9\xE2\x96]\x82\x82`@Qa\x01\xF8\x92\x91\x90a\x076V[`@Q\x80\x91\x03\x90\xA1PPV[a\x02\x0Ca\x02FV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x02:W`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x02C\x81a\x02rV[PV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01+W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x021V[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[_[\x81Q\x81\x10\x15a\x03yW_a\x02\xEF\x83\x83\x81Q\x81\x10a\x02\xE2Wa\x02\xE2a\x08%V[` \x02` \x01\x01Qa\x01SV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16a\x03\\W\x82\x82\x81Q\x81\x10a\x03\x19Wa\x03\x19a\x08%V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q`@\x80Qc4\xA7V\x1F`\xE0\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x021V[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16\x90U`\x01\x01a\x02\xC3V[PPV[_[\x81Q\x81\x10\x15a\x03yW_a\x03\xAE\x83\x83\x81Q\x81\x10a\x03\x9EWa\x03\x9Ea\x08%V[` \x02` \x01\x01Q_\x01Qa\x01SV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15a\x04\x1DW\x82\x82\x81Q\x81\x10a\x03\xD9Wa\x03\xD9a\x08%V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x021V[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01a\x03\x7FV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04uWa\x04ua\x04>V[`@R\x90V[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04uWa\x04ua\x04>V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x04\xC7Wa\x04\xC7a\x04>V[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\x04\xDFW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x05\x02Wa\x05\x02a\x04>V[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`\x80\x82\x84\x03\x12\x15a\x05CW_\x80\xFD[a\x05M\x83\x83a\x04\xCFV[\x93\x92PPPV[_g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x05mWa\x05ma\x04>V[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\x05\x86W_\x80\xFD[\x815` a\x05\x9Ba\x05\x96\x83a\x05TV[a\x04\x9EV[\x82\x81R`\xE0\x92\x83\x02\x85\x01\x82\x01\x92\x82\x82\x01\x91\x90\x87\x85\x11\x15a\x05\xB9W_\x80\xFD[\x83\x87\x01[\x85\x81\x10\x15a\x06=W\x80\x89\x03\x82\x81\x12\x15a\x05\xD4W_\x80\xFD[a\x05\xDCa\x04RV[a\x05\xE6\x8B\x84a\x04\xCFV[\x81R`@\x80`\x7F\x19\x84\x01\x12\x15a\x05\xFAW_\x80\xFD[a\x06\x02a\x04{V[`\x80\x85\x015\x81R`\xA0\x85\x015\x89\x82\x01R\x82\x89\x01R`\xC0\x84\x015\x92P\x82\x15\x15\x83\x14a\x06*W_\x80\xFD[\x81\x01\x91\x90\x91R\x84R\x92\x84\x01\x92\x81\x01a\x05\xBDV[P\x90\x97\x96PPPPPPPV[_\x80`@\x83\x85\x03\x12\x15a\x06[W_\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x06rW_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12a\x06\x85W_\x80\xFD[\x815` a\x06\x95a\x05\x96\x83a\x05TV[\x80\x83\x82R` \x82\x01\x91P` \x84`\x07\x1B\x87\x01\x01\x93P\x89\x84\x11\x15a\x06\xB6W_\x80\xFD[` \x86\x01\x95P[\x83\x86\x10\x15a\x06\xE1Wa\x06\xCF\x8A\x87a\x04\xCFV[\x82R\x82\x82\x01\x91P`\x80\x86\x01\x95Pa\x06\xBDV[\x96PPPP` \x85\x015\x91P\x80\x82\x11\x15a\x06\xF9W_\x80\xFD[Pa\x07\x06\x85\x82\x86\x01a\x05wV[\x91PP\x92P\x92\x90PV[_` \x82\x84\x03\x12\x15a\x07 W_\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x05MW_\x80\xFD[`@\x80\x82R\x83Q\x82\x82\x01\x81\x90R_\x91\x90` \x90``\x85\x01\x90\x82\x88\x01\x85[\x82\x81\x10\x15a\x07\x9AWa\x07\x87\x84\x83Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[`\x80\x93\x90\x93\x01\x92\x90\x84\x01\x90`\x01\x01a\x07SV[PPP\x84\x81\x03\x82\x86\x01R\x85Q\x80\x82R\x86\x83\x01\x91\x83\x01\x90_[\x81\x81\x10\x15a\x08\x17W\x83Qa\x07\xE8\x84\x82Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x80\x86\x01Q\x80Q`\x80\x86\x01R\x86\x01Q`\xA0\x85\x01R\x86\x01Q\x15\x15`\xC0\x84\x01R\x92\x84\x01\x92`\xE0\x90\x92\x01\x91`\x01\x01a\x07\xB2V[P\x90\x98\x97PPPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x17\0\n";
    /// The deployed bytecode of the contract.
    pub static PERMISSIONEDSTAKETABLE_DEPLOYED_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__DEPLOYED_BYTECODE);
    pub struct PermissionedStakeTable<M>(::ethers::contract::Contract<M>);
    impl<M> ::core::clone::Clone for PermissionedStakeTable<M> {
        fn clone(&self) -> Self {
            Self(::core::clone::Clone::clone(&self.0))
        }
    }
    impl<M> ::core::ops::Deref for PermissionedStakeTable<M> {
        type Target = ::ethers::contract::Contract<M>;
        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
    impl<M> ::core::ops::DerefMut for PermissionedStakeTable<M> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.0
        }
    }
    impl<M> ::core::fmt::Debug for PermissionedStakeTable<M> {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            f.debug_tuple(::core::stringify!(PermissionedStakeTable))
                .field(&self.address())
                .finish()
        }
    }
    impl<M: ::ethers::providers::Middleware> PermissionedStakeTable<M> {
        /// Creates a new contract instance with the specified `ethers` client at
        /// `address`. The contract derefs to a `ethers::Contract` object.
        pub fn new<T: Into<::ethers::core::types::Address>>(
            address: T,
            client: ::std::sync::Arc<M>,
        ) -> Self {
            Self(::ethers::contract::Contract::new(
                address.into(),
                PERMISSIONEDSTAKETABLE_ABI.clone(),
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
                PERMISSIONEDSTAKETABLE_ABI.clone(),
                PERMISSIONEDSTAKETABLE_BYTECODE.clone().into(),
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
        ///Calls the contract's `initializedAtBlock` (0x3e9df9b5) function
        pub fn initialized_at_block(
            &self,
        ) -> ::ethers::contract::builders::ContractCall<M, ::ethers::core::types::U256> {
            self.0
                .method_hash([62, 157, 249, 181], ())
                .expect("method not found (this should never happen)")
        }
        ///Calls the contract's `isStaker` (0x75d705e9) function
        pub fn is_staker(
            &self,
            staker: G2Point,
        ) -> ::ethers::contract::builders::ContractCall<M, bool> {
            self.0
                .method_hash([117, 215, 5, 233], (staker,))
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
        ///Calls the contract's `renounceOwnership` (0x715018a6) function
        pub fn renounce_ownership(&self) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([113, 80, 24, 166], ())
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
        ///Calls the contract's `update` (0xcbba7c78) function
        pub fn update(
            &self,
            stakers_to_remove: ::std::vec::Vec<G2Point>,
            new_stakers: ::std::vec::Vec<NodeInfo>,
        ) -> ::ethers::contract::builders::ContractCall<M, ()> {
            self.0
                .method_hash([203, 186, 124, 120], (stakers_to_remove, new_stakers))
                .expect("method not found (this should never happen)")
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
        ///Gets the contract's `StakersUpdated` event
        pub fn stakers_updated_filter(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, StakersUpdatedFilter>
        {
            self.0.event()
        }
        /// Returns an `Event` builder for all the events of this contract.
        pub fn events(
            &self,
        ) -> ::ethers::contract::builders::Event<::std::sync::Arc<M>, M, PermissionedStakeTableEvents>
        {
            self.0
                .event_with_filter(::core::default::Default::default())
        }
    }
    impl<M: ::ethers::providers::Middleware> From<::ethers::contract::Contract<M>>
        for PermissionedStakeTable<M>
    {
        fn from(contract: ::ethers::contract::Contract<M>) -> Self {
            Self::new(contract.address(), contract.client())
        }
    }
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
    ///Custom Error type `StakerAlreadyExists` with signature `StakerAlreadyExists((uint256,uint256,uint256,uint256))` and selector `0x360dc282`
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
        name = "StakerAlreadyExists",
        abi = "StakerAlreadyExists((uint256,uint256,uint256,uint256))"
    )]
    pub struct StakerAlreadyExists(pub G2Point);
    ///Custom Error type `StakerNotFound` with signature `StakerNotFound((uint256,uint256,uint256,uint256))` and selector `0x34a7561f`
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
        name = "StakerNotFound",
        abi = "StakerNotFound((uint256,uint256,uint256,uint256))"
    )]
    pub struct StakerNotFound(pub G2Point);
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
    pub enum PermissionedStakeTableErrors {
        InvalidInitialization(InvalidInitialization),
        NotInitializing(NotInitializing),
        OwnableInvalidOwner(OwnableInvalidOwner),
        OwnableUnauthorizedAccount(OwnableUnauthorizedAccount),
        StakerAlreadyExists(StakerAlreadyExists),
        StakerNotFound(StakerNotFound),
        /// The standard solidity revert string, with selector
        /// Error(string) -- 0x08c379a0
        RevertString(::std::string::String),
    }
    impl ::ethers::core::abi::AbiDecode for PermissionedStakeTableErrors {
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
                <InvalidInitialization as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InvalidInitialization(decoded));
            }
            if let Ok(decoded) = <NotInitializing as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::NotInitializing(decoded));
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
                <StakerAlreadyExists as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::StakerAlreadyExists(decoded));
            }
            if let Ok(decoded) = <StakerNotFound as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::StakerNotFound(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for PermissionedStakeTableErrors {
        fn encode(self) -> ::std::vec::Vec<u8> {
            match self {
                Self::InvalidInitialization(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                },
                Self::NotInitializing(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::OwnableInvalidOwner(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                },
                Self::OwnableUnauthorizedAccount(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                },
                Self::StakerAlreadyExists(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                },
                Self::StakerNotFound(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RevertString(s) => ::ethers::core::abi::AbiEncode::encode(s),
            }
        }
    }
    impl ::ethers::contract::ContractRevert for PermissionedStakeTableErrors {
        fn valid_selector(selector: [u8; 4]) -> bool {
            match selector {
                [0x08, 0xc3, 0x79, 0xa0] => true,
                _ if selector
                    == <InvalidInitialization as ::ethers::contract::EthError>::selector() =>
                {
                    true
                },
                _ if selector == <NotInitializing as ::ethers::contract::EthError>::selector() => {
                    true
                },
                _ if selector
                    == <OwnableInvalidOwner as ::ethers::contract::EthError>::selector() =>
                {
                    true
                },
                _ if selector
                    == <OwnableUnauthorizedAccount as ::ethers::contract::EthError>::selector() =>
                {
                    true
                },
                _ if selector
                    == <StakerAlreadyExists as ::ethers::contract::EthError>::selector() =>
                {
                    true
                },
                _ if selector == <StakerNotFound as ::ethers::contract::EthError>::selector() => {
                    true
                },
                _ => false,
            }
        }
    }
    impl ::core::fmt::Display for PermissionedStakeTableErrors {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::InvalidInitialization(element) => ::core::fmt::Display::fmt(element, f),
                Self::NotInitializing(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnableInvalidOwner(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnableUnauthorizedAccount(element) => ::core::fmt::Display::fmt(element, f),
                Self::StakerAlreadyExists(element) => ::core::fmt::Display::fmt(element, f),
                Self::StakerNotFound(element) => ::core::fmt::Display::fmt(element, f),
                Self::RevertString(s) => ::core::fmt::Display::fmt(s, f),
            }
        }
    }
    impl ::core::convert::From<::std::string::String> for PermissionedStakeTableErrors {
        fn from(value: String) -> Self {
            Self::RevertString(value)
        }
    }
    impl ::core::convert::From<InvalidInitialization> for PermissionedStakeTableErrors {
        fn from(value: InvalidInitialization) -> Self {
            Self::InvalidInitialization(value)
        }
    }
    impl ::core::convert::From<NotInitializing> for PermissionedStakeTableErrors {
        fn from(value: NotInitializing) -> Self {
            Self::NotInitializing(value)
        }
    }
    impl ::core::convert::From<OwnableInvalidOwner> for PermissionedStakeTableErrors {
        fn from(value: OwnableInvalidOwner) -> Self {
            Self::OwnableInvalidOwner(value)
        }
    }
    impl ::core::convert::From<OwnableUnauthorizedAccount> for PermissionedStakeTableErrors {
        fn from(value: OwnableUnauthorizedAccount) -> Self {
            Self::OwnableUnauthorizedAccount(value)
        }
    }
    impl ::core::convert::From<StakerAlreadyExists> for PermissionedStakeTableErrors {
        fn from(value: StakerAlreadyExists) -> Self {
            Self::StakerAlreadyExists(value)
        }
    }
    impl ::core::convert::From<StakerNotFound> for PermissionedStakeTableErrors {
        fn from(value: StakerNotFound) -> Self {
            Self::StakerNotFound(value)
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
    #[ethevent(
        name = "StakersUpdated",
        abi = "StakersUpdated((uint256,uint256,uint256,uint256)[],((uint256,uint256,uint256,uint256),(uint256,uint256),bool)[])"
    )]
    pub struct StakersUpdatedFilter {
        pub removed: ::std::vec::Vec<G2Point>,
        pub added: ::std::vec::Vec<NodeInfo>,
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
    pub enum PermissionedStakeTableEvents {
        InitializedFilter(InitializedFilter),
        OwnershipTransferredFilter(OwnershipTransferredFilter),
        StakersUpdatedFilter(StakersUpdatedFilter),
    }
    impl ::ethers::contract::EthLogDecode for PermissionedStakeTableEvents {
        fn decode_log(
            log: &::ethers::core::abi::RawLog,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::Error> {
            if let Ok(decoded) = InitializedFilter::decode_log(log) {
                return Ok(PermissionedStakeTableEvents::InitializedFilter(decoded));
            }
            if let Ok(decoded) = OwnershipTransferredFilter::decode_log(log) {
                return Ok(PermissionedStakeTableEvents::OwnershipTransferredFilter(
                    decoded,
                ));
            }
            if let Ok(decoded) = StakersUpdatedFilter::decode_log(log) {
                return Ok(PermissionedStakeTableEvents::StakersUpdatedFilter(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData)
        }
    }
    impl ::core::fmt::Display for PermissionedStakeTableEvents {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::InitializedFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::OwnershipTransferredFilter(element) => ::core::fmt::Display::fmt(element, f),
                Self::StakersUpdatedFilter(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<InitializedFilter> for PermissionedStakeTableEvents {
        fn from(value: InitializedFilter) -> Self {
            Self::InitializedFilter(value)
        }
    }
    impl ::core::convert::From<OwnershipTransferredFilter> for PermissionedStakeTableEvents {
        fn from(value: OwnershipTransferredFilter) -> Self {
            Self::OwnershipTransferredFilter(value)
        }
    }
    impl ::core::convert::From<StakersUpdatedFilter> for PermissionedStakeTableEvents {
        fn from(value: StakersUpdatedFilter) -> Self {
            Self::StakersUpdatedFilter(value)
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
    ///Container type for all input parameters for the `isStaker` function with signature `isStaker((uint256,uint256,uint256,uint256))` and selector `0x75d705e9`
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
    #[ethcall(name = "isStaker", abi = "isStaker((uint256,uint256,uint256,uint256))")]
    pub struct IsStakerCall {
        pub staker: G2Point,
    }
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
    ///Container type for all input parameters for the `update` function with signature `update((uint256,uint256,uint256,uint256)[],((uint256,uint256,uint256,uint256),(uint256,uint256),bool)[])` and selector `0xcbba7c78`
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
        name = "update",
        abi = "update((uint256,uint256,uint256,uint256)[],((uint256,uint256,uint256,uint256),(uint256,uint256),bool)[])"
    )]
    pub struct UpdateCall {
        pub stakers_to_remove: ::std::vec::Vec<G2Point>,
        pub new_stakers: ::std::vec::Vec<NodeInfo>,
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
    pub enum PermissionedStakeTableCalls {
        HashBlsKey(HashBlsKeyCall),
        InitializedAtBlock(InitializedAtBlockCall),
        IsStaker(IsStakerCall),
        Owner(OwnerCall),
        RenounceOwnership(RenounceOwnershipCall),
        TransferOwnership(TransferOwnershipCall),
        Update(UpdateCall),
    }
    impl ::ethers::core::abi::AbiDecode for PermissionedStakeTableCalls {
        fn decode(
            data: impl AsRef<[u8]>,
        ) -> ::core::result::Result<Self, ::ethers::core::abi::AbiError> {
            let data = data.as_ref();
            if let Ok(decoded) = <HashBlsKeyCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::HashBlsKey(decoded));
            }
            if let Ok(decoded) =
                <InitializedAtBlockCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::InitializedAtBlock(decoded));
            }
            if let Ok(decoded) = <IsStakerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::IsStaker(decoded));
            }
            if let Ok(decoded) = <OwnerCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Owner(decoded));
            }
            if let Ok(decoded) =
                <RenounceOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::RenounceOwnership(decoded));
            }
            if let Ok(decoded) =
                <TransferOwnershipCall as ::ethers::core::abi::AbiDecode>::decode(data)
            {
                return Ok(Self::TransferOwnership(decoded));
            }
            if let Ok(decoded) = <UpdateCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Update(decoded));
            }
            Err(::ethers::core::abi::Error::InvalidData.into())
        }
    }
    impl ::ethers::core::abi::AbiEncode for PermissionedStakeTableCalls {
        fn encode(self) -> Vec<u8> {
            match self {
                Self::HashBlsKey(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::InitializedAtBlock(element) => {
                    ::ethers::core::abi::AbiEncode::encode(element)
                },
                Self::IsStaker(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Owner(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::RenounceOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::TransferOwnership(element) => ::ethers::core::abi::AbiEncode::encode(element),
                Self::Update(element) => ::ethers::core::abi::AbiEncode::encode(element),
            }
        }
    }
    impl ::core::fmt::Display for PermissionedStakeTableCalls {
        fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
            match self {
                Self::HashBlsKey(element) => ::core::fmt::Display::fmt(element, f),
                Self::InitializedAtBlock(element) => ::core::fmt::Display::fmt(element, f),
                Self::IsStaker(element) => ::core::fmt::Display::fmt(element, f),
                Self::Owner(element) => ::core::fmt::Display::fmt(element, f),
                Self::RenounceOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::TransferOwnership(element) => ::core::fmt::Display::fmt(element, f),
                Self::Update(element) => ::core::fmt::Display::fmt(element, f),
            }
        }
    }
    impl ::core::convert::From<HashBlsKeyCall> for PermissionedStakeTableCalls {
        fn from(value: HashBlsKeyCall) -> Self {
            Self::HashBlsKey(value)
        }
    }
    impl ::core::convert::From<InitializedAtBlockCall> for PermissionedStakeTableCalls {
        fn from(value: InitializedAtBlockCall) -> Self {
            Self::InitializedAtBlock(value)
        }
    }
    impl ::core::convert::From<IsStakerCall> for PermissionedStakeTableCalls {
        fn from(value: IsStakerCall) -> Self {
            Self::IsStaker(value)
        }
    }
    impl ::core::convert::From<OwnerCall> for PermissionedStakeTableCalls {
        fn from(value: OwnerCall) -> Self {
            Self::Owner(value)
        }
    }
    impl ::core::convert::From<RenounceOwnershipCall> for PermissionedStakeTableCalls {
        fn from(value: RenounceOwnershipCall) -> Self {
            Self::RenounceOwnership(value)
        }
    }
    impl ::core::convert::From<TransferOwnershipCall> for PermissionedStakeTableCalls {
        fn from(value: TransferOwnershipCall) -> Self {
            Self::TransferOwnership(value)
        }
    }
    impl ::core::convert::From<UpdateCall> for PermissionedStakeTableCalls {
        fn from(value: UpdateCall) -> Self {
            Self::Update(value)
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
    ///Container type for all return fields from the `isStaker` function with signature `isStaker((uint256,uint256,uint256,uint256))` and selector `0x75d705e9`
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
    pub struct IsStakerReturn(pub bool);
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
    ///`G2Point(uint256,uint256,uint256,uint256)`
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
    pub struct G2Point {
        pub x_0: ::ethers::core::types::U256,
        pub x_1: ::ethers::core::types::U256,
        pub y_0: ::ethers::core::types::U256,
        pub y_1: ::ethers::core::types::U256,
    }
    ///`EdOnBN254Point(uint256,uint256)`
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
    pub struct EdOnBN254Point {
        pub x: ::ethers::core::types::U256,
        pub y: ::ethers::core::types::U256,
    }
    ///`NodeInfo((uint256,uint256,uint256,uint256),(uint256,uint256),bool)`
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
    pub struct NodeInfo {
        pub bls_vk: G2Point,
        pub schnorr_vk: EdOnBN254Point,
        pub is_da: bool,
    }
}
