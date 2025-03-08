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
    const __BYTECODE: &[u8] = b"`\x80`@R4\x80\x15b\0\0\x10W_\x80\xFD[P`@Qb\0\x0E\xC18\x03\x80b\0\x0E\xC1\x839\x81\x01`@\x81\x90Rb\0\x003\x91b\0\x04\x14V[3\x80b\0\0ZW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[b\0\0e\x81b\0\0\x82V[Pb\0\0pb\0\0\xD1V[b\0\0{\x81b\0\x01\xE6V[Pb\0\x05RV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80Th\x01\0\0\0\0\0\0\0\0\x81\x04`\xFF\x16\x15\x90`\x01`\x01`@\x1B\x03\x16_\x81\x15\x80\x15b\0\x01\x1BWP\x82[\x90P_\x82`\x01`\x01`@\x1B\x03\x16`\x01\x14\x80\x15b\0\x017WP0;\x15[\x90P\x81\x15\x80\x15b\0\x01FWP\x80\x15[\x15b\0\x01eW`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84T`\x01`\x01`@\x1B\x03\x19\x16`\x01\x17\x85U\x83\x15b\0\x01\x94W\x84T`\xFF`@\x1B\x19\x16h\x01\0\0\0\0\0\0\0\0\x17\x85U[C`\x01U\x83\x15b\0\x01\xDFW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[_[\x81Q\x81\x10\x15b\0\x02\xB9W_b\0\x02#\x83\x83\x81Q\x81\x10b\0\x02\x0CWb\0\x02\x0Cb\0\x05>V[` \x02` \x01\x01Q_\x01Qb\0\x02\xBD` \x1B` \x1CV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15b\0\x02\x97W\x82\x82\x81Q\x81\x10b\0\x02RWb\0\x02Rb\0\x05>V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01b\0\0QV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01b\0\x01\xE8V[PPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01b\0\x02\xFB\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03QWb\0\x03Qb\0\x03\x18V[`@R\x90V[`@Q`\x80\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03QWb\0\x03Qb\0\x03\x18V[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03\xA7Wb\0\x03\xA7b\0\x03\x18V[`@R\x91\x90PV[_`@\x82\x84\x03\x12\x15b\0\x03\xC0W_\x80\xFD[`@\x80Q\x90\x81\x01`\x01`\x01`@\x1B\x03\x81\x11\x82\x82\x10\x17\x15b\0\x03\xE5Wb\0\x03\xE5b\0\x03\x18V[`@R\x82Q\x81R` \x92\x83\x01Q\x92\x81\x01\x92\x90\x92RP\x91\x90PV[\x80Q\x80\x15\x15\x81\x14b\0\x04\x0FW_\x80\xFD[\x91\x90PV[_` \x80\x83\x85\x03\x12\x15b\0\x04&W_\x80\xFD[\x82Q`\x01`\x01`@\x1B\x03\x80\x82\x11\x15b\0\x04=W_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12b\0\x04QW_\x80\xFD[\x81Q\x81\x81\x11\x15b\0\x04fWb\0\x04fb\0\x03\x18V[b\0\x04v\x84\x82`\x05\x1B\x01b\0\x03|V[\x81\x81R\x84\x81\x01\x92P`\xE0\x91\x82\x02\x84\x01\x85\x01\x91\x88\x83\x11\x15b\0\x04\x95W_\x80\xFD[\x93\x85\x01\x93[\x82\x85\x10\x15b\0\x052W\x84\x89\x03\x81\x81\x12\x15b\0\x04\xB3W_\x80\xFD[b\0\x04\xBDb\0\x03,V[`\x80\x80\x83\x12\x15b\0\x04\xCCW_\x80\xFD[b\0\x04\xD6b\0\x03WV[\x92P\x87Q\x83R\x88\x88\x01Q\x89\x84\x01R`@\x80\x89\x01Q\x81\x85\x01R``\x80\x8A\x01Q\x81\x86\x01RP\x83\x83Rb\0\x05\n\x8D\x83\x8B\x01b\0\x03\xAFV[\x8A\x84\x01Rb\0\x05\x1C`\xC0\x8A\x01b\0\x03\xFFV[\x90\x83\x01RP\x85RP\x93\x84\x01\x93\x92\x85\x01\x92b\0\x04\x9AV[P\x97\x96PPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD[a\ta\x80b\0\x05`_9_\xF3\xFE`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0\x85W_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0XW\x80c\x8D\xA5\xCB[\x14a\0\xDAW\x80c\x9B0\xA5\xE6\x14a\0\xF4W\x80c\xCB\xBA|x\x14a\x01\x07W\x80c\xF2\xFD\xE3\x8B\x14a\x01\x1AW_\x80\xFD[\x80c>\x9D\xF9\xB5\x14a\0\x89W\x80cqP\x18\xA6\x14a\0\xA5W\x80cu\xD7\x05\xE9\x14a\0\xAFW\x80c\x81)\xFC\x1C\x14a\0\xD2W[_\x80\xFD[a\0\x92`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xADa\x01-V[\0[a\0\xC2a\0\xBD6`\x04a\x06NV[a\x01@V[`@Q\x90\x15\x15\x81R` \x01a\0\x9CV[a\0\xADa\x01fV[_T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\x9CV[a\0\x92a\x01\x026`\x04a\x06NV[a\x02nV[a\0\xADa\x01\x156`\x04a\x07eV[a\x02\xC8V[a\0\xADa\x01(6`\x04a\x08+V[a\x03\x1FV[a\x015a\x03aV[a\x01>_a\x03\x8DV[V[_`\x02_a\x01M\x84a\x02nV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x92\x91PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x01\xABWP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x01\xC7WP0;\x15[\x90P\x81\x15\x80\x15a\x01\xD5WP\x80\x15[\x15a\x01\xF3W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x02\x1DW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\x02gW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x02\xAB\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\x02\xD0a\x03aV[a\x02\xD9\x82a\x03\xDCV[a\x02\xE2\x81a\x04\x98V[\x7FK\xB3\x1C\xD9\xAE\x87\xA3\xF8(\x95X\xC7\x9B\xE5#%\x06\xCE6\xDEBi\xC8\xBAE4\xA6\xFB\xE9\xE2\x96]\x82\x82`@Qa\x03\x13\x92\x91\x90a\x08QV[`@Q\x80\x91\x03\x90\xA1PPV[a\x03'a\x03aV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x03UW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x03^\x81a\x03\x8DV[PV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01>W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03LV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[_[\x81Q\x81\x10\x15a\x04\x94W_a\x04\n\x83\x83\x81Q\x81\x10a\x03\xFDWa\x03\xFDa\t@V[` \x02` \x01\x01Qa\x02nV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16a\x04wW\x82\x82\x81Q\x81\x10a\x044Wa\x044a\t@V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q`@\x80Qc4\xA7V\x1F`\xE0\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x03LV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16\x90U`\x01\x01a\x03\xDEV[PPV[_[\x81Q\x81\x10\x15a\x04\x94W_a\x04\xC9\x83\x83\x81Q\x81\x10a\x04\xB9Wa\x04\xB9a\t@V[` \x02` \x01\x01Q_\x01Qa\x02nV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15a\x058W\x82\x82\x81Q\x81\x10a\x04\xF4Wa\x04\xF4a\t@V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x03LV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01a\x04\x9AV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\x90Wa\x05\x90a\x05YV[`@R\x90V[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\x90Wa\x05\x90a\x05YV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\xE2Wa\x05\xE2a\x05YV[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\x05\xFAW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x06\x1DWa\x06\x1Da\x05YV[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`\x80\x82\x84\x03\x12\x15a\x06^W_\x80\xFD[a\x06h\x83\x83a\x05\xEAV[\x93\x92PPPV[_g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x06\x88Wa\x06\x88a\x05YV[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\x06\xA1W_\x80\xFD[\x815` a\x06\xB6a\x06\xB1\x83a\x06oV[a\x05\xB9V[\x82\x81R`\xE0\x92\x83\x02\x85\x01\x82\x01\x92\x82\x82\x01\x91\x90\x87\x85\x11\x15a\x06\xD4W_\x80\xFD[\x83\x87\x01[\x85\x81\x10\x15a\x07XW\x80\x89\x03\x82\x81\x12\x15a\x06\xEFW_\x80\xFD[a\x06\xF7a\x05mV[a\x07\x01\x8B\x84a\x05\xEAV[\x81R`@\x80`\x7F\x19\x84\x01\x12\x15a\x07\x15W_\x80\xFD[a\x07\x1Da\x05\x96V[`\x80\x85\x015\x81R`\xA0\x85\x015\x89\x82\x01R\x82\x89\x01R`\xC0\x84\x015\x92P\x82\x15\x15\x83\x14a\x07EW_\x80\xFD[\x81\x01\x91\x90\x91R\x84R\x92\x84\x01\x92\x81\x01a\x06\xD8V[P\x90\x97\x96PPPPPPPV[_\x80`@\x83\x85\x03\x12\x15a\x07vW_\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x07\x8DW_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12a\x07\xA0W_\x80\xFD[\x815` a\x07\xB0a\x06\xB1\x83a\x06oV[\x80\x83\x82R` \x82\x01\x91P` \x84`\x07\x1B\x87\x01\x01\x93P\x89\x84\x11\x15a\x07\xD1W_\x80\xFD[` \x86\x01\x95P[\x83\x86\x10\x15a\x07\xFCWa\x07\xEA\x8A\x87a\x05\xEAV[\x82R\x82\x82\x01\x91P`\x80\x86\x01\x95Pa\x07\xD8V[\x96PPPP` \x85\x015\x91P\x80\x82\x11\x15a\x08\x14W_\x80\xFD[Pa\x08!\x85\x82\x86\x01a\x06\x92V[\x91PP\x92P\x92\x90PV[_` \x82\x84\x03\x12\x15a\x08;W_\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06hW_\x80\xFD[`@\x80\x82R\x83Q\x82\x82\x01\x81\x90R_\x91\x90` \x90``\x85\x01\x90\x82\x88\x01\x85[\x82\x81\x10\x15a\x08\xB5Wa\x08\xA2\x84\x83Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[`\x80\x93\x90\x93\x01\x92\x90\x84\x01\x90`\x01\x01a\x08nV[PPP\x84\x81\x03\x82\x86\x01R\x85Q\x80\x82R\x86\x83\x01\x91\x83\x01\x90_[\x81\x81\x10\x15a\t2W\x83Qa\t\x03\x84\x82Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x80\x86\x01Q\x80Q`\x80\x86\x01R\x86\x01Q`\xA0\x85\x01R\x86\x01Q\x15\x15`\xC0\x84\x01R\x92\x84\x01\x92`\xE0\x90\x92\x01\x91`\x01\x01a\x08\xCDV[P\x90\x98\x97PPPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x17\0\n";
    /// The bytecode of the contract.
    pub static PERMISSIONEDSTAKETABLE_BYTECODE: ::ethers::core::types::Bytes =
        ::ethers::core::types::Bytes::from_static(__BYTECODE);
    #[rustfmt::skip]
    const __DEPLOYED_BYTECODE: &[u8] = b"`\x80`@R4\x80\x15a\0\x0FW_\x80\xFD[P`\x046\x10a\0\x85W_5`\xE0\x1C\x80c\x8D\xA5\xCB[\x11a\0XW\x80c\x8D\xA5\xCB[\x14a\0\xDAW\x80c\x9B0\xA5\xE6\x14a\0\xF4W\x80c\xCB\xBA|x\x14a\x01\x07W\x80c\xF2\xFD\xE3\x8B\x14a\x01\x1AW_\x80\xFD[\x80c>\x9D\xF9\xB5\x14a\0\x89W\x80cqP\x18\xA6\x14a\0\xA5W\x80cu\xD7\x05\xE9\x14a\0\xAFW\x80c\x81)\xFC\x1C\x14a\0\xD2W[_\x80\xFD[a\0\x92`\x01T\x81V[`@Q\x90\x81R` \x01[`@Q\x80\x91\x03\x90\xF3[a\0\xADa\x01-V[\0[a\0\xC2a\0\xBD6`\x04a\x06NV[a\x01@V[`@Q\x90\x15\x15\x81R` \x01a\0\x9CV[a\0\xADa\x01fV[_T`@Q`\x01`\x01`\xA0\x1B\x03\x90\x91\x16\x81R` \x01a\0\x9CV[a\0\x92a\x01\x026`\x04a\x06NV[a\x02nV[a\0\xADa\x01\x156`\x04a\x07eV[a\x02\xC8V[a\0\xADa\x01(6`\x04a\x08+V[a\x03\x1FV[a\x015a\x03aV[a\x01>_a\x03\x8DV[V[_`\x02_a\x01M\x84a\x02nV[\x81R` \x81\x01\x91\x90\x91R`@\x01_ T`\xFF\x16\x92\x91PPV[\x7F\xF0\xC5~\x16\x84\r\xF0@\xF1P\x88\xDC/\x81\xFE9\x1C9#\xBE\xC7>#\xA9f.\xFC\x9C\"\x9Cj\0\x80T`\x01`@\x1B\x81\x04`\xFF\x16\x15\x90g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16_\x81\x15\x80\x15a\x01\xABWP\x82[\x90P_\x82g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x16`\x01\x14\x80\x15a\x01\xC7WP0;\x15[\x90P\x81\x15\x80\x15a\x01\xD5WP\x80\x15[\x15a\x01\xF3W`@Qc\xF9.\xE8\xA9`\xE0\x1B\x81R`\x04\x01`@Q\x80\x91\x03\x90\xFD[\x84Tg\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x19\x16`\x01\x17\x85U\x83\x15a\x02\x1DW\x84T`\xFF`@\x1B\x19\x16`\x01`@\x1B\x17\x85U[C`\x01U\x83\x15a\x02gW\x84T`\xFF`@\x1B\x19\x16\x85U`@Q`\x01\x81R\x7F\xC7\xF5\x05\xB2\xF3q\xAE!u\xEEI\x13\xF4I\x9E\x1F&3\xA7\xB5\x93c!\xEE\xD1\xCD\xAE\xB6\x11Q\x81\xD2\x90` \x01`@Q\x80\x91\x03\x90\xA1[PPPPPV[_\x81_\x01Q\x82` \x01Q\x83`@\x01Q\x84``\x01Q`@Q` \x01a\x02\xAB\x94\x93\x92\x91\x90\x93\x84R` \x84\x01\x92\x90\x92R`@\x83\x01R``\x82\x01R`\x80\x01\x90V[`@Q` \x81\x83\x03\x03\x81R\x90`@R\x80Q\x90` \x01 \x90P\x91\x90PV[a\x02\xD0a\x03aV[a\x02\xD9\x82a\x03\xDCV[a\x02\xE2\x81a\x04\x98V[\x7FK\xB3\x1C\xD9\xAE\x87\xA3\xF8(\x95X\xC7\x9B\xE5#%\x06\xCE6\xDEBi\xC8\xBAE4\xA6\xFB\xE9\xE2\x96]\x82\x82`@Qa\x03\x13\x92\x91\x90a\x08QV[`@Q\x80\x91\x03\x90\xA1PPV[a\x03'a\x03aV[`\x01`\x01`\xA0\x1B\x03\x81\x16a\x03UW`@Qc\x1EO\xBD\xF7`\xE0\x1B\x81R_`\x04\x82\x01R`$\x01[`@Q\x80\x91\x03\x90\xFD[a\x03^\x81a\x03\x8DV[PV[_T`\x01`\x01`\xA0\x1B\x03\x163\x14a\x01>W`@Qc\x11\x8C\xDA\xA7`\xE0\x1B\x81R3`\x04\x82\x01R`$\x01a\x03LV[_\x80T`\x01`\x01`\xA0\x1B\x03\x83\x81\x16`\x01`\x01`\xA0\x1B\x03\x19\x83\x16\x81\x17\x84U`@Q\x91\x90\x92\x16\x92\x83\x91\x7F\x8B\xE0\x07\x9CS\x16Y\x14\x13D\xCD\x1F\xD0\xA4\xF2\x84\x19I\x7F\x97\"\xA3\xDA\xAF\xE3\xB4\x18okdW\xE0\x91\x90\xA3PPV[_[\x81Q\x81\x10\x15a\x04\x94W_a\x04\n\x83\x83\x81Q\x81\x10a\x03\xFDWa\x03\xFDa\t@V[` \x02` \x01\x01Qa\x02nV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16a\x04wW\x82\x82\x81Q\x81\x10a\x044Wa\x044a\t@V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01Q`@\x80Qc4\xA7V\x1F`\xE0\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x03LV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16\x90U`\x01\x01a\x03\xDEV[PPV[_[\x81Q\x81\x10\x15a\x04\x94W_a\x04\xC9\x83\x83\x81Q\x81\x10a\x04\xB9Wa\x04\xB9a\t@V[` \x02` \x01\x01Q_\x01Qa\x02nV[_\x81\x81R`\x02` R`@\x90 T\x90\x91P`\xFF\x16\x15a\x058W\x82\x82\x81Q\x81\x10a\x04\xF4Wa\x04\xF4a\t@V[` \x90\x81\x02\x91\x90\x91\x01\x81\x01QQ`@\x80Qc\x1B\x06\xE1A`\xE1\x1B\x81R\x82Q`\x04\x82\x01R\x92\x82\x01Q`$\x84\x01R\x81\x01Q`D\x83\x01R``\x01Q`d\x82\x01R`\x84\x01a\x03LV[_\x90\x81R`\x02` R`@\x90 \x80T`\xFF\x19\x16`\x01\x90\x81\x17\x90\x91U\x01a\x04\x9AV[cNH{q`\xE0\x1B_R`A`\x04R`$_\xFD[`@Q``\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\x90Wa\x05\x90a\x05YV[`@R\x90V[`@\x80Q\x90\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\x90Wa\x05\x90a\x05YV[`@Q`\x1F\x82\x01`\x1F\x19\x16\x81\x01g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x81\x11\x82\x82\x10\x17\x15a\x05\xE2Wa\x05\xE2a\x05YV[`@R\x91\x90PV[_`\x80\x82\x84\x03\x12\x15a\x05\xFAW_\x80\xFD[`@Q`\x80\x81\x01\x81\x81\x10g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x17\x15a\x06\x1DWa\x06\x1Da\x05YV[\x80`@RP\x80\x91P\x825\x81R` \x83\x015` \x82\x01R`@\x83\x015`@\x82\x01R``\x83\x015``\x82\x01RP\x92\x91PPV[_`\x80\x82\x84\x03\x12\x15a\x06^W_\x80\xFD[a\x06h\x83\x83a\x05\xEAV[\x93\x92PPPV[_g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x82\x11\x15a\x06\x88Wa\x06\x88a\x05YV[P`\x05\x1B` \x01\x90V[_\x82`\x1F\x83\x01\x12a\x06\xA1W_\x80\xFD[\x815` a\x06\xB6a\x06\xB1\x83a\x06oV[a\x05\xB9V[\x82\x81R`\xE0\x92\x83\x02\x85\x01\x82\x01\x92\x82\x82\x01\x91\x90\x87\x85\x11\x15a\x06\xD4W_\x80\xFD[\x83\x87\x01[\x85\x81\x10\x15a\x07XW\x80\x89\x03\x82\x81\x12\x15a\x06\xEFW_\x80\xFD[a\x06\xF7a\x05mV[a\x07\x01\x8B\x84a\x05\xEAV[\x81R`@\x80`\x7F\x19\x84\x01\x12\x15a\x07\x15W_\x80\xFD[a\x07\x1Da\x05\x96V[`\x80\x85\x015\x81R`\xA0\x85\x015\x89\x82\x01R\x82\x89\x01R`\xC0\x84\x015\x92P\x82\x15\x15\x83\x14a\x07EW_\x80\xFD[\x81\x01\x91\x90\x91R\x84R\x92\x84\x01\x92\x81\x01a\x06\xD8V[P\x90\x97\x96PPPPPPPV[_\x80`@\x83\x85\x03\x12\x15a\x07vW_\x80\xFD[\x825g\xFF\xFF\xFF\xFF\xFF\xFF\xFF\xFF\x80\x82\x11\x15a\x07\x8DW_\x80\xFD[\x81\x85\x01\x91P\x85`\x1F\x83\x01\x12a\x07\xA0W_\x80\xFD[\x815` a\x07\xB0a\x06\xB1\x83a\x06oV[\x80\x83\x82R` \x82\x01\x91P` \x84`\x07\x1B\x87\x01\x01\x93P\x89\x84\x11\x15a\x07\xD1W_\x80\xFD[` \x86\x01\x95P[\x83\x86\x10\x15a\x07\xFCWa\x07\xEA\x8A\x87a\x05\xEAV[\x82R\x82\x82\x01\x91P`\x80\x86\x01\x95Pa\x07\xD8V[\x96PPPP` \x85\x015\x91P\x80\x82\x11\x15a\x08\x14W_\x80\xFD[Pa\x08!\x85\x82\x86\x01a\x06\x92V[\x91PP\x92P\x92\x90PV[_` \x82\x84\x03\x12\x15a\x08;W_\x80\xFD[\x815`\x01`\x01`\xA0\x1B\x03\x81\x16\x81\x14a\x06hW_\x80\xFD[`@\x80\x82R\x83Q\x82\x82\x01\x81\x90R_\x91\x90` \x90``\x85\x01\x90\x82\x88\x01\x85[\x82\x81\x10\x15a\x08\xB5Wa\x08\xA2\x84\x83Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[`\x80\x93\x90\x93\x01\x92\x90\x84\x01\x90`\x01\x01a\x08nV[PPP\x84\x81\x03\x82\x86\x01R\x85Q\x80\x82R\x86\x83\x01\x91\x83\x01\x90_[\x81\x81\x10\x15a\t2W\x83Qa\t\x03\x84\x82Q\x80Q\x82R` \x81\x01Q` \x83\x01R`@\x81\x01Q`@\x83\x01R``\x81\x01Q``\x83\x01RPPV[\x80\x86\x01Q\x80Q`\x80\x86\x01R\x86\x01Q`\xA0\x85\x01R\x86\x01Q\x15\x15`\xC0\x84\x01R\x92\x84\x01\x92`\xE0\x90\x92\x01\x91`\x01\x01a\x08\xCDV[P\x90\x98\x97PPPPPPPPV[cNH{q`\xE0\x1B_R`2`\x04R`$_\xFD\xFE\xA1dsolcC\0\x08\x17\0\n";
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
        Initialize(InitializeCall),
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
            if let Ok(decoded) = <InitializeCall as ::ethers::core::abi::AbiDecode>::decode(data) {
                return Ok(Self::Initialize(decoded));
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
                Self::Initialize(element) => ::ethers::core::abi::AbiEncode::encode(element),
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
                Self::Initialize(element) => ::core::fmt::Display::fmt(element, f),
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
    impl ::core::convert::From<InitializeCall> for PermissionedStakeTableCalls {
        fn from(value: InitializeCall) -> Self {
            Self::Initialize(value)
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
